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
            // only), so every write reports. The client debounces the burst
            // bee emits when it rewrites a lane and its projections together.
            true
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
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let parent = p.parent();
    let parent_name = parent.and_then(|d| d.file_name()).and_then(|n| n.to_str());
    if name == "state.json" && parent_name == Some(".bee") {
        return true;
    }
    let grandparent_name = parent
        .and_then(|d| d.parent())
        .and_then(|d| d.file_name())
        .and_then(|n| n.to_str());
    parent_name == Some("lanes")
        && grandparent_name == Some(".bee")
        && p.extension().and_then(|e| e.to_str()) == Some("json")
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
        write(&dir, ".bee/lanes/board-approve-actions.json", "{\"gates\":{}}");
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
}
