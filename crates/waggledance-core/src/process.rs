//! Cross-platform process-spawn helpers shared by every crate that launches
//! a detached daemon (`waggledance` binary today, `waggledance-desktop` next).

/// Apply the platform detach settings to `cmd` so a spawned child outlives its
/// spawner: a new session on Unix (`setsid`), a detached console + new process
/// group on Windows. Extracted from `spawn_daemon_detached` so the detach itself
/// is testable without launching the full daemon.
pub fn apply_detach(cmd: &mut std::process::Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setsid() is async-signal-safe and is the only call made in the
        // forked child before exec. It puts the child in its own new session (as
        // session leader), detaching it from the spawner's controlling terminal
        // and process group so neither a SIGHUP on session close nor a
        // process-group-directed signal can reach it.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS: no inherited console. CREATE_NEW_PROCESS_GROUP: the
        // daemon does not receive Ctrl+C/Ctrl+Break sent to the spawner's group.
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
}

const MAX_STDERR_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone)]
pub struct RunOutput {
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub ok: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    Spawn(String),
    Timeout(u64),
}

pub fn run_with_deadline(
    cmd: &mut std::process::Command,
    stdin: Option<Vec<u8>>,
    timeout: std::time::Duration,
    cap: usize,
) -> Result<RunOutput, RunError> {
    use std::io::Write;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    let stdin_mode = if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    cmd.stdin(stdin_mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| RunError::Spawn(e.to_string()))?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        thread::spawn(move || {
            let _ = pipe.write_all(&bytes);
        });
    }
    let mut out_pipe = child.stdout.take().expect("stdout was piped");
    let mut err_pipe = child.stderr.take().expect("stderr was piped");
    let out_reader = thread::spawn(move || read_capped(&mut out_pipe, cap));
    let err_reader = thread::spawn(move || read_capped(&mut err_pipe, MAX_STDERR_BYTES));

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(e) => return Err(RunError::Spawn(e.to_string())),
        }
        if Instant::now() >= deadline {
            break None;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let Some(status) = status else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(RunError::Timeout(timeout.as_secs()));
    };

    let (stdout, truncated) = out_reader.join().unwrap_or_else(|_| (Vec::new(), false));
    let (stderr, _) = err_reader.join().unwrap_or_else(|_| (Vec::new(), false));
    Ok(RunOutput {
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        ok: status.success(),
        truncated,
    })
}

fn read_capped<R: std::io::Read>(reader: &mut R, cap: usize) -> (Vec<u8>, bool) {
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return (buf, false),
            Ok(n) => {
                if buf.len() + n > cap {
                    let room = cap.saturating_sub(buf.len());
                    buf.extend_from_slice(&chunk[..room]);
                    return (buf, true);
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            Err(_) => return (buf, false),
        }
    }
}
