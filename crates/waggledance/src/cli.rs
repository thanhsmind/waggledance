//! CLI surface (PRD §5.6). Registry commands operate on the shared SQLite DB;
//! the running daemon serves from the same DB. `serve`/`mcp` are the long-running
//! modes.

use crate::runtime;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use waggledance_core::bee_upgrade::{self, Outcome, UpgradeOptions, UpgradeReport};
use waggledance_core::Config;

#[derive(Parser)]
#[command(
    name = "waggledance",
    version,
    about = "Supervisor for AI agent workflows",
    after_help = "Agent verbs: projects, state, dispatch, await, runs, task, intake, schema, search.\nFind the verb for a job with `waggledance search <text>`; `waggledance schema` lists every verb and flag as JSON."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the supervisor daemon (background loops, no web server).
    Serve,
    /// Register a project.
    Register {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List registered projects.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show daemon status.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Remove a project from the registry (files are not deleted).
    Unregister { project_id: String },
    /// Stop the running daemon.
    Stop,
    /// Restart the daemon: stop the running one (if any), then start a fresh
    /// detached daemon. Useful after changing config (config changes).
    Restart,
    /// Diagnose & auto-fix integration with Claude Code.
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        fix: bool,
    },
    /// Run the MCP server over stdio (used by Claude Code).
    Mcp,
    /// Install (or --remove) the pi extension or the OpenClaw bridge.
    Setup {
        #[arg(value_enum)]
        target: crate::setup::Target,
        #[arg(long)]
        remove: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Send one message to wd's inbox (leaders use this to report back).")]
    Mail {
        #[command(subcommand)]
        action: MailAction,
    },
    #[command(
        about = "Tell waggledance this leader is waiting on handed-out work, or release the wait."
    )]
    Hold {
        #[arg(long, required_unless_present = "release", conflicts_with = "release")]
        why: Option<String>,
        #[arg(long)]
        minutes: Option<u64>,
        #[arg(long)]
        release: bool,
    },
    /// Edit configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    #[command(about = "Manage bee across registered projects.")]
    Bee {
        #[command(subcommand)]
        action: BeeAction,
    },
    /// Print the version (same value as `--version`).
    Version,
    #[command(about = "List registered projects as JSON.")]
    Projects(OutputArgs),
    #[command(about = "Show bee state: one summary row per project, or one project's digest.")]
    State {
        project: Option<String>,
        #[command(flatten)]
        out: OutputArgs,
    },
    #[command(
        about = "Dispatch a task to a project's leader, a fresh agent preset or a running pane; returns a run_id."
    )]
    Dispatch {
        #[arg(long, help = "Registered project id")]
        project: String,
        #[arg(
            long,
            conflicts_with_all = ["preset", "pane_id"],
            help = "Hand the task to the project's Paseo leader"
        )]
        leader: bool,
        #[arg(long, help = "Spawn a fresh agent from this preset label")]
        preset: Option<String>,
        #[arg(long, help = "Send the task to this running pane id")]
        pane_id: Option<String>,
        #[arg(long, conflicts_with = "task_file", help = "The task text")]
        task: Option<String>,
        #[arg(
            long,
            help = "Read the task text from this file; stdin when no task is given"
        )]
        task_file: Option<PathBuf>,
        #[arg(long, help = "Caller-chosen id so a retry never sends twice")]
        request_id: Option<String>,
        #[arg(
            long,
            help = "Run id or request id whose leader takes the task as a follow-up"
        )]
        follow_up_of: Option<String>,
        #[arg(
            long,
            conflicts_with = "no_handoff",
            help = "Start a fresh leader that takes over the follow-up's family"
        )]
        handoff: bool,
        #[arg(long, help = "Opt out of the automatic leader handoff")]
        no_handoff: bool,
        #[arg(
            long,
            help = "Send the task to the follow-up's leader as a context-only note"
        )]
        steer: bool,
    },
    #[command(
        about = "Wait for a dispatched run to finish or block; prints its status and transcript tail."
    )]
    Await {
        run_id: String,
        #[arg(long, help = "Max seconds to wait, clamped to 60")]
        timeout: Option<u64>,
        #[command(flatten)]
        out: OutputArgs,
    },
    #[command(
        about = "List dispatched runs with a one-line task summary; `runs show` prints one run in full."
    )]
    Runs {
        #[arg(long, help = "Only this project's runs")]
        project: Option<String>,
        #[command(flatten)]
        out: OutputArgs,
        #[command(subcommand)]
        action: Option<RunsAction>,
    },
    #[command(about = "Answer a leader's task by corr: reply, redo, accept, allow or deny.")]
    Task {
        corr: String,
        #[arg(value_enum)]
        action: TaskVerb,
        #[arg(long, help = "The user's words, required for reply and redo")]
        text: Option<String>,
        #[arg(long, help = "Permission request id for allow and deny")]
        req_id: Option<String>,
        #[command(flatten)]
        out: OutputArgs,
    },
    #[command(
        about = "Pull code work from anphabe: `intake run` makes one pass and prints the report; `intake list` prints ledger rows."
    )]
    Intake {
        #[command(subcommand)]
        action: IntakeAction,
    },
    #[command(about = "Print every agent verb, or one verb, with its flags and help as JSON.")]
    Schema { verb: Option<String> },
    #[command(about = "Find the agent verb for a job: ranks verbs by the words you give.")]
    Search {
        #[arg(required = true)]
        text: Vec<String>,
    },
}

