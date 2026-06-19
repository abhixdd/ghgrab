use crate::agent::{self, AgentEnvelope};
use crate::config::Config;
use crate::release::{self, FileTypePreference, ReleaseRequest, ReleaseSelectionCancelled};
use crate::ui;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::process::Command;

const GHGRAB_GITHUB_TOKEN_ENV: &str = "GHGRAB_GITHUB_TOKEN";
const GITHUB_TOKEN_ENV: &str = "GITHUB_TOKEN";

#[derive(Parser)]
#[command(name = "ghgrab", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    url: Option<String>,

    #[arg(long, help = "Download files to current directory")]
    cwd: bool,

    #[arg(long, help = "Download files directly into target without repo folder")]
    no_folder: bool,

    #[arg(
        long,
        help = "One-time GitHub token (not stored). Use `auto`/`gh` to read from GitHub CLI"
    )]
    token: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Config {
        #[command(subcommand)]
        action: ConfigCommand,
    },
    Agent {
        #[command(subcommand)]
        action: AgentCommand,
    },
    #[command(alias = "rel")]
    Release {
        repo: String,
        #[arg(long, help = "Download a specific release tag")]
        tag: Option<String>,
        #[arg(long, help = "Allow selecting prereleases when tag is not specified")]
        prerelease: bool,
        #[arg(long, help = "Regex for matching a specific release asset")]
        asset_regex: Option<String>,
        #[arg(long, help = "Override detected operating system")]
        os: Option<String>,
        #[arg(long, help = "Override detected architecture")]
        arch: Option<String>,
        #[arg(long, value_enum, default_value_t = ReleaseFileType::Any, help = "Preferred artifact type")]
        file_type: ReleaseFileType,
        #[arg(long, help = "Extract archive assets after download")]
        extract: bool,
        #[arg(long, help = "Custom output directory for this run")]
        out: Option<String>,
        #[arg(long, help = "Install the selected binary into the provided directory")]
        bin_path: Option<String>,
        #[arg(long, help = "Download files to current directory")]
        cwd: bool,
        #[arg(
            long,
            help = "One-time GitHub token for this run. Use `auto`/`gh` to read from GitHub CLI"
        )]
        token: Option<String>,
    },
    /// Start an MCP server over stdio, or install ghgrab into an MCP client
    Mcp {
        #[arg(long, help = "One-time GitHub token for MCP sessions")]
        token: Option<String>,

        #[arg(
            long,
            help = "Auto-configure ghgrab in an MCP client (Claude Desktop, Cursor, VS Code, Windsurf)"
        )]
        install: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Set {
        #[command(subcommand)]
        target: SetTarget,
    },
    Unset {
        #[command(subcommand)]
        target: UnsetTarget,
    },
    List,
}

#[derive(Subcommand)]
enum SetTarget {
    Token { value: String },
    Path { value: String },
}

#[derive(Subcommand)]
enum UnsetTarget {
    Token,
    Path,
}

#[derive(Subcommand)]
enum AgentCommand {
    Tree {
        url: String,
        #[arg(
            long,
            help = "One-time GitHub token for this run. Use `auto`/`gh` to read from GitHub CLI"
        )]
        token: Option<String>,
    },
    Download {
        url: String,
        #[arg(help = "Repo paths to download")]
        paths: Vec<String>,
        #[arg(long, help = "Download the entire repository")]
        repo: bool,
        #[arg(long, help = "Download a specific subtree path")]
        subtree: Option<String>,
        #[arg(long, help = "Download files to current directory")]
        cwd: bool,
        #[arg(long, help = "Download files directly into target without repo folder")]
        no_folder: bool,
        #[arg(long, help = "Custom output directory for this run")]
        out: Option<String>,
        #[arg(
            long,
            help = "One-time GitHub token for this run. Use `auto`/`gh` to read from GitHub CLI"
        )]
        token: Option<String>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ReleaseFileType {
    Any,
    Archive,
    Binary,
}

impl From<ReleaseFileType> for FileTypePreference {
    fn from(value: ReleaseFileType) -> Self {
        match value {
            ReleaseFileType::Any => FileTypePreference::Any,
            ReleaseFileType::Archive => FileTypePreference::Archive,
            ReleaseFileType::Binary => FileTypePreference::Binary,
        }
    }
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let default_config = Config::load().unwrap_or_default();

