use crate::config::write_atomic;
use crate::domain::Project;
use crate::error::{Error, Result};
use crate::process::{run_with_deadline, RunError};
use serde::Serialize;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const FORBIDDEN_KEYS: [&str; 3] = ["guards", "gate_bypass", "hooks"];

const EXPLAIN_TIMEOUT: Duration = Duration::from_secs(20);
const OUTPUT_CAP: usize = 1024 * 1024;

fn non_empty(v: Option<&str>) -> Option<&str> {
    v.filter(|s| !s.is_empty())
}

pub fn resolve_path(
    beehive_config_dir: Option<&str>,
    xdg_config_home: Option<&str>,
    home: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(dir) = non_empty(beehive_config_dir) {
        return Ok(Path::new(dir).join("config.json"));
    }
    if let Some(xdg) = non_empty(xdg_config_home) {
        return Ok(Path::new(xdg).join("beehive").join("config.json"));
    }
    let home = home.ok_or_else(|| Error::Config("cannot resolve the home directory".into()))?;
    Ok(home.join(".config").join("beehive").join("config.json"))
}

pub fn config_path() -> Result<PathBuf> {
    resolve_path(
        std::env::var("BEEHIVE_CONFIG_DIR").ok().as_deref(),
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        dirs::home_dir().as_deref(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPath(Vec<String>);

impl KeyPath {
    pub fn parse(raw: &str) -> Result<Self> {
        if raw.is_empty() {
            return Err(Error::Config("config key is empty".into()));
        }
        let segments: Vec<String> = raw.split('.').map(str::to_string).collect();
        if segments.iter().any(String::is_empty) {
            return Err(Error::Config(format!(
                "config key {raw:?} has an empty segment"
            )));
        }
        Ok(Self(segments))
    }

    pub fn first(&self) -> &str {
        &self.0[0]
    }

    pub fn dotted(&self) -> String {
        self.0.join(".")
    }

    pub fn is_forbidden(&self) -> bool {
        FORBIDDEN_KEYS.contains(&self.first())
    }
}

pub fn forbidden_keys_in(doc: &Map<String, Value>) -> Vec<&'static str> {
    FORBIDDEN_KEYS
        .into_iter()
        .filter(|k| doc.contains_key(*k))
        .collect()
}

fn forbidden_error(keys: &[&str]) -> Error {
    Error::Config(format!(
        "bee's global config never holds: {} (per-project only)",
        keys.join(", ")
    ))
}

pub struct GlobalConfig {
    path: PathBuf,
    exists: bool,
    doc: Map<String, Value>,
}

impl GlobalConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    path: path.to_path_buf(),
                    exists: false,
                    doc: Map::new(),
                })
            }
            Err(e) => return Err(Error::Config(format!("{}: {e}", path.display()))),
        };
        match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(doc)) => Ok(Self {
                path: path.to_path_buf(),
                exists: true,
                doc,
            }),
            Ok(_) => Err(Error::Config(format!(
                "{}: not a JSON object",
                path.display()
            ))),
            Err(e) => Err(Error::Config(format!(
                "{}: not valid JSON ({e})",
                path.display()
            ))),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn exists(&self) -> bool {
        self.exists
    }

    pub fn document(&self) -> &Map<String, Value> {
        &self.doc
    }

    pub fn get(&self, key: &KeyPath) -> Option<&Value> {
        lookup(&self.doc, key)
    }

    pub fn set(&mut self, key: &KeyPath, value: Value) -> Result<()> {
        if key.is_forbidden() {
            return Err(forbidden_error(&[key.first()]));
        }
        let (leaf, parents) = key.0.split_last().expect("KeyPath is never empty");
        let mut cur = &mut self.doc;
        let mut walked = Vec::new();
        for seg in parents {
            walked.push(seg.as_str());
            let slot = cur
                .entry(seg.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            cur = slot.as_object_mut().ok_or_else(|| {
                Error::Config(format!(
                    "cannot set {}: {} holds a non-object value",
                    key.dotted(),
                    walked.join(".")
                ))
            })?;
        }
        cur.insert(leaf.clone(), value);
        Ok(())
    }

    pub fn unset(&mut self, key: &KeyPath) -> bool {
        let (leaf, parents) = key.0.split_last().expect("KeyPath is never empty");
        let mut cur = &mut self.doc;
        for seg in parents {
            match cur.get_mut(seg).and_then(Value::as_object_mut) {
                Some(next) => cur = next,
                None => return false,
            }
        }
        cur.shift_remove(leaf).is_some()
    }

    pub fn save(&mut self) -> Result<()> {
        let bad = forbidden_keys_in(&self.doc);
        if !bad.is_empty() {
            return Err(forbidden_error(&bad));
        }
        let mut text = serde_json::to_string_pretty(&self.doc)
            .map_err(|e| Error::Config(format!("serialize: {e}")))?;
        text.push('\n');
        write_atomic(&self.path, text.as_bytes())?;
        self.exists = true;
        Ok(())
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ExplainRow {
    pub id: String,
    pub source: Option<String>,
    pub value: Option<Value>,
    pub error: Option<String>,
}

fn parse_explain_stdout(text: &str) -> Option<(Option<String>, Option<Value>)> {
    let start = text.find('{')?;
    let v = serde_json::from_str::<Value>(text[start..].trim()).ok()?;
    let source = v.get("source").and_then(Value::as_str).map(str::to_string);
    let value = v
        .get("effective")
        .or_else(|| v.get("value"))
        .filter(|x| !x.is_null())
        .cloned();
    Some((source, value))
}

fn explain_project(project: &Project, key: &str) -> ExplainRow {
    let row = |source, value, error| ExplainRow {
        id: project.id.clone(),
        source,
        value,
        error,
    };
    let mut cmd = Command::new(project.root_path.join(".bee/bin/bee"));
    cmd.current_dir(&project.root_path)
        .args(["config", "explain", "--key", key, "--json"]);
    let out = match run_with_deadline(&mut cmd, None, EXPLAIN_TIMEOUT, OUTPUT_CAP) {
        Ok(o) => o,
        Err(RunError::Spawn(e)) => return row(None, None, Some(format!("cannot run bee: {e}"))),
        Err(RunError::Timeout(s)) => {
            return row(
                None,
                None,
                Some(format!("bee config explain timed out after {s}s")),
            )
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);
    match (out.ok, parse_explain_stdout(&text)) {
        (true, Some((source, value))) => row(source, value, None),
        (true, None) => row(None, None, Some("bee printed no JSON object".into())),
        (false, _) => {
            let detail = out.stderr.trim();
            let detail = if detail.is_empty() {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                detail.to_string()
            };
            row(
                None,
                None,
                Some(format!("bee config explain failed: {detail}")),
            )
        }
    }
}

pub fn explain(projects: &[Project], key: &KeyPath, only: &[String]) -> Result<Vec<ExplainRow>> {
    let unknown: Vec<&str> = only
        .iter()
        .filter(|id| !projects.iter().any(|p| &p.id == *id))
        .map(String::as_str)
        .collect();
    if !unknown.is_empty() {
        return Err(Error::ProjectNotFound(unknown.join(", ")));
    }
    let dotted = key.dotted();
    Ok(projects
        .iter()
        .filter(|p| only.is_empty() || only.contains(&p.id))
        .map(|p| explain_project(p, &dotted))
        .collect())
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PruneLayer {
    Tracked,
    Local,
    Team,
}

impl PruneLayer {
    const ALL: [PruneLayer; 3] = [Self::Tracked, Self::Local, Self::Team];

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Tracked => "config.json",
            Self::Local => "config.local.json",
            Self::Team => "team.json",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tracked => "tracked",
            Self::Local => "local",
            Self::Team => "team",
        }
    }
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PruneAction {
    Pruned,
    Differs,
    Absent,
    Skipped,
    Error,
}

impl PruneAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pruned => "pruned",
            Self::Differs => "differs",
            Self::Absent => "absent",
            Self::Skipped => "skipped",
            Self::Error => "error",
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PruneRow {
    pub id: String,
    pub layer: Option<PruneLayer>,
    pub action: PruneAction,
    pub file: Option<PathBuf>,
    pub applied: bool,
    pub detail: Option<String>,
}

pub fn prune_decision(layer_value: &Value, global_value: &Value, force: bool) -> PruneAction {
    if force || layer_value == global_value {
        PruneAction::Pruned
    } else {
        PruneAction::Differs
    }
}

fn remove_path(map: &mut Map<String, Value>, segs: &[String]) -> bool {
    let (head, rest) = segs.split_first().expect("KeyPath is never empty");
    if rest.is_empty() {
        return map.shift_remove(head).is_some();
    }
    let Some(child) = map.get_mut(head).and_then(Value::as_object_mut) else {
        return false;
    };
    let removed = remove_path(child, rest);
    if removed && child.is_empty() {
        map.shift_remove(head);
    }
    removed
}

fn lookup<'a>(doc: &'a Map<String, Value>, key: &KeyPath) -> Option<&'a Value> {
    let mut segs = key.0.iter();
    let mut cur = doc.get(segs.next()?)?;
    for seg in segs {
        cur = cur.as_object()?.get(seg)?;
    }
    Some(cur)
}

fn prune_layer(
    project: &Project,
    layer: PruneLayer,
    key: &KeyPath,
    global_value: &Value,
    force: bool,
    apply: bool,
) -> Option<PruneRow> {
    let file = project.root_path.join(".bee").join(layer.file_name());
    let row = |action, applied, detail| {
        Some(PruneRow {
            id: project.id.clone(),
            layer: Some(layer),
            action,
            file: Some(file.clone()),
            applied,
            detail,
        })
    };
    let text = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => return row(PruneAction::Error, false, Some(e.to_string())),
    };
    let mut doc = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(doc)) => doc,
        Ok(_) => return row(PruneAction::Error, false, Some("not a JSON object".into())),
        Err(e) => {
            return row(
                PruneAction::Error,
                false,
                Some(format!("not valid JSON ({e})")),
            )
        }
    };
    let held = lookup(&doc, key)?;
    let action = prune_decision(held, global_value, force);
    if action == PruneAction::Differs {
        return row(action, false, Some(format!("holds {held}")));
    }
    if !apply {
        return row(action, false, None);
    }
    remove_path(&mut doc, &key.0);
    let written = serde_json::to_string_pretty(&doc)
        .map_err(|e| e.to_string())
        .and_then(|mut t| {
            t.push('\n');
            write_atomic(&file, t.as_bytes()).map_err(|e| e.to_string())
        });
    match written {
        Ok(()) => row(action, true, None),
        Err(e) => row(PruneAction::Error, false, Some(e)),
    }
}

