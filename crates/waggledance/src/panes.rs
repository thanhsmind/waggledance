use crate::herdr;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use waggledance_core::bee::{
    BeeActivity, BeeActivityState, BeeProjectRollup, BeeSession, BeeWorktree,
};
use waggledance_core::domain::Project;
use waggledance_core::paths_boundary::{is_contained_in_root, Boundary};

#[derive(Clone)]
pub(crate) struct PaneView {
    pub(crate) pane_id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) workspace: String,
    pub(crate) tab: String,
    pub(crate) bee_state: Option<BeeActivityState>,
    pub(crate) bee_feature: Option<String>,
}

pub(crate) fn project_panes(snapshot: &herdr::Snapshot, boundary: &Boundary) -> Vec<PaneView> {
    snapshot
        .panes
        .iter()
        .filter_map(|pane| {
            pane.cwd
                .as_deref()
                .and_then(|raw| boundary.validate_existing(Path::new(raw)).ok())
                .or_else(|| {
                    pane.foreground_cwd
                        .as_deref()
                        .and_then(|raw| boundary.validate_existing(Path::new(raw)).ok())
                })?;
            let agent = snapshot.agents.iter().find(|a| a.pane_id == pane.pane_id);
            Some(PaneView {
                pane_id: pane.pane_id.clone(),
                kind: agent
                    .map(|a| a.kind.clone())
                    .unwrap_or_else(|| "shell".to_string()),
                status: agent
                    .map(|a| a.status.as_str().to_string())
                    .unwrap_or_else(|| "shell".to_string()),
                workspace: snapshot.workspace_label_for_id(&pane.workspace_id),
                tab: snapshot.tab_label_for_id(&pane.tab_id),
                bee_state: None,
                bee_feature: None,
            })
        })
        .collect()
}