#[derive(Args, Clone, Default)]
pub struct OutputArgs {
    #[arg(
        long,
        global = true,
        help = "Print the tool's full structuredContent unchanged"
    )]
    pub full: bool,
    #[arg(
        long,
        global = true,
        value_delimiter = ',',
        help = "Keep only these comma-separated keys on each row"
    )]
    pub fields: Vec<String>,
    #[arg(long, global = true, help = "Cap the number of rows")]
    pub limit: Option<usize>,
}

#[derive(Subcommand)]
pub enum RunsAction {
    #[command(about = "Print one run in full, including its whole task.")]
    Show { run_id: String },
}

#[derive(Subcommand)]
pub enum IntakeAction {
    #[command(
        about = "Make one intake pass now and print what was dispatched, held, skipped and covered."
    )]
    Run,
    #[command(
        about = "Print intake ledger rows as JSON; held items unless --verdict says otherwise."
    )]
    List {
        #[arg(
            long,
            value_enum,
            default_value = "held",
            help = "Which verdict to list"
        )]
        verdict: IntakeVerdictArg,
    },
}

#[derive(ValueEnum, Clone, Copy)]
pub enum IntakeVerdictArg {
    Held,
    Dispatched,
    Skipped,
    Covered,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum TaskVerb {
    Reply,
    Redo,
    Accept,
    Allow,
    Deny,
}

#[derive(Subcommand)]
pub enum MailAction {
    #[command(about = "Queue one message for wd; the body is read from --body-file or stdin.")]
    Send {
        #[arg(long)]
        to: String,
        #[arg(long)]
        corr: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        body_file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum BeeAction {
    #[command(about = "Upgrade bee in every registered project; a dry run unless --apply.")]
    Upgrade {
        #[arg(long)]
        apply: bool,
        #[arg(long = "project")]
        project: Vec<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Open the config file (`~/.waggledance/config.toml`) in $EDITOR.
    Edit,
}

pub fn run(cli: Cli) -> Result<()> {
    // D2: arm the `~/.mdview` → `~/.waggledance` one-time migration for the
    // rest of this process. `run` is the single dispatch point every real
    // subcommand passes through (including `mcp` and `doctor`, which live
    // in their own modules), so arming it once here — before any of them
    // gets a chance to resolve the data directory — covers all of them
    // without a matching call at each one, and a re-exec of this same
    // binary (`runtime::spawn_daemon_detached`) re-arms itself the same way
    // by calling back into this function. No test in this workspace calls
    // `run`, so the migration stays disarmed for the entire test suite.
    waggledance_core::config::arm_data_dir_migration();
    match cli.command {
        Command::Serve => cmd_serve(),
        Command::Register { path, name, json } => cmd_register(&path, name.as_deref(), json),
        Command::List { json } => cmd_list(json),
        Command::Status { json } => cmd_status(json),
        Command::Unregister { project_id } => cmd_unregister(&project_id),
        Command::Stop => cmd_stop(),
        Command::Restart => cmd_restart(),
        Command::Doctor { json, dry_run, fix } => crate::doctor::run(json, dry_run, fix),
        Command::Mcp => crate::mcp::run(),
        Command::Setup {
            target,
            remove,
            json,
        } => crate::setup::run(target, remove, json),
        Command::Mail { action } => crate::mail::run(action),
        Command::Hold {
            why,
            minutes,
            release,
        } => crate::hold::run(why, minutes, release),
        Command::Config { action } => match action {
            ConfigAction::Edit => cmd_config_edit(),
        },
        Command::Bee { action } => match action {
            BeeAction::Upgrade {
                apply,
                project,
                json,
            } => cmd_bee_upgrade(apply, project, json),
        },
        Command::Version => {
            // Single source of truth: the workspace Cargo package version.
            println!("waggledance {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        agent => crate::agent_cli::run(agent),
    }
}

/// Resolved editor to spawn: the program name, its leading args (from
/// splitting a "program --flag"-shaped editor string), and the raw combined
/// string (unsplit, for error messages).
struct ResolvedEditor {
    display: String,
    program: String,
    args: Vec<String>,
}

/// Resolve which editor to spawn: prefer `visual` (`$VISUAL`), then `editor`
/// (`$EDITOR`), else a platform default (`notepad` on Windows, `vi`
/// elsewhere). Editors passed with args (e.g. "code --wait") are split on
/// whitespace; the config path is appended separately by the caller.
fn resolve_editor(visual: Option<String>, editor: Option<String>) -> ResolvedEditor {
    let editor = visual
        .or(editor)
        .unwrap_or_else(|| if cfg!(windows) { "notepad" } else { "vi" }.to_string());
    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("vi").to_string();
    let args: Vec<String> = parts.map(str::to_string).collect();
    ResolvedEditor {
        display: editor,
        program,
        args,
    }
}

/// Classify the post-edit re-read of the config file into the message
/// `cmd_config_edit` prints: valid TOML (saved), invalid TOML (warning with
/// the parse error), or an unreadable file (re-read error).
fn classify_config_edit_outcome(path: &Path, read_result: std::io::Result<String>) -> String {
    match read_result {
        Ok(text) => match toml::from_str::<Config>(&text) {
            Ok(_) => format!(
                "Saved {}.\nRestart the daemon to apply: waggledance restart",
                path.display()
            ),
            Err(e) => format!(
                "Warning: {} is not valid TOML ({e}).\nwaggledance will ignore it and fall back to defaults until you fix it.",
                path.display()
            ),
        },
        Err(e) => format!("Could not re-read {}: {e}", path.display()),
    }
}

fn cmd_config_edit() -> Result<()> {
    let path = waggledance_core::config::config_path();
    // Materialize the file with current (or default) values so the editor opens
    // a fully-populated config, not an empty/absent file.
    Config::load()
        .save()
        .with_context(|| format!("writing {}", path.display()))?;

    let resolved = resolve_editor(std::env::var("VISUAL").ok(), std::env::var("EDITOR").ok());
    let status = std::process::Command::new(&resolved.program)
        .args(&resolved.args)
        .arg(&path)
        .status()
        .with_context(|| format!("launching editor '{}'", resolved.display))?;
    if !status.success() {
        println!("Editor exited without success; config left unchanged on disk.");
        return Ok(());
    }

    // Validate what the user saved: a broken TOML would otherwise be silently
    // ignored (Config::load falls back to defaults), so warn loudly instead.
    println!(
        "{}",
        classify_config_edit_outcome(&path, std::fs::read_to_string(&path))
    );
    Ok(())
}

fn cmd_bee_upgrade(apply: bool, only: Vec<String>, json: bool) -> Result<()> {
    let engine = runtime::build_engine()?;
    let projects = engine.list_projects()?;
    let opts = UpgradeOptions {
        apply,
        only,
        ..Default::default()
    };
    let report = bee_upgrade::run(&projects, &opts).map_err(|e| anyhow::anyhow!("{e}"))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render_upgrade_text(&report));
    }
    Ok(())
}

fn status_word(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::UpToDate => "up_to_date",
        Outcome::Pending { .. } => "pending",
        Outcome::Upgraded { .. } => "upgraded",
        Outcome::AppliedNotCommitted { .. } => "applied_not_committed",
        Outcome::Skipped { .. } => "skipped",
        Outcome::Failed { .. } => "failed",
    }
}

fn render_upgrade_text(report: &UpgradeReport) -> String {
    let dash = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".to_string());
    let mut out = format!(
        "bee source: {} @ {} {}{}\ntarget: {}\n",
        report.source_root.display(),
        dash(&report.source_branch),
        dash(&report.source_head),
        if report.source_dirty { " (dirty)" } else { "" },
        dash(&report.target_version),
    );
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for p in &report.projects {
        let word = status_word(&p.outcome);
        let (files, detail) = match &p.outcome {
            Outcome::UpToDate => (None, ""),
            Outcome::Pending { files } => (Some(files.len()), ""),
            Outcome::Upgraded { commit, files } => (Some(files.len()), commit.as_str()),
            Outcome::AppliedNotCommitted { files, error } => (Some(files.len()), error.as_str()),
            Outcome::Skipped { reason, files } => (Some(files.len()), reason.as_str()),
            Outcome::Failed { error } => (None, error.as_str()),
        };
        out.push_str(&format!(
            "{}  {}  {}  {word}",
            p.id,
            dash(&p.branch),
            dash(&p.from_version)
        ));
        if let Some(n) = files {
            out.push_str(&format!("  {n} files"));
        }
        if !detail.is_empty() {
            out.push_str(&format!("  {detail}"));
        }
        if !p.notices.is_empty() {
            out.push_str(&format!("  {} notices", p.notices.len()));
        }
        out.push('\n');
        match counts.iter_mut().find(|(w, _)| *w == word) {
            Some((_, c)) => *c += 1,
            None => counts.push((word, 1)),
        }
    }
    for (word, c) in counts {
        out.push_str(&format!("{word}: {c}\n"));
    }
    if !report.applied {
        out.push_str("dry run — re-run with --apply to upgrade and commit\n");
    }
    out
}

fn cmd_serve() -> Result<()> {
    if runtime::running_daemon().is_some() {
        println!("A waggledance daemon is already running.");
        return Ok(());
    }
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(crate::daemon::serve())
}

fn cmd_register(path: &Path, name: Option<&str>, json: bool) -> Result<()> {
    let engine = runtime::build_engine()?;
    let project = engine.register(path, name)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "project_id": project.id, "name": project.name,
                "root_path": project.root_path
            })
        );
    } else {
        println!("Registered '{}' ({})", project.name, project.id);
        println!("  {}", project.root_path.display());
    }
    Ok(())
}

