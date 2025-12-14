use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use crate::github::{GitHubClient, RepoItem};

pub struct Downloader {
    client: GitHubClient,
    base_path: PathBuf,
}

impl Downloader {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Downloader {
            client: GitHubClient::new()?,
            base_path,
        })
    }

    pub async fn download_items(
        &self,
        items: &[RepoItem],
        _repo_path: &str, 
        progress_callback: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        
        for item in items {
            if !item.selected {
                continue;
            }

            let dest_path = self.base_path.join(&item.name);

            let result = if item.is_file() {
                self.download_file(item, dest_path, &progress_callback).await
            } else {
                self.download_folder(item, dest_path, &progress_callback).await
            };

            if let Err(e) = result {
                errors.push(format!("Failed to download {}: {}", item.name, e));
            }
        }
        Ok(errors)
    }

    async fn download_file(
        &self,
        item: &RepoItem,
        dest_path: PathBuf,
        progress_callback: &(impl Fn(String) + Send + Sync),
    ) -> Result<()> {
        let download_url = item
            .actual_download_url()
            .context("No download URL for file")?;

        let lfs_indicator = if item.is_lfs() { " [LFS]" } else { "" };
        progress_callback(format!("Downloading{}: {}", lfs_indicator, item.name));

        let response = reqwest::get(download_url)
            .await
            .context("Failed to download file")?;
        
        let content = response
            .bytes()
            .await
            .context("Failed to read file content")?;

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&dest_path, content)
            .context(format!("Failed to write file: {:?}", dest_path))?;

        Ok(())
    }

    fn download_folder<'a>(
        &'a self,
        item: &'a RepoItem,
        dest_path: PathBuf,
        progress_callback: &'a (impl Fn(String) + Send + Sync),
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            progress_callback(format!("Scanning folder: {}", item.name));
            
            fs::create_dir_all(&dest_path)?;
            let contents = self.client.fetch_contents(&item.url).await?;

            for sub_item in contents {
                let sub_dest_path = dest_path.join(&sub_item.name);
                
                if sub_item.is_file() {
                    self.download_file(&sub_item, sub_dest_path, progress_callback).await?;
                } else {
                    self.download_folder(&sub_item, sub_dest_path, progress_callback).await?;
                }
            }
            Ok(())
        })
    }
}
