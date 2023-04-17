mod cli;
mod plot;
mod profiler;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Profile {
            log_path,
            tick_duration,
            cmd,
        } => {
            profiler::profile(&log_path, tick_duration, &cmd).await?;
        }
    }

    Ok(())
}
