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
            let parent = p.parent().ok_or_else(|| {
                anyhow::anyhow!("Path does not exist and has no parent directory: {}", path)
            })?;

            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Parent directory does not exist for path: {}",
                    path
                ));
            }
            if !parent.is_dir() {
                return Err(anyhow::anyhow!(
                    "Parent path is not a directory for path: {}",
                    path
                ));
            }

            let metadata =
                fs::metadata(parent).context("Failed to get metadata for parent directory")?;
            if metadata.permissions().readonly() {
                return Err(anyhow::anyhow!(
                    "Parent directory is read-only for path: {}",
                    path
                ));
            }

            return Ok(());
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

fn get_log_path() -> Result<PathBuf> {
    let mut path = dirs::config_dir().context("Could not find config directory")?;
    path.push("ghgrab");
    path.push("ghgrab.log");
    Ok(path)
}

pub fn log_error(message: &str) {
    let Ok(log_path) = get_log_path() else {
        return;
    };
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    use std::io::Write;
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "ghgrab-config-{}-{}-{}",
            prefix,
            std::process::id(),
            nanos
        ))
    }

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

    #[test]
    fn test_validate_path_allows_new_directory_when_parent_exists() {
        let base = temp_dir("base");
        fs::create_dir_all(&base).unwrap();
        let target = base.join("downloads");

        let result = Config::validate_path(target.to_str().unwrap());

        let _ = fs::remove_dir_all(&base);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_rejects_missing_parent_directory() {
        let base = temp_dir("missing-parent");
        let target = base.join("downloads");

        let result = Config::validate_path(target.to_str().unwrap());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Parent directory does not exist"));
    }
}
