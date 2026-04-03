use crate::github::{GitHubClient, GitHubRelease, GitHubReleaseAsset};
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use std::fs;
use std::io::Cursor;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use url::Url;
use xz2::read::XzDecoder;
use zip::ZipArchive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTypePreference {
    Any,
    Archive,
    Binary,
}

#[derive(Debug, Clone)]
pub struct ReleaseRequest {
    pub repo: String,
    pub tag: Option<String>,
    pub include_prerelease: bool,
    pub asset_regex: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub file_type: FileTypePreference,
    pub extract: bool,
    pub output_path: Option<String>,
    pub cwd: bool,
    pub bin_path: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReleaseDownloadResult {
    pub owner: String,
    pub repo: String,
    pub tag: String,
    pub asset_name: String,
    pub download_path: String,
    pub installed_binary: Option<String>,
    pub extracted: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedRepo {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Release selection cancelled")]
pub struct ReleaseSelectionCancelled;

pub async fn download_release(request: ReleaseRequest) -> Result<ReleaseDownloadResult> {
    let parsed = parse_repo_reference(&request.repo)?;
    let client = GitHubClient::new(request.token.clone())?;
    let releases = client
        .fetch_releases(&parsed.owner, &parsed.repo)
        .await
        .context("Failed to fetch releases")?;

    let release = select_release(
        &releases,
        request.tag.as_deref(),
        request.include_prerelease,
    )?;
    let asset = select_asset(
        &release.assets,
        &parsed.repo,
        request.asset_regex.as_deref(),
        request.os.as_deref(),
        request.arch.as_deref(),
        request.file_type,
    )?;

    let base_dir = resolve_base_dir(request.output_path.clone(), request.cwd)?;
    fs::create_dir_all(&base_dir)?;

    let download_path = base_dir.join(&asset.name);
    let bytes = client
        .fetch_bytes(&asset.browser_download_url)
        .await
        .with_context(|| format!("Failed to download asset '{}'", asset.name))?;
    fs::write(&download_path, bytes.as_slice())
        .with_context(|| format!("Failed to write asset to '{}'", download_path.display()))?;

    let mut installed_binary = None;
    let extracted = request.extract && is_archive(&asset.name);

    if extracted {
        extract_archive(&download_path, &base_dir)?;
        if let Some(bin_dir) = request.bin_path.as_deref() {
            let installed = install_best_binary(&base_dir, &PathBuf::from(bin_dir), &parsed.repo)?;
            installed_binary = Some(installed.display().to_string());
        }
    } else if let Some(bin_dir) = request.bin_path.as_deref() {
        let target = install_file(&download_path, &PathBuf::from(bin_dir))?;
        installed_binary = Some(target.display().to_string());
    }

    Ok(ReleaseDownloadResult {
        owner: parsed.owner,
        repo: parsed.repo,
        tag: release.tag_name.clone(),
        asset_name: asset.name.clone(),
        download_path: download_path.display().to_string(),
        installed_binary,
        extracted,
    })
}

pub fn parse_repo_reference(value: &str) -> Result<ParsedRepo> {
    if value.starts_with("https://") || value.starts_with("http://") {
        let url = Url::parse(value).context("Invalid repository URL")?;
        if url.host_str() != Some("github.com") {
            bail!("Repository URL must point to github.com");
        }

        let parts: Vec<_> = url
            .path_segments()
            .ok_or_else(|| anyhow!("Invalid repository URL path"))?
            .filter(|segment| !segment.is_empty())
            .collect();
        if parts.len() < 2 {
            bail!("Repository URL must include owner and repository");
        }

        return Ok(ParsedRepo {
            owner: parts[0].to_string(),
            repo: parts[1].trim_end_matches(".git").to_string(),
        });
    }

    let mut parts = value.split('/');
    let owner = parts.next().unwrap_or_default().trim();
    let repo = parts.next().unwrap_or_default().trim();
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        bail!("Repository must be in the form owner/repo or a GitHub URL");
    }

    Ok(ParsedRepo {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

pub fn select_asset_name_for_request(
    assets: &[GitHubReleaseAsset],
    repo: &str,
    asset_regex: Option<&str>,
    os: Option<&str>,
    arch: Option<&str>,
    file_type: FileTypePreference,
) -> Result<String> {
    Ok(
        select_asset(assets, repo, asset_regex, os, arch, file_type)?
            .name
            .clone(),
    )
}

fn select_release<'a>(
    releases: &'a [GitHubRelease],
    tag: Option<&str>,
    include_prerelease: bool,
) -> Result<&'a GitHubRelease> {
    if releases.is_empty() {
        bail!("No releases found for this repository");
    }

    if let Some(tag) = tag {
        return releases
            .iter()
            .find(|release| release.tag_name == tag)
            .ok_or_else(|| anyhow!("Release tag '{}' was not found", tag));
    }

    releases
        .iter()
        .find(|release| !release.draft && (include_prerelease || !release.prerelease))
        .ok_or_else(|| anyhow!("No matching release found"))
}

fn select_asset<'a>(
    assets: &'a [GitHubReleaseAsset],
    repo: &str,
    asset_regex: Option<&str>,
    os: Option<&str>,
    arch: Option<&str>,
    file_type: FileTypePreference,
) -> Result<&'a GitHubReleaseAsset> {
    if assets.is_empty() {
        bail!("The selected release has no downloadable assets");
    }

    let regex = asset_regex
        .map(Regex::new)
        .transpose()
        .context("Invalid asset regex")?;
    let detected_os = normalize_os(os.unwrap_or(std::env::consts::OS));
    let detected_arch = normalize_arch(arch.unwrap_or(std::env::consts::ARCH));
    let candidates: Vec<_> = assets
        .iter()
        .filter(|asset| !looks_like_auxiliary(&asset.name))
        .filter(|asset| regex.as_ref().is_none_or(|re| re.is_match(&asset.name)))
        .map(|asset| {
            (
                asset,
                score_asset(&asset.name, repo, detected_os, detected_arch, file_type),
            )
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    let mut candidates = candidates;
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name.cmp(&b.0.name)));

    if candidates.is_empty() {
        bail!("No release asset matched the requested criteria");
    }

    if candidates.len() == 1 {
        return Ok(candidates[0].0);
    }

    if asset_regex.is_some() || has_clear_best_match(&candidates) {
        return Ok(candidates[0].0);
    }

    prompt_for_asset_selection(&candidates)
}

fn has_clear_best_match(candidates: &[(&GitHubReleaseAsset, i32)]) -> bool {
    if candidates.len() < 2 {
        return true;
    }
    candidates[0].1 >= candidates[1].1 + 20
}

fn prompt_for_asset_selection<'a>(
    candidates: &[(&'a GitHubReleaseAsset, i32)],
) -> Result<&'a GitHubReleaseAsset> {
    println!("Multiple release assets matched. Select one:");
    for (index, (asset, _)) in candidates.iter().enumerate() {
        println!(
            "  {}. {} ({})",
            index + 1,
            asset.name,
            format_size(asset.size)
        );
    }
    println!("Type q and press Enter to cancel.");

    loop {
        print!("Enter a number [1-{}]: ", candidates.len());
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read selection")?;

        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("q") {
            return Err(ReleaseSelectionCancelled.into());
        }

        let Ok(choice) = trimmed.parse::<usize>() else {
            eprintln!("Invalid selection '{}'. Enter a number.", trimmed);
            continue;
        };

        if (1..=candidates.len()).contains(&choice) {
            return Ok(candidates[choice - 1].0);
        }

        eprintln!(
            "Selection out of range. Choose between 1 and {}.",
            candidates.len()
        );
    }
}

fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let size_f = size as f64;