    match cli.command {
        Some(Commands::Config { action }) => match action {
            ConfigCommand::Set { target } => match target {
                SetTarget::Token { value } => {
                    let mut config = Config::load()?;
                    config.github_token = Some(value);
                    config.save()?;
                    println!("GitHub token saved successfully!");
                }
                SetTarget::Path { value } => {
                    if let Err(e) = Config::validate_path(&value) {
                        eprintln!("Invalid path: {}", e);
                    } else {
                        let mut config = Config::load()?;
                        config.download_path = Some(value);
                        config.save()?;
                        println!("Download path saved successfully!");
                    }
                }
            },
            ConfigCommand::Unset { target } => match target {
                UnsetTarget::Token => {
                    let mut config = Config::load()?;
                    config.github_token = None;
                    config.save()?;
                    println!("GitHub token removed successfully!");
                }
                UnsetTarget::Path => {
                    let mut config = Config::load()?;
                    config.download_path = None;
                    config.save()?;
                    println!("Download path removed successfully!");
                }
            },
            ConfigCommand::List => {
                let config = default_config;
                if let Some(token) = &config.github_token {
                    let masked = if token.len() > 8 {
                        format!("{}...{}", &token[..4], &token[token.len() - 4..])
                    } else {
                        "********".to_string()
                    };
                    println!("GitHub Token:  {}", masked);
                } else {
                    println!("GitHub Token:  Not set");
                }

                if let Some(path) = &config.download_path {
                    println!("Download Path: {}", path);
                } else {
                    println!("Download Path: Not set (using default Downloads folder)");
                }
            }
        },
        Some(Commands::Agent { action }) => match action {
            AgentCommand::Tree { url, token } => {
                let token = resolve_github_token(token, default_config.github_token.clone())?;
                let result = agent::fetch_tree(&url, token).await;
                print_agent_json("tree", result)?;
            }
            AgentCommand::Download {
                url,
                paths,
                repo,
                subtree,
                cwd,
                no_folder,
                out,
                token,
            } => {
                let token = resolve_github_token(token, default_config.github_token.clone())?;
                let out = out.or(default_config.download_path.clone());
                let selected_paths = build_download_request(paths, repo, subtree);
                let result = match selected_paths {
                    Ok(selected_paths) => {
                        agent::download_paths(&url, token, &selected_paths, out, cwd, no_folder)
                            .await
                    }
                    Err(error) => Err(error),
                };
                print_agent_json("download", result)?;
            }
        },
        Some(Commands::Release {
            repo,
            tag,
            prerelease,
            asset_regex,
            os,
            arch,
            file_type,
            extract,
            out,
            bin_path,
            cwd,
            token,
        }) => {
            let token = resolve_github_token(token, default_config.github_token.clone())?;
            let result = match release::download_release(ReleaseRequest {
                repo,
                tag,
                include_prerelease: prerelease,
                asset_regex,
                os,
                arch,
                file_type: file_type.into(),
                extract,
                output_path: out.or(default_config.download_path.clone()),
                cwd,
                bin_path,
                token,
                allow_prompt: true,
            })
            .await
            {
                Ok(result) => result,
                Err(error) if error.downcast_ref::<ReleaseSelectionCancelled>().is_some() => {
                    println!("Cancelled.");
                    return Ok(());
                }
                Err(error) => return Err(error),
            };

            println!("Downloaded release asset: {}", result.asset_name);
            println!("Release tag: {}", result.tag);
            println!("Saved to: {}", result.download_path);
            if let Some(installed_binary) = result.installed_binary {
                println!("Installed binary: {}", installed_binary);
            }
        }
        Some(Commands::Mcp { token, install }) => {
            if install {
                crate::mcp_install::install_mcp_client(default_config.github_token.clone())?;
            } else {
                let token = resolve_github_token(token, default_config.github_token.clone())?;
                crate::mcp::run_mcp_server(token, default_config.download_path.clone()).await?;
            }
        }
        None => {
            let url = cli.url;
            let download_path = default_config.download_path.clone();
            let token = resolve_github_token(cli.token, default_config.github_token.clone())?;
            let initial_icon_mode = default_config.icon_mode.unwrap_or(ui::IconMode::Emoji);

            let final_icon_mode = ui::run_tui(
                url,
                token,
                download_path,
                cli.cwd,
                cli.no_folder,
                initial_icon_mode,
            )
            .await?;
            if final_icon_mode != initial_icon_mode {
                let mut config = Config::load().unwrap_or_default();
                config.icon_mode = Some(final_icon_mode);
                let _ = config.save();
            }
        }
    }

    Ok(())
}

