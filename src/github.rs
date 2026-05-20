use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    GitHub,
    Gitea(String),
    GitLab(String),
}

impl Platform {
    fn detect(host: &str, segments: &[&str]) -> Self {
        match host {
            "github.com" => Platform::GitHub,
            "gitlab.com" => Platform::GitLab("gitlab.com".to_string()),
            "codeberg.org" => Platform::Gitea("codeberg.org".to_string()),
            h => {
                if segments.len() >= 3 && segments[2] == "-" {
                    Platform::GitLab(h.to_string())
                } else {
                    Platform::Gitea(h.to_string())
                }
            }
        }
    }

    pub fn is_github(&self) -> bool {
        matches!(self, Platform::GitHub)
    }

    pub fn is_gitea(&self) -> bool {
        matches!(self, Platform::Gitea(_))
    }

    pub fn is_gitlab(&self) -> bool {
        matches!(self, Platform::GitLab(_))
    }

    pub fn host(&self) -> &str {
        match self {
            Platform::GitHub => "github.com",
            Platform::Gitea(h) | Platform::GitLab(h) => h,
        }
    }

    fn auth_value(&self, token: &str) -> String {
        match self {
            Platform::GitLab(_) => format!("Bearer {}", token),
            _ => format!("token {}", token),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitHubUrl {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub platform: Platform,
}

impl GitHubUrl {
    pub fn parse(url_str: &str) -> Result<Self> {
        let url = Url::parse(url_str).context("Invalid URL format")?;

        let host = url.host_str().ok_or_else(|| anyhow!("URL has no host"))?;

        let path_segments: Vec<&str> = url
            .path_segments()
            .ok_or_else(|| anyhow!("Invalid URL path"))?
            .filter(|s| !s.is_empty())
            .collect();

        if path_segments.len() < 2 {
            return Err(anyhow!("URL must contain owner and repository"));
        }

        let owner = path_segments[0].to_string();
        let repo = path_segments[1].trim_end_matches(".git").to_string();

        let platform = Platform::detect(host, &path_segments);

        let (branch, path) = Self::parse_branch_and_path(&platform, &path_segments);

        Ok(GitHubUrl {
            owner,
            repo,
            branch,
            path,
            platform,
        })
    }

    fn parse_branch_and_path(platform: &Platform, segments: &[&str]) -> (String, String) {
        match platform {
            Platform::GitHub => {
                if segments.len() >= 4 && (segments[2] == "tree" || segments[2] == "blob") {
                    let branch = segments[3].to_string();
                    let path = if segments.len() > 4 {
                        segments[4..].join("/")
                    } else {
                        String::new()
                    };
                    (branch, path)
                } else {
                    ("main".to_string(), String::new())
                }
            }
            Platform::Gitea(_) => {
                if segments.len() >= 5 && segments[2] == "src" {
                    let branch = segments[4].to_string();
                    let path = if segments.len() > 5 {
                        segments[5..].join("/")
                    } else {
                        String::new()
                    };
                    (branch, path)
                } else {
                    ("main".to_string(), String::new())
                }
            }
            Platform::GitLab(_) => {
                if segments.len() >= 5
                    && segments[2] == "-"
                    && (segments[3] == "tree" || segments[3] == "blob")
                {
                    let branch = segments[4].to_string();
                    let path = if segments.len() > 5 {
                        segments[5..].join("/")
                    } else {
                        String::new()
                    };
                    (branch, path)
                } else {
                    ("main".to_string(), String::new())
                }
            }
        }
    }

    pub fn ambiguous_ref_candidates(&self) -> Vec<Self> {
        if self.path.is_empty() {
            return Vec::new();
        }

        let path_segments: Vec<&str> = self
            .path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();

        if path_segments.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::with_capacity(path_segments.len());
        for split_idx in 1..=path_segments.len() {
            let branch_suffix = path_segments[..split_idx].join("/");
            let remaining_path = if split_idx < path_segments.len() {
                path_segments[split_idx..].join("/")
            } else {
                String::new()
            };

            candidates.push(Self {
                owner: self.owner.clone(),
                repo: self.repo.clone(),
                branch: format!("{}/{}", self.branch, branch_suffix),
                path: remaining_path,
                platform: self.platform.clone(),
            });
        }

        candidates
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
                if let Some(rest) = url.strip_prefix("git@") {
                    let rest = rest.trim_end_matches(".git");
                    if let Some((host, path)) = rest.split_once(':') {
                        return Some(format!("https://{}/{}", host, path));
                    }
                }
                if url.ends_with(".git") {
                    return Some(url.trim_end_matches(".git").to_string());
                }
                return Some(url);
            }
        }
        None
    }

    pub fn api_url(&self) -> String {
        self.contents_api_url_for_path(&self.path)
    }

    pub fn contents_api_url_for_path(&self, path: &str) -> String {
        let norm = path.replace('\\', "/");
        let norm = norm.trim_matches('/');
        match &self.platform {
            Platform::GitHub => {
                let base = format!(
                    "https://api.github.com/repos/{}/{}/contents",
                    self.owner, self.repo
                );
                if norm.is_empty() {
                    format!("{}?ref={}", base, self.branch)
                } else {
                    format!("{}/{}?ref={}", base, norm, self.branch)
                }
            }
            Platform::Gitea(host) => {
                let base = format!(
                    "https://{}/api/v1/repos/{}/{}/contents",
                    host, self.owner, self.repo
                );
                if norm.is_empty() {
                    format!("{}?ref={}", base, self.branch)
                } else {
                    format!("{}/{}?ref={}", base, norm, self.branch)
                }
            }
            Platform::GitLab(host) => {
                let project_str = format!("{}/{}", self.owner, self.repo);
                let project = urlencoding::encode(&project_str);
                let path_enc = urlencoding::encode(norm).to_string();
                format!(
                    "https://{}/api/v4/projects/{}/repository/tree?path={}&ref={}&per_page=100",
                    host, project, path_enc, self.branch
                )
            }
        }
    }

    pub fn raw_file_url_for_path(&self, file_path: &str) -> String {
        match &self.platform {
            Platform::GitHub => format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                self.owner, self.repo, self.branch, file_path
            ),
            Platform::Gitea(host) => format!(
                "https://{}/{}/{}/raw/branch/{}/{}",
                host, self.owner, self.repo, self.branch, file_path
            ),
            Platform::GitLab(host) => format!(
                "https://{}/{}/{}/-/raw/{}/{}",
                host, self.owner, self.repo, self.branch, file_path
            ),
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

#[derive(Debug, serde::Deserialize, Clone)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub content_type: Option<String>,
    pub size: u64,
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
    platform: Platform,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        Self::new_for_platform(token, Platform::GitHub)
    }

    pub fn new_for_platform(token: Option<String>, platform: Platform) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("ghgrab/2.0.0")
            .build()
            .context("Failed to create HTTP client")?;
        Ok(GitHubClient {
            client,
            token,
            platform,
        })
    }

