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
        let mut segs = key.0.iter();
        let mut cur = self.doc.get(segs.next()?)?;
        for seg in segs {
            cur = cur.as_object()?.get(seg)?;
        }
        Some(cur)
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
    let parsed = text
        .find('{')
        .and_then(|i| serde_json::from_str::<Value>(text[i..].trim()).ok());
    match (out.ok, parsed) {
        (true, Some(v)) => row(
            v.get("source").and_then(Value::as_str).map(str::to_string),
            v.get("value").cloned(),
            None,
        ),
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
        let rows = explain(&[project.clone()], &key("a.b"), &[]).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].error.is_some() && rows[0].source.is_none());
        assert!(explain(&[project], &key("a"), &["nope".to_string()]).is_err());
    }
}
