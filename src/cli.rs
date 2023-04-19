use clap::{Parser, Subcommand};

const DEFAULT_TICK_DURATION: u64 = 500; // ms
                                        // TODO: This needs to be determined
const DEFAULT_PROFILE_LOG_PATH: &str = "platypus_profile";

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Profile {
        /// The path to the dir to output logs from the metrics gathering
        #[arg(default_value_t = String::from(DEFAULT_PROFILE_LOG_PATH), long)]
        log_dir: String,

        /// How often the memory monitor should gather metrics
        #[arg(default_value_t = DEFAULT_TICK_DURATION, long)]
        tick_duration: u64,

        /// The cmd to profile
        cmd: String,
    },
    Plot,
}