    pub fn new_for_url(token: Option<String>, gh_url: &GitHubUrl) -> Result<Self> {
        Self::new_for_platform(token, gh_url.platform.clone())
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    async fn request(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<serde_json::Value>,
    ) -> std::result::Result<reqwest::Response, GitHubError> {
        let mut builder = self.client.request(method, url);

        if let Some(token) = &self.token {
            builder = builder.header("Authorization", self.platform.auth_value(token));
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
        if self.platform.is_gitlab() {
            return self.fetch_contents_gitlab(url).await;
        }
        let response = self.request(reqwest::Method::GET, url, None).await?;
        let items: Vec<RepoItem> = response
            .json()
            .await
            .context("Failed to parse API response")?;
        Ok(items)
    }

    async fn fetch_contents_gitlab(&self, url: &str) -> Result<Vec<RepoItem>> {
        #[derive(serde::Deserialize)]
        struct GitLabItem {
            name: String,
            #[serde(rename = "type")]
            item_type: String,
            path: String,
        }

        let response = self.request(reqwest::Method::GET, url, None).await?;
        let gitlab_items: Vec<GitLabItem> = response
            .json()
            .await
            .context("Failed to parse GitLab API response")?;

        let parsed = Url::parse(url).context("Invalid GitLab API URL")?;
        let host = parsed.host_str().unwrap_or("gitlab.com").to_string();

        let query: std::collections::HashMap<String, String> = parsed
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let branch = query
            .get("ref")
            .cloned()
            .unwrap_or_else(|| "main".to_string());

        let project_id = parsed
            .path()
            .split("/projects/")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .map(|s| urlencoding::decode(s).unwrap_or_default().to_string())
            .unwrap_or_default();

        Ok(gitlab_items
            .into_iter()
            .map(|item| {
                let item_type = if item.item_type == "blob" {
                    "file".to_string()
                } else {
                    "dir".to_string()
                };

                let project_encoded = urlencoding::encode(&project_id).to_string();

                let download_url = if item_type == "file" {
                    Some(format!(
                        "https://{}/api/v4/projects/{}/repository/files/{}/raw?ref={}",
                        host,
                        project_encoded,
                        urlencoding::encode(&item.path),
                        branch
                    ))
                } else {
                    None
                };

                let api_url = format!(
                    "https://{}/api/v4/projects/{}/repository/tree?path={}&ref={}&per_page=100",
                    host,
                    project_encoded,
                    urlencoding::encode(&item.path),
                    branch
                );

                RepoItem {
                    name: item.name,
                    item_type,
                    path: item.path,
                    download_url,
                    url: api_url,
                    size: None,
                    selected: false,
                    lfs_oid: None,
                    lfs_size: None,
                    lfs_download_url: None,
                }
            })
            .collect())
    }

    pub async fn fetch_recursive_tree(
        &self,
        gh_url: &GitHubUrl,
    ) -> std::result::Result<GitTreeResponse, GitHubError> {
        match &gh_url.platform {
            Platform::GitHub => {
                let url = format!(
                    "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
                    gh_url.owner, gh_url.repo, gh_url.branch
                );
                let response = self.request(reqwest::Method::GET, &url, None).await?;
                let tree: GitTreeResponse = response
                    .json()
                    .await
                    .map_err(|e| GitHubError::ApiError(e.to_string()))?;
                Ok(tree)
            }
            Platform::Gitea(host) => {
                let direct_url = format!(
                    "https://{}/api/v1/repos/{}/{}/git/trees/{}?recursive=1",
                    host, gh_url.owner, gh_url.repo, gh_url.branch
                );
                if let Ok(resp) = self.request(reqwest::Method::GET, &direct_url, None).await {
                    if let Ok(tree) = resp.json::<GitTreeResponse>().await {
                        return Ok(tree);
                    }
                }

                let branch_url = format!(
                    "https://{}/api/v1/repos/{}/{}/branches/{}",
                    host, gh_url.owner, gh_url.repo, gh_url.branch
                );
                let branch_resp = self
                    .request(reqwest::Method::GET, &branch_url, None)
                    .await?;

                #[derive(serde::Deserialize)]
                struct GiteaBranch {
                    commit: GiteaCommit,
                }
                #[derive(serde::Deserialize)]
                struct GiteaCommit {
                    id: String,
                }

                let branch_info: GiteaBranch = branch_resp
                    .json()
                    .await
                    .map_err(|e| GitHubError::ApiError(e.to_string()))?;

                let tree_url = format!(
                    "https://{}/api/v1/repos/{}/{}/git/trees/{}?recursive=1",
                    host, gh_url.owner, gh_url.repo, branch_info.commit.id
                );
                let tree_resp = self.request(reqwest::Method::GET, &tree_url, None).await?;
                let tree: GitTreeResponse = tree_resp
                    .json()
                    .await
                    .map_err(|e| GitHubError::ApiError(e.to_string()))?;
                Ok(tree)
            }
            Platform::GitLab(host) => {
                #[derive(serde::Deserialize)]
                struct GitLabItem {
                    id: String,
                    #[allow(dead_code)]
                    name: String,
                    #[serde(rename = "type")]
                    item_type: String,
                    path: String,
                    mode: String,
                }

                let project_str = format!("{}/{}", gh_url.owner, gh_url.repo);
                let project = urlencoding::encode(&project_str).to_string();

                let make_url = |page: u32| {
                    format!(
                        "https://{}/api/v4/projects/{}/repository/tree?ref={}&recursive=true&per_page=100&page={}",
                        host, project, gh_url.branch, page
                    )
                };

                let first_resp = self
                    .request(reqwest::Method::GET, &make_url(1), None)
                    .await?;

                let total_pages: u32 = first_resp
                    .headers()
                    .get("x-total-pages")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);

                let first_items: Vec<GitLabItem> = first_resp
                    .json()
                    .await
                    .map_err(|e| GitHubError::ApiError(e.to_string()))?;

                let mut all_entries: Vec<GitTreeEntry> = first_items
                    .into_iter()
                    .map(|item| GitTreeEntry {
                        path: item.path,
                        mode: item.mode,
                        entry_type: item.item_type,
                        size: None,
                        sha: item.id,
                        url: String::new(),
                    })
                    .collect();

                if total_pages > 1 {
                    let futures: Vec<_> = (2..=total_pages)
                        .map(|page| {
                            let url = make_url(page);
                            async move {
                                self.request(reqwest::Method::GET, &url, None)
                                    .await?
                                    .json::<Vec<GitLabItem>>()
                                    .await
                                    .map_err(|e| GitHubError::ApiError(e.to_string()))
                            }
                        })
                        .collect();

                    let pages = futures::future::join_all(futures).await;
                    for page_result in pages {
                        for item in page_result? {
                            all_entries.push(GitTreeEntry {
                                path: item.path,
                                mode: item.mode,
                                entry_type: item.item_type,
                                size: None,
                                sha: item.id,
                                url: String::new(),
                            });
                        }
                    }
                }

                Ok(GitTreeResponse {
                    tree: all_entries,
                    truncated: false,
                })
            }
        }
    }

    pub async fn fetch_default_branch(
        &self,
        gh_url: &GitHubUrl,
    ) -> std::result::Result<String, GitHubError> {
        let url = match &gh_url.platform {
            Platform::GitHub => {
                format!(
                    "https://api.github.com/repos/{}/{}",
                    gh_url.owner, gh_url.repo
                )
            }
            Platform::Gitea(host) => {
                format!(
                    "https://{}/api/v1/repos/{}/{}",
                    host, gh_url.owner, gh_url.repo
                )
            }
            Platform::GitLab(host) => {
                let project_str = format!("{}/{}", gh_url.owner, gh_url.repo);
                let project = urlencoding::encode(&project_str).to_string();
                format!("https://{}/api/v4/projects/{}", host, project)
            }
        };

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

    pub async fn fetch_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> std::result::Result<Vec<GitHubRelease>, GitHubError> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
        let response = self.request(reqwest::Method::GET, &url, None).await?;
        let releases: Vec<GitHubRelease> = response
            .json()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;
        Ok(releases)
    }

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
            builder = builder.header("Authorization", self.platform.auth_value(token));
        }

