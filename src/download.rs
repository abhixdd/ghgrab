use crate::github::{GitHubClient, RepoItem};
use anyhow::{Context, Result};
use futures::StreamExt;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub const DEFAULT_JOBS: usize = 4;

pub struct Downloader {
    client: GitHubClient,
    base_path: PathBuf,
    semaphore: Arc<Semaphore>,
}

impl Downloader {
    pub fn new(base_path: PathBuf, client: GitHubClient, jobs: usize) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Downloader {
            client,
            base_path,
            semaphore: Arc::new(Semaphore::new(jobs)),
        })
    }

    pub fn new_github(base_path: PathBuf, token: Option<String>, jobs: usize) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Downloader {
            client: GitHubClient::new(token)?,
            base_path,
            semaphore: Arc::new(Semaphore::new(jobs)),
        })
    }

    pub async fn download_items(
        &self,
        items: &[RepoItem],
        _repo_path: &str,
        progress_callback: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<Vec<String>> {
        let progress_callback = Arc::new(progress_callback);

        let results: Vec<Result<()>> = futures::stream::iter(items.iter().cloned())
            .map(|item| {
                let dest_path = self.base_path.join(&item.name);
                let cb = Arc::clone(&progress_callback);
                async move {
                    if item.is_file() {
                        self.download_file(&item, dest_path, cb.as_ref())
                            .await
                            .with_context(|| format!("Failed to download {}", item.name))
                    } else {
                        self.download_folder(&item, dest_path, cb.as_ref())
                            .await
                            .with_context(|| format!("Failed to download {}", item.name))
                    }
                }
            })
            .buffer_unordered(usize::MAX)
            .collect()
            .await;

        Ok(results
            .into_iter()
            .filter_map(|r| r.err())
            .map(|e| format!("{:#}", e))
            .collect())
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

        let _permit = Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .expect("semaphore closed");

        let content = self
            .client
            .fetch_bytes(download_url)
            .await
            .context("Failed to download file")?;

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&dest_path, content)
            .with_context(|| format!("Failed to write file: {:?}", dest_path))?;

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

            let contents = {
                let _permit = Arc::clone(&self.semaphore)
                    .acquire_owned()
                    .await
                    .expect("semaphore closed");
                self.client.fetch_contents(&item.url).await?
                // permit released here when _permit drops
            };

            let results: Vec<Result<()>> = futures::stream::iter(contents)
                .map(|sub_item| {
                    let sub_dest_path = dest_path.join(&sub_item.name);
                    async move {
                        if sub_item.is_file() {
                            self.download_file(&sub_item, sub_dest_path, progress_callback)
                                .await
                                .with_context(|| format!("Failed to download {}", sub_item.name))
                        } else {
                            self.download_folder(&sub_item, sub_dest_path, progress_callback)
                                .await
                                .with_context(|| format!("Failed to download {}", sub_item.name))
                        }
                    }
                })
                .buffer_unordered(usize::MAX)
                .collect()
                .await;

            let errors: Vec<String> = results
                .into_iter()
                .filter_map(|r| r.err())
                .map(|e| format!("{:#}", e))
                .collect();

            if !errors.is_empty() {
                return Err(anyhow::anyhow!("{}", errors.join("\n")));
            }

            Ok(())
        })
    }
}