pub(crate) fn resolve_github_token(
    cli_token: Option<String>,
    config_token: Option<String>,
) -> Result<Option<String>> {
    if let Some(token) = normalize_token(cli_token) {
        if is_auto_token_keyword(&token) {
            return resolve_github_token_from_gh_cli(true);
        }
        return Ok(Some(token));
    }

    if let Some(token) = resolve_github_token_from_env() {
        if is_auto_token_keyword(&token) {
            if let Some(auto_token) = resolve_github_token_from_gh_cli(false)? {
                return Ok(Some(auto_token));
            }
        } else {
            return Ok(Some(token));
        }
    }

    if let Some(token) = normalize_token(config_token) {
        return Ok(Some(token));
    }

    resolve_github_token_from_gh_cli(false)
}

fn resolve_github_token_from_env() -> Option<String> {
    [GHGRAB_GITHUB_TOKEN_ENV, GITHUB_TOKEN_ENV]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .and_then(|token| normalize_token(Some(token)))
}

fn normalize_token(token: Option<String>) -> Option<String> {
    token.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn is_auto_token_keyword(value: &str) -> bool {
    value.eq_ignore_ascii_case("auto") || value.eq_ignore_ascii_case("gh")
}

fn resolve_github_token_from_gh_cli(strict: bool) -> Result<Option<String>> {
    let output = match Command::new("gh").args(["auth", "token"]).output() {
        Ok(output) => output,
        Err(error) => {
            if strict {
                anyhow::bail!(
                    "Failed to execute `gh auth token`: {}. Ensure GitHub CLI is installed and available in PATH.",
                    error
                );
            }
            return Ok(None);
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.trim();
        if strict {
            if reason.is_empty() {
                anyhow::bail!(
                    "`gh auth token` failed. Ensure you are logged in with `gh auth login`."
                );
            }

            anyhow::bail!(
                "`gh auth token` failed: {}. Ensure you are logged in with `gh auth login`.",
                reason
            );
        }
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tokens = parse_gh_auth_token_stdout(&stdout);

    if tokens.is_empty() {
        if strict {
            anyhow::bail!(
                "No token found in `gh auth token` output. Ensure you are authenticated with GitHub CLI."
            );
        }
        return Ok(None);
    }

    if tokens.len() == 1 {
        eprintln!("Found token via GitHub CLI.");
    } else {
        eprintln!(
            "Found multiple tokens via GitHub CLI output ({}). Using one token.",
            tokens.len()
        );
    }

    Ok(Some(tokens[0].clone()))
}

fn parse_gh_auth_token_stdout(stdout: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tokens = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if seen.insert(trimmed.to_string()) {
            tokens.push(trimmed.to_string());
        }
    }

    tokens
}

fn build_download_request(
    paths: Vec<String>,
    repo: bool,
    subtree: Option<String>,
) -> Result<Vec<String>> {
    if repo && (!paths.is_empty() || subtree.is_some()) {
        anyhow::bail!("--repo cannot be combined with paths or --subtree");
    }

    if repo {
        return Ok(Vec::new());
    }

    if let Some(subtree) = subtree {
        if !paths.is_empty() {
            anyhow::bail!("--subtree cannot be combined with positional paths");
        }
        return Ok(vec![subtree]);
    }

    Ok(paths)
}

fn print_agent_json<T: serde::Serialize>(command: &str, result: anyhow::Result<T>) -> Result<()> {
    let payload = match result {
        Ok(data) => AgentEnvelope::success(command, data),
        Err(error) => {
            AgentEnvelope::<T>::error(command, agent::classify_error(&error), error.to_string())
        }
    };

    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_auto_token_keyword;
    use super::parse_gh_auth_token_stdout;

    #[test]
    fn parse_gh_auth_token_stdout_single_token() {
        let parsed = parse_gh_auth_token_stdout("ghp_single_token\n");
        assert_eq!(parsed, vec!["ghp_single_token".to_string()]);
    }

    #[test]
    fn parse_gh_auth_token_stdout_multiple_tokens() {
        let parsed = parse_gh_auth_token_stdout("ghp_one\nghp_two\n");
        assert_eq!(parsed, vec!["ghp_one".to_string(), "ghp_two".to_string()]);
    }

    #[test]
    fn parse_gh_auth_token_stdout_deduplicates_tokens() {
        let parsed = parse_gh_auth_token_stdout("ghp_same\nghp_same\n\n");
        assert_eq!(parsed, vec!["ghp_same".to_string()]);
    }

    #[test]
    fn auto_token_keywords_are_detected_case_insensitively() {
        assert!(is_auto_token_keyword("auto"));
        assert!(is_auto_token_keyword("AUTO"));
        assert!(is_auto_token_keyword("gh"));
        assert!(is_auto_token_keyword("Gh"));
        assert!(!is_auto_token_keyword("ghp_123"));
    }
}
