use anyhow::Result;
use clap::{Parser, Subcommand};
use ghgrab::agent::{self, AgentEnvelope};
use ghgrab::config::Config;

use ghgrab::ui;

const GHGRAB_GITHUB_TOKEN_ENV: &str = "GHGRAB_GITHUB_TOKEN";
const GITHUB_TOKEN_ENV: &str = "GITHUB_TOKEN";

#[derive(Parser)]
#[command(name = "ghgrab", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    url: Option<String>,

    #[arg(long, help = "Download files to current directory")]
    cwd: bool,

    #[arg(long, help = "Download files directly into target without repo folder")]
    no_folder: bool,

    #[arg(long, help = "One-time GitHub token (not stored)")]
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
        #[arg(long, help = "One-time GitHub token for this run")]
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
        #[arg(long, help = "One-time GitHub token for this run")]
        token: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let default_config = Config::load().unwrap_or_default();

    match cli.command {
        Some(Commands::Config { action }) => match action {
            ConfigCommand::Set { target } => match target {
                SetTarget::Token { value } => {
                    let mut config = Config::load()?;
                    config.github_token = Some(value);
                    config.save()?;
                    println!("✅ GitHub token saved successfully!");
                }
                SetTarget::Path { value } => {
                    if let Err(e) = Config::validate_path(&value) {
                        eprintln!("❌ Invalid path: {}", e);
                    } else {
                        let mut config = Config::load()?;
                        config.download_path = Some(value);
                        config.save()?;
                        println!("✅ Download path saved successfully!");
                    }
                }
            },
            ConfigCommand::Unset { target } => match target {
                UnsetTarget::Token => {
                    let mut config = Config::load()?;
                    config.github_token = None;
                    config.save()?;
                    println!("✅ GitHub token removed successfully!");
                }
                UnsetTarget::Path => {
                    let mut config = Config::load()?;
                    config.download_path = None;
                    config.save()?;
                    println!("✅ Download path removed successfully!");
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
                let token = resolve_github_token(token, default_config.github_token.clone());
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
                let token = resolve_github_token(token, default_config.github_token.clone());
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
        None => {
            let url = cli.url;

            let download_path = default_config.download_path.clone();

            let token = resolve_github_token(cli.token, default_config.github_token.clone());
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

fn resolve_github_token(cli_token: Option<String>, config_token: Option<String>) -> Option<String> {
    normalize_token(cli_token)
        .or_else(resolve_github_token_from_env)
        .or_else(|| normalize_token(config_token))
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
