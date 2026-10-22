use anyhow::{bail, Result};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use waggledance_core::config::OpenClawConfig;
use waggledance_core::domain::InboxMessage;
use waggledance_core::indexer::now_rfc3339;
use waggledance_core::Engine;

use crate::paseo::parse_document;

const RECIPIENT: &str = "wd";
const TICK: Duration = Duration::from_secs(2);
const BACKOFF: Duration = Duration::from_secs(30);
const PRUNE_EVERY: Duration = Duration::from_secs(3600);
const KEEP_ACKED_DAYS: i64 = 30;
const DEAD_AT_ATTEMPTS: i64 = 5;
const TURN_TIMEOUT: Duration = Duration::from_secs(900);
const GATEWAY_COMM: &str = "openclaw-gateway";

pub fn kind_label(kind: &str) -> Result<&'static str> {
    Ok(match kind {
        "result" => "kết quả",
        "question" => "câu hỏi",
        "blocked" => "đang chờ quyền",
        "permission" => "xin quyền",
        "error" => "lỗi",
        "closed" => "đã đóng",
        "progress" => "tiến độ",
        "board" => "bảng việc",
        other => bail!(
            "unknown kind `{other}`: use result, question, blocked, permission, error, closed, \
             progress or board"
        ),
    })
}

const REPORT_INSTRUCTION: &str = "Đối chiếu báo cáo này với đúng task bạn đã giao cho \
     waggledance, rồi viết tin nhắn người dùng sẽ đọc trên Telegram, bằng tiếng Việt: nêu việc \
     đã xong và việc còn lại, chép nguyên văn con số, đường dẫn và lời của leader; nếu báo cáo \
     cần người dùng quyết định thì hỏi thẳng người dùng";
const BOARD_INSTRUCTION: &str = "Đây là danh sách các task đang mở mà bạn đã giao cho \
     waggledance, không phải báo cáo về một task: viết cho người dùng trên Telegram, bằng tiếng \
     Việt, mỗi task một dòng, chép nguyên văn mã task, tên dự án và trạng thái leader";

fn frame(tag: &str, kind: &str, project: &str, text: &str) -> Result<String> {
    let label = kind_label(kind)?;
    let instruction = if kind == "board" {
        BOARD_INSTRUCTION
    } else {
        REPORT_INSTRUCTION
    };
    Ok(format!(
        "{instruction}: [waggledance#{tag}] {project} — {label}: {text}"
    ))
}

pub fn framed_message(kind: &str, project: &str, corr: &str, text: &str) -> Result<String> {
    frame(corr, kind, project, text)
}

pub fn inbox_frame(message: &InboxMessage, project: &str) -> Result<String> {
    frame(
        &format!("{} #{}", message.corr, message.seq),
        &message.kind,
        project,
        &message.body,
    )
}

fn is_gateway_comm(comm: &str) -> bool {
    comm.len() >= GATEWAY_COMM.len() - 1 && GATEWAY_COMM.starts_with(comm)
}

fn gateway_appdir(proc_root: &Path) -> Option<String> {
    for entry in std::fs::read_dir(proc_root).ok()?.flatten() {
        let pid_dir = entry.path();
        let comm = match std::fs::read_to_string(pid_dir.join("comm")) {
            Ok(comm) => comm,
            Err(_) => continue,
        };
        if !is_gateway_comm(comm.trim()) {
            continue;
        }
        let environ = match std::fs::read(pid_dir.join("environ")) {
            Ok(environ) => environ,
            Err(_) => continue,
        };
        let found = environ
            .split(|b| *b == 0)
            .filter_map(|kv| std::str::from_utf8(kv).ok())
            .find_map(|kv| kv.strip_prefix("APPDIR="))
            .filter(|dir| !dir.is_empty());
        if let Some(dir) = found {
            return Some(dir.to_string());
        }
    }
    None
}