fn cmd_list(json: bool) -> Result<()> {
    let engine = runtime::build_engine()?;
    let projects = engine.list_projects()?;
    if json {
        let arr: Vec<_> = projects
            .iter()
            .map(|p| serde_json::json!({ "id": p.id, "name": p.name, "root_path": p.root_path }))
            .collect();
        println!("{}", serde_json::json!({ "projects": arr }));
    } else if projects.is_empty() {
        println!("No projects registered.");
    } else {
        for p in &projects {
            println!("{:<20} {}", p.id, p.root_path.display());
        }
    }
    Ok(())
}

fn cmd_status(json: bool) -> Result<()> {
    let engine = runtime::build_engine()?;
    let daemon = runtime::running_daemon();
    let projects = engine.list_projects()?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "running": daemon.is_some(),
                "pid": daemon.as_ref().map(|d| d.pid),
                "version": env!("CARGO_PKG_VERSION"),
                "project_count": projects.len(),
            })
        );
    } else {
        match &daemon {
            Some(d) => println!("running: pid {}", d.pid),
            None => println!("running: no"),
        }
        println!("projects: {}", projects.len());
    }
    Ok(())
}

fn cmd_unregister(id: &str) -> Result<()> {
    let engine = runtime::build_engine()?;
    engine.unregister(id)?;
    println!("Unregistered '{id}'.");
    Ok(())
}

