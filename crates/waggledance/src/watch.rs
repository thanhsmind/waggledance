//! Filesystem watcher: notify-debouncer-full (200ms) → incremental reindex →
//! broadcast a reload-signal. Watches each project known at daemon start
//! (PRD FR-08/FR-09/FR-09b).

use anyhow::Result;
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use waggledance_core::Engine;

pub type WatchHandle = Debouncer<notify::RecommendedWatcher, FileIdMap>;

/// Build a debouncer watching every registered project. The returned handle
/// must be kept alive for the daemon's lifetime.
pub fn spawn_watchers(
    engine: Arc<Engine>,
    reload_tx: broadcast::Sender<String>,
) -> Result<WatchHandle> {
    let debounce = Duration::from_millis(engine.config.indexing.debounce_ms.max(50));
    let cb_engine = engine.clone();

    let mut debouncer = new_debouncer(debounce, None, move |res: DebounceEventResult| {
        if let Ok(events) = res {
            let paths: Vec<_> = events.into_iter().flat_map(|e| e.paths.clone()).collect();
            let changed = reindex_paths(&cb_engine, &paths);
            if !changed.is_empty() {
                let payload = serde_json::json!({ "changed": changed }).to_string();
                let _ = reload_tx.send(payload);
            }
        }
    })?;

    for project in engine.list_projects().unwrap_or_default() {
        let root = project.root_path.clone();
        if root.exists() {
            debouncer
                .watcher()
                .watch(&root, RecursiveMode::Recursive)
                .ok();
            debouncer.cache().add_root(&root, RecursiveMode::Recursive);
        }
    }
    Ok(debouncer)
}

/// Reindex the given paths incrementally. Returns the changed documents as
/// `<project_id>/<repo-relative-path>` entries (slash-separated on every
/// platform) — the reload payload clients match their own URL against.
///
/// A path is only included when it actually changed (D2, `backlog-groom-1`):
/// a touch / byte-identical rewrite reindexes but reports no change, so it
/// broadcasts no reload. Deletions and brand-new paths always count as
/// changed, unchanged from before.
///
/// Two kinds of path get here. Markdown is *indexed and* reported: it is the
/// content the daemon serves. The bee state files named by [`is_bee_signal`]
/// are reported only — they never enter the markdown index (they are not
/// documents and would pollute search), they exist in this list because a
/// board card's text is derived from them (board-approve-actions D4: a gate
/// approval writes `.bee/lanes/<feature>.json`, and without a broadcast the
/// card a human just approved keeps showing the stop they cleared until a
/// manual reload). Every other path — session heartbeats, cells, logs,
/// reservations — is still dropped here.
///
/// `.bee/state.json` is reported only when its *gate-relevant projection*
/// moved (bap-6, [`state_gate_projection`]). bee rewrites that file on
/// worker and waiting-on churn every turn, and broadcasting each of those
/// writes reloads every open board on the pace of an agent's typing.
fn reindex_paths(engine: &Engine, paths: &[std::path::PathBuf]) -> Vec<String> {
    let projects = engine.list_projects().unwrap_or_default();
    let mut changed = Vec::new();

    for path in paths {
        // Report-only paths skip the indexer entirely; anything that is
        // neither markdown nor a bee signal never reaches the socket.
        let report_only = if is_markdown(path) {
            false
        } else if is_bee_signal(path) {
            true
        } else {
            continue;
        };
        let Some(project) = projects.iter().find(|p| path.starts_with(&p.root_path)) else {
            continue;
        };
        let content_changed = if report_only {
            // No stored copy to diff against (the engine indexes markdown
            // only), so a lane write always reports; the client debounces the
            // burst bee emits when it rewrites a lane and its projections
            // together. `.bee/state.json` is the exception (bap-6): it is
            // rewritten on worker and waiting-on churn every turn, so it is
            // diffed against its own gate-relevant projection first.
            if is_bee_state_file(path) {
                state_gate_projection_changed(path)
            } else {
                true
            }
        } else if path.exists() {
            // Reindex the file and refresh its outgoing links (keeps backlinks
            // live). Only a genuine content change reports true.
            engine
                .index_file_incremental(project, path)
                .unwrap_or(false)
        } else {
            // Removed/renamed away — drop from index (survives atomic-save because
            // the debounced batch also carries the recreated path).
            let _ = engine.remove_file(project, path);
            true
        };
        if content_changed {
            if let Ok(rel) = path.strip_prefix(&project.root_path) {
                let rel = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                changed.push(format!("{}/{}", project.id, rel));
            }
        }
    }
    changed
}

fn is_markdown(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("md") | Some("markdown")
    )
}

