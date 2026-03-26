use crate::download::Downloader;
use crate::github::{GitHubClient, GitHubError, GitHubUrl, RepoItem};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::PathBuf;

pub const AGENT_API_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize)]
pub struct AgentTreeEntry {
    pub path: String,
    pub kind: String,
    pub size: Option<u64>,
    pub download_url: Option<String>,
    pub is_lfs: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTreeResponse {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub truncated: bool,
    pub entries: Vec<AgentTreeEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentDownloadResponse {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub output_dir: String,
    pub downloaded_paths: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentEnvelope<T> {
    pub api_version: &'static str,
    pub ok: bool,
    pub command: String,
    pub data: Option<T>,
    pub error: Option<AgentErrorResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentErrorResponse {
    pub code: String,
    pub message: String,
}

impl<T> AgentEnvelope<T> {
    pub fn success(command: impl Into<String>, data: T) -> Self {
        Self {
            api_version: AGENT_API_VERSION,
            ok: true,
            command: command.into(),
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        command: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            api_version: AGENT_API_VERSION,
            ok: false,
            command: command.into(),
            data: None,
            error: Some(AgentErrorResponse {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

pub async fn fetch_tree(url: &str, token: Option<String>) -> Result<AgentTreeResponse> {
    let gh_url = GitHubUrl::parse(url)?;
    let client = GitHubClient::new(token.clone())?;
    let (gh_url, entries, truncated) = load_tree(&client, gh_url).await?;

    Ok(AgentTreeResponse {
        owner: gh_url.owner,
        repo: gh_url.repo,
        branch: gh_url.branch,
        path: gh_url.path,
        truncated,
        entries: entries
            .into_iter()
            .map(|item| AgentTreeEntry {
                size: item.actual_size(),
                download_url: item.actual_download_url().cloned(),
                is_lfs: item.is_lfs(),
                path: item.path,
                kind: item.item_type,
            })
            .collect(),
    })
}

pub async fn download_paths(
    url: &str,
    token: Option<String>,
    selected_paths: &[String],
    output_path: Option<String>,
    cwd: bool,
    no_folder: bool,
) -> Result<AgentDownloadResponse> {
    let gh_url = GitHubUrl::parse(url)?;
    let client = GitHubClient::new(token.clone())?;
    let (gh_url, entries, truncated) = load_tree(&client, gh_url).await?;

    let items_to_download = if truncated {
        resolve_requested_items_with_fallback(&client, &gh_url, &entries, selected_paths).await?
    } else {
        resolve_requested_items(&entries, selected_paths)?
    };
    let repo_name = gh_url.repo.clone();

    let base_dir = resolve_base_dir(output_path, cwd)?;
    let download_dir = if no_folder {
        base_dir
    } else {
        base_dir.join(&repo_name)
    };

    let downloader = Downloader::new(download_dir.clone(), token)?;
    let errors = downloader
        .download_items(&items_to_download, "", |_| {})
        .await?;

    Ok(AgentDownloadResponse {
        owner: gh_url.owner,
        repo: gh_url.repo,
        branch: gh_url.branch,
        output_dir: download_dir.display().to_string(),
        downloaded_paths: items_to_download
            .iter()
            .map(|item| item.path.clone())
            .collect(),
        errors,
    })
}

async fn resolve_requested_items_with_fallback(
    client: &GitHubClient,
    gh_url: &GitHubUrl,
    entries: &[RepoItem],
    selected_paths: &[String],
) -> Result<Vec<RepoItem>> {
    if selected_paths.is_empty() {
        return fetch_path_items_recursive(client, gh_url, "").await;
    }

    let mut results = Vec::new();
    for selected_path in selected_paths {
        let normalized = normalize_requested_path(selected_path);

        if let Some(item) = entries.iter().find(|item| item.path == normalized) {
            if item.is_file() {
                results.push(item.clone());
            } else {
                let mut nested = fetch_path_items_recursive(client, gh_url, &normalized).await?;
                results.append(&mut nested);
            }
            continue;
        }

        let mut nested = fetch_path_items_recursive(client, gh_url, &normalized).await?;
        if nested.is_empty() {
            return Err(anyhow!(
                "Path '{}' was not found in the repository tree",
                normalized
            ));
        }
        results.append(&mut nested);
    }

    results.sort_by(|a, b| a.path.cmp(&b.path));
    results.dedup_by(|a, b| a.path == b.path);
    Ok(results)
}

pub fn classify_error(error: &anyhow::Error) -> &'static str {
    if let Some(gh_error) = error.downcast_ref::<GitHubError>() {
        return match gh_error {
            GitHubError::InvalidToken => "invalid_token",
            GitHubError::RateLimitReached(_) => "rate_limit",
            GitHubError::NotFound(_) => "not_found",
            GitHubError::ApiError(_) => "github_api_error",
            GitHubError::Other(_) => "internal_error",
        };
    }

    let message = error.to_string().to_lowercase();
    if message.contains("invalid url") || message.contains("not a github url") {
        "invalid_url"
    } else if message.contains("cannot be combined") {
        "invalid_arguments"
    } else if message.contains("was not found") || message.contains("not found") {
        "not_found"
    } else if message.contains("downloads directory") {
        "output_path_error"
    } else {
        "internal_error"
    }
}

async fn load_tree(
    client: &GitHubClient,
    mut gh_url: GitHubUrl,
) -> Result<(GitHubUrl, Vec<RepoItem>, bool)> {
    let mut tree_result = client
        .fetch_recursive_tree(&gh_url.owner, &gh_url.repo, &gh_url.branch)
        .await;

    if let Err(GitHubError::NotFound(_)) = &tree_result {
        if gh_url.branch == "main" {
            // Try to detect the actual default branch from the API
            if let Ok(default_branch) = client
                .fetch_default_branch(&gh_url.owner, &gh_url.repo)
                .await
            {
                if default_branch != "main" {
                    gh_url.branch = default_branch;
                    tree_result = client
                        .fetch_recursive_tree(&gh_url.owner, &gh_url.repo, &gh_url.branch)
                        .await;
                }
            }
        }
    }

    match tree_result {
        Ok(tree_response) => {
            let mut items =
                map_tree_to_items(tree_response, &gh_url.owner, &gh_url.repo, &gh_url.branch);
            client
                .resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo, &gh_url.branch)
                .await;

            if gh_url.path.is_empty() {
                Ok((gh_url, items, false))
            } else {
                let prefix = format!("{}/", gh_url.path);
                let filtered = items
                    .into_iter()
                    .filter(|item| item.path == gh_url.path || item.path.starts_with(&prefix))
                    .collect();
                Ok((gh_url, filtered, false))
            }
        }
        Err(_) => {
            let mut items = client.fetch_contents(&gh_url.api_url()).await?;
            client
                .resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo, &gh_url.branch)
                .await;
            Ok((gh_url, items, true))
        }
    }
}

async fn fetch_path_items_recursive(
    client: &GitHubClient,
    gh_url: &GitHubUrl,
    path: &str,
) -> Result<Vec<RepoItem>> {
    let request_url = contents_api_url(&gh_url.owner, &gh_url.repo, &gh_url.branch, path);
    let mut items = client.fetch_contents(&request_url).await?;
    client
        .resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo, &gh_url.branch)
        .await;

    let mut results = Vec::new();
    for item in items {
        if item.is_file() {
            results.push(item);
        } else {
            let mut nested =
                Box::pin(fetch_path_items_recursive(client, gh_url, &item.path)).await?;
            if nested.is_empty() {
                continue;
            }
            results.append(&mut nested);
        }
    }

    Ok(results)
}

fn contents_api_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    let base = format!("https://api.github.com/repos/{}/{}/contents", owner, repo);
    let normalized = normalize_requested_path(path);
    if normalized.is_empty() {
        format!("{}?ref={}", base, branch)
    } else {
        format!("{}/{}?ref={}", base, normalized, branch)
    }
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

fn resolve_requested_items(items: &[RepoItem], selected_paths: &[String]) -> Result<Vec<RepoItem>> {
    if selected_paths.is_empty() {
        return Ok(items
            .iter()
            .filter(|item| item.is_file())
            .cloned()
            .collect());
    }

    let mut results = Vec::new();

    for requested_path in selected_paths {
        let normalized = normalize_requested_path(requested_path);
        let exact_match = items.iter().find(|item| item.path == normalized).cloned();

        if let Some(item) = exact_match {
            if item.is_file() {
                results.push(item);
            } else {
                let prefix = format!("{}/", item.path);
                let prefix_len = item.path.rfind('/').map(|idx| idx + 1).unwrap_or(0);

                let mut folder_entries: Vec<RepoItem> = items
                    .iter()
                    .filter(|candidate| candidate.is_file() && candidate.path.starts_with(&prefix))
                    .cloned()
                    .map(|mut file_item| {
                        file_item.name = file_item.path[prefix_len..].to_string();
                        file_item
                    })
                    .collect();

                if folder_entries.is_empty() {
                    return Err(anyhow!(
                        "No downloadable files found under '{}'",
                        normalized
                    ));
                }

                results.append(&mut folder_entries);
            }
            continue;
        }

        let prefix = format!("{}/", normalized);
        let mut folder_entries: Vec<RepoItem> = items
            .iter()
            .filter(|candidate| candidate.is_file() && candidate.path.starts_with(&prefix))
            .cloned()
            .map(|mut file_item| {
                file_item.name = file_item.path.clone();
                file_item
            })
            .collect();

        if folder_entries.is_empty() {
            return Err(anyhow!(
                "Path '{}' was not found in the repository tree",
                normalized
            ));
        }

        results.append(&mut folder_entries);
    }

    results.sort_by(|a, b| a.path.cmp(&b.path));
    results.dedup_by(|a, b| a.path == b.path);
    Ok(results)
}

fn normalize_requested_path(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}

fn map_tree_to_items(
    tree: crate::github::GitTreeResponse,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Vec<RepoItem> {
    tree.tree
        .into_iter()
        .map(|entry| {
            let name = entry
                .path
                .split('/')
                .next_back()
                .unwrap_or(&entry.path)
                .to_string();
            let item_type = if entry.entry_type == "tree" {
                "dir".to_string()
            } else {
                "file".to_string()
            };

            let download_url = if item_type == "file" {
                Some(format!(
                    "https://raw.githubusercontent.com/{}/{}/{}/{}",
                    owner, repo, branch, entry.path
                ))
            } else {
                None
            };

            RepoItem {
                name,
                item_type,
                url: format!(
                    "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
                    owner, repo, &entry.path, branch
                ),
                path: entry.path,
                download_url,
                size: entry.size,
                selected: false,
                lfs_oid: None,
                lfs_size: None,
                lfs_download_url: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> RepoItem {
        RepoItem {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            item_type: "file".to_string(),
            path: path.to_string(),
            download_url: Some(format!("https://example.com/{}", path)),
            url: format!("https://example.com/api/{}", path),
            size: Some(10),
            selected: false,
            lfs_oid: None,
            lfs_size: None,
            lfs_download_url: None,
        }
    }

    fn dir(path: &str) -> RepoItem {
        RepoItem {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            item_type: "dir".to_string(),
            path: path.to_string(),
            download_url: None,
            url: format!("https://example.com/api/{}", path),
            size: None,
            selected: false,
            lfs_oid: None,
            lfs_size: None,
            lfs_download_url: None,
        }
    }

    #[test]
    fn resolves_single_file_selection() {
        let items = vec![file("src/main.rs"), file("README.md")];
        let selected = resolve_requested_items(&items, &[String::from("src/main.rs")]).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].path, "src/main.rs");
    }

    #[test]
    fn resolves_directory_selection_to_nested_files() {
        let items = vec![
            dir("src"),
            file("src/main.rs"),
            file("src/lib.rs"),
            file("README.md"),
        ];

        let selected = resolve_requested_items(&items, &[String::from("src")]).unwrap();
        let paths: Vec<String> = selected.into_iter().map(|item| item.path).collect();
        assert_eq!(paths, vec!["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn rejects_missing_selection() {
        let items = vec![file("src/main.rs")];
        let error = resolve_requested_items(&items, &[String::from("docs")]).unwrap_err();
        assert!(error.to_string().contains("was not found"));
    }

    #[test]
    fn normalizes_windows_style_paths() {
        assert_eq!(normalize_requested_path("\\src\\main.rs\\"), "src/main.rs");
    }

    #[test]
    fn builds_contents_api_url_for_root() {
        assert_eq!(
            contents_api_url("o", "r", "main", ""),
            "https://api.github.com/repos/o/r/contents?ref=main"
        );
    }

    #[test]
    fn builds_contents_api_url_for_nested_path() {
        assert_eq!(
            contents_api_url("o", "r", "main", "\\src\\tools\\"),
            "https://api.github.com/repos/o/r/contents/src/tools?ref=main"
        );
    }

    #[test]
    fn wraps_success_response() {
        let envelope = AgentEnvelope::success("tree", 123usize);
        assert!(envelope.ok);
        assert_eq!(envelope.api_version, AGENT_API_VERSION);
        assert_eq!(envelope.command, "tree");
        assert_eq!(envelope.data, Some(123));
        assert!(envelope.error.is_none());
    }

    #[test]
    fn classifies_invalid_url_errors() {
        let error = anyhow!("Invalid URL format");
        assert_eq!(classify_error(&error), "invalid_url");
    }
}