    if size_f >= GB {
        format!("{:.1} GB", size_f / GB)
    } else if size_f >= MB {
        format!("{:.1} MB", size_f / MB)
    } else if size_f >= KB {
        format!("{:.1} KB", size_f / KB)
    } else {
        format!("{} B", size)
    }
}

fn normalize_os(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "windows" | "win32" | "win64" | "pc-windows-msvc" => "windows",
        "macos" | "darwin" | "osx" | "apple-darwin" => "darwin",
        _ => "linux",
    }
}

fn normalize_arch(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" => "amd64",
        "aarch64" | "arm64" => "arm64",
        "x86" | "i386" | "i686" => "386",
        other if other.starts_with("armv7") => "armv7",
        _ => "amd64",
    }
}

fn score_asset(
    name: &str,
    repo: &str,
    detected_os: &str,
    detected_arch: &str,
    file_type: FileTypePreference,
) -> i32 {
    let lower = name.to_ascii_lowercase();
    let mut score = 0;

    if lower.contains(&repo.to_ascii_lowercase()) {
        score += 5;
    }
    if matches_os(&lower, detected_os) {
        score += 40;
    } else {
        score -= 40;
    }
    if matches_arch(&lower, detected_arch) {
        score += 30;
    } else {
        score -= 30;
    }
    if matches_file_type(&lower, file_type) {
        score += 25;
    } else if !matches!(file_type, FileTypePreference::Any) {
        score -= 25;
    }
    if detected_os == "windows" && lower.ends_with(".exe") {
        score += 20;
    }
    if detected_os != "windows" && !lower.ends_with(".exe") {
        score += 10;
    }
    if is_archive(&lower) {
        score += 5;
    }

    score
}

