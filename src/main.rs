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
            log_dir,
            tick_duration,
            cmd,
        } => {
            profiler::profile(&log_dir, tick_duration, &cmd).await?;
        }
        Commands::Plot => {
            plot::plot()?;
        }
    }

    Ok(())
}
