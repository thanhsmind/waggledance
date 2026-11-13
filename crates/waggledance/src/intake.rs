use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{interval_at, MissedTickBehavior};
use waggledance_core::config::IntakeConfig;
use waggledance_core::domain::Project;
use waggledance_core::intake::{IntakeItem, IntakeVerdict};
use waggledance_core::Engine;

use crate::notify::Notifier;

const ALLOWED_TOOLS: &str =
    "mcp__mcp-gateway__call_tool,mcp__mcp-gateway__get_tool_definition,mcp__mcp-gateway__search_tools";
const RUN_TIMEOUT: Duration = Duration::from_secs(300);
const ERROR_HEAD_CHARS: usize = 300;
const FIRST_PASS_AFTER: Duration = Duration::from_secs(60);

pub trait Runner {
    fn run(&self, prompt: &str) -> Result<String>;
}

pub trait Dispatcher {
    fn dispatch(&self, project: &str, task: &str, request_id: &str) -> Result<String>;
}

pub struct ClaudeRunner {
    binary: String,
    model: String,
    scratch: PathBuf,
}

impl ClaudeRunner {
    pub fn new(config: &IntakeConfig) -> Self {
        Self {
            binary: config.claude_binary.clone(),
            model: config.model.clone(),
            scratch: waggledance_core::config::data_dir().join("intake-scratch"),
        }
    }
}

impl Runner for ClaudeRunner {
    fn run(&self, prompt: &str) -> Result<String> {
        std::fs::create_dir_all(&self.scratch)
            .with_context(|| format!("could not create {}", self.scratch.display()))?;
        let mut child = Command::new(&self.binary)
            .args(["-p", "--model", &self.model, "--output-format", "json"])
            .args(["--allowedTools", ALLOWED_TOOLS])
            .current_dir(&self.scratch)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("could not start `{}`", self.binary))?;
        let mut stdin = child.stdin.take().context("no stdin on the claude run")?;
        let text = prompt.to_string();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(text.as_bytes());
        });
        let mut stdout = child.stdout.take().context("no stdout on the claude run")?;
        let mut stderr = child.stderr.take().context("no stderr on the claude run")?;
        let out_reader = std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = stdout.read_to_string(&mut buf);
            buf
        });
        let err_reader = std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = stderr.read_to_string(&mut buf);
            buf
        });
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() > RUN_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                return Err(anyhow!(
                    "the claude run timed out after {} s",
                    RUN_TIMEOUT.as_secs()
                ));
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        let _ = writer.join();
        let out = out_reader.join().unwrap_or_default();
        let err = err_reader.join().unwrap_or_default();
        if !status.success() {
            return Err(anyhow!(
                "the claude run exited with {status}: {}",
                head(err.trim())
            ));
        }
        let envelope: Value = serde_json::from_str(out.trim()).map_err(|e| {
            anyhow!(
                "the claude run printed no JSON envelope ({e}): {}",
                head(&out)
            )
        })?;
        if envelope.get("is_error").and_then(Value::as_bool) == Some(true) {
            return Err(anyhow!(
                "the claude run reported an error: {}",
                head(envelope.get("result").and_then(Value::as_str).unwrap_or(""))
            ));
        }
        envelope
            .get("result")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| anyhow!("the claude envelope has no result: {}", head(&out)))
    }
}

pub struct McpDispatcher<'a> {
    engine: &'a Engine,
}

