use ghgrab::github::GitHubReleaseAsset;
use ghgrab::release::{parse_repo_reference, select_asset_name_for_request, FileTypePreference};

fn asset(name: &str) -> GitHubReleaseAsset {
    GitHubReleaseAsset {
        name: name.to_string(),
        browser_download_url: format!("https://example.com/{}", name),
        content_type: None,
        size: 10,
    }
}

#[test]
fn parses_owner_repo_reference() {
    let parsed = parse_repo_reference("owner/repo").unwrap();
    assert_eq!(parsed.owner, "owner");
    assert_eq!(parsed.repo, "repo");
}

#[test]
fn parses_github_url_reference() {
    let parsed = parse_repo_reference("https://github.com/owner/repo").unwrap();
    assert_eq!(parsed.owner, "owner");
    assert_eq!(parsed.repo, "repo");
}

#[test]
fn rejects_invalid_repo_reference() {
    let error = parse_repo_reference("not-valid").unwrap_err().to_string();
    assert!(error.contains("owner/repo"));
}

#[test]
fn selects_best_matching_asset_for_os_and_arch() {
    let assets = vec![
        asset("tool_darwin_arm64.tar.gz"),
        asset("tool_linux_x86_64.tar.gz"),
    ];

    let selected = select_asset_name_for_request(
        &assets,
        "tool",
        None,
        Some("linux"),
        Some("amd64"),
        FileTypePreference::Archive,
    )
    .unwrap();

    assert_eq!(selected, "tool_linux_x86_64.tar.gz");
}

#[test]
fn filters_auxiliary_assets() {
    let assets = vec![
        asset("tool_checksums.txt"),
        asset("tool_linux_x86_64.tar.gz"),
    ];

    let selected = select_asset_name_for_request(
        &assets,
        "tool",
        Some("linux"),
        Some("linux"),
        Some("amd64"),
        FileTypePreference::Archive,
    )
    .unwrap();

    assert_eq!(selected, "tool_linux_x86_64.tar.gz");
}

#[test]
fn regex_can_force_specific_asset() {
    let assets = vec![
        asset("tool_windows_amd64.zip"),
        asset("tool_linux_x86_64.tar.gz"),
    ];

    let selected = select_asset_name_for_request(
        &assets,
        "tool",
        Some("windows"),
        Some("linux"),
        Some("amd64"),
        FileTypePreference::Any,
    )
    .unwrap();

    assert_eq!(selected, "tool_windows_amd64.zip");
}

#[test]
fn arch_override_prefers_arm64_asset() {
    let assets = vec![
        asset("tool_linux_x86_64.tar.gz"),
        asset("tool_linux_aarch64.tar.gz"),
    ];

    let selected = select_asset_name_for_request(
        &assets,
        "tool",
        None,
        Some("linux"),
        Some("arm64"),
        FileTypePreference::Archive,
    )
    .unwrap();

    assert_eq!(selected, "tool_linux_aarch64.tar.gz");
}

#[test]
fn file_type_binary_prefers_non_archive() {
    let assets = vec![
        asset("tool_windows_amd64.zip"),
        asset("tool_windows_amd64.exe"),
    ];

    let selected = select_asset_name_for_request(
        &assets,
        "tool",
        None,
        Some("windows"),
        Some("amd64"),
        FileTypePreference::Binary,
    )
    .unwrap();

    assert_eq!(selected, "tool_windows_amd64.exe");
}
