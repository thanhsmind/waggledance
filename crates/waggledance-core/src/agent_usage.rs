use serde::Serialize;
use serde_json::Value;
use std::collections::hash_map::Entry as MapEntry;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const HEAD_BYTES: usize = 1024;
const DAY_MS: i64 = 86_400_000;

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

#[derive(Clone, Default)]
struct LogState {
    run: AgentRun,
    seen: HashMap<(String, String), usize>,
    model: Option<String>,
    prev_total: u64,
    prev_cached: u64,
}

impl LogState {
    fn feed(&mut self, kind: Kind, text: &str) {
        for v in json_lines(text) {
            match kind {
                Kind::Claude | Kind::ClaudeSubagent => self.claude(&v),
                Kind::Pi => self.pi(&v),
                Kind::Codex => self.codex(&v),
                Kind::Paseo => {}
            }
        }
    }

    fn claude(&mut self, v: &Value) {
        let run = &mut self.run;
        if run.cwd.is_empty() {
            if let Some(cwd) = str_of(v, "cwd") {
                run.cwd = cwd.into();
            }
        }
        if run.title.is_empty() {
            let summary = (str_of(v, "type") == Some("summary"))
                .then(|| str_of(v, "summary"))
                .flatten();
            if let Some(t) = str_of(v, "aiTitle").or(summary) {
                run.title = t.into();
            }
        }
        let Some(msg) = v.get("message") else {
            return;
        };
        let (Some(u), Some(ts)) = (msg.get("usage"), ts_ms(v)) else {
            return;
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
            return;
        };
        let request = str_of(v, "requestId").unwrap_or_default();
        match self.seen.entry((id.into(), request.into())) {
            MapEntry::Occupied(at) => run.events[*at.get()] = event,
            MapEntry::Vacant(slot) => {
                slot.insert(run.events.len());
                run.events.push(event);
            }
        }
    }

