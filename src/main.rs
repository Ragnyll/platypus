use std::{process::Command};
use sysinfo::{Pid, PidExt, ProcessStatus, System, SystemExt, ProcessExt};
use tokio::{fs::File, io::AsyncWriteExt, time::Duration};

const DEFAULT_TICK_DURATION: u64 = 500; // ms

#[tokio::main]
async fn main() {
    let mut sys = System::new_all();
    let process_to_profile = Command::new("btm").spawn().expect("failed to start cmd");

    let pid = Pid::from_u32(process_to_profile.id());
    println!("Profiling proces: {pid:?}");
    gather_metric_on_timer(Duration::from_millis(DEFAULT_TICK_DURATION), &mut sys, &pid).await;
}

// it looks like the process goes to zombie while the profiler is still running, but could be
// sleeping at other times
async fn gather_metric_on_timer(duration: Duration, sys: &mut System, pid: &Pid) {
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
                    let output = p.memory();
                    log_output_to_file(output.to_string().as_bytes()).await;
                }
            },
            None => {
                // process not found, it must have exited with some unhandled status
                break 'MetricLoop;
            }
        };
    }
}

async fn log_output_to_file(bfr: &[u8]) {
    let mut file = File::open("profile.txt").await.unwrap();
    file.write_all(bfr).await.unwrap();
}
