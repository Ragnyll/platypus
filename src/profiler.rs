use anyhow::{anyhow, Result};
use chrono::Local;
use std::{collections::HashMap, path::PathBuf, process::Command, sync::Arc};
use sysinfo::{Pid, PidExt, ProcessExt, ProcessStatus, System, SystemExt};
use tokio::{fs, fs::File, io::AsyncWriteExt, sync::Mutex, time::Duration};

const MEMORY_FILE: (&str, &str) = ("memory.log", "memory");
const CPU_FILE: (&str, &str) = ("cpu.log", "cpu");

pub async fn profile(log_path: &str, tick_duration: u64, cmd: &str) -> Result<()> {
    let mut metric_files = prepare_output_paths(log_path).await?;
    let cmd = build_cmd_with_args(cmd);
    let process_to_profile = cmd?.spawn()?;

    let pid = Pid::from_u32(process_to_profile.id());
    gather_metrics(
        Duration::from_millis(tick_duration),
        &pid,
        &mut metric_files,
    )
    .await?;
    //gather_metric_on_timer(Duration::from_millis(tick_duration), &mut sys, &pid, file).await?;

    Ok(())
}

/// Creates files and handle for all the metrics to be gathered. This is returned as a map of
/// metric name to [`File`] handle.
async fn prepare_output_paths(log_dir: &str) -> Result<HashMap<String, File>> {
    // make sure that the log_dir does not already exist, but as a file
    if fs::try_exists(log_dir).await? && fs::metadata(log_dir).await?.is_file() {
        return Err(anyhow!("log_dir cannot exist as a file!"));
    }
    fs::create_dir_all(log_dir).await?;

    let mut metric_file_handles = HashMap::new();
    let metric_log_path: PathBuf = [log_dir, MEMORY_FILE.0].iter().collect();

    metric_file_handles.insert(
        String::from(MEMORY_FILE.1),
        File::create(metric_log_path).await?,
    );

    let metric_log_path: PathBuf = [log_dir, CPU_FILE.0].iter().collect();
    metric_file_handles.insert(
        String::from(CPU_FILE.1),
        File::create(metric_log_path).await?,
    );

    Ok(metric_file_handles)
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

async fn gather_metrics(
    duration: Duration,
    pid: &Pid,
    output_files: &mut HashMap<String, File>,
) -> Result<()> {
    // using an async mutex because you need to wait across locks so there is not block on
    // obtaining a lock
    let sys = Arc::new(Mutex::new(System::new_all()));
    // if any of the futures finish then that means that the proes being monitored has finished.
    tokio::select! {
        _ = gather_metric_on_timer(duration, sys.clone(), &pid, output_files.remove(MEMORY_FILE.1).unwrap()) => {},
        _ = gather_metric_on_timer(duration, sys, &pid, output_files.remove(CPU_FILE.1).unwrap()) => {},

    }

    Ok(())
}

async fn gather_metric_on_timer(
    duration: Duration,
    sys: Arc<Mutex<System>>,
    pid: &Pid,
    mut file: File,
) -> Result<()> {
    'MetricLoop: loop {
        tokio::time::sleep(duration).await;
        let mut sys = sys.lock().await;
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
