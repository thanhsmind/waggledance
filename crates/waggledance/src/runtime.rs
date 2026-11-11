use anyhow::Result;
use serde::{Deserialize, Serialize};
use waggledance_core::config::{self, Config};
use waggledance_core::{Engine, SqliteStore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub started_at: String,
    #[serde(default)]
    pub version: Option<String>,
}

pub fn build_engine() -> Result<Engine> {
    let config = Config::load();
    let store = SqliteStore::open(&config::registry_db_path())?;
    Ok(Engine::new(store, config))
}

pub fn write_lock(info: &DaemonInfo) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(info)?;
    config::write_atomic(&config::daemon_lock_path(), &bytes)?;
    Ok(())
}

pub fn read_lock() -> Option<DaemonInfo> {
    let text = std::fs::read_to_string(config::daemon_lock_path()).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn remove_lock() {
    let _ = std::fs::remove_file(config::daemon_lock_path());
}

#[cfg(unix)]
fn process_is_waggledance(pid: u32) -> bool {
    std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("waggledance"))
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn process_is_waggledance(pid: u32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("waggledance"))
        .unwrap_or(false)
}

pub fn running_daemon() -> Option<DaemonInfo> {
    let info = read_lock()?;
    process_is_waggledance(info.pid).then_some(info)
}

pub(crate) use waggledance_core::process::apply_detach;

pub fn spawn_daemon_detached() -> Result<()> {
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    apply_detach(&mut cmd);
    cmd.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[cfg(unix)]
    #[test]
    fn apply_detach_puts_child_in_its_own_session() {
        use std::process::{Command, Stdio};
        let mut cmd = Command::new("sleep");
        cmd.arg("0.4")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        super::apply_detach(&mut cmd);
        let mut child = cmd.spawn().expect("spawn sleep");
        let pid = child.id() as i32;
        std::thread::sleep(Duration::from_millis(80));
        let child_sid = unsafe { libc::getsid(pid) };
        let my_sid = unsafe { libc::getsid(0) };
        child.kill().ok();
        child.wait().ok();
        assert_eq!(child_sid, pid, "detached child must lead its own session");
        assert_ne!(
            child_sid, my_sid,
            "detached child must not share our session"
        );
    }

    #[test]
    fn a_lock_written_by_an_older_build_still_parses() {
        let legacy =
            r#"{"pid":42,"host":"127.0.0.1","port":7700,"started_at":"2026-07-15T00:00:00Z"}"#;
        let info: DaemonInfo = serde_json::from_str(legacy).unwrap();
        assert_eq!(info.pid, 42);
        assert_eq!(info.version, None);
    }

    #[cfg(unix)]
    #[test]
    fn a_live_process_that_is_not_waggledance_is_not_a_daemon() {
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .expect("spawn sleep");
        assert!(!process_is_waggledance(child.id()));
        child.kill().ok();
        child.wait().ok();
    }
}
