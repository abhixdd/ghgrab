use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct GitHubUrl {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub path: String,
}

impl GitHubUrl {
    pub fn parse(url_str: &str) -> Result<Self> {
        let url = Url::parse(url_str).context("Invalid URL format")?;

        if url.host_str() != Some("github.com") {
            return Err(anyhow!("Not a GitHub URL"));
        }

        let path_segments: Vec<&str> = url
            .path_segments()
            .ok_or_else(|| anyhow!("Invalid URL path"))?
            .collect();

        if path_segments.len() < 2 {
            return Err(anyhow!("URL must contain owner and repository"));
        }

        let owner = path_segments[0].to_string();
        let repo = path_segments[1].to_string();

        let (branch, path) = if path_segments.len() >= 4
            && (path_segments[2] == "tree" || path_segments[2] == "blob")
        {
            let branch = path_segments[3].to_string();
            let path = if path_segments.len() > 4 {
                path_segments[4..].join("/")
            } else {
                String::new()
            };
            (branch, path)
        } else {
            ("main".to_string(), String::new())
        };

        Ok(GitHubUrl {
            owner,
            repo,
            branch,
            path,
        })
    }

    pub fn get_local_git_remote() -> Option<String> {
        use std::process::Command;
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
            .ok()?;

        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !url.is_empty() {
                if url.starts_with("git@github.com:") {
                    let path = url
                        .trim_start_matches("git@github.com:")
                        .trim_end_matches(".git");
                    return Some(format!("https://github.com/{}", path));
                }
                if url.contains("github.com") && url.ends_with(".git") {
                    return Some(url.trim_end_matches(".git").to_string());
                }
                return Some(url);
            }
        }
        None
    }

    pub fn api_url(&self) -> String {
        let base = format!(
            "https://api.github.com/repos/{}/{}/contents",
            self.owner, self.repo
        );
        if self.path.is_empty() {
            format!("{}?ref={}", base, self.branch)
        } else {
            format!("{}/{}?ref={}", base, self.path, self.branch)
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RepoItem {
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub path: String,
    pub download_url: Option<String>,
    pub url: String,
    #[allow(dead_code)]
    pub size: Option<u64>,
    #[serde(skip)]
    pub selected: bool,
    #[serde(skip)]
    pub lfs_oid: Option<String>,
    #[serde(skip)]
    pub lfs_size: Option<u64>,
    #[serde(skip)]
    pub lfs_download_url: Option<String>,
}

impl RepoItem {
    pub fn is_dir(&self) -> bool {
        self.item_type == "dir"
    }

    pub fn is_file(&self) -> bool {
        self.item_type == "file"
    }

    pub fn is_lfs(&self) -> bool {
        self.lfs_oid.is_some()
    }

    pub fn actual_size(&self) -> Option<u64> {
        self.lfs_size.or(self.size)
    }

    pub fn actual_download_url(&self) -> Option<&String> {
        self.lfs_download_url
            .as_ref()
            .or(self.download_url.as_ref())
    }
}

#[derive(Debug, Clone)]
pub struct LfsPointer {
    pub oid: String,
    pub size: u64,
}

impl LfsPointer {
    pub fn parse(content: &str) -> Option<Self> {
        if !content.starts_with("version https://git-lfs.github.com/spec/v1") {
            return None;
        }

        let mut oid = None;
        let mut size = None;

        for line in content.lines() {
            if line.starts_with("oid sha256:") {
                oid = Some(line.trim_start_matches("oid sha256:").to_string());
            } else if line.starts_with("size ") {
                size = line.trim_start_matches("size ").parse().ok();
            }
        }

        match (oid, size) {
            (Some(oid), Some(size)) => Some(LfsPointer { oid, size }),
            _ => None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct LfsBatchResponse {
    objects: Vec<LfsResponseObject>,
}

#[derive(Debug, serde::Deserialize)]
struct LfsResponseObject {
    #[allow(dead_code)]
    oid: String,
    #[allow(dead_code)]
    size: u64,
    actions: Option<LfsActions>,
}

#[derive(Debug, serde::Deserialize)]
struct LfsActions {
    download: Option<LfsDownloadAction>,
}

#[derive(Debug, serde::Deserialize)]
struct LfsDownloadAction {
    href: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitTreeResponse {
    pub tree: Vec<GitTreeEntry>,
    pub truncated: bool,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct SearchResult {
    pub items: Vec<SearchItem>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct SearchItem {
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: u32,
    pub fork: bool,
    pub language: Option<String>,
    pub pushed_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitTreeEntry {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub size: Option<u64>,
    pub sha: String,
    pub url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("Invalid token. Falling back to public API.")]
    InvalidToken,
    #[error("Rate limit exceeded for {0}. Consider adding a token for more limits.")]
    RateLimitReached(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("API Error: {0}")]
    ApiError(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("ghgrab/1.2.0")
            .build()
            .context("Failed to create HTTP client")?;
        Ok(GitHubClient { client, token })
    }

    async fn request(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<serde_json::Value>,
    ) -> std::result::Result<reqwest::Response, GitHubError> {
        let mut builder = self.client.request(method, url);

        if let Some(token) = &self.token {
            builder = builder.header("Authorization", format!("token {}", token));
        }

        if let Some(body) = body {
            builder = builder.json(&body);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;

        match response.status().as_u16() {
            200..=299 => Ok(response),
            401 if self.token.is_some() => Err(GitHubError::InvalidToken),
            403 => {
                let remaining = response
                    .headers()
                    .get("X-RateLimit-Remaining")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(1);

                if remaining == 0 {
                    let level = if self.token.is_some() {
                        "authenticated user"
                    } else {
                        "unauthenticated user"
                    };
                    Err(GitHubError::RateLimitReached(level.to_string()))
                } else if self.token.is_some() {
                    Err(GitHubError::InvalidToken)
                } else {
                    Err(GitHubError::ApiError(format!(
                        "Forbidden: {}",
                        response.status()
                    )))
                }
            }
            404 => Err(GitHubError::NotFound(url.to_string())),
            _ => Err(GitHubError::ApiError(format!("HTTP {}", response.status()))),
        }
    }

    pub async fn fetch_contents(&self, url: &str) -> Result<Vec<RepoItem>> {
        let response = self.request(reqwest::Method::GET, url, None).await?;

        if !response.status().is_success() {
            return Err(anyhow!("GitHub API error: {}", response.status()));
        }

        let items: Vec<RepoItem> = response
            .json()
            .await
            .context("Failed to parse GitHub API response")?;

        Ok(items)
    }

    pub async fn fetch_recursive_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> std::result::Result<GitTreeResponse, GitHubError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
            owner, repo, branch
        );
        let response = self.request(reqwest::Method::GET, &url, None).await?;

        let tree: GitTreeResponse = response
            .json()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;
        Ok(tree)
    }

    pub async fn fetch_default_branch(
        &self,
        owner: &str,
        repo: &str,
    ) -> std::result::Result<String, GitHubError> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = self.request(reqwest::Method::GET, &url, None).await?;

        #[derive(serde::Deserialize)]
        struct RepoInfo {
            default_branch: String,
        }

        let info: RepoInfo = response
            .json()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;
        Ok(info.default_branch)
    }

    pub async fn search_repositories(
        &self,
        query: &str,
    ) -> std::result::Result<Vec<SearchItem>, GitHubError> {
        let url = format!(
            "https://api.github.com/search/repositories?q={}&sort=stars&order=desc",
            urlencoding::encode(query)
        );
        let response = self.request(reqwest::Method::GET, &url, None).await?;

        let result: SearchResult = response
            .json()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;
        Ok(result.items)
    }

    // Fetch raw content
    pub async fn fetch_raw_content(&self, url: &str) -> Result<String> {
        let response = self.request(reqwest::Method::GET, url, None).await?;

        let content = response.text().await.context("Failed to read content")?;
        Ok(content)
    }

    pub async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.request(reqwest::Method::GET, url, None).await?;
        let bytes = response
            .bytes()
            .await
            .context("Failed to read binary content")?;
        Ok(bytes.to_vec())
    }

    pub async fn fetch_partial_content(&self, url: &str, max_bytes: u64) -> Result<String> {
        let mut builder = self.client.request(reqwest::Method::GET, url);

        if let Some(token) = &self.token {
            builder = builder.header("Authorization", format!("token {}", token));
        }

        // Add Range header for partial download
        builder = builder.header("Range", format!("bytes=0-{}", max_bytes));

        let response = builder
            .send()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;

        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
        {
            return Err(anyhow!("GitHub API error: {}", response.status()));
        }

        let content = response.text().await.context("Failed to read content")?;
        Ok(content)
    }

    // Call LFS batch API
    pub async fn get_lfs_download_url(
        &self,
        owner: &str,
        repo: &str,
        oid: &str,
        size: u64,
    ) -> Result<String> {
        let batch_url = format!(
            "https://github.com/{}/{}.git/info/lfs/objects/batch",
            owner, repo
        );

        let request_body = serde_json::json!({
            "operation": "download",
            "transfers": ["basic"],
            "objects": [
                {
                    "oid": oid,
                    "size": size
                }
            ]
        });

        let response = self
            .request(reqwest::Method::POST, &batch_url, Some(request_body))
            .await?;

        let batch_response: LfsBatchResponse = response
            .json()
            .await
            .context("Failed to parse LFS response")?;

        batch_response
            .objects
            .into_iter()
            .next()
            .and_then(|obj| obj.actions)
            .and_then(|actions| actions.download)
            .map(|download| download.href)
            .ok_or_else(|| anyhow!("No download URL in LFS response"))
    }

    pub async fn resolve_lfs_files(
        &self,
        items: &mut [RepoItem],
        owner: &str,
        repo: &str,
        branch: &str,
    ) {
        for item in items.iter_mut() {
            if item.is_file() {
                if let Some(size) = item.size {
                    if size < 1024 {
                        if let Some(download_url) = &item.download_url {
                            if let Ok(content) = self.fetch_raw_content(download_url).await {
                                if let Some(pointer) = LfsPointer::parse(&content) {
                                    item.lfs_oid = Some(pointer.oid.clone());
                                    item.lfs_size = Some(pointer.size);

                                    if let Ok(lfs_url) = self
                                        .get_lfs_download_url(
                                            owner,
                                            repo,
                                            &pointer.oid,
                                            pointer.size,
                                        )
                                        .await
                                    {
                                        item.lfs_download_url = Some(lfs_url);
                                    } else {
                                        let media_url = format!(
                                            "https://media.githubusercontent.com/media/{}/{}/{}/{}",
                                            owner, repo, branch, item.path
                                        );
                                        item.lfs_download_url = Some(media_url);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GitHubUrl parsing tests ---

    #[test]
    fn test_parse_github_url() {
        let url = "https://github.com/rust-lang/rust/tree/master/src/tools";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "rust");
        assert_eq!(parsed.branch, "master");
        assert_eq!(parsed.path, "src/tools");
    }

    #[test]
    fn test_parse_root_url() {
        let url = "https://github.com/rust-lang/rust";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "rust");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "");
    }

    #[test]
    fn test_parse_blob_url() {
        let url = "https://github.com/owner/repo/blob/main/src/lib.rs";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "owner");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "src/lib.rs");
    }

    #[test]
    fn test_parse_branch_only_url() {
        let url = "https://github.com/owner/repo/tree/develop";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "owner");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "develop");
        assert_eq!(parsed.path, "");
    }

    #[test]
    fn test_parse_deep_path() {
        let url = "https://github.com/org/project/tree/v2.0/src/core/engine";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "org");
        assert_eq!(parsed.repo, "project");
        assert_eq!(parsed.branch, "v2.0");
        assert_eq!(parsed.path, "src/core/engine");
    }

    #[test]
    fn test_parse_invalid_non_github_url() {
        let url = "https://gitlab.com/user/repo";
        assert!(GitHubUrl::parse(url).is_err());
    }

    #[test]
    fn test_parse_invalid_not_a_url() {
        assert!(GitHubUrl::parse("not a url").is_err());
    }

    #[test]
    fn test_parse_invalid_no_repo() {
        let url = "https://github.com/owner";
        assert!(GitHubUrl::parse(url).is_err());
    }

    // --- api_url tests ---

    #[test]
    fn test_api_url_with_path() {
        let gh = GitHubUrl {
            owner: "owner".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: "src/lib.rs".into(),
        };
        assert_eq!(
            gh.api_url(),
            "https://api.github.com/repos/owner/repo/contents/src/lib.rs?ref=main"
        );
    }

    #[test]
    fn test_api_url_without_path() {
        let gh = GitHubUrl {
            owner: "owner".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: String::new(),
        };
        assert_eq!(
            gh.api_url(),
            "https://api.github.com/repos/owner/repo/contents?ref=main"
        );
    }

    // --- LfsPointer tests ---

    #[test]
    fn test_lfs_pointer_parse_valid() {
        let content =
            "version https://git-lfs.github.com/spec/v1\noid sha256:abc123def456\nsize 12345";
        let pointer = LfsPointer::parse(content).unwrap();
        assert_eq!(pointer.oid, "abc123def456");
        assert_eq!(pointer.size, 12345);
    }

    #[test]
    fn test_lfs_pointer_parse_not_lfs() {
        let content = "This is just a regular file content";
        assert!(LfsPointer::parse(content).is_none());
    }

    #[test]
    fn test_lfs_pointer_parse_missing_oid() {
        let content = "version https://git-lfs.github.com/spec/v1\nsize 12345";
        assert!(LfsPointer::parse(content).is_none());
    }

    #[test]
    fn test_lfs_pointer_parse_missing_size() {
        let content = "version https://git-lfs.github.com/spec/v1\noid sha256:abc123";
        assert!(LfsPointer::parse(content).is_none());
    }

    // --- RepoItem tests ---

    fn make_test_item(item_type: &str) -> RepoItem {
        RepoItem {
            name: "test.rs".to_string(),
            item_type: item_type.to_string(),
            path: "src/test.rs".to_string(),
            download_url: Some("https://example.com/test.rs".to_string()),
            url: "https://api.github.com/repos/o/r/contents/src/test.rs".to_string(),
            size: Some(1024),
            selected: false,
            lfs_oid: None,
            lfs_size: None,
            lfs_download_url: None,
        }
    }

    #[test]
    fn test_repo_item_is_dir() {
        let item = make_test_item("dir");
        assert!(item.is_dir());
        assert!(!item.is_file());
    }

    #[test]
    fn test_repo_item_is_file() {
        let item = make_test_item("file");
        assert!(item.is_file());
        assert!(!item.is_dir());
    }

    #[test]
    fn test_repo_item_not_lfs() {
        let item = make_test_item("file");
        assert!(!item.is_lfs());
        assert_eq!(item.actual_size(), Some(1024));
        assert_eq!(
            item.actual_download_url().map(|s| s.as_str()),
            Some("https://example.com/test.rs")
        );
    }

    #[test]
    fn test_repo_item_lfs() {
        let mut item = make_test_item("file");
        item.lfs_oid = Some("abc123".to_string());
        item.lfs_size = Some(999999);
        item.lfs_download_url = Some("https://lfs.example.com/abc123".to_string());

        assert!(item.is_lfs());
        assert_eq!(
            item.actual_download_url().map(|s| s.as_str()),
            Some("https://lfs.example.com/abc123")
        );
    }

    #[test]
    fn test_github_error_formatting() {
        assert_eq!(
            format!("{}", GitHubError::InvalidToken),
            "Invalid token. Falling back to public API."
        );
        assert_eq!(
            format!(
                "{}",
                GitHubError::RateLimitReached("unauthenticated".to_string())
            ),
            "Rate limit exceeded for unauthenticated. Consider adding a token for more limits."
        );
        assert_eq!(
            format!("{}", GitHubError::NotFound("src/missing.rs".to_string())),
            "Resource not found: src/missing.rs"
        );
    }
}
