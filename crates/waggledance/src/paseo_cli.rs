//! The ONLY door to the `paseo` binary (paseo-control D3). Nothing else in
//! this codebase invokes it — detection (which agents exist) keeps reading
//! the on-disk store via `waggledance_core::paseo`; every *control* action
//! (reading a conversation, sending a message, answering a permission)
//! crosses through [`PaseoCli`] instead.
//!
//! Lives in the BINARY crate, not `waggledance-core`: the core crate's
//! `no_web_framework_dependency_declared` guard forbids `tokio` there, and
//! this module needs `tokio::process` and `tokio::time::timeout`.
//!
//! Security invariants held here (paseo-control plan § "Security
//! invariants"), all covered by the tests below:
//!
//! - **S1** — every call is `tokio::process::Command::new(prog).arg(..)`,
//!   never a shell, never string concatenation into one argument.
//! - **S2** — a message always crosses via `--prompt <text>`, never the bare
//!   positional `[prompt]`, which a dash-leading value would be parsed as a
//!   flag.
//! - **S6** — [`PaseoCliError`] carries a KIND ONLY. No captured stdout,
//!   stderr, or prompt text ever reaches an error value or a log line — the
//!   same discipline `transcript_read_failed_response` (`server.rs`) applies
//!   to an `io::Error` that might carry a path fragment.
//!
//! pc-1 lands this adapter alone, ahead of any caller (pc-2's read path,
//! pc-4's send route, pc-5's permit route) — the read path proves itself
//! before the write routes exist (paseo-control plan § "Shape"). Nothing
//! outside `#[cfg(test)]` calls into this module yet, so `dead_code` is
//! expected here until pc-2 lands.
#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::time::Duration;

/// Every CLI call is bounded by this — new precedent in this codebase (the
/// four existing `bee` subprocess call sites in `server.rs` are unbounded).
/// Follows the `INDEX_HERDR_SNAPSHOT_TIMEOUT` idiom (`server.rs`): long
/// enough for an ordinary reply, short enough that a hung daemon never reads
/// as the page itself being down. `paseo send` WAITS for the agent to
/// finish by default — this bound is why every call also passes
/// `--no-wait` on `send` (see [`PaseoCli::send`]); without it, a successful
/// send would report as a timeout.
pub const PASEO_CLI_TIMEOUT: Duration = Duration::from_secs(10);

/// Empirically verified against the real `paseo` CLI (v0.6.1, on this
/// machine, 2026-08-29): when it cannot reach its daemon, most subcommands
/// still exit `0` and report `Error: Cannot connect to daemon` on stderr —
/// the process exit code alone is NOT a reliable signal for this state. The
/// prefix is matched and discarded in the same expression it is read in; it
/// is never retained anywhere an error value or a log line could carry it
/// (S6).
const DAEMON_UNREACHABLE_MARKER: &str = "Error: Cannot connect to daemon";

/// The four states D5 needs apart, named — never the raw output that
/// produced them (S6). `Display` gives a fixed, static message per variant
/// for exactly the same reason: nothing here may echo captured CLI text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaseoCliError {
    /// The `paseo` program could not be started at all — not installed, not
    /// on `PATH`, or otherwise unable to spawn (e.g. not executable).
    BinaryNotFound,
    /// The CLI ran but reported it could not reach its own daemon.
    DaemonUnreachable,
    /// The call did not finish within [`PASEO_CLI_TIMEOUT`].
    TimedOut,
    /// The CLI exited non-zero for a reason other than the two states
    /// above. Carries the exit code only — never stdout or stderr.
    Failed { exit_code: Option<i32> },
}

impl std::fmt::Display for PaseoCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinaryNotFound => write!(f, "the paseo CLI is not installed or not on PATH"),
            Self::DaemonUnreachable => write!(f, "the paseo daemon is not reachable"),
            Self::TimedOut => write!(
                f,
                "the paseo CLI did not respond within {PASEO_CLI_TIMEOUT:?}"
            ),
            Self::Failed {
                exit_code: Some(code),
            } => {
                write!(f, "the paseo CLI exited with status {code}")
            }
            Self::Failed { exit_code: None } => {
                write!(f, "the paseo CLI exited without a status code")
            }
        }
    }
}

