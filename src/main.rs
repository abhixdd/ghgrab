mod download;
mod github;
mod ui;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let initial_url = if args.len() > 1 {
        Some(args[1].clone())
    } else {
        None
    };
    
    ui::run_tui(initial_url).await?;
    Ok(())
}