pub fn openclaw_cli(cfg: &OpenClawConfig, proc_root: &Path) -> Result<Vec<String>> {
    if !cfg.cli.trim().is_empty() {
        let words: Vec<String> = cfg.cli.split_whitespace().map(str::to_string).collect();
        if !words.is_empty() {
            return Ok(words);
        }
    }
    match gateway_appdir(proc_root) {
        Some(appdir) => Ok(vec![
            "node".into(),
            format!("{appdir}/resources/openclaw/openclaw.mjs"),
        ]),
        None => bail!(
            "no OpenClaw CLI: no running `{GATEWAY_COMM}` process names an APPDIR under {}, and \
             `[openclaw] cli` in config is empty — start the gateway, or set `cli` to the full \
             openclaw command",
            proc_root.display()
        ),
    }
}

pub fn report_argv(cli: &[String], cfg: &OpenClawConfig, message: &str) -> Result<Vec<String>> {
    if cfg.agent_id.trim().is_empty() {
        bail!(
            "no `[openclaw] agent_id` in config — name the OpenClaw agent whose main session \
             receives supervisor reports"
        );
    }
    let mut argv = cli.to_vec();
    for arg in [
        "agent",
        "--agent",
        cfg.agent_id.as_str(),
        "--message",
        message,
        "--deliver",
        "--channel",
        "telegram",
        "--json",
    ] {
        argv.push(arg.to_string());
    }
    if !cfg.telegram_to.trim().is_empty() {
        argv.push("--reply-to".into());
        argv.push(cfg.telegram_to.clone());
    }
    Ok(argv)
}

pub async fn cli_run(argv: &[String]) -> Result<String> {
    let shown = argv.join(" ");
    let child = tokio::process::Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("running `{shown}`: {e}"))?;
    let output = match tokio::time::timeout(TURN_TIMEOUT, child.wait_with_output()).await {
        Ok(out) => out?,
        Err(_) => bail!("`{shown}` timed out after {}s", TURN_TIMEOUT.as_secs()),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        bail!(
            "`{shown}` failed ({}): {}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
            stdout.trim()
        );
    }
    Ok(stdout)
}

pub fn cli_status(stdout: &str, shown: &str) -> Result<String> {
    let doc: serde_json::Value = parse_document(stdout)
        .map_err(|e| anyhow::anyhow!("`{shown}` printed no JSON document ({e})"))?;
    let status = doc
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    if status != "ok" {
        let shown_status = if status.is_empty() { "<none>" } else { &status };
        bail!(
            "`{shown}` reported status {shown_status}: {}",
            stdout.trim()
        );
    }
    Ok(status)
}

pub fn enabled(engine: &Engine) -> bool {
    !engine.config.openclaw.agent_id.trim().is_empty()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Idle,
    Acked,
    Retry,
}

pub struct Wake<R> {
    engine: Arc<Engine>,
    proc_root: PathBuf,
    turn: R,
}