fn stop_daemon() -> Option<(u32, bool)> {
    let info = runtime::read_lock()?;
    if runtime::running_daemon().is_none() {
        runtime::remove_lock();
        return Some((info.pid, false));
    }
    #[cfg(unix)]
    let ok = std::process::Command::new("kill")
        .arg(info.pid.to_string())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    #[cfg(not(unix))]
    let ok = std::process::Command::new("taskkill")
        .args(["/PID", &info.pid.to_string(), "/F"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let orphaned = !ok && runtime::running_daemon().is_some();
    if !orphaned {
        runtime::remove_lock();
    }
    Some((info.pid, ok))
}

/// Map `stop_daemon()`'s outcome to `cmd_stop`'s exact printed message: a
/// successful kill, a failed kill (process likely already gone), or no
/// daemon recorded at all.
fn stop_outcome_message(outcome: Option<(u32, bool)>) -> String {
    match outcome {
        Some((pid, true)) => format!("Stopped daemon (pid {pid})."),
        Some((pid, false)) => format!("Could not stop pid {pid}. It may already be gone."),
        None => "No daemon running.".to_string(),
    }
}

/// Map `stop_daemon()`'s outcome to `cmd_restart`'s exact printed message
/// for the stop phase: a daemon was found and a stop was attempted (the
/// kill result itself doesn't change the message here, since restart
/// proceeds to spawn either way), or no daemon was running to begin with.
fn restart_stop_message(outcome: Option<(u32, bool)>) -> String {
    match outcome {
        Some((pid, _)) => format!("Stopped daemon (pid {pid})."),
        None => "No daemon was running.".to_string(),
    }
}

fn cmd_stop() -> Result<()> {
    println!("{}", stop_outcome_message(stop_daemon()));
    Ok(())
}

fn cmd_restart() -> Result<()> {
    println!("{}", restart_stop_message(stop_daemon()));
    // Wait for the old process to actually exit so the lock is free.
    for _ in 0..30 {
        if runtime::running_daemon().is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // Start a fresh daemon fully detached so it outlives this CLI invocation.
    runtime::spawn_daemon_detached()?;
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some(info) = runtime::running_daemon() {
            println!("Started daemon (pid {}).", info.pid);
            return Ok(());
        }
    }
    println!("Started daemon; not yet confirmed up (check `waggledance status`).");
    Ok(())
}

#[cfg(test)]
mod config_edit_tests {
    use super::*;

    #[test]
    fn visual_takes_precedence_over_editor() {
        let resolved = resolve_editor(Some("code".to_string()), Some("vim".to_string()));
        assert_eq!(resolved.program, "code");
        assert!(resolved.args.is_empty());
    }

    #[test]
    fn platform_default_when_neither_set() {
        let resolved = resolve_editor(None, None);
        let expected = if cfg!(windows) { "notepad" } else { "vi" };
        assert_eq!(resolved.program, expected);
        assert!(resolved.args.is_empty());
    }

    #[test]
    fn editor_falls_back_when_visual_unset() {
        let resolved = resolve_editor(None, Some("vim".to_string()));
        assert_eq!(resolved.program, "vim");
        assert!(resolved.args.is_empty());
    }

    #[test]
    fn editor_with_args_splits_program_and_args() {
        let resolved = resolve_editor(Some("code --wait".to_string()), None);
        assert_eq!(resolved.program, "code");
        assert_eq!(resolved.args, vec!["--wait".to_string()]);
        assert_eq!(resolved.display, "code --wait");
    }

    #[test]
    fn valid_toml_reports_saved() {
        let path = Path::new("/tmp/waggledance-test-config.toml");
        let msg = classify_config_edit_outcome(path, Ok(String::new()));
        assert!(msg.starts_with("Saved "));
        assert!(msg.contains("Restart the daemon to apply: waggledance restart"));
    }

    #[test]
    fn invalid_toml_reports_warning() {
        let path = Path::new("/tmp/waggledance-test-config.toml");
        let msg = classify_config_edit_outcome(path, Ok("not = [valid".to_string()));
        assert!(msg.starts_with("Warning: "));
        assert!(msg.contains("is not valid TOML"));
    }

    #[test]
    fn unreadable_file_reports_read_error() {
        let path = Path::new("/tmp/waggledance-test-config.toml");
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let msg = classify_config_edit_outcome(path, Err(err));
        assert!(msg.starts_with("Could not re-read "));
    }
}

#[cfg(test)]
mod stop_restart_message_tests {
    use super::*;

    #[test]
    fn stop_outcome_message_reports_success() {
        assert_eq!(
            stop_outcome_message(Some((1234, true))),
            "Stopped daemon (pid 1234)."
        );
    }

    #[test]
    fn stop_outcome_message_reports_failed_kill() {
        assert_eq!(
            stop_outcome_message(Some((1234, false))),
            "Could not stop pid 1234. It may already be gone."
        );
    }

    #[test]
    fn stop_outcome_message_reports_no_daemon() {
        assert_eq!(stop_outcome_message(None), "No daemon running.");
    }

    #[test]
    fn restart_stop_message_reports_stopped_regardless_of_kill_result() {
        assert_eq!(
            restart_stop_message(Some((1234, true))),
            "Stopped daemon (pid 1234)."
        );
        assert_eq!(
            restart_stop_message(Some((1234, false))),
            "Stopped daemon (pid 1234)."
        );
    }

    #[test]
    fn restart_stop_message_reports_no_daemon_was_running() {
        assert_eq!(restart_stop_message(None), "No daemon was running.");
    }
}

#[cfg(test)]
mod bee_upgrade_text_tests {
    use super::*;
    use waggledance_core::bee_upgrade::{Outcome, ProjectUpgrade, UpgradeReport};

    fn project(id: &str, outcome: Outcome) -> ProjectUpgrade {
        ProjectUpgrade {
            id: id.to_string(),
            name: id.to_string(),
            root: PathBuf::from(format!("/r/{id}")),
            branch: Some("main".to_string()),
            from_version: Some("0.9.0".to_string()),
            to_version: Some("1.0.0".to_string()),
            notices: Vec::new(),
            outside_repo: Vec::new(),
            outcome,
        }
    }

    fn report(applied: bool, dirty: bool, projects: Vec<ProjectUpgrade>) -> UpgradeReport {
        UpgradeReport {
            source_root: PathBuf::from("/src/bee"),
            source_head: Some("abc1234".to_string()),
            source_branch: Some("main".to_string()),
            source_dirty: dirty,
            target_version: Some("1.0.0".to_string()),
            applied,
            projects,
        }
    }

    #[test]
    fn dry_run_ends_with_the_apply_hint() {
        let text = render_upgrade_text(&report(
            false,
            false,
            vec![project(
                "p1",
                Outcome::Pending {
                    files: vec!["a".into()],
                },
            )],
        ));
        assert_eq!(
            text.lines().last(),
            Some("dry run — re-run with --apply to upgrade and commit")
        );
        assert!(text.starts_with("bee source: /src/bee @ main abc1234\ntarget: 1.0.0\n"));
        assert!(text.contains("pending: 1"));
    }

    #[test]
    fn upgraded_line_carries_its_commit_sha() {
        let text = render_upgrade_text(&report(
            true,
            false,
            vec![project(
                "p1",
                Outcome::Upgraded {
                    commit: "deadbeef".into(),
                    files: vec!["a".into(), "b".into()],
                },
            )],
        ));
        let line = text.lines().find(|l| l.starts_with("p1")).unwrap();
        assert!(line.contains("upgraded"), "{line}");
        assert!(line.contains("2 files"), "{line}");
        assert!(line.contains("deadbeef"), "{line}");
        assert!(!text.contains("dry run"));
    }

    #[test]
    fn skipped_line_carries_its_reason() {
        let mut p = project(
            "p2",
            Outcome::Skipped {
                reason: "linked worktree".into(),
                files: Vec::new(),
            },
        );
        p.branch = None;
        p.from_version = None;
        let text = render_upgrade_text(&report(false, false, vec![p]));
        let line = text.lines().find(|l| l.starts_with("p2")).unwrap();
        assert!(line.contains("  -  -  skipped"), "{line}");
        assert!(line.contains("linked worktree"), "{line}");
        assert!(text.contains("skipped: 1"));
    }

    #[test]
    fn applied_not_committed_line_carries_its_error() {
        let text = render_upgrade_text(&report(
            true,
            false,
            vec![project(
                "p3",
                Outcome::AppliedNotCommitted {
                    files: vec!["a".into()],
                    error: "commit hook rejected".into(),
                },
            )],
        ));
        let line = text.lines().find(|l| l.starts_with("p3")).unwrap();
        assert!(line.contains("applied_not_committed"), "{line}");
        assert!(line.contains("commit hook rejected"), "{line}");
    }

    #[test]
    fn dirty_source_is_marked_in_the_header() {
        let text = render_upgrade_text(&report(true, true, Vec::new()));
        assert!(text.starts_with("bee source: /src/bee @ main abc1234 (dirty)\n"));
    }
}
