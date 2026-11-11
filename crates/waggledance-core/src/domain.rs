//! Domain types. Pure data — no dependency on Axum/Tauri/SQLite.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A registered project: a root directory the supervisor works in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: PathBuf,
    /// RFC3339 timestamps.
    pub created_at: String,
    pub last_seen_at: String,
    /// D6 opt-in: true only when this project has been switched into
    /// orchestrator-dispatch mode. Effective only when `terminal.enabled` is
    /// also on (the caller combines the two — see `Engine::orchestration_allowed`).
    pub orchestration_enabled: bool,
}

/// One orchestrator dispatch: a task sent to one pane, from baseline capture
/// to a terminal status. Durable per D7 so a restarted orchestrator recovers
/// the fleet by reading run state instead of carrying a prompt-side roster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub project_id: String,
    pub pane_id: String,
    /// The preset used to spawn the pane, or `None` when `dispatch` targeted
    /// an already-running pane instead of spawning one (D3).
    pub preset_label: Option<String>,
    pub task: String,
    /// Pre-send transcript capture the run's delta and marker-freshness are
    /// measured against (D5).
    pub baseline: String,
    /// The split completion token minted for this run (D5).
    pub marker: String,
    /// Protocol status enum string: pending/working/done/blocked/failed/timeout.
    pub status: String,
    // The bee feature a run was started for (board-run-actions D3) lives on
    // the `runs.feature` COLUMN, not on this struct: it is written by
    // `SqliteStore::insert_run`'s own `feature` argument and read back by
    // `list_live_runs_for_feature` / `run_feature`. It is not a field here
    // yet only because this struct is built by literal in six files across
    // both crates, none of which the board cells could reserve; folding it in
    // is a mechanical follow-up, and every caller that needs the value
    // already has a store method for it. bra-3 added the second such caller:
    // `server.rs::project_live_board_runs`, which filters to the `working`
    // rows first so the side read only ever touches a handful of them.
    /// RFC3339 timestamps.
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct LeaderRun {
    pub run: Run,
    pub request_id: Option<String>,
    pub follow_up_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxMessage {
    pub seq: i64,
    pub recipient: String,
    pub corr: String,
    pub project_id: String,
    pub kind: String,
    pub body: String,
    pub created_at: String,
    pub acked_at: Option<String>,
    pub attempts: i64,
    pub dead_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WdTask {
    pub corr: String,
    pub project_id: String,
    pub leader_id: Option<String>,
    pub state: String,
    pub created_at: String,
    pub last_delivered_at: Option<String>,
    pub last_mail_at: Option<String>,
    pub nudged_at: Option<String>,
    pub progress_sent_at: Option<String>,
    pub changed_at: String,
    pub board_sent_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub req_id: String,
    pub corr: String,
    pub leader_id: String,
    pub tool: String,
    pub description: String,
    pub seen_at: String,
    pub answered_at: Option<String>,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaderHold {
    pub leader_id: String,
    pub why: String,
    pub set_at: String,
    pub until: String,
}
