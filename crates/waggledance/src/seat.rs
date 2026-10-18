use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

use waggledance_core::config::{OpenClawConfig, SupervisorConfig};
use waggledance_core::Config;

use crate::paseo::{command, detect_binary, parse_document, parse_listed, paseo_unavailable};

const SUPERVISOR_LABEL: &str = "waggledance.role=supervisor";
const PASEO_TIMEOUT: Duration = Duration::from_secs(15);
const REPORT_TIMEOUT: Duration = Duration::from_secs(900);
const GATEWAY_COMM: &str = "openclaw-gateway";

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
        "board" => "bảng việc",
        other => bail!(
            "unknown kind `{other}`: use result, question, blocked, error, closed, progress or board"
        ),
    })
}

pub fn framed_message(kind: &str, project: &str, corr: &str, text: &str) -> Result<String> {
    let label = kind_label(kind)?;
    Ok(format!(
        "Đối chiếu báo cáo này với đúng task bạn đã giao cho waggledance, rồi viết tin nhắn \
         người dùng sẽ đọc trên Telegram, bằng tiếng Việt: nêu việc đã xong và việc còn lại, \
         chép nguyên văn con số, đường dẫn và lời của leader; nếu báo cáo cần người dùng quyết \
         định thì hỏi thẳng người dùng: [waggledance#{corr}] {project} — {label}: {text}"
    ))
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

fn cli_run(rt: &tokio::runtime::Runtime, argv: &[String]) -> Result<String> {
    let shown = argv.join(" ");
    let program = argv[0].clone();
    let args: Vec<String> = argv[1..].to_vec();
    let (run_program, run_args, run_shown) = (program, args, shown.clone());
    let waited: Result<std::process::Output> = rt.block_on(async move {
        let child = tokio::process::Command::new(&run_program)
            .args(&run_args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| anyhow::anyhow!("running `{run_shown}`: {e}"))?;
        match tokio::time::timeout(REPORT_TIMEOUT, child.wait_with_output()).await {
            Ok(out) => Ok(out?),
            Err(_) => bail!(
                "`{run_shown}` timed out after {}s",
                REPORT_TIMEOUT.as_secs()
            ),
        }
    });
    let output = waited?;
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

fn report(
    config: &Config,
    proc_root: &Path,
    kind: &str,
    project: &str,
    corr: &str,
    text: &str,
) -> Result<String> {
    let message = framed_message(kind, project, corr, text)?;
    let cli = openclaw_cli(&config.openclaw, proc_root)?;
    let argv = report_argv(&cli, &config.openclaw, &message)?;
    let rt = tokio::runtime::Runtime::new()?;
    let stdout = cli_run(&rt, &argv)?;
    cli_status(&stdout, &argv.join(" "))
}

fn report_cmd(kind: &str, project: &str, corr: &str, text: &str, json: bool) -> Result<()> {
    let config = Config::load();
    let status = report(&config, Path::new("/proc"), kind, project, corr, text)?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "status": status, "sent": true, "via": "openclaw-cli" })
        );
    } else {
        println!("report sent ({kind}) for {corr}/{project} through the OpenClaw CLI");
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
                "claude/claude-sonnet-5-5",
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

    fn fake_proc_root(label: &str, gateway: Option<(&str, Option<&str>)>) -> PathBuf {
        let root = scratch(label);
        match gateway {
            Some((comm, appdir)) => {
                let pid = root.join("4242");
                std::fs::create_dir_all(&pid).unwrap();
                std::fs::write(pid.join("comm"), format!("{comm}\n")).unwrap();
                let mut environ: Vec<u8> = Vec::new();
                for kv in ["HOME=/home/u", "PATH=/usr/bin"] {
                    environ.extend_from_slice(kv.as_bytes());
                    environ.push(0);
                }
                if let Some(dir) = appdir {
                    environ.extend_from_slice(format!("APPDIR={dir}").as_bytes());
                    environ.push(0);
                }
                std::fs::write(pid.join("environ"), environ).unwrap();
            }
            None => {
                let pid = root.join("9");
                std::fs::create_dir_all(&pid).unwrap();
                std::fs::write(pid.join("comm"), "node\n").unwrap();
                std::fs::write(pid.join("environ"), b"HOME=/home/u\0").unwrap();
            }
        }
        root
    }

    #[test]
    fn the_framed_message_carries_the_tag_the_label_and_the_instruction() {
        let message = framed_message("result", "waggledance", "t-abc123", "xong rồi").unwrap();
        assert!(
            message.contains("[waggledance#t-abc123] waggledance — kết quả: xong rồi"),
            "{message}"
        );
        assert!(message.contains("Telegram"), "{message}");
        assert!(message.contains("tiếng Việt"), "{message}");
        assert!(message.contains("nguyên văn"), "{message}");
        assert!(message.contains("hỏi thẳng người dùng"), "{message}");
    }

    #[test]
    fn the_framed_message_maps_every_kind_and_refuses_an_unknown_one() {
        for (kind, label) in [
            ("result", "kết quả"),
            ("question", "câu hỏi"),
            ("blocked", "đang chờ quyền"),
            ("error", "lỗi"),
            ("closed", "đã đóng"),
            ("progress", "tiến độ"),
            ("board", "bảng việc"),
        ] {
            let message = framed_message(kind, "p", "t-1", "x").unwrap();
            assert!(
                message.contains(&format!("— {label}: x")),
                "{kind}: {message}"
            );
        }
        let err = framed_message("nonsense", "p", "t-1", "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("nonsense"), "{err}");
    }

    #[test]
    fn the_report_argv_names_the_agent_the_delivery_and_the_json_and_never_local() {
        let cli = vec!["node".to_string(), "/mnt/openclaw.mjs".to_string()];
        let with_to = OpenClawConfig {
            agent_id: "wd".into(),
            telegram_to: "12345".into(),
            ..OpenClawConfig::default()
        };
        let argv = report_argv(&cli, &with_to, "msg").unwrap();
        assert_eq!(
            argv,
            vec![
                "node",
                "/mnt/openclaw.mjs",
                "agent",
                "--agent",
                "wd",
                "--message",
                "msg",
                "--deliver",
                "--channel",
                "telegram",
                "--json",
                "--reply-to",
                "12345",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );
        assert!(!argv.iter().any(|a| a == "--local"), "{argv:?}");

        let bare = OpenClawConfig {
            agent_id: "wd".into(),
            ..OpenClawConfig::default()
        };
        let argv = report_argv(&cli, &bare, "msg").unwrap();
        assert!(!argv.iter().any(|a| a == "--reply-to"), "{argv:?}");
        assert!(!argv.iter().any(|a| a == "--local"), "{argv:?}");
    }

    #[test]
    fn a_report_without_an_agent_id_names_the_missing_config_key() {
        let err = report_argv(&["node".to_string()], &OpenClawConfig::default(), "m")
            .unwrap_err()
            .to_string();
        assert!(err.contains("agent_id"), "{err}");
    }

    #[test]
    fn a_kernel_truncated_comm_still_names_the_gateway() {
        let root = fake_proc_root("gw-trunc", Some(("openclaw-gatewa", Some("/tmp/.mount_Y"))));
        assert_eq!(
            openclaw_cli(&OpenClawConfig::default(), &root).unwrap()[1],
            "/tmp/.mount_Y/resources/openclaw/openclaw.mjs"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_comm_that_only_prefixes_the_gateway_name_is_not_the_gateway() {
        let root = fake_proc_root("gw-prefix", Some(("openclaw", Some("/tmp/.mount_Z"))));
        assert!(openclaw_cli(&OpenClawConfig::default(), &root).is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_cli_is_discovered_from_a_gateway_process_and_its_appdir() {
        let root = fake_proc_root("gw", Some(("openclaw-gateway", Some("/tmp/.mount_X"))));
        let cli = openclaw_cli(&OpenClawConfig::default(), &root).unwrap();
        assert_eq!(
            cli,
            vec![
                "node".to_string(),
                "/tmp/.mount_X/resources/openclaw/openclaw.mjs".to_string()
            ]
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_config_cli_overrides_the_gateway_discovery() {
        let root = fake_proc_root(
            "gw-override",
            Some(("openclaw-gateway", Some("/tmp/.mount_X"))),
        );
        let cfg = OpenClawConfig {
            cli: "bun /opt/openclaw/cli.mjs --flag".into(),
            ..OpenClawConfig::default()
        };
        assert_eq!(
            openclaw_cli(&cfg, &root).unwrap(),
            vec!["bun", "/opt/openclaw/cli.mjs", "--flag"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn with_no_gateway_and_no_override_the_refusal_names_both() {
        let root = fake_proc_root("gw-none", None);
        let err = openclaw_cli(&OpenClawConfig::default(), &root)
            .unwrap_err()
            .to_string();
        assert!(err.contains("openclaw-gateway"), "{err}");
        assert!(err.contains("[openclaw] cli"), "{err}");
        std::fs::remove_dir_all(&root).ok();
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

    fn cli_script(label: &str, body: &str) -> PathBuf {
        let dir = scratch(label);
        let path = dir.join("fake-openclaw.sh");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn config_over(script: &Path) -> Config {
        let mut config = Config::default();
        config.openclaw.cli = script.to_string_lossy().into_owned();
        config.openclaw.agent_id = "wd".into();
        config.openclaw.hook_url = "http://127.0.0.1:1/hooks/agent".into();
        config
    }

    #[test]
    fn a_report_runs_the_cli_in_the_wd_session_and_never_posts_to_the_hook() {
        let dir = scratch("report-argv");
        let argv_file = dir.join("argv.txt");
        let script = cli_script(
            "report-argv-script",
            &format!(
                "printf '%s\\n' \"$@\" > {}\necho '{{\"status\":\"ok\",\"result\":{{\"payloads\":[]}}}}'",
                argv_file.display()
            ),
        );
        let status = report(
            &config_over(&script),
            &dir,
            "question",
            "waggledance",
            "t-abc123",
            "màu gì?",
        )
        .unwrap();
        assert_eq!(status, "ok");

        let argv = std::fs::read_to_string(&argv_file).unwrap();
        let argv: Vec<&str> = argv.lines().collect();
        assert_eq!(
            argv,
            vec![
                "agent",
                "--agent",
                "wd",
                "--message",
                &framed_message("question", "waggledance", "t-abc123", "màu gì?").unwrap(),
                "--deliver",
                "--channel",
                "telegram",
                "--json",
            ]
        );
        assert!(!argv.contains(&"--local"));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(script.parent().unwrap()).ok();
    }

    #[test]
    fn a_report_adds_reply_to_only_when_a_telegram_target_is_set() {
        let dir = scratch("report-reply-to");
        let argv_file = dir.join("argv.txt");
        let script = cli_script(
            "report-reply-to-script",
            &format!(
                "printf '%s\\n' \"$@\" > {}\necho '{{\"status\":\"ok\"}}'",
                argv_file.display()
            ),
        );
        let mut config = config_over(&script);
        config.openclaw.telegram_to = "777".into();
        report(&config, &dir, "board", "waggledance", "t-1", "x").unwrap();
        let argv = std::fs::read_to_string(&argv_file).unwrap();
        let argv: Vec<&str> = argv.lines().collect();
        let at = argv.iter().position(|a| *a == "--reply-to").unwrap();
        assert_eq!(argv[at + 1], "777");
        assert!(argv[0..at].contains(&"--json"));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(script.parent().unwrap()).ok();
    }

    #[test]
    fn a_report_exits_non_zero_when_the_cli_does() {
        let dir = scratch("report-cli-red");
        let script = cli_script("report-cli-red-script", "echo 'no gateway' >&2\nexit 3");
        let err = report(&config_over(&script), &dir, "result", "p", "t-1", "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("no gateway"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(script.parent().unwrap()).ok();
    }

    #[test]
    fn a_report_exits_non_zero_when_the_cli_status_is_not_ok() {
        let dir = scratch("report-status-red");
        let script = cli_script("report-status-red-script", "echo '{\"status\":\"error\"}'");
        let err = report(&config_over(&script), &dir, "result", "p", "t-1", "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("status error"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(script.parent().unwrap()).ok();
    }

    #[test]
    fn a_report_refuses_before_running_anything_when_no_gateway_and_no_override_exist() {
        let root = fake_proc_root("report-no-gateway", None);
        let err = report(&Config::default(), &root, "result", "p", "t-1", "x")
            .unwrap_err()
            .to_string();
        assert!(err.contains("openclaw-gateway"), "{err}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_embedded_doc_matches_the_repo_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/waggledance-supervisor-agent.md");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DOC);
    }
}