pub(crate) fn herdr_session_ids(snapshot: Option<&herdr::Snapshot>) -> HashSet<String> {
    snapshot
        .map(|s| {
            s.agents
                .iter()
                .filter_map(|a| a.session_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_session_feature(
    cell_feature_by_id: &HashMap<&str, &str>,
    worktrees: &[BeeWorktree],
    parent_dir: Option<&Path>,
    session: &BeeSession,
) -> Option<String> {
    let activity = session.activity.as_ref();

    if let Some(feature) = activity
        .and_then(|a| a.cell.as_deref())
        .and_then(|id| cell_feature_by_id.get(id).copied())
    {
        return Some(feature.to_string());
    }

    if let (Some(parent), Some(cwd)) = (parent_dir, activity.and_then(|a| a.cwd.as_deref())) {
        let cwd = Path::new(cwd);
        for w in worktrees {
            let Some(feature) = w.feature.as_deref() else {
                continue;
            };
            if is_contained_in_root(cwd, &parent.join(&w.id)) {
                return Some(feature.to_string());
            }
        }
    }

    if let Some(lane) = session.lane.as_deref() {
        return Some(lane.to_string());
    }

    activity.and_then(|a| a.feature.clone())
}

pub(crate) fn project_bee_activity(
    project: &Project,
    rollup: &BeeProjectRollup,
    herdr_sessions: &HashSet<String>,
) -> HashMap<String, BeeActivity> {
    let snapshot = &rollup.snapshot;
    let mut roots = vec![project.root_path.clone()];
    if let Some(parent) = project.root_path.parent() {
        for w in &snapshot.worktrees {
            roots.push(parent.join(&w.id));
        }
    }
    let Ok(boundary) = Boundary::new(roots) else {
        return HashMap::new();
    };
    let mut cell_features: HashMap<&str, &str> = HashMap::new();
    for bucket in [
        &snapshot.buckets.doing,
        &snapshot.buckets.waiting,
        &snapshot.buckets.stuck,
        &snapshot.buckets.done,
    ] {
        for cell in bucket {
            cell_features
                .entry(cell.id.as_str())
                .or_insert(&cell.feature);
        }
    }

    let mut out: HashMap<String, BeeActivity> = HashMap::new();
    for session in &snapshot.sessions {
        if !session.live && !herdr_sessions.contains(&session.id) {
            continue;
        }
        let Some(activity) = session.activity.as_ref() else {
            continue;
        };
        let Some(cwd) = activity.cwd.as_deref() else {
            continue;
        };
        if boundary.validate_existing(Path::new(cwd)).is_err() {
            continue;
        }
        let Some(pane) = activity.pane.as_deref() else {
            continue;
        };
        let mut resolved = activity.clone();
        resolved.feature = resolve_session_feature(
            &cell_features,
            &snapshot.worktrees,
            project.root_path.parent(),
            session,
        );
        match out.get(pane) {
            Some(existing) if existing.at >= resolved.at => {}
            _ => {
                out.insert(pane.to_string(), resolved);
            }
        }
    }
    out
}

pub(crate) fn apply_bee_activity(panes: &mut [PaneView], by_pane: &HashMap<String, BeeActivity>) {
    for pane in panes.iter_mut() {
        let Some(activity) = by_pane.get(&pane.pane_id) else {
            continue;
        };
        pane.bee_state = Some(activity.state.clone());
        pane.bee_feature = activity.feature.clone();
    }
}

#[cfg(test)]
mod session_feature_resolution_tests {
    use super::*;
    use waggledance_core::bee::{
        BeeActivity, BeeActivityState, BeeSession, BeeSignal, BeeWorktree,
    };

    fn activity(cwd: Option<&str>, cell: Option<&str>, feature: Option<&str>) -> BeeActivity {
        BeeActivity {
            state: BeeActivityState::Working,
            event: "PreToolUse".to_string(),
            tool_name: None,
            tool_use_id: None,
            at: "2026-08-23T06:00:00Z".to_string(),
            age_seconds: Some(3.0),
            pane: Some("%1".to_string()),
            cwd: cwd.map(str::to_string),
            feature: feature.map(str::to_string),
            cell: cell.map(str::to_string),
        }
    }

    fn session(lane: Option<&str>, activity: Option<BeeActivity>) -> BeeSession {
        BeeSession {
            id: "s1".to_string(),
            started_at: None,
            heartbeat_age_minutes: 0.5,
            live: true,
            workspace_id: None,
            source: None,
            lane: lane.map(str::to_string),
            activity,
            signal: BeeSignal::Live,
            work: None,
        }
    }

    fn worktree(id: &str, feature: Option<&str>) -> BeeWorktree {
        BeeWorktree {
            id: id.to_string(),
            resolved: true,
            unresolved_reason: None,
            feature: feature.map(str::to_string),
            phase: None,
            mode: None,
            branch: None,
            created_at: None,
            live: true,
            heartbeat_age_minutes: Some(0.5),
            merged_pending: false,
        }
    }

    fn cells<'a>(pairs: &[(&'a str, &'a str)]) -> std::collections::HashMap<&'a str, &'a str> {
        pairs.iter().copied().collect()
    }

    fn parent() -> &'static std::path::Path {
        std::path::Path::new("/home/dev/projects")
    }

    #[test]
    fn held_cell_outranks_worktree_lane_and_self_report() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd--wt--agent-board/crates"),
                Some("afr-1"),
                Some("stale-lane"),
            )),
        );
        let out = resolve_session_feature(
            &cells(&[("afr-1", "agent-feature-resolution")]),
            &[worktree("wd--wt--agent-board", Some("agent-board"))],
            Some(parent()),
            &s,
        );
        assert_eq!(out.as_deref(), Some("agent-feature-resolution"));
    }

    #[test]
    fn worktree_directory_outranks_lane_and_self_report() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd--wt--agent-board/crates/waggledance"),
                None,
                Some("stale-lane"),
            )),
        );
        let out = resolve_session_feature(
            &cells(&[]),
            &[worktree("wd--wt--agent-board", Some("agent-board"))],
            Some(parent()),
            &s,
        );
        assert_eq!(out.as_deref(), Some("agent-board"));
    }

    #[test]
    fn lane_outranks_self_report_when_no_cell_and_no_worktree() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd/crates"),
                None,
                Some("stale-lane"),
            )),
        );
        let out = resolve_session_feature(
            &cells(&[]),
            &[worktree("wd--wt--agent-board", Some("agent-board"))],
            Some(parent()),
            &s,
        );
        assert_eq!(out.as_deref(), Some("board-trim"));
    }

    #[test]
    fn activity_feature_is_the_fallback() {
        let s = session(
            None,
            Some(activity(
                Some("/home/dev/projects/wd/crates"),
                None,
                Some("self-reported"),
            )),
        );
        let out = resolve_session_feature(&cells(&[]), &[], Some(parent()), &s);
        assert_eq!(out.as_deref(), Some("self-reported"));
    }

    #[test]
    fn nothing_to_go_on_resolves_to_none() {
        let s = session(
            None,
            Some(activity(Some("/home/dev/projects/wd"), None, None)),
        );
        assert_eq!(
            resolve_session_feature(&cells(&[]), &[], Some(parent()), &s),
            None
        );
        assert_eq!(
            resolve_session_feature(&cells(&[]), &[], Some(parent()), &session(None, None)),
            None
        );
    }

    #[test]
    fn unknown_cell_id_falls_through_instead_of_erasing_the_feature() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd/crates"),
                Some("archived-9"),
                None,
            )),
        );
        let out = resolve_session_feature(
            &cells(&[("afr-1", "agent-feature-resolution")]),
            &[],
            Some(parent()),
            &s,
        );
        assert_eq!(out.as_deref(), Some("board-trim"));
    }

    #[test]
    fn featureless_worktree_does_not_swallow_the_lane() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd--wt--dangling/crates"),
                None,
                None,
            )),
        );
        let out = resolve_session_feature(
            &cells(&[]),
            &[worktree("wd--wt--dangling", None)],
            Some(parent()),
            &s,
        );
        assert_eq!(out.as_deref(), Some("board-trim"));
    }

    #[test]
    fn sibling_prefix_directory_is_not_inside_the_worktree() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd--wt--agent-board-extra/crates"),
                None,
                None,
            )),
        );
        let out = resolve_session_feature(
            &cells(&[]),
            &[worktree("wd--wt--agent-board", Some("agent-board"))],
            Some(parent()),
            &s,
        );
        assert_eq!(out.as_deref(), Some("board-trim"));
    }

    #[test]
    fn no_parent_directory_skips_the_worktree_rung() {
        let s = session(
            Some("board-trim"),
            Some(activity(
                Some("/home/dev/projects/wd--wt--agent-board/crates"),
                None,
                None,
            )),
        );
        let out = resolve_session_feature(
            &cells(&[]),
            &[worktree("wd--wt--agent-board", Some("agent-board"))],
            None,
            &s,
        );
        assert_eq!(out.as_deref(), Some("board-trim"));
    }
}