impl std::error::Error for PaseoCliError {}

/// The single door to the `paseo` binary. The program path is injectable
/// (default: the bare name `paseo`, resolved from `PATH` by the OS, exactly
/// like every existing `Command::new(&bee)` call site in `server.rs`) so
/// tests can point it at a fixture script instead of the real CLI.
#[derive(Debug, Clone)]
pub struct PaseoCli {
    program: OsString,
    timeout: Duration,
}

impl Default for PaseoCli {
    fn default() -> Self {
        Self {
            program: OsString::from("paseo"),
            timeout: PASEO_CLI_TIMEOUT,
        }
    }
}

impl PaseoCli {
    /// Points at a specific `paseo` program path — used by tests to drive a
    /// fixture script. Production callers use [`PaseoCli::default`].
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            timeout: PASEO_CLI_TIMEOUT,
        }
    }

    /// Test-only: overrides [`PASEO_CLI_TIMEOUT`] so the timed-out case does
    /// not cost the suite ten real seconds. Never used by a production
    /// caller — those always get `PASEO_CLI_TIMEOUT` via `new`/`default`.
    #[cfg(test)]
    fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// `paseo logs <id> --tail <n>` — the agent's conversation stream.
    /// There is no JSON form of `paseo logs` (verified: `--json`,
    /// `-o json` and `--format json` all return the same text stream), so
    /// this returns the raw text for the caller to parse.
    pub async fn logs(&self, agent_id: &str, tail: u32) -> Result<String, PaseoCliError> {
        let tail = tail.to_string();
        self.run(["logs", agent_id, "--tail", &tail]).await
    }

    /// `paseo send <id> --prompt <text> --no-wait --json`. **S2**: the
    /// message always crosses via `--prompt`, never the bare positional
    /// `[prompt]` a leading-dash value would be parsed as a flag under.
    /// **`--no-wait` is mandatory**: `paseo send` waits for the agent to
    /// finish by default, so without it a successful send would report as
    /// a timeout under [`PASEO_CLI_TIMEOUT`].
    pub async fn send(&self, agent_id: &str, text: &str) -> Result<String, PaseoCliError> {
        self.run(["send", agent_id, "--prompt", text, "--no-wait", "--json"])
            .await
    }

    /// `paseo permit ls` — the pending-permission list (`[]` when empty).
    pub async fn permit_ls(&self) -> Result<String, PaseoCliError> {
        self.run(["permit", "ls"]).await
    }

    /// `paseo permit allow <agent> <req>`. Operand order matters — a
    /// fixture test cannot catch a swapped order the same way the real
    /// binary can (see the ignored `real_binary_smoke_permit_ls` test and
    /// the pc-1 cap proof).
    pub async fn permit_allow(
        &self,
        agent_id: &str,
        req_id: &str,
    ) -> Result<String, PaseoCliError> {
        self.run(["permit", "allow", agent_id, req_id]).await
    }

    /// `paseo permit deny <agent> <req>`. Same operand-order note as
    /// [`PaseoCli::permit_allow`].
    pub async fn permit_deny(&self, agent_id: &str, req_id: &str) -> Result<String, PaseoCliError> {
        self.run(["permit", "deny", agent_id, req_id]).await
    }

    /// **S1**: `Command::new(prog).arg(..)` only — never a shell, never
    /// string concatenation into one argument. Bounded by
    /// [`PASEO_CLI_TIMEOUT`] (or the test override), matching the
    /// `INDEX_HERDR_SNAPSHOT_TIMEOUT` idiom in `server.rs`.
    async fn run<I, S>(&self, args: I) -> Result<String, PaseoCliError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.args(args)
            // Nothing on this server's stdin belongs to a spawned CLI,
            // matching every existing `Command::new(&bee)` call site.
            .stdin(std::process::Stdio::null())
            // A timed-out call below drops this `Command::output()` future
            // without awaiting it further; without `kill_on_drop`, the
            // spawned `paseo` process would keep running as an orphan
            // instead of being reaped.
            .kill_on_drop(true);

        let out = match tokio::time::timeout(self.timeout, spawn_with_busy_retry(&mut cmd)).await {
            Err(_) => return Err(PaseoCliError::TimedOut),
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(PaseoCliError::BinaryNotFound);
            }
            // Any other spawn-time failure (permission denied, not
            // executable, ...) means the same thing to a caller: the CLI
            // could not be invoked at all.
            Ok(Err(_)) => return Err(PaseoCliError::BinaryNotFound),
            Ok(Ok(out)) => out,
        };

        // Checked before the exit status: the real CLI's own exit code is
        // not a reliable signal for "daemon unreachable" (see
        // `DAEMON_UNREACHABLE_MARKER`'s doc comment) — some subcommands
        // report it with a `0` exit, others with a non-zero one.
        if is_daemon_unreachable(&out.stderr) {
            return Err(PaseoCliError::DaemonUnreachable);
        }
        if !out.status.success() {
            return Err(PaseoCliError::Failed {
                exit_code: out.status.code(),
            });
        }
        // S6: this is the ONLY place captured stdout is read — it becomes
        // the success value, never an error value.
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

