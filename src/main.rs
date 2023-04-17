use anyhow::{anyhow, Result};
use chrono::Local;
use clap::Parser;
use std::process::Command;
use sysinfo::{Pid, PidExt, ProcessStatus, System, SystemExt, ProcessExt};
use tokio::{fs::File, io::AsyncWriteExt, time::Duration};

const DEFAULT_TICK_DURATION: u64 = 500; // ms
const DEFAULT_PROFILE_LOG_PATH: &str = "profile.log";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// The path to where to output logs from the metrics gathering
    #[arg(default_value_t = String::from(DEFAULT_PROFILE_LOG_PATH), long)]
    log_path: String,

    /// How often the memory monitor should gather metrics
    #[arg(default_value_t = DEFAULT_TICK_DURATION, long)]
    tick_duration: u64,

    /// The cmd to profile
    cmd: String,
}

/// builds a command with args seperateing the flags by whitespace
fn build_cmd_with_args(cmd_string: &str) -> Result<Command> {
    let sp: Vec<String> = cmd_string.split(' ').map(String::from).collect();
    // in practice this is not possible
    if sp.is_empty() {
        return Err(anyhow!("Cannot run a nonexistent command"));
    }
    let mut cmd = Command::new(&sp[0]);
    cmd.args(&sp[1..]);
    Ok(cmd)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut sys = System::new_all();
    let file = File::create(&cli.log_path)
        .await
        .expect("Unable to open profile log file {}");
    let cmd = build_cmd_with_args(&cli.cmd);
    let process_to_profile = cmd?.spawn().expect("failed to start cmd");

    let pid = Pid::from_u32(process_to_profile.id());
    gather_metric_on_timer(
        Duration::from_millis(cli.tick_duration),
        &mut sys,
        &pid,
        file,
    )
    .await?;
    Ok(())
}

// it looks like the process goes to zombie while the profiler is still running, but could be
// sleeping at other times
async fn gather_metric_on_timer(
    duration: Duration,
    sys: &mut System,
    pid: &Pid,
    mut file: File,
) -> Result<()> {
    'MetricLoop: loop {
        tokio::time::sleep(duration).await;
        sys.refresh_processes();
        match sys.process(*pid) {
            Some(p) => match p.status() {
                ProcessStatus::Zombie => {
                    // process has been zombied, that means that the chiled finished but has not
                    // exited
                    break 'MetricLoop;
                }
                _ => {
                    let dt = Local::now();
                    let output = p.memory().to_string();
                    file.write_all(format!("{dt}: {output}\n").as_bytes())
                        .await?;
                }
            },
            None => {
                // process not found, it must have exited with some unhandled status
                break 'MetricLoop;
            }
        };
    }

    file.flush().await?;
    Ok(())
}
