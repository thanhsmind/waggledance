use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

use waggledance_core::config::{read_hook_token, OpenClawConfig, SupervisorConfig};
use waggledance_core::Config;

use crate::paseo::{command, detect_binary, parse_document, parse_listed, paseo_unavailable};

const SUPERVISOR_LABEL: &str = "waggledance.role=supervisor";
const PASEO_TIMEOUT: Duration = Duration::from_secs(15);
const HOOK_TIMEOUT: Duration = Duration::from_secs(15);
const BODY_HEAD: usize = 300;

pub const DOC: &str = include_str!("../../../docs/waggledance-supervisor-agent.md");

const BOOTSTRAP: &str =
    "Read CLAUDE.md in this directory, create your heartbeat if none exists, then reply ready.";

type Runner<'a> = &'a mut dyn FnMut(&[&str]) -> Result<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ensured {
    pub agent_id: String,
    pub created: bool,
    pub instructions_stale: bool,
    pub cwd: String,
}

pub fn supervisor_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("supervisor")
}

pub fn supervisor_ls_args() -> [&'static str; 5] {
    ["ls", "-g", "--label", SUPERVISOR_LABEL, "--json"]
}

fn supervisor_run_args(dir: &Path, cfg: &SupervisorConfig) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--json".into(),
        "--cwd".into(),
        dir.to_string_lossy().into_owned(),
        "--provider".into(),
        cfg.provider.clone(),
    ];
    for (flag, value) in [("--mode", &cfg.mode), ("--thinking", &cfg.thinking)] {
        if !value.is_empty() {
            args.push(flag.into());
            args.push(value.clone());
        }
    }
    args.push("--title".into());
    args.push(cfg.title.clone());
    args.push("--label".into());
    args.push(SUPERVISOR_LABEL.into());
    args.push(BOOTSTRAP.into());
    args
}

pub fn write_doc(dir: &Path) -> Result<bool> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join("CLAUDE.md");
    if std::fs::read_to_string(&path).is_ok_and(|have| have == DOC) {
        return Ok(false);
    }
    std::fs::write(&path, DOC).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

fn ensure_json(e: &Ensured) -> serde_json::Value {
    serde_json::json!({
        "agent_id": e.agent_id,
        "created": e.created,
        "instructions_stale": e.instructions_stale,
        "cwd": e.cwd,
    })
}

fn ensure_over(run: Runner, dir: &Path, cfg: &SupervisorConfig) -> Result<Ensured> {
    let listed = run(&supervisor_ls_args())?;
    let found = parse_listed(&listed)
        .map_err(|e| anyhow::anyhow!("reading `paseo ls --label {SUPERVISOR_LABEL} --json`: {e}"))?
        .into_iter()
        .next();
    let stale = write_doc(dir)?;
    if let Some(agent) = found {
        let cwd = if agent.cwd.is_empty() {
            dir.to_string_lossy().into_owned()
        } else {
            agent.cwd.clone()
        };
        return Ok(Ensured {
            agent_id: agent.id,
            created: false,
            instructions_stale: stale,
            cwd,
        });
    }
    let args = supervisor_run_args(dir, cfg);
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run(&argv)?;
    let agent_id = agent_id_from(&out)?;
    Ok(Ensured {
        agent_id,
        created: true,
        instructions_stale: false,
        cwd: dir.to_string_lossy().into_owned(),
    })
}

fn agent_id_from(out: &str) -> Result<String> {
    let noise = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("no output");
    let doc: serde_json::Value = parse_document(out).map_err(|e| {
        anyhow::anyhow!("`paseo run -d --json` printed no JSON document ({e}); first line: {noise}")
    })?;
    doc.get("agentId")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .with_context(|| format!("`paseo run -d --json` printed no agentId; first line: {noise}"))
}