pub fn prune(
    projects: &[Project],
    global: &GlobalConfig,
    key: &KeyPath,
    only: &[String],
    force: bool,
    apply: bool,
) -> Result<Vec<PruneRow>> {
    if key.is_forbidden() {
        return Err(forbidden_error(&[key.first()]));
    }
    let unknown: Vec<&str> = only
        .iter()
        .filter(|id| !projects.iter().any(|p| &p.id == *id))
        .map(String::as_str)
        .collect();
    if !unknown.is_empty() {
        return Err(Error::ProjectNotFound(unknown.join(", ")));
    }
    let global_value = global.get(key).ok_or_else(|| {
        Error::Config(format!(
            "{} is not set in {}; run `waggledance bee config set {} <value>` first so no project is left without a value",
            key.dotted(),
            global.path().display(),
            key.dotted()
        ))
    })?;
    let mut rows = Vec::new();
    for project in projects
        .iter()
        .filter(|p| only.is_empty() || only.contains(&p.id))
    {
        let note = |detail: &str| PruneRow {
            id: project.id.clone(),
            layer: None,
            action: PruneAction::Skipped,
            file: None,
            applied: false,
            detail: Some(detail.into()),
        };
        if !project.root_path.is_dir() {
            rows.push(note("root missing"));
            continue;
        }
        if !project.root_path.join(".bee").is_dir() {
            rows.push(note("no bee"));
            continue;
        }
        let before = rows.len();
        rows.extend(
            PruneLayer::ALL
                .into_iter()
                .filter_map(|l| prune_layer(project, l, key, global_value, force, apply)),
        );
        if rows.len() == before {
            rows.push(PruneRow {
                action: PruneAction::Absent,
                detail: None,
                ..note("")
            });
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wd-bgc-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn explain_stdout_reads_the_effective_value_and_source() {
        let text = r#"{"key":"commands.test","effective":["a"],"source":"tracked","tracked":["a"],"local":null}"#;
        assert_eq!(
            parse_explain_stdout(text),
            Some((Some("tracked".into()), Some(json!(["a"]))))
        );
    }

    #[test]
    fn explain_stdout_skips_a_leading_non_json_line() {
        let text = "notice: something\n{\"effective\":1,\"source\":\"local\"}\n";
        assert_eq!(
            parse_explain_stdout(text),
            Some((Some("local".into()), Some(json!(1))))
        );
    }

    #[test]
    fn explain_stdout_yields_null_for_a_null_effective_and_no_source() {
        assert_eq!(
            parse_explain_stdout(r#"{"key":"k","effective":null}"#),
            Some((None, None))
        );
    }

    fn key(s: &str) -> KeyPath {
        KeyPath::parse(s).unwrap()
    }

    #[test]
    fn path_prefers_beehive_dir_then_xdg_then_home() {
        let home = Path::new("/h");
        assert_eq!(
            resolve_path(Some("/b"), Some("/x"), Some(home)).unwrap(),
            Path::new("/b/config.json")
        );
        assert_eq!(
            resolve_path(None, Some("/x"), Some(home)).unwrap(),
            Path::new("/x/beehive/config.json")
        );
        assert_eq!(
            resolve_path(None, None, Some(home)).unwrap(),
            Path::new("/h/.config/beehive/config.json")
        );
    }

    #[test]
    fn empty_env_values_count_as_unset() {
        let home = Path::new("/h");
        assert_eq!(
            resolve_path(Some(""), Some("/x"), Some(home)).unwrap(),
            Path::new("/x/beehive/config.json")
        );
        assert_eq!(
            resolve_path(Some(""), Some(""), Some(home)).unwrap(),
            Path::new("/h/.config/beehive/config.json")
        );
        assert!(resolve_path(None, None, None).is_err());
    }

    #[test]
    fn key_path_refuses_empty_key_and_segments() {
        for bad in ["", ".", "a.", ".a", "a..b"] {
            assert!(KeyPath::parse(bad).is_err(), "{bad:?}");
        }
        assert_eq!(key("herding.agents").dotted(), "herding.agents");
        assert_eq!(key("herding.agents").first(), "herding");
    }

    #[test]
    fn forbidden_check_reads_the_first_segment() {
        assert!(key("guards.x").is_forbidden());
        assert!(key("hooks").is_forbidden());
        assert!(key("gate_bypass").is_forbidden());
        assert!(!key("herding.guards").is_forbidden());
        let doc = json!({"a": 1, "hooks": {}, "guards": 1});
        assert_eq!(
            forbidden_keys_in(doc.as_object().unwrap()),
            vec!["guards", "hooks"]
        );
    }

    #[test]
    fn set_get_unset_round_trip_is_byte_exact() {
        let dir = tmp("roundtrip");
        let path = dir.join("config.json");
        let mut cfg = GlobalConfig::load(&path).unwrap();
        assert!(!cfg.exists());
        assert!(cfg.document().is_empty());
        cfg.set(&key("herding.agents"), json!(["a", "b"])).unwrap();
        cfg.set(&key("zeta"), json!(1)).unwrap();
        cfg.set(&key("alpha"), json!(true)).unwrap();
        cfg.save().unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\n  \"herding\": {\n    \"agents\": [\n      \"a\",\n      \"b\"\n    ]\n  },\n  \"zeta\": 1,\n  \"alpha\": true\n}\n"
        );
        let mut cfg = GlobalConfig::load(&path).unwrap();
        assert!(cfg.exists());
        assert_eq!(cfg.get(&key("herding.agents")), Some(&json!(["a", "b"])));
        assert_eq!(cfg.get(&key("herding.nope")), None);
        assert!(cfg.unset(&key("herding.agents")));
        assert!(!cfg.unset(&key("herding.agents")));
        assert!(!cfg.unset(&key("missing.deep")));
        cfg.save().unwrap();
        assert_eq!(cfg.get(&key("herding.agents")), None);
    }

    #[test]
    fn set_refuses_a_non_object_intermediate() {
        let mut cfg = GlobalConfig::load(&tmp("intermediate").join("config.json")).unwrap();
        cfg.set(&key("a"), json!(1)).unwrap();
        let err = cfg.set(&key("a.b"), json!(2)).unwrap_err();
        assert!(err.to_string().contains("non-object"), "{err}");
        assert_eq!(cfg.get(&key("a")), Some(&json!(1)));
    }

    #[test]
    fn missing_file_reads_as_empty_object() {
        let cfg = GlobalConfig::load(&tmp("missing").join("config.json")).unwrap();
        assert!(!cfg.exists());
        assert!(cfg.document().is_empty());
    }

    #[test]
    fn malformed_file_errors_and_is_never_overwritten() {
        let dir = tmp("malformed");
        let path = dir.join("config.json");
        for body in ["{not json", "[1,2]"] {
            std::fs::write(&path, body).unwrap();
            let err = GlobalConfig::load(&path).err().expect("load must fail");
            assert!(err.to_string().contains("config.json"), "{err}");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), body);
        }
    }

    #[test]
    fn set_of_forbidden_keys_is_refused_and_writes_nothing() {
        let dir = tmp("forbidden");
        let path = dir.join("config.json");
        let mut cfg = GlobalConfig::load(&path).unwrap();
        for k in ["guards", "gate_bypass", "hooks", "guards.x"] {
            let err = cfg.set(&key(k), json!(1)).unwrap_err();
            assert!(err.to_string().contains(key(k).first()), "{err}");
        }
        assert!(cfg.document().is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn unset_of_a_hand_edited_forbidden_key_lets_save_succeed() {
        let dir = tmp("cleanup");
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"guards":{"x":1},"hooks":{},"keep":1}"#).unwrap();
        let mut cfg = GlobalConfig::load(&path).unwrap();
        let err = cfg.save().unwrap_err().to_string();
        assert!(err.contains("guards") && err.contains("hooks"), "{err}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"guards":{"x":1},"hooks":{},"keep":1}"#
        );
        assert!(cfg.unset(&key("guards")));
        assert!(cfg.save().is_err());
        assert!(cfg.unset(&key("hooks")));
        cfg.save().unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\n  \"keep\": 1\n}\n"
        );
    }

    #[test]
    fn a_write_leaves_performance_jsonl_untouched() {
        let dir = tmp("perf");
        let perf = dir.join("performance.jsonl");
        std::fs::write(&perf, "{\"a\":1}\n").unwrap();
        let mut cfg = GlobalConfig::load(&dir.join("config.json")).unwrap();
        cfg.set(&key("k"), json!("v")).unwrap();
        cfg.save().unwrap();
        assert_eq!(std::fs::read_to_string(&perf).unwrap(), "{\"a\":1}\n");
    }

    #[test]
    fn explain_reports_a_missing_bee_on_its_own_row() {
        let dir = tmp("explain");
        let project = Project {
            id: "p1".into(),
            name: "p1".into(),
            root_path: dir,
            created_at: String::new(),
            last_seen_at: String::new(),
            orchestration_enabled: false,
        };
        let rows = explain(std::slice::from_ref(&project), &key("a.b"), &[]).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].error.is_some() && rows[0].source.is_none());
        assert!(explain(&[project], &key("a"), &["nope".to_string()]).is_err());
    }

    fn project_in(dir: &Path, id: &str, files: &[(&str, &str)]) -> Project {
        let root = dir.join(id);
        std::fs::create_dir_all(root.join(".bee")).unwrap();
        for (name, body) in files {
            std::fs::write(root.join(".bee").join(name), body).unwrap();
        }
        Project {
            id: id.into(),
            name: id.into(),
            root_path: root,
            created_at: String::new(),
            last_seen_at: String::new(),
            orchestration_enabled: false,
        }
    }

    fn global_with(dir: &Path, body: &str) -> GlobalConfig {
        let path = dir.join("global.json");
        std::fs::write(&path, body).unwrap();
        GlobalConfig::load(&path).unwrap()
    }

    fn read(p: &Project, name: &str) -> String {
        std::fs::read_to_string(p.root_path.join(".bee").join(name)).unwrap()
    }

    #[test]
    fn decision_prunes_equal_or_forced_and_keeps_a_difference() {
        assert_eq!(
            prune_decision(&json!({"a": 1}), &json!({"a": 1}), false),
            PruneAction::Pruned
        );
        assert_eq!(
            prune_decision(&json!(1), &json!(2), false),
            PruneAction::Differs
        );
        assert_eq!(
            prune_decision(&json!(1), &json!(2), true),
            PruneAction::Pruned
        );
    }

    #[test]
    fn prune_removes_an_equal_value_and_keeps_other_keys_in_order() {
        let dir = tmp("prune-equal");
        let p = project_in(
            &dir,
            "p",
            &[("config.json", r#"{"zeta":1,"team":{"a":1},"alpha":2}"#)],
        );
        let g = global_with(&dir, r#"{"team":{"a":1}}"#);
        let rows = prune(std::slice::from_ref(&p), &g, &key("team"), &[], false, true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, PruneAction::Pruned);
        assert_eq!(rows[0].layer, Some(PruneLayer::Tracked));
        assert!(rows[0].applied);
        assert_eq!(
            read(&p, "config.json"),
            "{\n  \"zeta\": 1,\n  \"alpha\": 2\n}\n"
        );
    }

    #[test]
    fn prune_keeps_a_different_value_unless_forced() {
        let dir = tmp("prune-differs");
        let body = r#"{"team":{"a":2}}"#;
        let p = project_in(&dir, "p", &[("config.local.json", body)]);
        let g = global_with(&dir, r#"{"team":{"a":1}}"#);
        let rows = prune(std::slice::from_ref(&p), &g, &key("team"), &[], false, true).unwrap();
        assert_eq!(rows[0].action, PruneAction::Differs);
        assert_eq!(rows[0].layer, Some(PruneLayer::Local));
        assert!(!rows[0].applied);
        assert_eq!(read(&p, "config.local.json"), body);
        let rows = prune(std::slice::from_ref(&p), &g, &key("team"), &[], true, true).unwrap();
        assert_eq!(rows[0].action, PruneAction::Pruned);
        assert_eq!(read(&p, "config.local.json"), "{}\n");
    }

    #[test]
    fn prune_of_a_nested_key_drops_the_parent_it_empties() {
        let dir = tmp("prune-nested");
        let p = project_in(
            &dir,
            "p",
            &[
                ("team.json", r#"{"herding":{"agents":["a"]},"keep":1}"#),
                (
                    "config.json",
                    r#"{"herding":{"agents":["a"],"other":true}}"#,
                ),
            ],
        );
        let g = global_with(&dir, r#"{"herding":{"agents":["a"]}}"#);
        let rows = prune(
            std::slice::from_ref(&p),
            &g,
            &key("herding.agents"),
            &[],
            false,
            true,
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(read(&p, "team.json"), "{\n  \"keep\": 1\n}\n");
        assert_eq!(
            read(&p, "config.json"),
            "{\n  \"herding\": {\n    \"other\": true\n  }\n}\n"
        );
    }

    #[test]
    fn a_dry_run_writes_nothing_and_an_unheld_key_is_absent() {
        let dir = tmp("prune-dry");
        let body = r#"{"team":1}"#;
        let held = project_in(&dir, "held", &[("config.json", body)]);
        let bare = project_in(&dir, "bare", &[]);
        let g = global_with(&dir, r#"{"team":1}"#);
        let projects = [held.clone(), bare];
        let rows = prune(&projects, &g, &key("team"), &[], false, false).unwrap();
        assert_eq!(rows[0].action, PruneAction::Pruned);
        assert!(!rows[0].applied);
        assert_eq!(rows[1].action, PruneAction::Absent);
        assert_eq!(rows[1].layer, None);
        assert_eq!(read(&held, "config.json"), body);
    }

    #[test]
    fn prune_refuses_a_key_the_global_file_does_not_set() {
        let dir = tmp("prune-noglobal");
        let body = r#"{"team":1}"#;
        let p = project_in(&dir, "p", &[("config.json", body)]);
        let g = global_with(&dir, "{}");
        let err = prune(std::slice::from_ref(&p), &g, &key("team"), &[], false, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("waggledance bee config set"), "{err}");
        assert_eq!(read(&p, "config.json"), body);
    }

    #[test]
    fn prune_refuses_forbidden_keys_and_unknown_projects() {
        let dir = tmp("prune-refuse");
        let p = project_in(&dir, "p", &[]);
        let g = global_with(&dir, "{}");
        for k in ["guards", "gate_bypass.x", "hooks"] {
            assert!(prune(std::slice::from_ref(&p), &g, &key(k), &[], false, true).is_err());
        }
        let g = global_with(&dir, r#"{"team":1}"#);
        let err = prune(
            std::slice::from_ref(&p),
            &g,
            &key("team"),
            &["nope".to_string()],
            false,
            true,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProjectNotFound(_)), "{err}");
    }

    #[test]
    fn prune_reports_invalid_json_and_missing_roots_without_touching_files() {
        let dir = tmp("prune-bad");
        let bad = project_in(
            &dir,
            "bad",
            &[("config.json", "{nope"), ("team.json", "[1]")],
        );
        let nobee = Project {
            root_path: dir.join("nobee"),
            id: "nobee".into(),
            ..bad.clone()
        };
        std::fs::create_dir_all(&nobee.root_path).unwrap();
        let gone = Project {
            root_path: dir.join("gone"),
            id: "gone".into(),
            ..bad.clone()
        };
        let g = global_with(&dir, r#"{"team":1}"#);
        let projects = [bad.clone(), nobee, gone];
        let rows = prune(&projects, &g, &key("team"), &[], false, true).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].action, PruneAction::Error);
        assert_eq!(rows[0].layer, Some(PruneLayer::Tracked));
        assert_eq!(rows[1].action, PruneAction::Error);
        assert_eq!(rows[1].layer, Some(PruneLayer::Team));
        assert_eq!(rows[2].detail.as_deref(), Some("no bee"));
        assert_eq!(rows[3].detail.as_deref(), Some("root missing"));
        assert_eq!(read(&bad, "config.json"), "{nope");
        assert_eq!(read(&bad, "team.json"), "[1]");
    }
}
