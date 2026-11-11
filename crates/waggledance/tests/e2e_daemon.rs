use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn scratch_home(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "waggledance-e2e-daemon-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(".waggledance"))
        .expect("create scratch HOME/.waggledance dir");
    dir
}

fn lock_path(home: &Path) -> PathBuf {
    home.join(".waggledance").join("daemon.lock")
}

fn write_lock(home: &Path, pid: u32) {
    let body = serde_json::json!({
        "pid": pid,
        "started_at": "2026-08-06T00:00:00Z",
    });
    std::fs::write(lock_path(home), serde_json::to_vec_pretty(&body).unwrap())
        .expect("write daemon.lock");
}

fn dead_pid() -> u32 {
    let mut child = if cfg!(windows) {
        Command::new("cmd").args(["/C", "exit 0"]).spawn()
    } else {
        Command::new("true").spawn()
    }
    .expect("spawn short-lived helper process");
    let pid = child.id();
    child.wait().expect("wait for short-lived helper to exit");
    pid
}

fn wait_for_lock(home: &Path, timeout: Duration) -> serde_json::Value {
    let path = lock_path(home);
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(&text) {
                return info;
            }
        }
        if Instant::now() >= deadline {
            panic!(
                "daemon.lock never appeared/parsed within {timeout:?} at {}",
                path.display()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn run_with_timeout(bin: &str, args: &[&str], home: &Path, timeout: Duration) -> Option<Output> {
    let mut child = Command::new(bin)
        .args(args)
        .env("HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn waggledance");

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut s) = child.stdout.take() {
                    let _ = s.read_to_end(&mut stdout);
                }
                if let Some(mut s) = child.stderr.take() {
                    let _ = s.read_to_end(&mut stderr);
                }
                return Some(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

fn spawn_serve(bin: &str, home: &Path) -> ChildGuard {
    let child = Command::new(bin)
        .arg("serve")
        .env("HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn waggledance serve");
    ChildGuard(child)
}

#[cfg(target_os = "linux")]
fn listening_tcp_inodes() -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(text) = std::fs::read_to_string(table) else {
            continue;
        };
        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() > 9 && cols[3] == "0A" {
                out.insert(cols[9].to_string());
            }
        }
    }
    out
}

#[cfg(target_os = "linux")]
fn process_listens_on_tcp(pid: u32) -> bool {
    let listening = listening_tcp_inodes();
    let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
        return false;
    };
    fds.flatten()
        .filter_map(|fd| std::fs::read_link(fd.path()).ok())
        .filter_map(|target| {
            let t = target.to_string_lossy().into_owned();
            t.strip_prefix("socket:[")
                .and_then(|rest| rest.strip_suffix(']'))
                .map(str::to_string)
        })
        .any(|inode| listening.contains(&inode))
}

#[test]
fn stop_returns_promptly_against_a_stale_lock_and_clears_it() {
    let bin = env!("CARGO_BIN_EXE_waggledance");
    let home = scratch_home("stale");
    let pid = dead_pid();
    write_lock(&home, pid);

    let bound = Duration::from_secs(5);
    let output = run_with_timeout(bin, &["stop"], &home, bound)
        .unwrap_or_else(|| panic!("waggledance stop did not return within {bound:?}"));

    assert!(
        output.status.success(),
        "waggledance stop exited non-zero: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !lock_path(&home).exists(),
        "stale lock should be cleared after stop"
    );
}

#[test]
fn stop_with_no_lock_reports_no_daemon_running() {
    let bin = env!("CARGO_BIN_EXE_waggledance");
    let home = scratch_home("no-lock");

    let output = run_with_timeout(bin, &["stop"], &home, Duration::from_secs(5))
        .expect("waggledance stop did not return promptly with no lock at all");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No daemon running."),
        "expected 'No daemon running.', got: {stdout}"
    );
}

#[cfg(unix)]
#[test]
fn stop_does_not_kill_an_unrelated_live_process_named_by_a_stale_lock() {
    let bin = env!("CARGO_BIN_EXE_waggledance");
    let home = scratch_home("unrelated-pid");

    let unrelated = Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn unrelated live process");
    let unrelated_pid = unrelated.id();
    let mut guard = ChildGuard(unrelated);
    write_lock(&home, unrelated_pid);

    let output = run_with_timeout(bin, &["stop"], &home, Duration::from_secs(5))
        .expect("waggledance stop did not return promptly against an unrelated-pid lock");
    assert!(output.status.success());

    assert!(
        guard
            .0
            .try_wait()
            .expect("try_wait on unrelated process")
            .is_none(),
        "the unrelated live process (pid {unrelated_pid}) must not have been killed"
    );
    assert!(
        !lock_path(&home).exists(),
        "the stale lock should still be cleared even though nothing was killed"
    );
}

#[test]
fn serve_writes_the_lock_status_reports_it_running_and_stop_ends_it() {
    let bin = env!("CARGO_BIN_EXE_waggledance");
    let home = scratch_home("serve");

    let mut guard = spawn_serve(bin, &home);
    let info = wait_for_lock(&home, Duration::from_secs(10));
    assert_eq!(info["pid"], guard.0.id());

    #[cfg(target_os = "linux")]
    assert!(
        !process_listens_on_tcp(guard.0.id()),
        "the headless daemon must not hold a listening TCP socket"
    );

    let status = run_with_timeout(bin, &["status", "--json"], &home, Duration::from_secs(5))
        .expect("waggledance status did not return promptly");
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["running"], true, "{status}");
    assert_eq!(status["pid"], guard.0.id(), "{status}");

    let output = run_with_timeout(bin, &["stop"], &home, Duration::from_secs(5))
        .expect("waggledance stop did not return promptly against a live daemon");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Stopped daemon"),
        "expected a 'Stopped daemon' message, got: {stdout}"
    );
    assert!(!lock_path(&home).exists());

    let _ = guard.0.wait();

    let after = run_with_timeout(bin, &["status", "--json"], &home, Duration::from_secs(5))
        .expect("waggledance status did not return promptly");
    let after: serde_json::Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(after["running"], false, "{after}");
}

#[test]
fn the_viewer_subcommands_are_gone() {
    let bin = env!("CARGO_BIN_EXE_waggledance");
    let home = scratch_home("removed-commands");
    for args in [&["open", "x.md"][..], &["search", "x"], &["refresh"]] {
        let output = run_with_timeout(bin, args, &home, Duration::from_secs(5))
            .expect("waggledance did not return promptly");
        assert!(!output.status.success(), "{args:?} must fail");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unrecognized subcommand"),
            "{args:?}: {stderr}"
        );
    }
}
