use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub github_token: Option<String>,
    pub download_path: Option<String>,
    pub icon_mode: Option<crate::ui::IconMode>,
}

impl Config {
    pub fn validate_path(path: &str) -> Result<()> {
        let p = PathBuf::from(path);
        if !p.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path));
        }
        if !p.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {}", path));
        }

        let metadata = fs::metadata(&p).context("Failed to get metadata for path")?;
        if metadata.permissions().readonly() {
            return Err(anyhow::anyhow!("Path is read-only: {}", path));
        }

        Ok(())
    }

    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&config_path).context("Failed to read config file")?;

        let config: Config =
            serde_json::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(config_path, content).context("Failed to write config file")?;

        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf> {
    let mut path = dirs::config_dir().context("Could not find config directory")?;
    path.push("ghgrab");
    path.push("config.json");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.github_token.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            github_token: Some("test_token".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test_token"));

        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.github_token, Some("test_token".to_string()));
    }
}