impl Dispatcher for McpDispatcher<'_> {
    fn dispatch(&self, project: &str, task: &str, request_id: &str) -> Result<String> {
        let response = crate::mcp::call_tool_on(
            self.engine,
            "waggledance_dispatch",
            json!({
                "project": project,
                "task": task,
                "leader": true,
                "request_id": request_id
            }),
        );
        if let Some(message) = response.pointer("/error/message").and_then(Value::as_str) {
            return Err(anyhow!("{message}"));
        }
        let result = response.get("result").cloned().unwrap_or(Value::Null);
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            let text = result
                .pointer("/content/0/text")
                .and_then(Value::as_str)
                .unwrap_or("dispatch refused");
            return Err(anyhow!("{text}"));
        }
        result
            .pointer("/structuredContent/run_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| anyhow!("dispatch returned no run_id"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Candidate {
    #[serde(deserialize_with = "text_of")]
    source: String,
    #[serde(deserialize_with = "text_of")]
    id: String,
    #[serde(deserialize_with = "text_of")]
    title: String,
    #[serde(deserialize_with = "text_of")]
    url: String,
    #[serde(deserialize_with = "text_of")]
    body: String,
    #[serde(deserialize_with = "text_of")]
    page_url: String,
    #[serde(deserialize_with = "text_of")]
    selector: String,
    #[serde(deserialize_with = "texts_of")]
    image_urls: Vec<String>,
    #[serde(deserialize_with = "maybe_text_of")]
    project: Option<String>,
    #[serde(deserialize_with = "number_of")]
    confidence: f64,
    #[serde(deserialize_with = "text_of")]
    reason: String,
    #[serde(deserialize_with = "flag_of")]
    is_code: bool,
    #[serde(deserialize_with = "maybe_text_of")]
    twin_of: Option<String>,
}

impl Default for Candidate {
    fn default() -> Self {
        Self {
            source: String::new(),
            id: String::new(),
            title: String::new(),
            url: String::new(),
            body: String::new(),
            page_url: String::new(),
            selector: String::new(),
            image_urls: Vec::new(),
            project: None,
            confidence: 0.0,
            reason: String::new(),
            is_code: false,
            twin_of: None,
        }
    }
}

fn plain_text(v: Value) -> String {
    match v {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn text_of<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    Ok(plain_text(Value::deserialize(d)?))
}

fn maybe_text_of<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    let text = plain_text(Value::deserialize(d)?);
    let trimmed = text.trim();
    Ok((!trimmed.is_empty() && trimmed != "null").then(|| trimmed.to_string()))
}

fn texts_of<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<String>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Array(items) => items
            .into_iter()
            .map(plain_text)
            .filter(|s| !s.is_empty())
            .collect(),
        Value::Null => Vec::new(),
        other => vec![plain_text(other)],
    })
}

fn number_of<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<f64, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    })
}

fn flag_of<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Bool(b) => b,
        Value::String(s) => s.trim().eq_ignore_ascii_case("true"),
        _ => false,
    })
}

