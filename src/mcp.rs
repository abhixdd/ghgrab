use crate::agent;
use crate::github::{GitHubClient, GitHubUrl};
use crate::release;
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone)]
pub struct GhGrabMcp {
    pub token: Option<String>,
    pub download_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RepoTreeParams {
    #[doc = "The URL of the repository (e.g. https://github.com/owner/repo)"]
    url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DownloadFilesParams {
    #[doc = "The URL of the repository (e.g. https://github.com/owner/repo)"]
    url: String,
    #[doc = "Specific paths/folders inside the repository to download"]
    paths: Vec<String>,
    #[doc = "Optional custom output directory path"]
    output_dir: Option<String>,
    #[doc = "If true, downloads directly into the target directory without creating a repo-named folder"]
    no_folder: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DownloadReleaseParams {
    #[doc = "The repository reference (e.g. owner/repo or github.com URL)"]
    repo: String,
    #[doc = "Specific release tag name (e.g. v1.0.0). Defaults to the latest release."]
    tag: Option<String>,
    #[doc = "OS override (e.g. windows, linux, macos). Defaults to current OS."]
    os: Option<String>,
    #[doc = "Architecture override (e.g. x86_64, aarch64). Defaults to current architecture."]
    arch: Option<String>,
    #[doc = "If true, extracts archive assets after download. Defaults to true."]
    extract: Option<bool>,
    #[doc = "Optional custom regex to match a specific asset name"]
    asset_regex: Option<String>,
    #[doc = "Optional custom output directory path"]
    output_dir: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchReposParams {
    #[doc = "The search query (keyword/term)"]
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileParams {
    #[doc = "The repository URL (e.g. https://github.com/owner/repo)"]
    url: String,
    #[doc = "The path to the file within the repository (e.g. src/lib.rs)"]
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFilePreviewParams {
    #[doc = "The repository URL (e.g. https://github.com/owner/repo)"]
    url: String,
    #[doc = "The path to the file within the repository (e.g. src/lib.rs)"]
    path: String,
    #[doc = "Maximum bytes to fetch. Defaults to 8192 (8KB)."]
    max_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListReleasesParams {
    #[doc = "The owner of the repository"]
    owner: String,
    #[doc = "The name of the repository"]
    repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RepoInfoParams {
    #[doc = "The URL of the repository (e.g. https://github.com/owner/repo)"]
    url: String,
}

#[tool_router(server_handler)]
impl GhGrabMcp {
    #[tool(
        description = "List all files and directories in a Git repository. Supports GitHub, GitLab, Codeberg, Gitea, and Forgejo."
    )]
    async fn repo_tree(&self, Parameters(params): Parameters<RepoTreeParams>) -> CallToolResult {
        match agent::fetch_tree(&params.url, self.token.clone()).await {
            Ok(res) => CallToolResult::structured(serde_json::to_value(res).unwrap()),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to fetch repo tree: {}", e)
            })),
        }
    }

    #[tool(
        description = "Download specific files or folders from a Git repository to the local filesystem."
    )]
    async fn download_files(
        &self,
        Parameters(params): Parameters<DownloadFilesParams>,
    ) -> CallToolResult {
        let output = params.output_dir.or(self.download_path.clone());
        let no_folder = params.no_folder.unwrap_or(false);
        match agent::download_paths(
            &params.url,
            self.token.clone(),
            &params.paths,
            output,
            false,
            no_folder,
        )
        .await
        {
            Ok(res) => CallToolResult::structured(serde_json::to_value(res).unwrap()),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to download files: {}", e)
            })),
        }
    }

    #[tool(description = "Download a GitHub release asset with OS/arch auto-detection.")]
    async fn download_release(
        &self,
        Parameters(params): Parameters<DownloadReleaseParams>,
    ) -> CallToolResult {
        let req = release::ReleaseRequest {
            repo: params.repo,
            tag: params.tag,
            include_prerelease: false,
            asset_regex: params.asset_regex,
            os: params.os,
            arch: params.arch,
            file_type: release::FileTypePreference::Any,
            extract: params.extract.unwrap_or(true),
            output_path: params.output_dir.or(self.download_path.clone()),
            cwd: false,
            bin_path: None,
            token: self.token.clone(),
            allow_prompt: false, // Ensure non-interactive
        };

        match release::download_release(req).await {
            Ok(res) => CallToolResult::structured(serde_json::json!({
                "owner": res.owner,
                "repo": res.repo,
                "tag": res.tag,
                "asset_name": res.asset_name,
                "download_path": res.download_path,
                "installed_binary": res.installed_binary,
                "extracted": res.extracted
            })),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to download release: {}", e)
            })),
        }
    }

    #[tool(description = "Search GitHub repositories by keyword.")]
    async fn search_repos(
        &self,
        Parameters(params): Parameters<SearchReposParams>,
    ) -> CallToolResult {
        let client_res = GitHubClient::new(self.token.clone());
        let client = match client_res {
            Ok(c) => c,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };

        match client.search_repositories(&params.query).await {
            Ok(repos) => CallToolResult::structured(serde_json::to_value(repos).unwrap()),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Search failed: {}", e)
            })),
        }
    }

    #[tool(description = "Read the raw text content of a single file from a repository.")]
    async fn read_file(&self, Parameters(params): Parameters<ReadFileParams>) -> CallToolResult {
        let gh_url = match GitHubUrl::parse(&params.url) {
            Ok(url) => url,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };
        let client = match GitHubClient::new_for_url(self.token.clone(), &gh_url) {
            Ok(c) => c,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };

        let raw_url = gh_url.raw_file_url_for_path(&params.path);
        match client.fetch_raw_content(&raw_url).await {
            Ok(content) => CallToolResult::structured(serde_json::json!({
                "path": params.path,
                "content": content
            })),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to read file: {}", e)
            })),
        }
    }

    #[tool(
        description = "Read a preview (first N bytes) of a file from a repository. Useful for peeking at large files."
    )]
    async fn read_file_preview(
        &self,
        Parameters(params): Parameters<ReadFilePreviewParams>,
    ) -> CallToolResult {
        let gh_url = match GitHubUrl::parse(&params.url) {
            Ok(url) => url,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };
        let client = match GitHubClient::new_for_url(self.token.clone(), &gh_url) {
            Ok(c) => c,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };

        let raw_url = gh_url.raw_file_url_for_path(&params.path);
        let max_bytes = params.max_bytes.unwrap_or(8192);
        match client.fetch_partial_content(&raw_url, max_bytes).await {
            Ok(content) => CallToolResult::structured(serde_json::json!({
                "path": params.path,
                "content": content,
                "preview_bytes": max_bytes
            })),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to preview file: {}", e)
            })),
        }
    }

    #[tool(description = "List all releases for a GitHub repository.")]
    async fn list_releases(
        &self,
        Parameters(params): Parameters<ListReleasesParams>,
    ) -> CallToolResult {
        let client = match GitHubClient::new(self.token.clone()) {
            Ok(c) => c,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };

        match client.fetch_releases(&params.owner, &params.repo).await {
            Ok(releases) => CallToolResult::structured(serde_json::to_value(releases).unwrap()),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to list releases: {}", e)
            })),
        }
    }

    #[tool(
        description = "Get repository metadata: default branch, platform type, and parsed URL components."
    )]
    async fn repo_info(&self, Parameters(params): Parameters<RepoInfoParams>) -> CallToolResult {
        let gh_url = match GitHubUrl::parse(&params.url) {
            Ok(url) => url,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };
        let client = match GitHubClient::new_for_url(self.token.clone(), &gh_url) {
            Ok(c) => c,
            Err(e) => {
                return CallToolResult::structured_error(
                    serde_json::json!({ "error": e.to_string() }),
                )
            }
        };

        match client.fetch_default_branch(&gh_url).await {
            Ok(default_branch) => CallToolResult::structured(serde_json::json!({
                "owner": gh_url.owner,
                "repo": gh_url.repo,
                "platform": format!("{:?}", gh_url.platform),
                "url_branch": gh_url.branch,
                "url_path": gh_url.path,
                "default_branch": default_branch
            })),
            Err(e) => CallToolResult::structured_error(serde_json::json!({
                "error": format!("Failed to fetch default branch: {}", e)
            })),
        }
    }
}

pub async fn run_mcp_server(token: Option<String>, download_path: Option<String>) -> Result<()> {
    let service = GhGrabMcp {
        token,
        download_path,
    };
    let server = service.serve(rmcp::transport::stdio()).await?;
    server.waiting().await?;
    Ok(())
}