impl<R, F> Wake<R>
where
    R: Fn(Vec<String>) -> F,
    F: Future<Output = Result<String>>,
{
    pub fn new(engine: Arc<Engine>, proc_root: PathBuf, turn: R) -> Self {
        Self {
            engine,
            proc_root,
            turn,
        }
    }

    pub async fn run(self) {
        let mut pruned: Option<Instant> = None;
        loop {
            if pruned.is_none_or(|at| at.elapsed() >= PRUNE_EVERY) {
                prune(&self.engine, OffsetDateTime::now_utc());
                pruned = Some(Instant::now());
            }
            let pause = match self.tick().await {
                Outcome::Retry => BACKOFF,
                Outcome::Idle | Outcome::Acked => TICK,
            };
            tokio::time::sleep(pause).await;
        }
    }

    async fn tick(&self) -> Outcome {
        let cfg = &self.engine.config.openclaw;
        if cfg.agent_id.trim().is_empty() {
            return Outcome::Idle;
        }
        let message = match self.engine.next_inbox_message(RECIPIENT) {
            Ok(Some(message)) => message,
            Ok(None) => return Outcome::Idle,
            Err(e) => {
                tracing::warn!("wake could not read the wd inbox: {e}");
                return Outcome::Retry;
            }
        };
        let cli = match openclaw_cli(cfg, &self.proc_root) {
            Ok(cli) => cli,
            Err(e) => {
                tracing::warn!("wake could not reach wd for #{}: {e}", message.seq);
                return Outcome::Retry;
            }
        };
        let project = self
            .engine
            .get_project(&message.project_id)
            .ok()
            .flatten()
            .map_or_else(|| message.project_id.clone(), |p| p.name);
        let turned = match inbox_frame(&message, &project).and_then(|m| report_argv(&cli, cfg, &m))
        {
            Ok(argv) => {
                let shown = argv.join(" ");
                match (self.turn)(argv).await {
                    Ok(stdout) => cli_status(&stdout, &shown).map(drop),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        };
        match turned {
            Ok(()) => match self.engine.ack_inbox_message(message.seq, &now_rfc3339()) {
                Ok(()) => Outcome::Acked,
                Err(e) => {
                    tracing::warn!("wake could not ack #{}: {e}", message.seq);
                    Outcome::Retry
                }
            },
            Err(e) => {
                match self
                    .engine
                    .fail_inbox_message(message.seq, DEAD_AT_ATTEMPTS, &now_rfc3339())
                {
                    Ok(true) => tracing::warn!(
                        "wake gave up on wd inbox #{} after {DEAD_AT_ATTEMPTS} attempts: {e}",
                        message.seq
                    ),
                    Ok(false) => {}
                    Err(store) => {
                        tracing::warn!("wake could not record a failed #{}: {store}", message.seq)
                    }
                }
                Outcome::Retry
            }
        }
    }
}

fn prune(engine: &Engine, now: OffsetDateTime) {
    let cutoff = now - time::Duration::days(KEEP_ACKED_DAYS);
    let Ok(cutoff) = cutoff.format(&Rfc3339) else {
        return;
    };
    if let Err(e) = engine.prune_inbox(&cutoff) {
        tracing::warn!("wake could not prune the wd inbox: {e}");
    }
    if let Err(e) = engine.prune_tasks(&cutoff) {
        tracing::warn!("wake could not prune closed wd tasks: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use waggledance_core::domain::Project;
    use waggledance_core::{Config, SqliteStore};

    const NOW: &str = "2026-10-22T10:00:00Z";
    const OK: &str = r#"{"status":"ok","result":{"payloads":[]}}"#;

    fn engine_with(agent_id: &str) -> Arc<Engine> {
        let mut config = Config::default();
        config.openclaw.agent_id = agent_id.into();
        config.openclaw.cli = "fake-openclaw".into();
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        engine
            .store
            .upsert_project(&Project {
                id: "proj-1".into(),
                name: "waggledance".into(),
                root_path: std::env::temp_dir().join("proj-1"),
                created_at: NOW.into(),
                last_seen_at: NOW.into(),
                orchestration_enabled: true,
            })
            .unwrap();
        Arc::new(engine)
    }

    fn mail(engine: &Engine, corr: &str, body: &str) -> i64 {
        engine
            .insert_inbox_message(RECIPIENT, corr, "proj-1", "result", body, NOW)
            .unwrap()
    }

    type Calls = Arc<Mutex<Vec<Vec<String>>>>;
    type Scripted = Wake<Box<dyn Fn(Vec<String>) -> std::future::Ready<Result<String>>>>;

    fn scripted(engine: Arc<Engine>, replies: Vec<Result<String>>) -> (Scripted, Calls) {
        let calls: Calls = Arc::default();
        let replies = Mutex::new(VecDeque::from(replies));
        let seen = calls.clone();
        let turn: Box<dyn Fn(Vec<String>) -> std::future::Ready<Result<String>>> =
            Box::new(move |argv| {
                seen.lock().unwrap().push(argv);
                std::future::ready(
                    replies
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("a scripted reply"),
                )
            });
        (
            Wake::new(engine, PathBuf::from("/nonexistent"), turn),
            calls,
        )
    }

    fn message_of(argv: &[String]) -> &str {
        let at = argv.iter().position(|a| a == "--message").unwrap();
        &argv[at + 1]
    }

    #[tokio::test]
    async fn wake_acks_a_message_after_an_ok_turn() {
        let engine = engine_with("wd");
        mail(&engine, "t-1", "Done: x");
        let (wake, calls) = scripted(engine.clone(), vec![Ok(OK.into())]);
        assert_eq!(wake.tick().await, Outcome::Acked);
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert!(engine.next_inbox_message(RECIPIENT).unwrap().is_none());
    }

    #[tokio::test]
    async fn wake_leaves_a_message_unacked_on_a_non_zero_exit_or_a_status_not_ok() {
        for reply in [
            Err(anyhow::anyhow!("`openclaw agent` failed (exit status: 3)")),
            Ok(r#"{"status":"error"}"#.to_string()),
        ] {
            let engine = engine_with("wd");
            let seq = mail(&engine, "t-1", "Done: x");
            let (wake, _) = scripted(engine.clone(), vec![reply]);
            assert_eq!(wake.tick().await, Outcome::Retry);
            let pending = engine.next_inbox_message(RECIPIENT).unwrap().unwrap();
            assert_eq!((pending.seq, pending.attempts), (seq, 1));
            assert!(pending.acked_at.is_none());
        }
    }

    #[tokio::test]
    async fn wake_marks_a_message_dead_on_its_fifth_failure_and_wakes_the_next() {
        let engine = engine_with("wd");
        let poison = mail(&engine, "t-1", "one");
        let next = mail(&engine, "t-2", "two");
        let mut replies: Vec<Result<String>> =
            (0..5).map(|_| Err(anyhow::anyhow!("boom"))).collect();
        replies.push(Ok(OK.into()));
        let (wake, calls) = scripted(engine.clone(), replies);
        for _ in 0..4 {
            assert_eq!(wake.tick().await, Outcome::Retry);
            assert_eq!(
                engine.next_inbox_message(RECIPIENT).unwrap().unwrap().seq,
                poison
            );
        }
        assert_eq!(wake.tick().await, Outcome::Retry);
        assert_eq!(
            engine.next_inbox_message(RECIPIENT).unwrap().unwrap().seq,
            next
        );
        assert_eq!(wake.tick().await, Outcome::Acked);
        let calls = calls.lock().unwrap();
        assert!(message_of(&calls[5]).contains(&format!("#{next}]")));
    }

    #[tokio::test]
    async fn wake_delivers_three_messages_in_seq_order() {
        let engine = engine_with("wd");
        let seqs: Vec<i64> = ["a", "b", "c"]
            .into_iter()
            .map(|body| mail(&engine, "t-1", body))
            .collect();
        let (wake, calls) = scripted(engine.clone(), (0..3).map(|_| Ok(OK.into())).collect());
        for _ in 0..3 {
            assert_eq!(wake.tick().await, Outcome::Acked);
        }
        assert_eq!(wake.tick().await, Outcome::Idle);
        let calls = calls.lock().unwrap();
        for (call, seq) in calls.iter().zip(&seqs) {
            assert!(message_of(call).contains(&format!("[waggledance#t-1 #{seq}]")));
        }
    }

    #[tokio::test]
    async fn wake_does_not_ack_a_message_that_lands_during_a_turn() {
        let engine = engine_with("wd");
        mail(&engine, "t-1", "first");
        let during = engine.clone();
        let wake = Wake::new(
            engine.clone(),
            PathBuf::from("/nonexistent"),
            move |_argv| {
                mail(&during, "t-2", "second");
                std::future::ready(Ok(OK.to_string()))
            },
        );
        assert_eq!(wake.tick().await, Outcome::Acked);
        let pending = engine.next_inbox_message(RECIPIENT).unwrap().unwrap();
        assert_eq!((pending.corr.as_str(), pending.attempts), ("t-2", 0));
    }

    #[tokio::test]
    async fn wake_runs_nothing_without_an_agent_id() {
        let engine = engine_with("");
        mail(&engine, "t-1", "x");
        let (wake, calls) = scripted(engine.clone(), vec![]);
        assert_eq!(wake.tick().await, Outcome::Idle);
        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(
            engine
                .next_inbox_message(RECIPIENT)
                .unwrap()
                .unwrap()
                .attempts,
            0
        );
        assert!(!enabled(&engine));
        assert!(enabled(&engine_with("wd")));
    }

    #[tokio::test]
    async fn wake_runs_the_openclaw_argv_with_the_project_name_and_the_framed_message() {
        let engine = engine_with("wd");
        let seq = mail(&engine, "t-abc123", "Done: shipped");
        let (wake, calls) = scripted(engine.clone(), vec![Ok(OK.into())]);
        wake.tick().await;
        let pending = InboxMessage {
            seq,
            recipient: RECIPIENT.into(),
            corr: "t-abc123".into(),
            project_id: "proj-1".into(),
            kind: "result".into(),
            body: "Done: shipped".into(),
            created_at: NOW.into(),
            acked_at: None,
            attempts: 0,
            dead_at: None,
        };
        let expected = report_argv(
            &["fake-openclaw".to_string()],
            &engine.config.openclaw,
            &inbox_frame(&pending, "waggledance").unwrap(),
        )
        .unwrap();
        assert_eq!(calls.lock().unwrap()[0], expected);
    }

    #[test]
    fn the_wake_frame_carries_the_seq_after_the_corr_and_the_instruction() {
        let message = InboxMessage {
            seq: 7,
            recipient: RECIPIENT.into(),
            corr: "t-1".into(),
            project_id: "proj-1".into(),
            kind: "permission".into(),
            body: "Bash ls".into(),
            created_at: NOW.into(),
            acked_at: None,
            attempts: 0,
            dead_at: None,
        };
        let framed = inbox_frame(&message, "waggledance").unwrap();
        assert!(
            framed.ends_with("[waggledance#t-1 #7] waggledance — xin quyền: Bash ls"),
            "{framed}"
        );
        assert!(framed.starts_with("Đối chiếu báo cáo này"), "{framed}");
    }

    #[tokio::test]
    async fn the_board_frame_wakes_wd_with_its_own_instruction() {
        let engine = engine_with("wd");
        let seq = engine
            .insert_inbox_message(
                RECIPIENT,
                "board",
                "all",
                "board",
                "c-1 waggledance idle",
                NOW,
            )
            .unwrap();
        let (wake, calls) = scripted(engine.clone(), vec![Ok(OK.into())]);
        assert_eq!(wake.tick().await, Outcome::Acked);
        let calls = calls.lock().unwrap();
        let framed = message_of(&calls[0]);
        assert!(
            framed.ends_with(&format!(
                "[waggledance#board #{seq}] all — bảng việc: c-1 waggledance idle"
            )),
            "{framed}"
        );
        assert!(!framed.starts_with("Đối chiếu báo cáo này"), "{framed}");
        assert!(framed.contains("task đang mở"), "{framed}");
    }

    #[test]
    fn wake_prune_also_drops_closed_tasks_older_than_thirty_days() {
        let engine = engine_with("wd");
        engine
            .insert_task("c-old", "proj-1", "2026-08-01T00:00:00Z")
            .unwrap();
        engine.close_task("c-old", "2026-09-01T00:00:00Z").unwrap();
        engine
            .insert_task("c-open", "proj-1", "2026-08-01T00:00:00Z")
            .unwrap();
        let now = OffsetDateTime::parse(NOW, &Rfc3339).unwrap();
        prune(&engine, now);
        assert!(engine.get_task("c-old").unwrap().is_none());
        assert!(engine.get_task("c-open").unwrap().is_some());
    }

    #[test]
    fn wake_prune_drops_only_acked_rows_older_than_thirty_days() {
        let engine = engine_with("wd");
        let old = mail(&engine, "t-1", "old");
        let recent = mail(&engine, "t-2", "recent");
        mail(&engine, "t-3", "pending");
        engine
            .ack_inbox_message(old, "2026-09-01T00:00:00Z")
            .unwrap();
        engine
            .ack_inbox_message(recent, "2026-10-20T00:00:00Z")
            .unwrap();
        let now = OffsetDateTime::parse(NOW, &Rfc3339).unwrap();
        prune(&engine, now);
        assert_eq!(engine.prune_inbox("9999-01-01T00:00:00Z").unwrap(), 1);
    }

    #[tokio::test]
    async fn wake_cli_run_awaits_a_child_inside_the_running_runtime() {
        let out = cli_run(&["/bin/echo".to_string(), "hello".to_string()])
            .await
            .unwrap();
        assert_eq!(out.trim(), "hello");
        let err = cli_run(&[
            "/bin/sh".to_string(),
            "-c".to_string(),
            "echo boom >&2; exit 3".to_string(),
        ])
        .await
        .unwrap_err()
        .to_string();
        assert!(err.contains("boom"), "{err}");
    }
}