impl Candidate {
    fn key(&self) -> String {
        format!("{}-{}", self.source, self.id)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub dispatched: Vec<Value>,
    pub held: Vec<Value>,
    pub skipped: Vec<Value>,
    pub covered: Vec<Value>,
    pub already_seen: Vec<Value>,
}

fn head(text: &str) -> String {
    text.chars().take(ERROR_HEAD_CHARS).collect()
}

fn parse(output: &str) -> Result<Vec<Candidate>> {
    let text = output.trim();
    let from_slice = |s: &str| serde_json::from_str::<Vec<Candidate>>(s);
    let first = match from_slice(text) {
        Ok(items) => return Ok(items),
        Err(e) => e,
    };
    let why = match (text.find('['), text.rfind(']')) {
        (Some(start), Some(end)) if start < end => match from_slice(&text[start..=end]) {
            Ok(items) => return Ok(items),
            Err(e) => e,
        },
        _ => first,
    };
    Err(anyhow!(
        "the intake run did not return a JSON array ({why}): {}",
        head(text)
    ))
}

fn prompt(config: &IntakeConfig, projects: &[Project]) -> String {
    let registered: Vec<Value> = projects
        .iter()
        .map(|p| {
            json!({ "id": p.id, "name": p.name, "root_path": p.root_path.display().to_string() })
        })
        .collect();
    format!(
        "You are an intake classifier. Use only the mcp-gateway tools to READ from anphabe; never call any tool that creates, updates, replies to, resolves or deletes anything.\n\
         The user is {email}.\n\
         Collect: (1) Visual Feedback root comments with status open that the user wrote, that mention the user, or that sit on a review page the user created; (2) the user's open anphabe tasks in both owner and collab mode.\n\
         Keep only code work: bugs, features, dashboards, websites, MCP functions. Digital and marketing work (ads, posts, campaigns, content) is not code: still list it with is_code false.\n\
         Match each item to one of these registered projects by its id, or null when none fits:\n{projects}\n\
         When a task and a comment describe the same work, set twin_of on the later one to the key of the first.\n\
         Return only a JSON array, no prose. Each element has: key (\"<source>-<id>\"), source (\"task\" or \"feedback\"), id, title, url, body, page_url, selector, image_urls (array), project (id or null), confidence (0 to 1), reason, is_code (boolean), twin_of (key or null).",
        email = config.user_email,
        projects = serde_json::to_string_pretty(&registered).unwrap_or_default()
    )
}

fn task_text(c: &Candidate) -> String {
    let mut lines = vec![
        format!("Anphabe {} to do in this project: {}", c.source, c.title),
        String::new(),
        c.body.clone(),
        String::new(),
        format!("Link: {}", c.url),
    ];
    if !c.page_url.is_empty() {
        lines.push(format!("Page: {}", c.page_url));
    }
    if !c.selector.is_empty() {
        lines.push(format!("Element: {}", c.selector));
    }
    for image in &c.image_urls {
        lines.push(format!("Image: {image}"));
    }
    lines.push(String::new());
    lines.push(
        "Do this work in this project's own chain. Everything needed to start is above; no anphabe lookup is required, and nothing is to be written back to anphabe."
            .to_string(),
    );
    lines.join("\n")
}

fn row(item: &IntakeItem) -> Value {
    json!({
        "key": item.key,
        "source": item.source,
        "id": item.item_id,
        "title": item.title,
        "url": item.url,
        "project": item.project,
        "confidence": item.confidence,
        "reason": item.reason,
        "verdict": item.verdict.as_str(),
        "run_id": item.run_id,
        "notified": item.notified,
        "first_seen": item.first_seen
    })
}

fn ledger(engine: &Engine) -> Result<Vec<IntakeItem>> {
    let mut all = Vec::new();
    for verdict in [
        IntakeVerdict::Dispatched,
        IntakeVerdict::Held,
        IntakeVerdict::Skipped,
        IntakeVerdict::Covered,
    ] {
        all.extend(engine.list_intake_items(verdict)?);
    }
    Ok(all)
}

pub fn pass(engine: &Engine, runner: &dyn Runner, dispatcher: &dyn Dispatcher) -> Result<Report> {
    let config = &engine.config.intake;
    if config.user_email.trim().is_empty() {
        return Err(anyhow!(
            "[intake] user_email is empty: set it in the config before running an intake pass"
        ));
    }
    let projects = engine.list_projects()?;
    let output = runner.run(&prompt(config, &projects))?;
    let candidates = parse(&output)?;

    let existing = ledger(engine)?;
    let mut seen: HashSet<String> = existing.iter().map(|i| i.key.clone()).collect();
    let mut dispatched_keys: HashSet<String> = existing
        .iter()
        .filter(|i| i.verdict == IntakeVerdict::Dispatched)
        .map(|i| i.key.clone())
        .collect();
    let mut report = Report::default();

    for c in candidates {
        let key = c.key();
        if c.id.is_empty() || seen.contains(&key) {
            if let Some(item) = existing.iter().find(|i| i.key == key) {
                report.already_seen.push(row(item));
            }
            continue;
        }
        let registered = c
            .project
            .as_deref()
            .filter(|id| projects.iter().any(|p| p.id == *id));
        let mut run_id = None;
        let (verdict, reason) = if !c.is_code {
            (IntakeVerdict::Skipped, c.reason.clone())
        } else if c
            .twin_of
            .as_ref()
            .is_some_and(|t| dispatched_keys.contains(t))
        {
            (IntakeVerdict::Covered, c.reason.clone())
        } else if let (Some(project), true) = (registered, c.confidence >= config.min_confidence) {
            match dispatcher.dispatch(project, &task_text(&c), &format!("intake-{key}")) {
                Ok(id) => {
                    run_id = Some(id);
                    (IntakeVerdict::Dispatched, c.reason.clone())
                }
                Err(e) => (IntakeVerdict::Held, format!("dispatch failed: {e}")),
            }
        } else {
            (IntakeVerdict::Held, c.reason.clone())
        };
        let item = IntakeItem {
            key: key.clone(),
            source: c.source.clone(),
            item_id: c.id.clone(),
            title: c.title.clone(),
            url: c.url.clone(),
            project: c.project.clone(),
            confidence: c.confidence,
            reason,
            verdict,
            run_id,
            notified: false,
            first_seen: waggledance_core::engine::now_rfc3339(),
        };
        engine.insert_intake_item(&item)?;
        seen.insert(key.clone());
        let bucket = match verdict {
            IntakeVerdict::Dispatched => {
                dispatched_keys.insert(key);
                &mut report.dispatched
            }
            IntakeVerdict::Held => &mut report.held,
            IntakeVerdict::Skipped => &mut report.skipped,
            IntakeVerdict::Covered => &mut report.covered,
        };
        bucket.push(row(&item));
    }
    Ok(report)
}

pub fn run_pass(engine: &Engine) -> Result<Report> {
    let runner = ClaudeRunner::new(&engine.config.intake);
    pass(engine, &runner, &McpDispatcher { engine })
}

pub fn every(engine: &Engine) -> Duration {
    Duration::from_secs(engine.config.intake.interval_minutes.max(1) * 60)
}

pub async fn announce(engine: &Engine, notifier: &dyn Notifier) -> Result<usize> {
    let fresh: Vec<IntakeItem> = engine
        .list_intake_items(IntakeVerdict::Held)?
        .into_iter()
        .filter(|i| !i.notified)
        .collect();
    if fresh.is_empty() {
        return Ok(0);
    }
    let lines: Vec<String> = fresh
        .iter()
        .map(|i| {
            format!(
                "{} | project: {} | confidence {:.2} | {}",
                i.title,
                i.project.as_deref().unwrap_or("none"),
                i.confidence,
                i.url
            )
        })
        .collect();
    let body = format!("{} held intake items\n{}", fresh.len(), lines.join("\n"));
    notifier
        .send("intake", &body)
        .await
        .map_err(|e| anyhow!("intake announcement failed: {e}"))?;
    let keys: Vec<String> = fresh.into_iter().map(|i| i.key).collect();
    engine.mark_intake_notified(&keys)?;
    Ok(keys.len())
}

pub async fn run(engine: Arc<Engine>, notifier: Arc<dyn Notifier>) {
    let mut ticks = interval_at(
        tokio::time::Instant::now() + FIRST_PASS_AFTER,
        every(&engine),
    );
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        let e = engine.clone();
        match tokio::task::spawn_blocking(move || run_pass(&e)).await {
            Ok(Ok(report)) => tracing::info!(
                "intake pass: {} dispatched, {} held, {} skipped, {} covered, {} already seen",
                report.dispatched.len(),
                report.held.len(),
                report.skipped.len(),
                report.covered.len(),
                report.already_seen.len()
            ),
            Ok(Err(e)) => tracing::warn!("intake pass failed: {e}"),
            Err(e) => tracing::warn!("intake pass task failed: {e}"),
        }
        if let Err(e) = announce(&engine, notifier.as_ref()).await {
            tracing::warn!("{e}");
        }
    }
}

