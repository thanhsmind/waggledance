use serde::Serialize;
use serde_json::Value;
use std::collections::hash_map::Entry as MapEntry;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct UsageEvent {
    pub ts_ms: i64,
    pub model: String,
    pub new: u64,
    pub cached: u64,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentRun {
    pub key: String,
    pub provider: String,
    pub session_id: String,
    pub cwd: String,
    pub title: String,
    pub parent_key: Option<String>,
    pub subagent: bool,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub events: Vec<UsageEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaseoAgent {
    pub id: String,
    pub provider: String,
    pub session_id: String,
    pub title: String,
    pub parent_id: Option<String>,
}

impl PaseoAgent {
    fn run_key(&self) -> String {
        format!("{}:{}", self.provider, self.session_id)
    }
}

#[derive(Clone, Debug)]
pub struct AgentHomes {
    pub claude: PathBuf,
    pub pi: PathBuf,
    pub codex: PathBuf,
    pub paseo: PathBuf,
}

impl Default for AgentHomes {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let paseo = std::env::var_os("PASEO_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".paseo"));
        Self {
            claude: home.join(".claude"),
            pi: home.join(".pi"),
            codex: home.join(".codex"),
            paseo,
        }
    }
}

fn json_lines(text: &str) -> impl Iterator<Item = Value> + '_ {
    text.lines().filter_map(|l| serde_json::from_str(l).ok())
}

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn num(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn ts_ms(v: &Value) -> Option<i64> {
    let dt = OffsetDateTime::parse(str_of(v, "timestamp")?, &Rfc3339).ok()?;
    Some((dt.unix_timestamp_nanos() / 1_000_000) as i64)
}

fn finish(mut run: AgentRun) -> AgentRun {
    run.events.retain(|e| e.new > 0 || e.cached > 0);
    run.started_ms = run.events.iter().map(|e| e.ts_ms).min().unwrap_or(0);
    run.ended_ms = run.events.iter().map(|e| e.ts_ms).max().unwrap_or(0);
    run
}

fn read_claude(text: &str) -> AgentRun {
    let mut run = AgentRun {
        provider: "claude".into(),
        ..Default::default()
    };
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for v in json_lines(text) {
        if run.cwd.is_empty() {
            if let Some(cwd) = str_of(&v, "cwd") {
                run.cwd = cwd.into();
            }
        }
        if run.title.is_empty() {
            let summary = (str_of(&v, "type") == Some("summary"))
                .then(|| str_of(&v, "summary"))
                .flatten();
            if let Some(t) = str_of(&v, "aiTitle").or(summary) {
                run.title = t.into();
            }
        }
        let Some(msg) = v.get("message") else {
            continue;
        };
        let (Some(u), Some(ts)) = (msg.get("usage"), ts_ms(&v)) else {
            continue;
        };
        let event = UsageEvent {
            ts_ms: ts,
            model: str_of(msg, "model").unwrap_or_default().into(),
            new: num(u, "input_tokens")
                + num(u, "output_tokens")
                + num(u, "cache_creation_input_tokens"),
            cached: num(u, "cache_read_input_tokens"),
        };
        let Some(id) = str_of(msg, "id") else {
            run.events.push(event);
            continue;
        };
        let request = str_of(&v, "requestId").unwrap_or_default();
        match seen.entry((id.into(), request.into())) {
            MapEntry::Occupied(at) => run.events[*at.get()] = event,
            MapEntry::Vacant(slot) => {
                slot.insert(run.events.len());
                run.events.push(event);
            }
        }
    }
    run
}

pub fn parse_claude(session_id: &str, text: &str) -> AgentRun {
    let mut run = read_claude(text);
    run.key = format!("claude:{session_id}");
    run.session_id = session_id.into();
    finish(run)
}

pub fn parse_claude_subagent(
    session_id: &str,
    agent: &str,
    text: &str,
    meta: Option<&str>,
) -> AgentRun {
    let meta: Value = meta
        .and_then(|m| serde_json::from_str(m).ok())
        .unwrap_or(Value::Null);
    let mut run = read_claude(text);
    if let Some(model) = str_of(&meta, "model") {
        for e in run.events.iter_mut().filter(|e| e.model.is_empty()) {
            e.model = model.into();
        }
    }
    run.title = str_of(&meta, "description").unwrap_or_default().into();
    run.key = format!("claude:{session_id}/{agent}");
    run.session_id = session_id.into();
    run.parent_key = Some(format!("claude:{session_id}"));
    run.subagent = true;
    finish(run)
}

pub fn parse_pi(text: &str) -> Option<AgentRun> {
    let mut run = AgentRun {
        provider: "pi".into(),
        ..Default::default()
    };
    for v in json_lines(text) {
        if str_of(&v, "type") == Some("session") {
            if run.session_id.is_empty() {
                run.session_id = str_of(&v, "id").unwrap_or_default().into();
                run.cwd = str_of(&v, "cwd").unwrap_or_default().into();
            }
            continue;
        }
        let Some(msg) = v.get("message") else {
            continue;
        };
        let (Some(u), Some(ts)) = (msg.get("usage"), ts_ms(&v)) else {
            continue;
        };
        let model = match (str_of(msg, "provider"), str_of(msg, "model")) {
            (Some(p), Some(m)) => format!("{p}/{m}"),
            (None, Some(m)) => m.into(),
            _ => String::new(),
        };
        run.events.push(UsageEvent {
            ts_ms: ts,
            model,
            new: num(u, "input") + num(u, "output") + num(u, "cacheWrite"),
            cached: num(u, "cacheRead"),
        });
    }
    if run.session_id.is_empty() {
        return None;
    }
    run.key = format!("pi:{}", run.session_id);
    Some(finish(run))
}

pub fn parse_codex(text: &str) -> Option<AgentRun> {
    let mut run = AgentRun {
        provider: "codex".into(),
        ..Default::default()
    };
    let mut model: Option<String> = None;
    let (mut prev_total, mut prev_cached) = (0u64, 0u64);
    for v in json_lines(text) {
        let Some(p) = v.get("payload") else {
            continue;
        };
        let kind = str_of(&v, "type");
        if kind == Some("session_meta") && run.session_id.is_empty() {
            run.session_id = str_of(p, "id").unwrap_or_default().into();
            run.cwd = str_of(p, "cwd").unwrap_or_default().into();
        }
        if matches!(kind, Some("session_meta" | "turn_context")) && model.is_none() {
            model = str_of(p, "model").filter(|m| !m.is_empty()).map(Into::into);
        }
        if kind != Some("event_msg") || str_of(p, "type") != Some("token_count") {
            continue;
        }
        let Some(t) = p.get("info").and_then(|i| i.get("total_token_usage")) else {
            continue;
        };
        let (total, cached) = (num(t, "total_tokens"), num(t, "cached_input_tokens"));
        let delta = total.saturating_sub(prev_total);
        let delta_cached = cached.saturating_sub(prev_cached);
        prev_total = total;
        prev_cached = cached;
        let Some(ts) = ts_ms(&v).filter(|_| delta > 0) else {
            continue;
        };
        run.events.push(UsageEvent {
            ts_ms: ts,
            model: String::new(),
            new: delta.saturating_sub(delta_cached),
            cached: delta_cached,
        });
    }
    if run.session_id.is_empty() {
        return None;
    }
    let model = model.unwrap_or_else(|| "codex".into());
    for e in &mut run.events {
        e.model = model.clone();
    }
    run.key = format!("codex:{}", run.session_id);
    Some(finish(run))
}

pub fn parse_paseo(text: &str) -> Option<PaseoAgent> {
    let v: Value = serde_json::from_str(text).ok()?;
    let persistence = v.get("persistence")?;
    let label = |k: &str| {
        v.get("labels")
            .and_then(|l| str_of(l, k))
            .filter(|s| !s.is_empty())
    };
    Some(PaseoAgent {
        id: str_of(&v, "id")?.into(),
        provider: str_of(persistence, "provider")
            .or(str_of(&v, "provider"))?
            .into(),
        session_id: str_of(persistence, "sessionId")?.into(),
        title: label("bee_cell")
            .or(str_of(&v, "title"))
            .unwrap_or_default()
            .into(),
        parent_id: label("paseo.parent-agent-id").map(Into::into),
    })
}

pub fn apply_paseo(runs: &mut [AgentRun], agents: &[PaseoAgent]) {
    let by_id: HashMap<&str, &PaseoAgent> = agents.iter().map(|a| (a.id.as_str(), a)).collect();
    let index: HashMap<String, usize> = runs
        .iter()
        .enumerate()
        .map(|(i, r)| (r.key.clone(), i))
        .collect();
    for a in agents {
        let Some(&i) = index.get(&a.run_key()) else {
            continue;
        };
        if !a.title.is_empty() {
            runs[i].title = a.title.clone();
        }
        if let Some(parent) = a.parent_id.as_deref().and_then(|id| by_id.get(id)) {
            runs[i].parent_key = Some(parent.run_key());
            runs[i].subagent = true;
        }
    }
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn fold_worktree(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    let mut folded = false;
    for c in p.components() {
        if let (false, Component::Normal(name)) = (folded, c) {
            let name = name.to_string_lossy();
            if let Some(at) = name.find("--wt--") {
                out.push(&name[..at]);
                folded = true;
                continue;
            }
        }
        out.push(c.as_os_str());
    }
    out
}

pub fn project_for<'a>(cwd: &Path, roots: &'a [(String, String, PathBuf)]) -> Option<&'a str> {
    let cwd = fold_worktree(&canonical(cwd));
    roots
        .iter()
        .filter_map(|(id, _, root)| {
            let root = canonical(root);
            cwd.starts_with(&root)
                .then(|| (root.components().count(), id.as_str()))
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, id)| id)
}

#[derive(Clone, Copy)]
enum Kind {
    Claude,
    ClaudeSubagent,
    Pi,
    Codex,
    Paseo,
}

enum Parsed {
    Run(Option<AgentRun>),
    Paseo(Option<PaseoAgent>),
}

struct CacheEntry {
    mtime: SystemTime,
    len: u64,
    parsed: Parsed,
}

#[derive(Default)]
pub struct AgentLogCache {
    files: Mutex<HashMap<PathBuf, CacheEntry>>,
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some(ext))
        .collect()
}