        builder = builder.header("Range", format!("bytes=0-{}", max_bytes));

        let response = builder
            .send()
            .await
            .map_err(|e| GitHubError::ApiError(e.to_string()))?;

        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
        {
            return Err(anyhow!("API error: {}", response.status()));
        }

        let content = response.text().await.context("Failed to read content")?;
        Ok(content)
    }

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
            "objects": [{"oid": oid, "size": size}]
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

    /// Resolve GitHub LFS pointers in a list of items. No-op for non-GitHub platforms.
    pub async fn resolve_lfs_files(
        &self,
        items: &mut [RepoItem],
        owner: &str,
        repo: &str,
        branch: &str,
    ) {
        if !self.platform.is_github() {
            return;
        }
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
        assert_eq!(parsed.platform, Platform::GitHub);
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
    fn test_ambiguous_ref_candidates_for_github_branch_with_slash() {
        let parsed =
            GitHubUrl::parse("https://github.com/org/project/tree/feature/foo/src/core").unwrap();

        let candidates = parsed.ambiguous_ref_candidates();

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].branch, "feature/foo");
        assert_eq!(candidates[0].path, "src/core");
        assert_eq!(candidates[1].branch, "feature/foo/src");
        assert_eq!(candidates[1].path, "core");
        assert_eq!(candidates[2].branch, "feature/foo/src/core");
        assert_eq!(candidates[2].path, "");
    }

    #[test]
    fn test_ambiguous_ref_candidates_for_branch_only_url() {
        let parsed = GitHubUrl::parse("https://github.com/org/project/tree/feature/foo").unwrap();

        let candidates = parsed.ambiguous_ref_candidates();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].branch, "feature/foo");
        assert_eq!(candidates[0].path, "");
    }

    #[test]
    fn test_parse_gitlab_root_url() {
        let url = "https://gitlab.com/user/repo";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "");
        assert_eq!(parsed.platform, Platform::GitLab("gitlab.com".to_string()));
    }

    #[test]
    fn test_parse_gitlab_tree_url() {
        let url = "https://gitlab.com/user/repo/-/tree/develop/src/core";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "develop");
        assert_eq!(parsed.path, "src/core");
        assert_eq!(parsed.platform, Platform::GitLab("gitlab.com".to_string()));
    }

    #[test]
    fn test_parse_codeberg_root_url() {
        let url = "https://codeberg.org/user/repo";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.platform, Platform::Gitea("codeberg.org".to_string()));
    }

    #[test]
    fn test_parse_codeberg_src_url() {
        let url = "https://codeberg.org/user/repo/src/branch/main/docs/guide";
        let parsed = GitHubUrl::parse(url).unwrap();
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.path, "docs/guide");
        assert_eq!(parsed.platform, Platform::Gitea("codeberg.org".to_string()));
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
            platform: Platform::GitHub,
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
            platform: Platform::GitHub,
        };
        assert_eq!(
            gh.api_url(),
            "https://api.github.com/repos/owner/repo/contents?ref=main"
        );
    }

    #[test]
    fn test_gitea_api_url() {
        let gh = GitHubUrl {
            owner: "user".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: "src".into(),
            platform: Platform::Gitea("codeberg.org".to_string()),
        };
        assert_eq!(
            gh.api_url(),
            "https://codeberg.org/api/v1/repos/user/repo/contents/src?ref=main"
        );
    }

    #[test]
    fn test_gitlab_api_url() {
        let gh = GitHubUrl {
            owner: "user".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: "src".into(),
            platform: Platform::GitLab("gitlab.com".to_string()),
        };
        let url = gh.api_url();
        assert!(url.contains("/api/v4/projects/user%2Frepo/repository/tree"));
        assert!(url.contains("ref=main"));
    }

    #[test]
    fn test_raw_file_url_github() {
        let gh = GitHubUrl {
            owner: "owner".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: String::new(),
            platform: Platform::GitHub,
        };
        assert_eq!(
            gh.raw_file_url_for_path("src/lib.rs"),
            "https://raw.githubusercontent.com/owner/repo/main/src/lib.rs"
        );
    }

    #[test]
    fn test_raw_file_url_gitea() {
        let gh = GitHubUrl {
            owner: "user".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: String::new(),
            platform: Platform::Gitea("codeberg.org".to_string()),
        };
        assert_eq!(
            gh.raw_file_url_for_path("src/lib.rs"),
            "https://codeberg.org/user/repo/raw/branch/main/src/lib.rs"
        );
    }

    #[test]
    fn test_raw_file_url_gitlab() {
        let gh = GitHubUrl {
            owner: "user".into(),
            repo: "repo".into(),
            branch: "main".into(),
            path: String::new(),
            platform: Platform::GitLab("gitlab.com".to_string()),
        };
        assert_eq!(
            gh.raw_file_url_for_path("src/lib.rs"),
            "https://gitlab.com/user/repo/-/raw/main/src/lib.rs"
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