fn paseo_exec(rt: &tokio::runtime::Runtime, binary: &str, args: &[&str]) -> Result<String> {
    let owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    let shown = owned.join(" ");
    let (run_binary, run_owned, run_shown) = (binary.to_string(), owned, shown.clone());
    let waited: Result<std::process::Output> = rt.block_on(async move {
        let child = command(&run_binary, &run_owned)
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("running `{run_binary} {run_shown}`: {e}"))?;
        match tokio::time::timeout(PASEO_TIMEOUT, child.wait_with_output()).await {
            Ok(out) => Ok(out?),
            Err(_) => bail!(
                "`{run_binary} {run_shown}` timed out after {}s",
                PASEO_TIMEOUT.as_secs()
            ),
        }
    });
    let output = waited?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        bail!(
            "`{binary} {shown}` failed: {}{}",
            String::from_utf8_lossy(&output.stderr).trim(),
            stdout.trim()
        );
    }
    Ok(stdout)
}

fn ensure(json: bool) -> Result<()> {
    let config = Config::load();
    if let Some(reason) = paseo_unavailable(&config.paseo) {
        bail!("paseo_unavailable: {reason}");
    }
    let binary = detect_binary(&config.paseo)
        .binary
        .unwrap_or_else(|| "paseo".into());
    let dir = supervisor_dir(&waggledance_core::config::resolve_data_dir(None));
    let rt = tokio::runtime::Runtime::new()?;
    let mut run = |args: &[&str]| -> Result<String> { paseo_exec(&rt, &binary, args) };
    let done = ensure_over(&mut run, &dir, &config.supervisor)?;
    if json {
        println!("{}", ensure_json(&done));
    } else {
        println!(
            "supervisor: {} (created: {}, instructions_stale: {})",
            done.agent_id, done.created, done.instructions_stale
        );
        println!("  {}", done.cwd);
    }
    Ok(())
}

fn kind_label(kind: &str) -> Result<&'static str> {
    Ok(match kind {
        "result" => "kết quả",
        "question" => "câu hỏi",
        "blocked" => "đang chờ quyền",
        "error" => "lỗi",
        "closed" => "đã đóng",
        "progress" => "tiến độ",
        other => bail!(
            "unknown kind `{other}`: use result, question, blocked, error, closed or progress"
        ),
    })
}

fn hook_token_or_refuse(dir: &Path) -> Result<String> {
    read_hook_token(dir).ok_or_else(|| {
        anyhow::anyhow!(
            "no hook token at {} — run `waggledance setup openclaw` to write it",
            waggledance_core::config::hook_token_path(dir).display()
        )
    })
}

pub fn hook_body(
    cfg: &OpenClawConfig,
    kind: &str,
    project: &str,
    corr: &str,
    text: &str,
) -> Result<serde_json::Value> {
    let label = kind_label(kind)?;
    let message = format!(
        "Chuyển nguyên văn cho người dùng, không diễn giải lại, không dịch, không thêm lời: \
         [waggledance#{corr}] {project} — {label}: {text}"
    );
    let mut body = serde_json::json!({
        "message": message,
        "name": "waggledance",
        "deliver": true,
        "channel": "telegram",
        "wakeMode": "now",
    });
    let object = body.as_object_mut().expect("object literal");
    if !cfg.agent_id.is_empty() {
        object.insert("agentId".into(), serde_json::json!(cfg.agent_id));
    }
    if !cfg.telegram_to.is_empty() {
        object.insert("to".into(), serde_json::json!(cfg.telegram_to));
    }
    if !cfg.hook_model.is_empty() {
        object.insert("model".into(), serde_json::json!(cfg.hook_model));
    }
    object.insert("thinking".into(), serde_json::json!(cfg.hook_thinking));
    Ok(body)
}

fn head(text: &str) -> String {
    text.chars().take(BODY_HEAD).collect()
}