fn files_deep(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    out.extend(files(dir, ext));
    for sub in subdirs(dir) {
        files_deep(&sub, ext, out);
    }
}

fn name_of(p: Option<&Path>) -> String {
    p.and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn stem_of(p: &Path) -> String {
    p.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn discover(homes: &AgentHomes) -> Vec<(PathBuf, Kind)> {
    let mut found = Vec::new();
    for dir in subdirs(&homes.claude.join("projects")) {
        found.extend(files(&dir, "jsonl").into_iter().map(|f| (f, Kind::Claude)));
        for session in subdirs(&dir) {
            found.extend(
                files(&session.join("subagents"), "jsonl")
                    .into_iter()
                    .filter(|f| stem_of(f).starts_with("agent-"))
                    .map(|f| (f, Kind::ClaudeSubagent)),
            );
        }
    }
    for dir in subdirs(&homes.pi.join("agent").join("sessions")) {
        found.extend(files(&dir, "jsonl").into_iter().map(|f| (f, Kind::Pi)));
    }
    let mut codex = Vec::new();
    files_deep(&homes.codex.join("sessions"), "jsonl", &mut codex);
    found.extend(codex.into_iter().map(|f| (f, Kind::Codex)));
    for dir in subdirs(&homes.paseo.join("agents")) {
        found.extend(files(&dir, "json").into_iter().map(|f| (f, Kind::Paseo)));
    }
    found
}

fn parse_file(path: &Path, kind: Kind, text: &str) -> Parsed {
    match kind {
        Kind::Claude => Parsed::Run(Some(parse_claude(&stem_of(path), text))),
        Kind::ClaudeSubagent => {
            let session = name_of(path.parent().and_then(Path::parent));
            let meta = fs::read_to_string(path.with_extension("meta.json")).ok();
            Parsed::Run(Some(parse_claude_subagent(
                &session,
                &stem_of(path),
                text,
                meta.as_deref(),
            )))
        }
        Kind::Pi => Parsed::Run(parse_pi(text)),
        Kind::Codex => Parsed::Run(parse_codex(text)),
        Kind::Paseo => Parsed::Paseo(parse_paseo(text)),
    }
}

fn millis(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl AgentLogCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn runs(&self, homes: &AgentHomes, since_ms: i64) -> Vec<AgentRun> {
        let mut runs = Vec::new();
        let mut agents = Vec::new();
        let mut cache = self.files.lock().unwrap_or_else(|e| e.into_inner());
        for (path, kind) in discover(homes) {
            let Ok(md) = fs::metadata(&path) else {
                continue;
            };
            let mtime = md.modified().unwrap_or(UNIX_EPOCH);
            if !matches!(kind, Kind::Paseo) && millis(mtime) < since_ms {
                continue;
            }
            let fresh = cache
                .get(&path)
                .is_some_and(|e| e.mtime == mtime && e.len == md.len());
            if !fresh {
                let Ok(bytes) = fs::read(&path) else {
                    continue;
                };
                let parsed = parse_file(&path, kind, &String::from_utf8_lossy(&bytes));
                cache.insert(
                    path.clone(),
                    CacheEntry {
                        mtime,
                        len: md.len(),
                        parsed,
                    },
                );
            }
            match &cache[&path].parsed {
                Parsed::Run(Some(r)) if !r.events.is_empty() => runs.push(r.clone()),
                Parsed::Paseo(Some(a)) => agents.push(a.clone()),
                _ => {}
            }
        }
        drop(cache);
        apply_paseo(&mut runs, &agents);
        runs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const CLAUDE_MAIN: &str = r#"{"type":"ai-title","aiTitle":"Usage tab","sessionId":"s1"}
{"type":"user","cwd":"/work/proj","sessionId":"s1","timestamp":"2026-10-23T01:00:00Z","message":{"role":"user","content":"hi"}}
{"type":"assistant","cwd":"/work/other","requestId":"r1","timestamp":"2026-10-23T01:00:01Z","message":{"id":"m1","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":3,"cache_read_input_tokens":4}}}
{"type":"assistant","requestId":"r1","timestamp":"2026-10-23T01:00:02Z","message":{"id":"m1","model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":20,"cache_creation_input_tokens":30,"cache_read_input_tokens":40}}}
{not json
{"type":"assistant","requestId":"r2","message":{"id":"m2","model":"claude-opus-5-5","usage":{"input_tokens":99}}}
{"type":"assistant","requestId":"r3","timestamp":"2026-10-23T02:00:00Z","message":{"id":"m3","model":"claude-sonnet-5-5","usage":{"input_tokens":5,"output_tokens":0,"cache_read_input_tokens":7}}}
"#;

    const CLAUDE_SUB: &str = r#"{"type":"assistant","isSidechain":true,"cwd":"/work/proj--wt--feat","requestId":"q1","timestamp":"2026-10-23T03:00:00Z","message":{"id":"n1","usage":{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":11}}}
"#;

    const PI: &str = r#"{"type":"session","version":3,"id":"p1","timestamp":"2026-10-23T02:58:34.663Z","cwd":"/work/proj"}
{"type":"message","id":"a","timestamp":"2026-10-23T02:58:38.668Z","message":{"role":"assistant","provider":"hive","model":"deepseek-flash","usage":{"input":100,"output":5,"cacheRead":50,"cacheWrite":7,"totalTokens":162}}}
{"type":"message","id":"b","timestamp":"2026-10-23T02:59:00Z","message":{"role":"user","content":"x"}}
"#;

    const CODEX: &str = r#"{"type":"session_meta","timestamp":"2026-10-23T01:00:00Z","payload":{"id":"c1","cwd":"/work/proj","model":null}}
{"type":"turn_context","timestamp":"2026-10-23T01:00:01Z","payload":{"model":"gpt-5.5","cwd":"/work/proj"}}
{"type":"event_msg","timestamp":"2026-10-23T01:00:02Z","payload":{"type":"token_count","info":null}}
{"type":"event_msg","timestamp":"2026-10-23T01:00:03Z","payload":{"type":"token_count","info":{"total_token_usage":{"cached_input_tokens":40,"total_tokens":100}}}}
{"type":"event_msg","timestamp":"2026-10-23T01:00:04Z","payload":{"type":"token_count","info":{"total_token_usage":{"cached_input_tokens":40,"total_tokens":100}}}}
{"type":"turn_context","timestamp":"2026-10-23T01:00:05Z","payload":{"model":"gpt-other"}}
{"type":"event_msg","timestamp":"2026-10-23T01:00:06Z","payload":{"type":"token_count","info":{"total_token_usage":{"cached_input_tokens":100,"total_tokens":250}}}}
"#;

    fn ms(s: &str) -> i64 {
        (OffsetDateTime::parse(s, &Rfc3339)
            .unwrap()
            .unix_timestamp_nanos()
            / 1_000_000) as i64
    }

    fn ev(ts: &str, model: &str, new: u64, cached: u64) -> UsageEvent {
        UsageEvent {
            ts_ms: ms(ts),
            model: model.into(),
            new,
            cached,
        }
    }

    #[test]
    fn claude_main_session_dedupes_and_reads_title_and_cwd() {
        let run = parse_claude("s1", CLAUDE_MAIN);
        assert_eq!(run.key, "claude:s1");
        assert_eq!(run.provider, "claude");
        assert_eq!(run.cwd, "/work/proj");
        assert_eq!(run.title, "Usage tab");
        assert!(!run.subagent);
        assert_eq!(run.parent_key, None);
        assert_eq!(
            run.events,
            vec![
                ev("2026-10-23T01:00:02Z", "claude-opus-5-5", 60, 40),
                ev("2026-10-23T02:00:00Z", "claude-sonnet-5-5", 5, 7),
            ]
        );
        assert_eq!(run.started_ms, ms("2026-10-23T01:00:02Z"));
        assert_eq!(run.ended_ms, ms("2026-10-23T02:00:00Z"));
    }

    #[test]
    fn claude_subagent_takes_title_and_model_from_meta() {
        let meta = r#"{"agentType":"bee-review","description":"review (opus)","model":"opus"}"#;
        let run = parse_claude_subagent("s1", "agent-a1", CLAUDE_SUB, Some(meta));
        assert_eq!(run.key, "claude:s1/agent-a1");
        assert_eq!(run.parent_key.as_deref(), Some("claude:s1"));
        assert!(run.subagent);
        assert_eq!(run.title, "review (opus)");
        assert_eq!(run.events, vec![ev("2026-10-23T03:00:00Z", "opus", 5, 11)]);
        let bare = parse_claude_subagent("s1", "agent-a1", CLAUDE_SUB, None);
        assert_eq!(bare.title, "");
    }

    #[test]
    fn pi_session_names_provider_and_model() {
        let run = parse_pi(PI).unwrap();
        assert_eq!(run.key, "pi:p1");
        assert_eq!(run.cwd, "/work/proj");
        assert_eq!(
            run.events,
            vec![ev(
                "2026-10-23T02:58:38.668Z",
                "hive/deepseek-flash",
                112,
                50
            )]
        );
        assert_eq!(parse_pi("{\"type\":\"message\"}"), None);
    }

    #[test]
    fn codex_emits_positive_deltas_of_cumulative_totals() {
        let run = parse_codex(CODEX).unwrap();
        assert_eq!(run.key, "codex:c1");
        assert_eq!(run.cwd, "/work/proj");
        assert_eq!(
            run.events,
            vec![
                ev("2026-10-23T01:00:03Z", "gpt-5.5", 60, 40),
                ev("2026-10-23T01:00:06Z", "gpt-5.5", 90, 60),
            ]
        );
    }

    fn paseo(id: &str, provider: &str, session: &str, title: &str, labels: &str) -> String {
        format!(
            r#"{{"id":"{id}","provider":"{provider}","title":"{title}","labels":{labels},"persistence":{{"sessionId":"{session}","provider":"{provider}"}}}}"#
        )
    }

    #[test]
    fn paseo_labels_name_the_run_and_its_parent() {
        let parent = parse_paseo(&paseo("A", "claude", "s1", "lead the feature", "{}")).unwrap();
        let child = parse_paseo(&paseo(
            "B",
            "pi",
            "p1",
            "ud-5 paseo-pi-flash",
            r#"{"bee_cell":"ud-5","paseo.parent-agent-id":"A"}"#,
        ))
        .unwrap();
        let plain = parse_paseo(&paseo("C", "pi", "p2", "titled", r#"{"bee_cell":""}"#)).unwrap();
        assert_eq!(child.title, "ud-5");
        assert_eq!(plain.title, "titled");
        assert_eq!(parse_paseo("{\"id\":\"x\"}"), None);

        let mut runs = vec![
            parse_claude("s1", CLAUDE_MAIN),
            parse_pi(PI).unwrap(),
            AgentRun {
                key: "pi:p2".into(),
                ..Default::default()
            },
        ];
        apply_paseo(&mut runs, &[parent, child, plain]);
        assert_eq!(runs[0].title, "lead the feature");
        assert!(!runs[0].subagent);
        assert_eq!(runs[1].title, "ud-5");
        assert!(runs[1].subagent);
        assert_eq!(runs[1].parent_key.as_deref(), Some("claude:s1"));
        assert_eq!(runs[2].title, "titled");
        assert!(!runs[2].subagent);
    }

    #[test]
    fn project_for_folds_worktrees_follows_symlinks_and_picks_the_longest_root() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("proj/inner")).unwrap();
        fs::create_dir_all(base.join("proj--wt--feat/src")).unwrap();
        fs::create_dir_all(base.join("elsewhere")).unwrap();
        std::os::unix::fs::symlink(base.join("proj"), base.join("link")).unwrap();
        let roots = vec![
            ("p".to_string(), "proj".to_string(), base.join("link")),
            (
                "i".to_string(),
                "inner".to_string(),
                base.join("proj/inner"),
            ),
        ];
        assert_eq!(
            project_for(&base.join("proj--wt--feat/src"), &roots),
            Some("p")
        );
        assert_eq!(project_for(&base.join("proj"), &roots), Some("p"));
        assert_eq!(project_for(&base.join("link/inner"), &roots), Some("i"));
        assert_eq!(project_for(&base.join("elsewhere"), &roots), None);
        assert_eq!(project_for(Path::new("/no/such/place"), &roots), None);
    }

    fn touch(path: &Path, at: SystemTime) {
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(at)
            .unwrap();
    }

    #[test]
    fn cache_reads_every_source_and_reparses_only_a_changed_file() {
        let dir = tempfile::tempdir().unwrap();
        let homes = AgentHomes {
            claude: dir.path().join("claude"),
            pi: dir.path().join("pi"),
            codex: dir.path().join("codex"),
            paseo: dir.path().join("paseo"),
        };
        let claude = homes.claude.join("projects/-work-proj");
        fs::create_dir_all(claude.join("s1/subagents")).unwrap();
        fs::write(claude.join("s1.jsonl"), CLAUDE_MAIN).unwrap();
        fs::write(claude.join("s1/subagents/agent-a1.jsonl"), CLAUDE_SUB).unwrap();
        fs::write(
            claude.join("s1/subagents/agent-a1.meta.json"),
            r#"{"description":"review","model":"opus"}"#,
        )
        .unwrap();
        let pi = homes.pi.join("agent/sessions/--work-proj--");
        fs::create_dir_all(&pi).unwrap();
        fs::write(pi.join("x_p1.jsonl"), PI).unwrap();
        let codex = homes.codex.join("sessions/2026/10/23");
        fs::create_dir_all(&codex).unwrap();
        fs::write(codex.join("rollout-c1.jsonl"), CODEX).unwrap();
        fs::write(codex.join("old.jsonl"), CODEX.replace("\"c1\"", "\"c0\"")).unwrap();
        touch(
            &codex.join("old.jsonl"),
            UNIX_EPOCH + Duration::from_secs(1_000),
        );
        let paseo_dir = homes.paseo.join("agents/home-work-proj");
        fs::create_dir_all(&paseo_dir).unwrap();
        fs::write(
            paseo_dir.join("A.json"),
            paseo("A", "claude", "s1", "lead", "{}"),
        )
        .unwrap();
        fs::write(
            paseo_dir.join("B.json"),
            paseo(
                "B",
                "pi",
                "p1",
                "t",
                r#"{"bee_cell":"ud-5","paseo.parent-agent-id":"A"}"#,
            ),
        )
        .unwrap();

        let cache = AgentLogCache::new();
        let since = ms("2026-01-01T00:00:00Z");
        let mut runs = cache.runs(&homes, since);
        runs.sort_by(|a, b| a.key.cmp(&b.key));
        let keys: Vec<&str> = runs.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["claude:s1", "claude:s1/agent-a1", "codex:c1", "pi:p1"]
        );
        assert_eq!(runs[0].title, "lead");
        assert_eq!(runs[1].title, "review");
        assert!(runs[1].subagent);
        assert_eq!(runs[3].title, "ud-5");
        assert_eq!(runs[3].parent_key.as_deref(), Some("claude:s1"));

        let main = claude.join("s1.jsonl");
        let stamp = fs::metadata(&main).unwrap().modified().unwrap();
        fs::write(
            &main,
            CLAUDE_MAIN.replace("\"input_tokens\":10", "\"input_tokens\":19"),
        )
        .unwrap();
        touch(&main, stamp);
        fs::write(
            pi.join("x_p1.jsonl"),
            PI.replace("\"input\":100", "\"input\":1000"),
        )
        .unwrap();

        let again = cache.runs(&homes, since);
        let find = |k: &str| again.iter().find(|r| r.key == k).unwrap().clone();
        assert_eq!(find("claude:s1").events[0].new, 60);
        assert_eq!(find("pi:p1").events[0].new, 1012);
        assert_eq!(
            parse_claude("s1", &fs::read_to_string(&main).unwrap()).events[0].new,
            69
        );
    }
}