    fn pi(&mut self, v: &Value) {
        let run = &mut self.run;
        if str_of(v, "type") == Some("session") {
            if run.session_id.is_empty() {
                run.session_id = str_of(v, "id").unwrap_or_default().into();
                run.cwd = str_of(v, "cwd").unwrap_or_default().into();
            }
            return;
        }
        let Some(msg) = v.get("message") else {
            return;
        };
        let (Some(u), Some(ts)) = (msg.get("usage"), ts_ms(v)) else {
            return;
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

    fn codex(&mut self, v: &Value) {
        let Some(p) = v.get("payload") else {
            return;
        };
        let kind = str_of(v, "type");
        if kind == Some("session_meta") && self.run.session_id.is_empty() {
            self.run.session_id = str_of(p, "id").unwrap_or_default().into();
            self.run.cwd = str_of(p, "cwd").unwrap_or_default().into();
        }
        if matches!(kind, Some("session_meta" | "turn_context")) && self.model.is_none() {
            self.model = str_of(p, "model").filter(|m| !m.is_empty()).map(Into::into);
        }
        if kind != Some("event_msg") || str_of(p, "type") != Some("token_count") {
            return;
        }
        let Some(t) = p.get("info").and_then(|i| i.get("total_token_usage")) else {
            return;
        };
        let (total, cached) = (num(t, "total_tokens"), num(t, "cached_input_tokens"));
        let delta = total.saturating_sub(self.prev_total);
        let delta_cached = cached.saturating_sub(self.prev_cached);
        self.prev_total = total;
        self.prev_cached = cached;
        let Some(ts) = ts_ms(v).filter(|_| delta > 0) else {
            return;
        };
        self.run.events.push(UsageEvent {
            ts_ms: ts,
            model: String::new(),
            new: delta.saturating_sub(delta_cached),
            cached: delta_cached,
        });
    }
}

fn fed(kind: Kind, text: &str) -> LogState {
    let mut state = LogState::default();
    state.feed(kind, text);
    state
}

fn claude_run(state: &LogState, session_id: &str) -> AgentRun {
    let mut run = state.run.clone();
    run.provider = "claude".into();
    run.key = format!("claude:{session_id}");
    run.session_id = session_id.into();
    finish(run)
}

fn claude_subagent_run(
    state: &LogState,
    session_id: &str,
    agent: &str,
    meta: Option<&str>,
) -> AgentRun {
    let meta: Value = meta
        .and_then(|m| serde_json::from_str(m).ok())
        .unwrap_or(Value::Null);
    let mut run = claude_run(state, session_id);
    if let Some(model) = str_of(&meta, "model") {
        for e in run.events.iter_mut().filter(|e| e.model.is_empty()) {
            e.model = model.into();
        }
    }
    run.title = str_of(&meta, "description").unwrap_or_default().into();
    run.key = format!("claude:{session_id}/{agent}");
    run.parent_key = Some(format!("claude:{session_id}"));
    run.subagent = true;
    run
}

fn pi_run(state: &LogState) -> Option<AgentRun> {
    if state.run.session_id.is_empty() {
        return None;
    }
    let mut run = state.run.clone();
    run.provider = "pi".into();
    run.key = format!("pi:{}", run.session_id);
    Some(finish(run))
}

fn codex_run(state: &LogState) -> Option<AgentRun> {
    if state.run.session_id.is_empty() {
        return None;
    }
    let mut run = state.run.clone();
    let model = state.model.as_deref().unwrap_or("codex");
    for e in &mut run.events {
        e.model = model.into();
    }
    run.provider = "codex".into();
    run.key = format!("codex:{}", run.session_id);
    Some(finish(run))
}

pub fn parse_claude(session_id: &str, text: &str) -> AgentRun {
    claude_run(&fed(Kind::Claude, text), session_id)
}

pub fn parse_claude_subagent(
    session_id: &str,
    agent: &str,
    text: &str,
    meta: Option<&str>,
) -> AgentRun {
    claude_subagent_run(&fed(Kind::ClaudeSubagent, text), session_id, agent, meta)
}

pub fn parse_pi(text: &str) -> Option<AgentRun> {
    pi_run(&fed(Kind::Pi, text))
}

pub fn parse_codex(text: &str) -> Option<AgentRun> {
    codex_run(&fed(Kind::Codex, text))
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

pub fn apply_paseo(runs: &mut [Arc<AgentRun>], agents: &[PaseoAgent]) {
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
        let parent = a.parent_id.as_deref().and_then(|id| by_id.get(id));
        if a.title.is_empty() && parent.is_none() {
            continue;
        }
        let run = Arc::make_mut(&mut runs[i]);
        if !a.title.is_empty() {
            run.title = a.title.clone();
        }
        if let Some(parent) = parent {
            run.parent_key = Some(parent.run_key());
            run.subagent = true;
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

#[derive(Clone, Default)]
struct Log {
    state: LogState,
    offset: u64,
    head: Vec<u8>,
    run: Option<Arc<AgentRun>>,
}

#[derive(Clone)]
enum Parsed {
    Log(Arc<Log>),
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
    parsed_bytes: AtomicU64,
    #[cfg(test)]
    read_hook: Option<fn(&AgentLogCache)>,
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

fn build(path: &Path, kind: Kind, state: &LogState) -> Option<AgentRun> {
    match kind {
        Kind::Claude => Some(claude_run(state, &stem_of(path))),
        Kind::ClaudeSubagent => {
            let session = name_of(path.parent().and_then(Path::parent));
            let meta = fs::read_to_string(path.with_extension("meta.json")).ok();
            Some(claude_subagent_run(
                state,
                &session,
                &stem_of(path),
                meta.as_deref(),
            ))
        }
        Kind::Pi => pi_run(state),
        Kind::Codex => codex_run(state),
        Kind::Paseo => None,
    }
}

fn starts_with(file: &mut fs::File, head: &[u8]) -> bool {
    let mut buf = vec![0; head.len()];
    file.read_exact(&mut buf).is_ok() && buf == head
}

fn complete_prefix(buf: &[u8]) -> usize {
    let end = buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let tail = &buf[end..];
    if !tail.is_empty() && serde_json::from_slice::<Value>(tail).is_ok() {
        buf.len()
    } else {
        end
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

    fn read_log(&self, path: &Path, kind: Kind, len: u64, prev: Option<&Log>) -> Option<Log> {
        let mut file = fs::File::open(path).ok()?;
        let mut log = match prev {
            Some(p) if p.offset <= len && starts_with(&mut file, &p.head) => p.clone(),
            _ => Log::default(),
        };
        file.seek(SeekFrom::Start(log.offset)).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        let used = complete_prefix(&buf);
        let take = HEAD_BYTES.saturating_sub(log.head.len()).min(used);
        log.head.extend_from_slice(&buf[..take]);
        self.parsed_bytes.fetch_add(used as u64, Ordering::Relaxed);
        log.state.feed(kind, &String::from_utf8_lossy(&buf[..used]));
        log.offset += used as u64;
        log.run = build(path, kind, &log.state).map(Arc::new);
        Some(log)
    }

    fn read(&self, path: &Path, kind: Kind, len: u64, prev: Option<Parsed>) -> Option<Parsed> {
        #[cfg(test)]
        if let Some(hook) = self.read_hook {
            hook(self);
        }
        if matches!(kind, Kind::Paseo) {
            let text = fs::read_to_string(path).ok()?;
            return Some(Parsed::Paseo(parse_paseo(&text)));
        }
        let prev = match prev {
            Some(Parsed::Log(log)) => Some(log),
            _ => None,
        };
        let log = self.read_log(path, kind, len, prev.as_deref())?;
        Some(Parsed::Log(Arc::new(log)))
    }

    pub fn runs(&self, homes: &AgentHomes, since_ms: i64) -> Vec<Arc<AgentRun>> {
        let mut keep = HashSet::new();
        let mut found = Vec::new();
        for (path, kind) in discover(homes) {
            let Ok(md) = fs::metadata(&path) else {
                continue;
            };
            let mtime = md.modified().unwrap_or(UNIX_EPOCH);
            let paseo = matches!(kind, Kind::Paseo);
            if paseo || millis(mtime) >= since_ms - DAY_MS {
                keep.insert(path.clone());
            }
            if paseo || millis(mtime) >= since_ms {
                found.push((path, kind, mtime, md.len()));
            }
        }
        let prior: Vec<Option<(SystemTime, u64, Parsed)>> = {
            let mut cache = self.files.lock().unwrap_or_else(|e| e.into_inner());
            cache.retain(|path, _| keep.contains(path));
            found
                .iter()
                .map(|(path, ..)| cache.get(path).map(|e| (e.mtime, e.len, e.parsed.clone())))
                .collect()
        };
        let mut parsed = Vec::new();
        let mut fresh = Vec::new();
        for ((path, kind, mtime, len), prior) in found.into_iter().zip(prior) {
            match prior {
                Some((m, l, p)) if m == mtime && l == len => parsed.push(p),
                prior => {
                    if let Some(p) = self.read(&path, kind, len, prior.map(|(.., p)| p)) {
                        parsed.push(p.clone());
                        fresh.push((path, mtime, len, p));
                    }
                }
            }
        }
        if !fresh.is_empty() {
            let mut cache = self.files.lock().unwrap_or_else(|e| e.into_inner());
            for (path, mtime, len, parsed) in fresh {
                if cache
                    .get(&path)
                    .is_none_or(|e| (mtime, len) > (e.mtime, e.len))
                {
                    cache.insert(path, CacheEntry { mtime, len, parsed });
                }
            }
        }
        let mut runs = Vec::new();
        let mut agents = Vec::new();
        for p in parsed {
            match p {
                Parsed::Log(log) => {
                    runs.extend(log.run.as_ref().filter(|r| !r.events.is_empty()).cloned())
                }
                Parsed::Paseo(Some(a)) => agents.push(a),
                Parsed::Paseo(None) => {}
            }
        }
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
            Arc::new(parse_claude("s1", CLAUDE_MAIN)),
            Arc::new(parse_pi(PI).unwrap()),
            Arc::new(AgentRun {
                key: "pi:p2".into(),
                ..Default::default()
            }),
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

    fn homes_in(dir: &Path) -> AgentHomes {
        AgentHomes {
            claude: dir.join("claude"),
            pi: dir.join("pi"),
            codex: dir.join("codex"),
            paseo: dir.join("paseo"),
        }
    }

    fn append(path: &Path, text: &str) -> u64 {
        use std::io::Write;
        fs::File::options()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
        text.len() as u64
    }

    fn logs_fixture(dir: &Path) -> (AgentHomes, PathBuf, PathBuf) {
        let homes = homes_in(dir);
        let claude = homes.claude.join("projects/-work-proj");
        fs::create_dir_all(&claude).unwrap();
        fs::write(claude.join("s1.jsonl"), CLAUDE_MAIN).unwrap();
        let codex = homes.codex.join("sessions/2026/10/23");
        fs::create_dir_all(&codex).unwrap();
        fs::write(codex.join("rollout-c1.jsonl"), CODEX).unwrap();
        (
            homes,
            claude.join("s1.jsonl"),
            codex.join("rollout-c1.jsonl"),
        )
    }

    fn run_of(runs: &[Arc<AgentRun>], key: &str) -> Arc<AgentRun> {
        runs.iter().find(|r| r.key == key).unwrap().clone()
    }

    #[test]
    fn appended_lines_are_the_only_bytes_parsed_and_keep_dedupe_and_deltas() {
        let dir = tempfile::tempdir().unwrap();
        let (homes, main, rollout) = logs_fixture(dir.path());
        let cache = AgentLogCache::new();
        let since = ms("2026-01-01T00:00:00Z");
        cache.runs(&homes, since);
        let first = cache.parsed_bytes.load(Ordering::Relaxed);
        assert_eq!(first, (CLAUDE_MAIN.len() + CODEX.len()) as u64);

        let dup = |n: u64| {
            format!(
                r#"{{"type":"assistant","requestId":"r3","timestamp":"2026-10-23T02:00:0{n}Z","message":{{"id":"m3","model":"claude-sonnet-5-5","usage":{{"input_tokens":{n}00,"cache_read_input_tokens":7}}}}}}
"#
            )
        };
        let mut grown = append(&main, &dup(1));
        grown += append(
            &rollout,
            r#"{"type":"event_msg","timestamp":"2026-10-23T01:00:07Z","payload":{"type":"token_count","info":{"total_token_usage":{"cached_input_tokens":100,"total_tokens":400}}}}
"#,
        );
        let runs = cache.runs(&homes, since);
        assert_eq!(cache.parsed_bytes.load(Ordering::Relaxed), first + grown);
        let claude = run_of(&runs, "claude:s1");
        assert_eq!(claude.events.len(), 2);
        assert_eq!(claude.events[1].new, 100);
        let codex = run_of(&runs, "codex:c1");
        assert_eq!(
            codex.events.last(),
            Some(&ev("2026-10-23T01:00:07Z", "gpt-5.5", 150, 0))
        );
        assert_eq!(codex.events.len(), 3);

        append(&main, &dup(2));
        let runs = cache.runs(&homes, since);
        let claude = run_of(&runs, "claude:s1");
        assert_eq!(claude.events.len(), 2);
        assert_eq!(claude.events[1].new, 200);
        assert_eq!(
            *claude,
            parse_claude("s1", &fs::read_to_string(&main).unwrap())
        );
    }

    #[test]
    fn a_partial_line_waits_until_it_is_complete() {
        let dir = tempfile::tempdir().unwrap();
        let (homes, _, rollout) = logs_fixture(dir.path());
        let cache = AgentLogCache::new();
        let since = ms("2026-01-01T00:00:00Z");
        cache.runs(&homes, since);
        let line = r#"{"type":"event_msg","timestamp":"2026-10-23T01:00:07Z","payload":{"type":"token_count","info":{"total_token_usage":{"cached_input_tokens":100,"total_tokens":400}}}}"#;
        let (start, end) = line.split_at(40);
        append(&rollout, start);
        assert_eq!(
            run_of(&cache.runs(&homes, since), "codex:c1").events.len(),
            2
        );
        append(&rollout, end);
        assert_eq!(
            run_of(&cache.runs(&homes, since), "codex:c1").events.len(),
            3
        );
    }

    #[test]
    fn a_shrunk_log_is_reparsed_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let (homes, _, rollout) = logs_fixture(dir.path());
        let cache = AgentLogCache::new();
        let since = ms("2026-01-01T00:00:00Z");
        cache.runs(&homes, since);
        let before = cache.parsed_bytes.load(Ordering::Relaxed);
        let short: String = CODEX.lines().take(4).map(|l| format!("{l}\n")).collect();
        fs::write(&rollout, &short).unwrap();
        let runs = cache.runs(&homes, since);
        assert_eq!(
            cache.parsed_bytes.load(Ordering::Relaxed),
            before + short.len() as u64
        );
        assert_eq!(
            run_of(&runs, "codex:c1").events,
            vec![ev("2026-10-23T01:00:03Z", "gpt-5.5", 60, 40)]
        );
    }

    #[test]
    fn deleted_and_stale_logs_leave_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let (homes, main, rollout) = logs_fixture(dir.path());
        let cache = AgentLogCache::new();
        let since = ms("2026-01-01T00:00:00Z");
        cache.runs(&homes, since);
        assert_eq!(cache.files.lock().unwrap().len(), 2);
        fs::remove_file(&main).unwrap();
        touch(&rollout, UNIX_EPOCH + Duration::from_secs(1_000));
        assert!(cache.runs(&homes, since).is_empty());
        assert!(cache.files.lock().unwrap().is_empty());
    }

    #[test]
    fn the_cache_lock_is_free_while_a_log_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let (homes, main, _) = logs_fixture(dir.path());
        let cache = AgentLogCache {
            read_hook: Some(|c| assert!(c.files.try_lock().is_ok())),
            ..Default::default()
        };
        let since = ms("2026-01-01T00:00:00Z");
        cache.runs(&homes, since);
        append(&main, "\n");
        std::thread::scope(|s| {
            let other = s.spawn(|| cache.runs(&homes, since).len());
            assert_eq!(cache.runs(&homes, since).len(), 2);
            assert_eq!(other.join().unwrap(), 2);
        });
        assert!(cache.parsed_bytes.load(Ordering::Relaxed) > 0);
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
        let homes = homes_in(dir.path());
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
