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
    jobs: usize,
    semaphore: Arc<Semaphore>,
}

impl Downloader {
    pub fn new(base_path: PathBuf, client: GitHubClient, jobs: usize) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Downloader {
            client,
            base_path,
            jobs,
            semaphore: Arc::new(Semaphore::new(jobs)),
        })
    }

    pub fn new_github(base_path: PathBuf, token: Option<String>, jobs: usize) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Downloader {
            client: GitHubClient::new(token)?,
            base_path,
            jobs,
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

        let nested: Vec<Vec<anyhow::Error>> = futures::stream::iter(items.iter().cloned())
            .map(|item| {
                let dest_path = self.base_path.join(&item.name);
                let cb = Arc::clone(&progress_callback);
                async move {
                    if item.is_file() {
                        match self.download_file(&item, dest_path, cb.as_ref()).await {
                            Ok(()) => vec![],
                            Err(e) => vec![e.context(format!("Failed to download {}", item.name))],
                        }
                    } else {
                        self.download_folder(&item, dest_path, cb.as_ref()).await
                    }
                }
            })
            .buffer_unordered(self.jobs)
            .collect()
            .await;

        // Errors carry their full anyhow context chain until this single boundary,
        // where they're rendered to strings for the CLI/agent output.
        Ok(nested
            .into_iter()
            .flatten()
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<anyhow::Error>> + Send + 'a>> {
        Box::pin(async move {
            progress_callback(format!("Scanning folder: {}", item.name));

            if let Err(e) = fs::create_dir_all(&dest_path) {
                return vec![
                    anyhow::Error::new(e).context(format!("Failed to create folder {}", item.name))
                ];
            }

            let contents = {
                let _permit = Arc::clone(&self.semaphore)
                    .acquire_owned()
                    .await
                    .expect("semaphore closed");
                match self.client.fetch_contents(&item.url).await {
                    Ok(contents) => contents,
                    Err(e) => {
                        return vec![e.context(format!("Failed to list folder {}", item.name))]
                    }
                }
                // permit released here when _permit drops
            };

            let nested: Vec<Vec<anyhow::Error>> = futures::stream::iter(contents)
                .map(|sub_item| {
                    let sub_dest_path = dest_path.join(&sub_item.name);
                    async move {
                        if sub_item.is_file() {
                            match self
                                .download_file(&sub_item, sub_dest_path, progress_callback)
                                .await
                            {
                                Ok(()) => vec![],
                                Err(e) => {
                                    vec![e.context(format!("Failed to download {}", sub_item.name))]
                                }
                            }
                        } else {
                            self.download_folder(&sub_item, sub_dest_path, progress_callback)
                                .await
                        }
                    }
                })
                .buffer_unordered(self.jobs)
                .collect()
                .await;

            nested.into_iter().flatten().collect()
        })
    }
}
