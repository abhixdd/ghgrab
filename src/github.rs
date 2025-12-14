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

        let (branch, path) = if path_segments.len() >= 4 && path_segments[2] == "tree" {
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
        self.lfs_download_url.as_ref().or(self.download_url.as_ref())
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

#[derive(Debug, serde::Serialize)]
struct LfsBatchRequest {
    operation: String,
    transfers: Vec<String>,
    objects: Vec<LfsObject>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LfsObject {
    oid: String,
    size: u64,
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

pub struct GitHubClient {
    client: reqwest::Client,
}

impl GitHubClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("ghgrab/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;
        Ok(GitHubClient { client })
    }

    pub async fn fetch_contents(&self, url: &str) -> Result<Vec<RepoItem>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to send request to GitHub API")?;

        if response.status().as_u16() == 403 {
            return Err(anyhow!("Rate limit exceeded. Please try again later."));
        }

        if response.status().as_u16() == 404 {
            return Err(anyhow!("Path not found in repository"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("GitHub API error: {}", response.status()));
        }

        let items: Vec<RepoItem> = response
            .json()
            .await
            .context("Failed to parse GitHub API response")?;

        Ok(items)
    }

    // Fetch raw content 
    pub async fn fetch_raw_content(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch raw content")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch file content"));
        }

        let content = response.text().await.context("Failed to read content")?;
        Ok(content)
    }

    // Call LFS batch API 
    pub async fn get_lfs_download_url(&self, owner: &str, repo: &str, oid: &str, size: u64) -> Result<String> {
        let batch_url = format!("https://github.com/{}/{}.git/info/lfs/objects/batch", owner, repo);

        let request = LfsBatchRequest {
            operation: "download".to_string(),
            transfers: vec!["basic".to_string()],
            objects: vec![LfsObject {
                oid: oid.to_string(),
                size,
            }],
        };

        let response = self
            .client
            .post(&batch_url)
            .header("Accept", "application/vnd.git-lfs+json")
            .header("Content-Type", "application/vnd.git-lfs+json")
            .json(&request)
            .send()
            .await
            .context("Failed to call LFS batch API")?;

        if !response.status().is_success() {
            return Err(anyhow!("LFS batch API error: {}", response.status()));
        }

        let batch_response: LfsBatchResponse = response.json().await.context("Failed to parse LFS response")?;

        batch_response
            .objects
            .into_iter()
            .next()
            .and_then(|obj| obj.actions)
            .and_then(|actions| actions.download)
            .map(|download| download.href)
            .ok_or_else(|| anyhow!("No download URL in LFS response"))
    }

    pub async fn resolve_lfs_files(&self, items: &mut Vec<RepoItem>, owner: &str, repo: &str) {
        for item in items.iter_mut() {
            if item.is_file() {
                if let Some(size) = item.size {
                    if size < 1024 {
                        if let Some(download_url) = &item.download_url {
                            if let Ok(content) = self.fetch_raw_content(download_url).await {
                                if let Some(pointer) = LfsPointer::parse(&content) {
                                    item.lfs_oid = Some(pointer.oid.clone());
                                    item.lfs_size = Some(pointer.size);

                                    if let Ok(lfs_url) = self.get_lfs_download_url(owner, repo, &pointer.oid, pointer.size).await {
                                        item.lfs_download_url = Some(lfs_url);
                                    } else {
                                        let media_url = format!(
                                            "https://media.githubusercontent.com/media/{}/{}/master/{}",
                                            owner, repo, item.path
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
}
