use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    ghgrab::cli::run().await
}
