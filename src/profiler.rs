use anyhow::{anyhow, Result};
use chrono::Local;
use std::process::Command;
use sysinfo::{Pid, PidExt, ProcessExt, ProcessStatus, System, SystemExt};
use tokio::{fs::File, io::AsyncWriteExt, time::Duration};

pub async fn profile(log_path: &str, tick_duration: u64, cmd: &str) -> Result<()> {
    let mut sys = System::new_all();

    let file = File::create(log_path)
        .await
        .expect("Unable to open profile log file {}");
    let cmd = build_cmd_with_args(cmd);
    let process_to_profile = cmd?.spawn().expect("failed to start cmd");

    let pid = Pid::from_u32(process_to_profile.id());
    gather_metric_on_timer(Duration::from_millis(tick_duration), &mut sys, &pid, file).await?;

    Ok(())
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
                    // process has been zombied, that means that the child finished but parent has not
                    // exited in this case
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