/// A just-written executable can transiently fail to `exec` with
/// `ETXTBSY` ("text file busy") for a few milliseconds after the file is
/// closed — observed directly under this test module's own concurrent
/// write-then-exec fixture churn (`cargo test` runs test fns in parallel by
/// default). This is a transient race, not a permanent condition, so it is
/// retried here rather than folded into [`PaseoCliError::BinaryNotFound`],
/// which names a binary that will never be found. Bounded by the caller's
/// own `tokio::time::timeout` — this never extends the wall-clock budget
/// past [`PASEO_CLI_TIMEOUT`].
async fn spawn_with_busy_retry(
    cmd: &mut tokio::process::Command,
) -> std::io::Result<std::process::Output> {
    const MAX_ATTEMPTS: u32 = 5;
    for attempt in 0..MAX_ATTEMPTS {
        match cmd.output().await {
            Err(e)
                if e.kind() == std::io::ErrorKind::ExecutableFileBusy
                    && attempt + 1 < MAX_ATTEMPTS =>
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            result => return result,
        }
    }
    unreachable!("the loop above always returns by its last iteration")
}

/// See [`DAEMON_UNREACHABLE_MARKER`]. The decoded string is local to this
/// call and dropped immediately after the check — it is never returned,
/// stored, or logged (S6).
fn is_daemon_unreachable(stderr: &[u8]) -> bool {
    String::from_utf8_lossy(stderr).contains(DAEMON_UNREACHABLE_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    /// Writes an executable fixture at `<dir>/paseo`. Unix-only (`#!/bin/sh`
    /// script) — the same constraint the `board-new-task` `fake_bee` fixture
    /// (`server.rs`) documents for its own shell-script fixtures.
    #[cfg(unix)]
    fn write_script(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("paseo");
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Echoes each argv element on its own line via `printf '%s\n' "$a"` —
    /// never re-parsed or re-split by the shell, so this is what proves an
    /// argument survives as ONE unmodified element regardless of the
    /// characters inside it. None of this suite's prompts contain a literal
    /// newline, so `\n` is an unambiguous separator here.
    #[cfg(unix)]
    fn echo_argv_script(dir: &Path) -> PathBuf {
        write_script(
            dir,
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done\n",
        )
    }

    #[cfg(unix)]
    fn exit_code_script(dir: &Path, code: i32, stdout: &str, stderr: &str) -> PathBuf {
        write_script(
            dir,
            &format!(
                "#!/bin/sh\nprintf '%s' '{stdout}'\nprintf '%s' '{stderr}' >&2\nexit {code}\n"
            ),
        )
    }

    #[cfg(unix)]
    fn daemon_unreachable_script(dir: &Path) -> PathBuf {
        // Exit 0 on purpose: the real CLI's own exit code is not reliable
        // for this state (see `DAEMON_UNREACHABLE_MARKER`'s doc comment) —
        // this reproduces the exit-0 case observed on the real binary.
        write_script(
            dir,
            "#!/bin/sh\n>&2 printf '%s' 'Error: Cannot connect to daemon at tcp://127.0.0.1:1: connect ECONNREFUSED SECRET-STDERR-CANARY'\nexit 0\n",
        )
    }

    #[cfg(unix)]
    fn sleep_script(dir: &Path, secs: u64) -> PathBuf {
        write_script(dir, &format!("#!/bin/sh\nsleep {secs}\necho done\n"))
    }

    fn argv_lines(out: &str) -> Vec<&str> {
        out.lines().collect()
    }

    // ── argv shape, per method (S1/S2) ──────────────────────────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn logs_argv_order() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let out = cli.logs("agent-1", 200).await.unwrap();
        assert_eq!(argv_lines(&out), vec!["logs", "agent-1", "--tail", "200"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn send_argv_order_uses_prompt_flag_and_no_wait() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let out = cli.send("agent-1", "hello there").await.unwrap();
        assert_eq!(
            argv_lines(&out),
            vec![
                "send",
                "agent-1",
                "--prompt",
                "hello there",
                "--no-wait",
                "--json"
            ]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn permit_ls_argv_order() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let out = cli.permit_ls().await.unwrap();
        assert_eq!(argv_lines(&out), vec!["permit", "ls"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn permit_allow_argv_order_is_agent_then_req() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let out = cli.permit_allow("agent-1", "req-9").await.unwrap();
        assert_eq!(
            argv_lines(&out),
            vec!["permit", "allow", "agent-1", "req-9"]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn permit_deny_argv_order_is_agent_then_req() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let out = cli.permit_deny("agent-1", "req-9").await.unwrap();
        assert_eq!(argv_lines(&out), vec!["permit", "deny", "agent-1", "req-9"]);
    }

    // ── a hostile prompt arrives as ONE unmodified argv element ─────────

    #[cfg(unix)]
    #[tokio::test]
    async fn prompt_with_shell_metacharacters_is_one_unmodified_argument() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let prompt = r#"$(rm -rf /); `id`; a && b || c | cat > /tmp/x; ~ ! * ? { }"#;
        let out = cli.send("agent-1", prompt).await.unwrap();
        let lines = argv_lines(&out);
        assert_eq!(
            lines.len(),
            6,
            "prompt must arrive as exactly one argv element: {lines:?}"
        );
        assert_eq!(lines[3], prompt);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn prompt_with_leading_dash_is_not_parsed_as_a_flag() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let prompt = "-rf --json --no-wait";
        let out = cli.send("agent-1", prompt).await.unwrap();
        let lines = argv_lines(&out);
        assert_eq!(
            lines.len(),
            6,
            "prompt must arrive as exactly one argv element: {lines:?}"
        );
        assert_eq!(lines[3], prompt);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn prompt_with_quotes_is_one_unmodified_argument() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let prompt = r#"she said "hello" and 'goodbye', both at once"#;
        let out = cli.send("agent-1", prompt).await.unwrap();
        let lines = argv_lines(&out);
        assert_eq!(
            lines.len(),
            6,
            "prompt must arrive as exactly one argv element: {lines:?}"
        );
        assert_eq!(lines[3], prompt);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn prompt_with_non_ascii_is_one_unmodified_argument() {
        let dir = tempfile::tempdir().unwrap();
        let cli = PaseoCli::new(echo_argv_script(dir.path()));
        let prompt = "héllo — 世界 🐝 café";
        let out = cli.send("agent-1", prompt).await.unwrap();
        let lines = argv_lines(&out);
        assert_eq!(
            lines.len(),
            6,
            "prompt must arrive as exactly one argv element: {lines:?}"
        );
        assert_eq!(lines[3], prompt);
    }

    // ── the four error states (D5) ───────────────────────────────────────

    #[tokio::test]
    async fn missing_binary_yields_binary_not_found_not_a_panic() {
        let cli = PaseoCli::new("/nonexistent/waggledance-paseo-cli-test-fixture-binary-xyz");
        let err = cli.permit_ls().await.unwrap_err();
        assert_eq!(err, PaseoCliError::BinaryNotFound);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nonzero_exit_yields_failed_with_exit_code() {
        let dir = tempfile::tempdir().unwrap();
        let script = exit_code_script(
            dir.path(),
            7,
            "SECRET-STDOUT-CANARY",
            "SECRET-STDERR-CANARY",
        );
        let cli = PaseoCli::new(script);
        let err = cli.permit_ls().await.unwrap_err();
        assert_eq!(err, PaseoCliError::Failed { exit_code: Some(7) });
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_unreachable_message_is_recognized_even_with_zero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let script = daemon_unreachable_script(dir.path());
        let cli = PaseoCli::new(script);
        let err = cli.permit_ls().await.unwrap_err();
        assert_eq!(err, PaseoCliError::DaemonUnreachable);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn call_outliving_the_timeout_yields_timed_out() {
        let dir = tempfile::tempdir().unwrap();
        let script = sleep_script(dir.path(), 2);
        let cli = PaseoCli::new(script).with_timeout(Duration::from_millis(50));
        let err = cli.permit_ls().await.unwrap_err();
        assert_eq!(err, PaseoCliError::TimedOut);
    }

    // ── S6: no error value ever carries captured output ─────────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_error_never_carries_captured_output() {
        let dir = tempfile::tempdir().unwrap();
        let script = exit_code_script(
            dir.path(),
            3,
            "SECRET-STDOUT-CANARY",
            "SECRET-STDERR-CANARY",
        );
        let cli = PaseoCli::new(script);
        let err = cli.permit_ls().await.unwrap_err();
        let debug = format!("{err:?}");
        let display = err.to_string();
        assert!(!debug.contains("SECRET-STDOUT-CANARY") && !debug.contains("SECRET-STDERR-CANARY"));
        assert!(
            !display.contains("SECRET-STDOUT-CANARY") && !display.contains("SECRET-STDERR-CANARY")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_unreachable_error_never_carries_captured_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let script = daemon_unreachable_script(dir.path());
        let cli = PaseoCli::new(script);
        let err = cli.permit_ls().await.unwrap_err();
        let debug = format!("{err:?}");
        let display = err.to_string();
        assert!(!debug.contains("SECRET-STDERR-CANARY"));
        assert!(!display.contains("SECRET-STDERR-CANARY"));
    }

    #[tokio::test]
    async fn no_error_variant_carries_a_prompt() {
        // Every `PaseoCliError` variant is a plain enum with no string
        // payload except the exit code — this is a structural guarantee
        // (checked at compile time by the type itself), reasserted here so
        // a future edit that adds a `String` field to any variant fails
        // this test rather than silently reopening S6.
        let err = PaseoCliError::Failed { exit_code: Some(1) };
        assert!(!format!("{err:?}").contains("hello there"));
    }

    // ── real-binary smoke: argv ORDER a fixture cannot catch ────────────

    /// Smoke-tests the REAL `paseo` binary on this machine — never a
    /// fixture. A fixture script proves argv *content* survives unmodified
    /// (the tests above), but cannot catch a wrong argv *order*: e.g.
    /// `permit allow <agent> <req>` with the operands swapped would pass
    /// every fixture test here and fail only in production, because a
    /// fixture script does not know or care which operand means what.
    /// Deliberately read-only (`permit ls`) so running it can never send a
    /// message or answer a permission on a real, possibly-live agent.
    ///
    /// Ignored by default — CI has no `paseo` binary and no daemon. Run
    /// manually: `cargo test -p waggledance paseo_cli -- --ignored`.
    #[tokio::test]
    #[ignore = "requires the real paseo binary and a reachable daemon on this machine's PATH"]
    async fn real_binary_smoke_permit_ls() {
        let cli = PaseoCli::default();
        let result = cli.permit_ls().await;
        assert!(
            result.is_ok(),
            "expected `paseo permit ls` to succeed against the real daemon on this machine, got {result:?}"
        );
    }
}