fn matches_os(name: &str, detected_os: &str) -> bool {
    match detected_os {
        "windows" => ["windows", "win32", "win64", "pc-windows"]
            .iter()
            .any(|t| name.contains(t)),
        "darwin" => ["darwin", "macos", "apple-darwin", "osx"]
            .iter()
            .any(|t| name.contains(t)),
        _ => ["linux", "unknown-linux", "gnu", "musl"]
            .iter()
            .any(|t| name.contains(t)),
    }
}

fn matches_arch(name: &str, detected_arch: &str) -> bool {
    match detected_arch {
        "arm64" => ["arm64", "aarch64"].iter().any(|t| name.contains(t)),
        "386" => ["386", "i386", "i686", "x86"]
            .iter()
            .any(|t| name.contains(t)),
        "armv7" => ["armv7", "armv7l"].iter().any(|t| name.contains(t)),
        _ => ["amd64", "x86_64", "x64"].iter().any(|t| name.contains(t)),
    }
}

fn matches_file_type(name: &str, file_type: FileTypePreference) -> bool {
    match file_type {
        FileTypePreference::Any => true,
        FileTypePreference::Archive => is_archive(name),
        FileTypePreference::Binary => !is_archive(name),
    }
}

fn looks_like_auxiliary(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        ".sha256",
        ".sha512",
        ".sig",
        ".asc",
        "checksums",
        "checksum",
        "sbom",
    ]
    .iter()
    .any(|part| lower.contains(part))
}

fn is_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".zip")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.xz")
}

fn resolve_base_dir(output_path: Option<String>, cwd: bool) -> Result<PathBuf> {
    if cwd {
        std::env::current_dir().map_err(Into::into)
    } else if let Some(path) = output_path {
        Ok(PathBuf::from(path))
    } else {
        dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .ok_or_else(|| anyhow!("Could not find User Downloads directory"))
    }
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let bytes = fs::read(archive_path)?;
    let name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if name.ends_with(".zip") {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).context("Failed to read zip archive")?;
        for index in 0..archive.len() {
            let mut file = archive
                .by_index(index)
                .context("Failed to read zip entry")?;
            let out_path = destination.join(file.mangled_name());
            if file.is_dir() {
                fs::create_dir_all(&out_path)?;
                continue;
            }
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
        return Ok(());
    }

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let cursor = Cursor::new(bytes);
        let decoder = flate2::read::GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        archive.unpack(destination)?;
        return Ok(());
    }

    if name.ends_with(".tar.xz") {
        let cursor = Cursor::new(bytes);
        let decoder = XzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        archive.unpack(destination)?;
        return Ok(());
    }

    bail!("Unsupported archive type for '{}'", archive_path.display())
}

fn install_best_binary(extract_root: &Path, bin_dir: &Path, repo: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    collect_files(extract_root, &mut candidates)?;
    let selected = candidates
        .into_iter()
        .filter(|path| is_probable_binary(path))
        .max_by_key(|path| binary_score(path, repo))
        .ok_or_else(|| anyhow!("No installable binary found after extraction"))?;

    install_file(&selected, bin_dir)
}

fn install_file(source: &Path, bin_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(bin_dir)?;
    let target = bin_dir.join(
        source
            .file_name()
            .ok_or_else(|| anyhow!("Could not determine target filename"))?,
    );
    fs::copy(source, &target).with_context(|| {
        format!(
            "Failed to copy '{}' to '{}'",
            source.display(),
            target.display()
        )
    })?;
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target, perms)?;
    }
    Ok(target)
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn is_probable_binary(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|file| file.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.is_empty() {
        return false;
    }
    if ["readme", "license", "changelog"]
        .iter()
        .any(|p| name.starts_with(p))
    {
        return false;
    }
    if cfg!(windows) {
        return name.ends_with(".exe");
    }
    !name.contains(".so") && !name.contains(".dylib")
}

fn binary_score(path: &Path, repo: &str) -> i32 {
    let name = path
        .file_name()
        .and_then(|file| file.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut score = 0;
    if name == repo.to_ascii_lowercase() || name.starts_with(&repo.to_ascii_lowercase()) {
        score += 50;
    }
    if cfg!(windows) && name.ends_with(".exe") {
        score += 20;
    }
    score - path.components().count() as i32
}