pub fn list(engine: &Engine, verdict: IntakeVerdict) -> Result<Value> {
    let rows: Vec<Value> = engine.list_intake_items(verdict)?.iter().map(row).collect();
    Ok(json!({ "items": rows }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use waggledance_core::{Config, SqliteStore};

    struct Canned(String);

    impl Runner for Canned {
        fn run(&self, _prompt: &str) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    #[derive(Default)]
    struct Recording {
        calls: RefCell<Vec<(String, String, String)>>,
        fail: bool,
    }

    impl Dispatcher for Recording {
        fn dispatch(&self, project: &str, task: &str, request_id: &str) -> Result<String> {
            if self.fail {
                return Err(anyhow!("paseo is down"));
            }
            self.calls.borrow_mut().push((
                project.to_string(),
                task.to_string(),
                request_id.to_string(),
            ));
            Ok(format!("run-{}", self.calls.borrow().len()))
        }
    }

    fn engine() -> (Engine, String, tempfile::TempDir) {
        let mut config = Config::default();
        config.paseo.enabled = Some(false);
        config.intake.user_email = "me@example.test".into();
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        let dir = tempfile::Builder::new().prefix("alpha").tempdir().unwrap();
        let project = engine.register(dir.path(), Some("alpha")).unwrap();
        (engine, project.id, dir)
    }

    fn item(source: &str, id: &str, project: Option<&str>, confidence: f64) -> Value {
        json!({
            "key": format!("{source}-{id}"), "source": source, "id": id,
            "title": format!("title {id}"), "url": format!("https://x.test/{id}"),
            "body": "body text", "page_url": "https://p.test/a", "selector": "#btn",
            "image_urls": ["https://img.test/1.png"], "project": project,
            "confidence": confidence, "reason": "because", "is_code": true, "twin_of": null
        })
    }

    fn bare(items: Vec<Value>) -> String {
        Value::Array(items).to_string()
    }

    #[test]
    fn bare_and_fenced_output_parse_to_the_same_items() {
        let text = bare(vec![item("task", "1", Some("p"), 0.9)]);
        let fenced = format!("Here you go:\n```json\n{text}\n```\n");
        let a = parse(&text).unwrap();
        let b = parse(&fenced).unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].key(), b[0].key());
        assert_eq!(a[0].title, b[0].title);
    }

    #[test]
    fn null_fields_from_the_model_parse_as_empty() {
        let mut task = item("task", "2", None, 0.7);
        for field in ["url", "body", "page_url", "selector", "image_urls", "reason", "title"] {
            task[field] = Value::Null;
        }
        let fenced = format!("```json\n{}\n```", bare(vec![task]));
        let items = parse(&fenced).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].selector, "");
        assert!(items[0].image_urls.is_empty());
        assert_eq!(items[0].project, None);
    }

    #[test]
    fn a_confident_item_on_a_registered_project_is_dispatched_once_across_two_passes() {
        let (engine, project, _dir) = engine();
        let out = bare(vec![item("task", "7", Some(&project), 0.95)]);
        let dispatcher = Recording::default();
        let first = pass(&engine, &Canned(out.clone()), &dispatcher).unwrap();
        assert_eq!(first.dispatched.len(), 1);
        assert_eq!(first.dispatched[0]["run_id"], "run-1");
        let second = pass(&engine, &Canned(out), &dispatcher).unwrap();
        assert!(second.dispatched.is_empty());
        assert_eq!(second.already_seen.len(), 1);
        assert_eq!(second.already_seen[0]["key"], "task-7");
        let calls = dispatcher.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, project);
        assert_eq!(calls[0].2, "intake-task-7");
        assert!(calls[0].1.contains("https://x.test/7"));
        assert!(calls[0].1.contains("#btn"));
        assert!(calls[0].1.contains("https://img.test/1.png"));
    }

    #[test]
    fn a_non_code_item_is_skipped_and_never_dispatched() {
        let (engine, project, _dir) = engine();
        let mut digital = item("task", "2", Some(&project), 0.99);
        digital["is_code"] = json!(false);
        let dispatcher = Recording::default();
        let report = pass(&engine, &Canned(bare(vec![digital])), &dispatcher).unwrap();
        assert_eq!(report.skipped.len(), 1);
        assert!(dispatcher.calls.borrow().is_empty());
    }

    #[test]
    fn low_confidence_and_unknown_project_items_are_held() {
        let (engine, project, _dir) = engine();
        let out = bare(vec![
            item("task", "3", Some(&project), 0.5),
            item("feedback", "4", Some("no-such-project"), 0.99),
            item("feedback", "5", None, 0.99),
        ]);
        let dispatcher = Recording::default();
        let report = pass(&engine, &Canned(out), &dispatcher).unwrap();
        assert_eq!(report.held.len(), 3);
        assert!(dispatcher.calls.borrow().is_empty());
        let listed = list(&engine, IntakeVerdict::Held).unwrap();
        assert_eq!(listed["items"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn a_twin_of_a_dispatched_item_is_covered() {
        let (engine, project, _dir) = engine();
        let first = item("task", "8", Some(&project), 0.9);
        let mut twin = item("feedback", "9", Some(&project), 0.9);
        twin["twin_of"] = json!("task-8");
        let dispatcher = Recording::default();
        let report = pass(&engine, &Canned(bare(vec![first, twin])), &dispatcher).unwrap();
        assert_eq!(report.dispatched.len(), 1);
        assert_eq!(report.covered.len(), 1);
        assert_eq!(report.covered[0]["key"], "feedback-9");
        assert_eq!(dispatcher.calls.borrow().len(), 1);
    }

    #[test]
    fn a_dispatch_error_holds_the_item_with_the_error_as_reason() {
        let (engine, project, _dir) = engine();
        let out = bare(vec![item("task", "6", Some(&project), 0.9)]);
        let dispatcher = Recording {
            fail: true,
            ..Default::default()
        };
        let report = pass(&engine, &Canned(out), &dispatcher).unwrap();
        assert_eq!(report.held.len(), 1);
        assert!(report.held[0]["reason"]
            .as_str()
            .unwrap()
            .contains("paseo is down"));
    }

    #[test]
    fn garbage_output_fails_the_pass_and_records_nothing() {
        let (engine, _project, _dir) = engine();
        let garbage = "x".repeat(500);
        let err = pass(&engine, &Canned(garbage), &Recording::default()).unwrap_err();
        let message = err.to_string();
        assert!(message.contains(&"x".repeat(300)));
        assert!(!message.contains(&"x".repeat(301)));
        assert!(ledger(&engine).unwrap().is_empty());
    }

    #[test]
    fn an_empty_user_email_refuses_to_run() {
        let (mut engine, _project, _dir) = engine();
        engine.config.intake.user_email = String::new();
        let err = pass(&engine, &Canned("[]".into()), &Recording::default()).unwrap_err();
        assert!(err.to_string().contains("[intake] user_email"));
    }

    #[test]
    fn the_prompt_names_the_email_and_offers_no_write_tool() {
        let (engine, _project, _dir) = engine();
        let text = prompt(&engine.config.intake, &engine.list_projects().unwrap());
        assert!(text.contains("me@example.test"));
        assert!(text.contains("never call any tool that creates"));
        assert!(!ALLOWED_TOOLS.contains("write"));
    }

    struct Recorded(std::sync::Mutex<Vec<(String, String)>>);

    #[async_trait::async_trait]
    impl Notifier for Recorded {
        async fn send(&self, kind: &str, body: &str) -> crate::notify::Result<()> {
            self.0.lock().unwrap().push((kind.into(), body.into()));
            Ok(())
        }
    }

    fn held(key: &str, project: Option<&str>) -> IntakeItem {
        IntakeItem {
            key: key.into(),
            source: "task".into(),
            item_id: key.into(),
            title: format!("title {key}"),
            url: format!("https://x.test/{key}"),
            project: project.map(String::from),
            confidence: 0.5,
            reason: "unsure".into(),
            verdict: IntakeVerdict::Held,
            run_id: None,
            notified: false,
            first_seen: waggledance_core::engine::now_rfc3339(),
        }
    }

    #[tokio::test]
    async fn held_items_are_announced_once_across_passes() {
        let (engine, _, _dir) = engine();
        engine
            .insert_intake_item(&held("task-1", Some("alpha")))
            .unwrap();
        engine.insert_intake_item(&held("task-2", None)).unwrap();
        let notifier = Recorded(Default::default());
        assert_eq!(announce(&engine, &notifier).await.unwrap(), 2);
        assert_eq!(announce(&engine, &notifier).await.unwrap(), 0);
        engine.insert_intake_item(&held("task-3", None)).unwrap();
        assert_eq!(announce(&engine, &notifier).await.unwrap(), 1);
        let sent = notifier.0.lock().unwrap();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].0, "intake");
        assert!(sent[0]
            .1
            .contains("title task-1 | project: alpha | confidence 0.50 | https://x.test/task-1"));
        assert!(sent[0].1.contains("title task-2 | project: none"));
        assert!(!sent[1].1.contains("task-1"));
    }

    #[test]
    fn the_intake_interval_never_drops_below_a_minute() {
        let (mut engine, _, _dir) = engine();
        engine.config.intake.interval_minutes = 0;
        assert_eq!(every(&engine), Duration::from_secs(60));
    }
}