/// The bee state files a board card's text is derived from, and nothing else.
/// Exactly two shapes are accepted:
///
/// - `<root>/.bee/state.json` — the gate record of an unlaned feature.
/// - `<root>/.bee/lanes/<feature>.json` — the gate record of a laned one.
///
/// Session heartbeats (`.bee/sessions/*.json`) are deliberately excluded:
/// they rewrite on a timer and would turn a push channel into a metronome.
/// Cells, logs, reservations and every other `.bee/` path are excluded for
/// the same reason — they do not decide what a card says.
fn is_bee_signal(p: &Path) -> bool {
    if is_bee_state_file(p) {
        return true;
    }
    let parent = p.parent();
    let parent_name = parent.and_then(|d| d.file_name()).and_then(|n| n.to_str());
    let grandparent_name = parent
        .and_then(|d| d.parent())
        .and_then(|d| d.file_name())
        .and_then(|n| n.to_str());
    parent_name == Some("lanes")
        && grandparent_name == Some(".bee")
        && p.extension().and_then(|e| e.to_str()) == Some("json")
}

/// `<root>/.bee/state.json` — the unlaned gate record, and the one bee
/// signal that is rewritten constantly for reasons no board cares about.
fn is_bee_state_file(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()) == Some("state.json")
        && p.parent()
            .and_then(|d| d.file_name())
            .and_then(|n| n.to_str())
            == Some(".bee")
}

/// The only part of `.bee/state.json` a board card's text is derived from:
/// which feature the session is on, its phase, and which gates are approved.
/// Everything else in that file — the live worker list, the waiting-on mark,
/// heartbeats, handoff bookkeeping — is rewritten on ordinary turn churn and
/// changes nothing a card shows.
///
/// Kept as a `serde_json::Value` of exactly those three keys rather than a
/// typed struct: bee owns this file's schema, and a projection that only
/// *reads* three names by string cannot go stale when bee adds a fourth.
type GateProjection = serde_json::Value;

/// The last gate-relevant projection seen for each `.bee/state.json` on
/// disk, keyed by absolute path — the report-only sibling of the content
/// cache `reindex_paths` leans on for markdown (the engine's own index).
/// Process-wide and unbounded by design: there is one entry per registered
/// project's state file, so it is bounded by the project list.
fn state_projections(
) -> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, GateProjection>> {
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, GateProjection>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Read `path` and project it onto the three gate-relevant keys. `None` when
/// the file cannot be read or is not JSON — mid-write, mid-rename, or
/// truncated. That case reports nothing: the write that finishes it is a
/// separate event and will be compared normally.
fn state_gate_projection(path: &Path) -> Option<GateProjection> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let pick = |key: &str| value.get(key).cloned().unwrap_or(serde_json::Value::Null);
    Some(serde_json::json!({
        "feature": pick("feature"),
        "phase": pick("phase"),
        "approved_gates": pick("approved_gates"),
    }))
}