async fn post(url: &str, token: &str, body: &serde_json::Value) -> Result<u16> {
    let client = reqwest::Client::builder().timeout(HOOK_TIMEOUT).build()?;
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body)?)
        .send()
        .await;
    match resp {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(status.as_u16())
            } else {
                let text = resp.text().await.unwrap_or_default();
                bail!("hook at {url} returned {status}: {}", head(text.trim()))
            }
        }
        Err(e) => bail!("hook at {url} could not be reached: {e}"),
    }
}

fn report(
    config: &Config,
    dir: &Path,
    kind: &str,
    project: &str,
    corr: &str,
    text: &str,
) -> Result<u16> {
    let token = hook_token_or_refuse(dir)?;
    let body = hook_body(&config.openclaw, kind, project, corr, text)?;
    let url = config.openclaw.hook_url.clone();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(post(&url, &token, &body))
}

fn report_cmd(kind: &str, project: &str, corr: &str, text: &str, json: bool) -> Result<()> {
    let config = Config::load();
    let dir = waggledance_core::config::resolve_data_dir(None);
    let status = report(&config, &dir, kind, project, corr, text)?;
    if json {
        println!("{}", serde_json::json!({ "status": status, "sent": true }));
    } else {
        println!("report sent ({kind}) for {corr}/{project}");
    }
    Ok(())
}

