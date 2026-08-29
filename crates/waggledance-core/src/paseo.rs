//! Paseo agent detection — reads paseo's on-disk per-agent JSON store
//! directly (D2, `docs/history/paseo-support/CONTEXT.md`). Never calls the
//! paseo daemon HTTP API (`127.0.0.1:6767`): the disk store is always
//! readable with no daemon/network/auth, and a display view tolerates
//! last-written status.
//!
//! Layout read: `~/.paseo/agents/<project-slug>/<uuid>.json`, exactly two
//! levels deep. This module is sync-only filesystem work — no async, no new
//! web-framework dependency (`bee::tests::no_web_framework_dependency_declared`
//! forbids axum/tokio/hyper in this crate's manifest).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One live paseo agent, mapped from a store record. Only the fields the
/// display path needs survive the seam — `last_activity_at` stays the raw
/// RFC-3339 string; the render side formats it, so no time type crosses
/// here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaseoAgent {
    pub id: String,
    pub provider: String,
    pub cwd: PathBuf,
    pub title: String,
    pub last_status: String,
    pub last_activity_at: String,
    pub model: Option<String>,
}

/// Raw on-disk record shape, camelCase per the paseo store
/// (`archivedAt`, `lastStatus`, `lastActivityAt`). `model` lives nested
/// under `config.model`, not flattened — deserialize the inner struct
/// rather than hand-flattening it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreRecord {
    id: String,
    provider: String,
    cwd: PathBuf,
    title: String,
    last_status: String,
    last_activity_at: String,
    #[serde(default)]
    archived_at: Option<serde_json::Value>,
    #[serde(default)]
    config: Option<StoreRecordConfig>,
}

#[derive(Debug, Deserialize)]
struct StoreRecordConfig {
    #[serde(default)]
    model: Option<String>,
}

impl StoreRecord {
    /// D4's verbatim liveness filter: `archivedAt` absent or null AND
    /// `lastStatus != "closed"`.
    fn is_live(&self) -> bool {
        let archived = match &self.archived_at {
            None => false,
            Some(serde_json::Value::Null) => false,
            Some(_) => true,
        };
        !archived && self.last_status != "closed"
    }
}

impl From<StoreRecord> for PaseoAgent {
    fn from(record: StoreRecord) -> Self {
        PaseoAgent {
            id: record.id,
            provider: record.provider,
            cwd: record.cwd,
            title: record.title,
            last_status: record.last_status,
            last_activity_at: record.last_activity_at,
            model: record.config.and_then(|c| c.model),
        }
    }
}

/// `~/.paseo/agents` — the store root D2 reads, when a home directory is
/// resolvable. Callers pass the root explicitly to
/// [`list_live_agents`] so tests can inject a temp dir; this function only
/// supplies the real default for production call sites.
pub fn default_store_root() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".paseo/agents"))
}

/// Enumerates `<store_root>/<slug>/<uuid>.json`, exactly two levels deep,
/// and returns the live agents (D4). Tolerant throughout: a non-directory
/// entry at level one is skipped, nothing deeper than level two is read,
/// an unreadable directory is skipped, an unparseable file is skipped, and
/// a record missing a required field is skipped — never an error, never a
/// panic.
pub fn list_live_agents(store_root: &Path) -> Vec<PaseoAgent> {
    let mut agents = Vec::new();

    let Ok(slug_entries) = fs::read_dir(store_root) else {
        return agents;
    };

    for slug_entry in slug_entries.flatten() {
        let slug_path = slug_entry.path();
        let Ok(file_type) = slug_entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let Ok(agent_files) = fs::read_dir(&slug_path) else {
            continue;
        };

        for agent_entry in agent_files.flatten() {
            let agent_path = agent_entry.path();
            if agent_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let Ok(contents) = fs::read_to_string(&agent_path) else {
                continue;
            };
            let Ok(record) = serde_json::from_str::<StoreRecord>(&contents) else {
                continue;
            };
            if record.is_live() {
                agents.push(PaseoAgent::from(record));
            }
        }
    }

    agents
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_record(store_root: &Path, slug: &str, uuid: &str, body: &str) {
        let slug_dir = store_root.join(slug);
        fs::create_dir_all(&slug_dir).unwrap();
        fs::write(slug_dir.join(format!("{uuid}.json")), body).unwrap();
    }

    #[test]
    fn running_record_with_no_archived_at_is_returned() {
        let dir = tempdir().unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-1",
            r#"{
                "id": "agent-1",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "do the thing",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z",
                "config": {"model": "claude-sonnet-5"}
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "agent-1");
        assert_eq!(agents[0].last_status, "running");
        assert_eq!(agents[0].model.as_deref(), Some("claude-sonnet-5"));
    }

    #[test]
    fn closed_record_is_excluded() {
        let dir = tempdir().unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-2",
            r#"{
                "id": "agent-2",
                "provider": "codex",
                "cwd": "/home/user/my-project",
                "title": "done",
                "lastStatus": "closed",
                "lastActivityAt": "2026-08-29T12:00:00Z"
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert!(agents.is_empty());
    }

    #[test]
    fn archived_record_is_excluded_even_when_status_is_not_closed() {
        let dir = tempdir().unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-3",
            r#"{
                "id": "agent-3",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "stale",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z",
                "archivedAt": "2026-08-29T13:00:00Z"
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert!(agents.is_empty());
    }

    #[test]
    fn null_archived_at_still_counts_as_absent() {
        let dir = tempdir().unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-4",
            r#"{
                "id": "agent-4",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "still going",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z",
                "archivedAt": null
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert_eq!(agents.len(), 1);
    }

    #[test]
    fn malformed_file_is_skipped_and_remaining_live_agents_still_return() {
        let dir = tempdir().unwrap();
        write_record(dir.path(), "my-project", "broken", "{ not valid json");
        write_record(
            dir.path(),
            "my-project",
            "agent-5",
            r#"{
                "id": "agent-5",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "healthy",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z"
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "agent-5");
    }

    #[test]
    fn record_missing_a_required_field_is_skipped() {
        let dir = tempdir().unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-6",
            r#"{
                "id": "agent-6",
                "provider": "claude",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z"
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert!(agents.is_empty());
    }

    #[test]
    fn model_is_none_when_config_object_is_absent() {
        let dir = tempdir().unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-7",
            r#"{
                "id": "agent-7",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "no config",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z"
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].model, None);
    }

    #[test]
    fn non_directory_entry_at_level_one_is_skipped() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("stray-file.txt"), "not a dir").unwrap();
        write_record(
            dir.path(),
            "my-project",
            "agent-8",
            r#"{
                "id": "agent-8",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "fine",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z"
            }"#,
        );

        let agents = list_live_agents(dir.path());

        assert_eq!(agents.len(), 1);
    }

    #[test]
    fn nested_directory_deeper_than_two_levels_is_not_read() {
        let dir = tempdir().unwrap();
        let deep = dir.path().join("my-project").join("nested");
        fs::create_dir_all(&deep).unwrap();
        fs::write(
            deep.join("agent-9.json"),
            r#"{
                "id": "agent-9",
                "provider": "claude",
                "cwd": "/home/user/my-project",
                "title": "too deep",
                "lastStatus": "running",
                "lastActivityAt": "2026-08-29T12:00:00Z"
            }"#,
        )
        .unwrap();

        let agents = list_live_agents(dir.path());

        assert!(agents.is_empty());
    }

    #[test]
    fn unreadable_store_root_returns_empty_not_a_panic() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let agents = list_live_agents(&missing);

        assert!(agents.is_empty());
    }
}