/// `true` only when this write moved something a board renders (bap-6). The
/// first sight of a state file always counts as a change — there is nothing
/// to compare it against, and a board that just came up wants the render.
///
/// An unreadable or unparsable file leaves the remembered projection alone
/// rather than clearing it: a transient bad read must not turn the next good
/// write into a spurious "first sight" broadcast.
fn state_gate_projection_changed(path: &Path) -> bool {
    let Some(next) = state_gate_projection(path) else {
        return false;
    };
    let Ok(mut cache) = state_projections().lock() else {
        // A poisoned cache is not a reason to go silent on gate writes.
        return true;
    };
    match cache.get(path) {
        Some(prev) if *prev == next => false,
        _ => {
            cache.insert(path.to_path_buf(), next);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use waggledance_core::{Config, Engine, SqliteStore};

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    /// D2 (`backlog-groom-1`): a byte-identical reindex reports no change, a
    /// real content edit and a brand-new path each do.
    #[test]
    fn reindex_paths_reports_change_only_when_content_actually_changed() {
        let dir = std::env::temp_dir().join(format!("waggledance-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# A\nfirst");

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        let a = dir.join("docs/a.md");

        // The initial register() already indexed docs/a.md via its full scan
        // (same content on disk), so reindexing it again unchanged emits nothing.
        let changed = reindex_paths(&engine, std::slice::from_ref(&a));
        assert!(
            changed.is_empty(),
            "byte-identical reindex must emit no reload, got {changed:?}"
        );

        // A real content edit reports a reload for that path.
        std::fs::write(&a, "# A\nsecond").unwrap();
        let changed = reindex_paths(&engine, std::slice::from_ref(&a));
        assert_eq!(changed, vec![format!("{}/docs/a.md", project.id)]);

        // A brand-new path (no prior stored content) reports a reload too.
        write(&dir, "docs/b.md", "# B\nbrand new");
        let b = dir.join("docs/b.md");
        let changed = reindex_paths(&engine, &[b]);
        assert_eq!(changed, vec![format!("{}/docs/b.md", project.id)]);

        // Re-reindexing the same new file unchanged now emits nothing.
        let b = dir.join("docs/b.md");
        let changed = reindex_paths(&engine, &[b]);
        assert!(
            changed.is_empty(),
            "reindexing the now-stored new file unchanged must emit no reload, got {changed:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Deletions keep reporting as changed (removal always drives a reload).
    #[test]
    fn reindex_paths_still_reports_a_deleted_path_as_changed() {
        let dir =
            std::env::temp_dir().join(format!("waggledance-watch-del-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# A\ncontent");

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        let a = dir.join("docs/a.md");
        std::fs::remove_file(&a).unwrap();

        let changed = reindex_paths(&engine, &[a]);
        assert_eq!(changed, vec![format!("{}/docs/a.md", project.id)]);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// board-approve-actions (D4, `bap-5`): a gate approval writes bee state,
    /// not markdown, so the two paths a card's stop is read from have to reach
    /// the socket — and the noisy rest of `.bee/` must not follow them.
    #[test]
    fn reindex_paths_broadcasts_bee_gate_state_but_not_session_heartbeats() {
        let dir =
            std::env::temp_dir().join(format!("waggledance-watch-bee-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# A\ncontent");

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();

        // `bee gate --lane <feature>` writes here; the card reads its stop
        // from it, so it is broadcast.
        write(
            &dir,
            ".bee/lanes/board-approve-actions.json",
            "{\"gates\":{}}",
        );
        let lane = dir.join(".bee/lanes/board-approve-actions.json");
        assert_eq!(
            reindex_paths(&engine, std::slice::from_ref(&lane)),
            vec![format!(
                "{}/.bee/lanes/board-approve-actions.json",
                project.id
            )],
            "a lane gate write must reach /ws or the approved card stays stale"
        );

        // The unlaned spelling of the same record.
        write(&dir, ".bee/state.json", "{\"phase\":\"execute\"}");
        let state = dir.join(".bee/state.json");
        assert_eq!(
            reindex_paths(&engine, std::slice::from_ref(&state)),
            vec![format!("{}/.bee/state.json", project.id)]
        );

        // Session heartbeats rewrite on a timer and decide nothing a card
        // says — broadcasting them would reload every open board on a clock.
        write(&dir, ".bee/sessions/abc123.json", "{\"heartbeat\":1}");
        let session = dir.join(".bee/sessions/abc123.json");
        assert!(
            reindex_paths(&engine, std::slice::from_ref(&session)).is_empty(),
            "a session heartbeat must never be broadcast"
        );

        // The rest of `.bee/` stays dropped too — the widening is exactly two
        // shapes, never "the whole `.bee/` directory".
        for other in [
            ".bee/cells.jsonl",
            ".bee/holds.json",
            ".bee/decisions.jsonl",
            ".bee/lanes/nested/deep.json",
            ".bee/logs/run.json",
        ] {
            write(&dir, other, "x");
            let other_path = dir.join(other);
            assert!(
                reindex_paths(&engine, std::slice::from_ref(&other_path)).is_empty(),
                "{other} must not be broadcast"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// bap-6: `.bee/state.json` is rewritten on ordinary turn churn — the
    /// live worker list, the waiting-on mark, heartbeats — and broadcasting
    /// each of those reloads every open board for nothing. Only a move in
    /// the gate-relevant projection (`feature`, `phase`, `approved_gates`)
    /// is news; the lane file's own broadcast is untouched.
    #[test]
    fn reindex_paths_broadcasts_state_json_only_when_its_gate_projection_moves() {
        let dir =
            std::env::temp_dir().join(format!("waggledance-watch-proj-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# A\ncontent");

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        let state = dir.join(".bee/state.json");
        let expected = vec![format!("{}/.bee/state.json", project.id)];

        let record = |workers: &str, uat: bool| {
            format!(
                r#"{{"feature":"board-approve-actions","phase":"swarming",
                     "approved_gates":{{"shape":true,"execution":true,"uat":{uat}}},
                     "workers":[{workers}],"waiting_on":{{"kind":"turn-end"}}}}"#
            )
        };

        // First sight of the file: nothing to compare against, so it reports.
        std::fs::create_dir_all(state.parent().unwrap()).unwrap();
        std::fs::write(&state, record("\"w1\"", false)).unwrap();
        assert_eq!(
            reindex_paths(&engine, std::slice::from_ref(&state)),
            expected,
            "the first state.json a board sees must render it"
        );

        // Ordinary turn churn: a worker joins, the gates are untouched.
        std::fs::write(&state, record("\"w1\",\"w2\"", false)).unwrap();
        assert!(
            reindex_paths(&engine, std::slice::from_ref(&state)).is_empty(),
            "a worker/waiting-on rewrite must not reload every open board"
        );

        // The uat gate flips: that is exactly what a card renders.
        std::fs::write(&state, record("\"w1\",\"w2\"", true)).unwrap();
        assert_eq!(
            reindex_paths(&engine, std::slice::from_ref(&state)),
            expected,
            "a gate approval must reach /ws or the answered card stays stale"
        );

        // Unparsable (mid-write, truncated) reports nothing, and leaves the
        // remembered projection alone — the good write that follows is
        // compared against the gates, not against a cleared cache.
        std::fs::write(&state, "{\"feature\":").unwrap();
        assert!(
            reindex_paths(&engine, std::slice::from_ref(&state)).is_empty(),
            "half-written JSON is not a gate change"
        );
        std::fs::write(&state, record("\"w3\"", true)).unwrap();
        assert!(
            reindex_paths(&engine, std::slice::from_ref(&state)).is_empty(),
            "the same gates after a bad read must still be quiet"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