pub fn run(action: crate::cli::SupervisorAction) -> Result<()> {
    match action {
        crate::cli::SupervisorAction::Ensure { json } => ensure(json),
        crate::cli::SupervisorAction::Report {
            kind,
            project,
            corr,
            text,
            json,
        } => report_cmd(&kind, &project, &corr, &text, json),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SupervisorConfig {
        SupervisorConfig::default()
    }

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "waggledance-seat-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const LISTED: &str = r#"22:47:26.299 › [desktop] app startup { version: '0.11.2' }
[{"id":"sup-1","cwd":"/home/u/.waggledance/supervisor","status":"idle","created":"1 hours ago"}]
"#;

    #[test]
    fn a_listed_supervisor_is_reused_and_no_run_is_issued() {
        let dir = scratch("reuse");
        assert!(write_doc(&dir).unwrap());
        let mut calls: Vec<String> = Vec::new();
        let mut run = |args: &[&str]| -> Result<String> {
            calls.push(args.join(" "));
            assert_eq!(args[0], "ls");
            Ok(LISTED.to_string())
        };
        let done = ensure_over(&mut run, &dir, &cfg()).unwrap();
        assert_eq!(
            done,
            Ensured {
                agent_id: "sup-1".into(),
                created: false,
                instructions_stale: false,
                cwd: "/home/u/.waggledance/supervisor".into(),
            }
        );
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert!(dir.join("CLAUDE.md").is_file());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_reused_supervisor_whose_instructions_changed_is_reported_stale() {
        let dir = scratch("reuse-stale");
        std::fs::write(dir.join("CLAUDE.md"), "old instructions").unwrap();
        let mut run = |_: &[&str]| -> Result<String> { Ok(LISTED.to_string()) };
        let done = ensure_over(&mut run, &dir, &cfg()).unwrap();
        assert!(!done.created);
        assert!(done.instructions_stale);
        assert_eq!(std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(), DOC);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_empty_list_runs_once_with_the_exact_argv_and_parses_the_fixture() {
        let dir = scratch("create");
        let mut calls: Vec<Vec<String>> = Vec::new();
        let fixture = include_str!("../testdata/fixtures/paseo-0.11.2/npm-run-detached.out");
        let mut run = |args: &[&str]| -> Result<String> {
            calls.push(args.iter().map(|a| (*a).to_string()).collect());
            Ok(if args[0] == "ls" {
                "[]\n".to_string()
            } else {
                fixture.to_string()
            })
        };
        let done = ensure_over(&mut run, &dir, &cfg()).unwrap();
        assert!(done.created);
        assert_eq!(done.agent_id, "c4eb3cbd-22e6-45a8-acc3-c97f5d8b7a46");
        assert_eq!(done.cwd, dir.to_string_lossy());
        assert_eq!(calls.len(), 2, "{calls:?}");
        assert_eq!(
            calls[1],
            vec![
                "run",
                "-d",
                "--json",
                "--cwd",
                &dir.to_string_lossy(),
                "--provider",
                "claude/claude-haiku-5-5",
                "--mode",
                "bypassPermissions",
                "--title",
                "waggledance supervisor",
                "--label",
                "waggledance.role=supervisor",
                BOOTSTRAP,
            ]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_thinking_value_adds_its_flag_only_when_set() {
        let dir = scratch("thinking");
        let mut with = cfg();
        with.thinking = "low".into();
        let args = supervisor_run_args(&dir, &with);
        let at = args.iter().position(|a| a == "--thinking").unwrap();
        assert_eq!(args[at + 1], "low");
        assert!(!supervisor_run_args(&dir, &cfg())
            .iter()
            .any(|a| a == "--thinking"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_doc_writes_once_and_leaves_a_fresh_file_alone() {
        let dir = scratch("stale");
        assert!(write_doc(&dir).unwrap());
        assert!(!write_doc(&dir).unwrap());
        assert_eq!(std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(), DOC);
        std::fs::write(dir.join("CLAUDE.md"), "old instructions").unwrap();
        assert!(write_doc(&dir).unwrap());
        assert_eq!(std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(), DOC);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_created_supervisor_reports_no_stale_instructions() {
        let dir = scratch("created-stale");
        std::fs::write(dir.join("CLAUDE.md"), "old").unwrap();
        let fixture = include_str!("../testdata/fixtures/paseo-0.11.2/npm-run-detached.out");
        let mut run = |args: &[&str]| -> Result<String> {
            Ok(if args[0] == "ls" {
                "[]\n".into()
            } else {
                fixture.into()
            })
        };
        let done = ensure_over(&mut run, &dir, &cfg()).unwrap();
        assert!(done.created);
        assert!(!done.instructions_stale);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_run_document_without_an_agent_id_names_the_first_noise_line() {
        let dir = scratch("noagent");
        let mut run = |args: &[&str]| -> Result<String> {
            Ok(if args[0] == "ls" {
                "[]\n".into()
            } else {
                "23:51:21.452 › [login-shell-env] start {\n".into()
            })
        };
        let err = ensure_over(&mut run, &dir, &cfg()).unwrap_err().to_string();
        assert!(err.contains("23:51:21.452"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hook_body_carries_the_verbatim_instruction_the_tag_and_the_label() {
        let cfg = OpenClawConfig::default();
        let body = hook_body(&cfg, "result", "waggledance", "t-abc123", "xong rồi").unwrap();
        let message = body["message"].as_str().unwrap();
        assert_eq!(
            message,
            "Chuyển nguyên văn cho người dùng, không diễn giải lại, không dịch, không thêm lời: \
             [waggledance#t-abc123] waggledance — kết quả: xong rồi"
        );
        assert_eq!(body["name"], "waggledance");
        assert_eq!(body["deliver"], true);
        assert_eq!(body["channel"], "telegram");
        assert_eq!(body["wakeMode"], "now");
        assert_eq!(body["thinking"], "low");
        assert!(body.get("to").is_none());
        assert!(body.get("model").is_none());
        assert!(body.get("agentId").is_none());
    }

    #[test]
    fn hook_body_maps_every_kind_and_refuses_an_unknown_one() {
        let cfg = OpenClawConfig::default();
        for (kind, label) in [
            ("result", "kết quả"),
            ("question", "câu hỏi"),
            ("blocked", "đang chờ quyền"),
            ("error", "lỗi"),
            ("closed", "đã đóng"),
            ("progress", "tiến độ"),
        ] {
            let body = hook_body(&cfg, kind, "p", "t-1", "x").unwrap();
            let message = body["message"].as_str().unwrap();
            assert!(
                message.contains(&format!("— {label}: x")),
                "{kind}: {message}"
            );
        }
        let err = hook_body(&cfg, "nonsense", "p", "t-1", "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("nonsense"), "{err}");
    }

    #[test]
    fn hook_body_adds_to_and_model_only_when_configured() {
        let cfg = OpenClawConfig {
            agent_id: "wd".into(),
            telegram_to: "12345".into(),
            hook_model: "gemini-flash".into(),
            hook_thinking: "minimal".into(),
            ..OpenClawConfig::default()
        };
        let body = hook_body(&cfg, "progress", "p", "t-1", "x").unwrap();
        assert_eq!(body["agentId"], "wd");
        assert_eq!(body["to"], "12345");
        assert_eq!(body["model"], "gemini-flash");
        assert_eq!(body["thinking"], "minimal");
    }

    #[test]
    fn paseo_exec_runs_the_child_inside_the_runtime_and_captures_its_stdout() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let out = paseo_exec(&rt, "/bin/echo", &["hello", "seat"]).unwrap();
        assert_eq!(out.trim(), "hello seat");
    }

    #[test]
    fn paseo_exec_reports_a_non_zero_exit_with_its_stderr() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = paseo_exec(&rt, "/bin/sh", &["-c", "echo boom >&2; exit 3"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("boom"), "{err}");
    }

    #[test]
    fn report_refuses_without_a_token_file_before_any_request() {
        let dir = scratch("notoken");
        assert_eq!(read_hook_token(&dir), None);
        let err = hook_token_or_refuse(&dir).unwrap_err().to_string();
        assert!(err.contains("setup openclaw"), "{err}");
        assert!(err.contains("hook.token"), "{err}");

        let mut config = Config::default();
        config.openclaw.hook_url = "http://127.0.0.1:1/hooks/agent".into();
        let err = report(&config, &dir, "result", "p", "t-1", "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("setup openclaw"), "{err}");
        assert!(!err.contains("127.0.0.1:1"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn report_posts_the_body_with_the_bearer_header_and_reports_the_status() {
        use axum::routing::post;
        use axum::{extract::State, Json};
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct Seen {
            auth: Option<String>,
            body: serde_json::Value,
        }

        let seen: Arc<Mutex<Option<Seen>>> = Arc::new(Mutex::new(None));
        let handler = move |State(seen): State<Arc<Mutex<Option<Seen>>>>,
                            headers: axum::http::HeaderMap,
                            Json(body): Json<serde_json::Value>| async move {
            *seen.lock().unwrap() = Some(Seen {
                auth: headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string),
                body,
            });
            axum::http::StatusCode::OK
        };
        let app = axum::Router::new()
            .route("/hooks/agent", post(handler))
            .with_state(seen.clone());

        let (tx, rx) = std::sync::mpsc::channel();
        let server_seen = seen.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                tx.send(listener.local_addr().unwrap()).unwrap();
                let _ = axum::serve(listener, app).await;
            });
            drop(server_seen);
        });
        let addr = rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let dir = scratch("report-ok");
        std::fs::write(waggledance_core::config::hook_token_path(&dir), "tok-9\n").unwrap();
        let mut config = Config::default();
        config.openclaw.hook_url = format!("http://{addr}/hooks/agent");

        let status = report(
            &config,
            &dir,
            "question",
            "waggledance",
            "t-abc123",
            "màu gì?",
        )
        .unwrap();
        assert_eq!(status, 200);

        let seen = seen.lock().unwrap().clone().unwrap();
        assert_eq!(seen.auth.as_deref(), Some("Bearer tok-9"));
        assert_eq!(seen.body["deliver"], true);
        assert_eq!(seen.body["channel"], "telegram");
        assert!(seen.body["message"].as_str().unwrap().contains("câu hỏi"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_embedded_doc_matches_the_repo_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/waggledance-supervisor-agent.md");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DOC);
    }
}
