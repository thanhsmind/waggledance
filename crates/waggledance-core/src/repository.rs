//! SQLite adapter: project registry + file index + title/path search.
//! Behind a `Mutex<Connection>` so it is Send+Sync for the async daemon.

use crate::domain::{
    InboxMessage, IndexedFile, LeaderHold, LeaderRun, PermissionRequest, Project, Run,
    SearchResult, WdTask,
};
use crate::error::Result;
use crate::short_link;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open (creating if needed) the registry DB and run migrations.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::from_conn(conn)
    }

    /// In-memory store (tests).
    pub fn open_in_memory() -> Result<Self> {
        Self::from_conn(Connection::open_in_memory()?)
    }

    fn from_conn(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        // The MCP process and the daemon both open registry.db; WAL is on but
        // without a busy_timeout a writer racing the watcher's reindex hits
        // SQLITE_BUSY immediately instead of waiting (plan.md Approach 7).
        conn.pragma_update(None, "busy_timeout", 5000).ok();
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ---- projects ----

    pub fn upsert_project(&self, p: &Project) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO projects(id,name,root_path,created_at,last_seen_at,orchestration_enabled)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET name=?2, root_path=?3, last_seen_at=?5",
            params![
                p.id,
                p.name,
                p.root_path.to_string_lossy(),
                p.created_at,
                p.last_seen_at,
                p.orchestration_enabled
            ],
        )?;
        Ok(())
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,name,root_path,created_at,last_seen_at,orchestration_enabled FROM projects WHERE id=?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows.next()?.map(row_to_project))
    }

    pub fn find_project_by_root(&self, root: &Path) -> Result<Option<Project>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,name,root_path,created_at,last_seen_at,orchestration_enabled FROM projects WHERE root_path=?1",
        )?;
        let mut rows = stmt.query(params![root.to_string_lossy()])?;
        Ok(rows.next()?.map(row_to_project))
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT id,name,root_path,created_at,last_seen_at,orchestration_enabled FROM projects ORDER BY last_seen_at DESC")?;
        let rows = stmt.query_map([], |r| Ok(row_to_project(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn delete_project(&self, id: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM files WHERE project_id=?1", params![id])?;
        c.execute("DELETE FROM links WHERE project_id=?1", params![id])?;
        c.execute("DELETE FROM runs WHERE project_id=?1", params![id])?;
        c.execute("DELETE FROM inbox WHERE project_id=?1", params![id])?;
        c.execute(
            "DELETE FROM permission_requests
             WHERE corr IN (SELECT corr FROM tasks WHERE project_id=?1)",
            params![id],
        )?;
        c.execute("DELETE FROM tasks WHERE project_id=?1", params![id])?;
        c.execute("DELETE FROM projects WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn set_orchestration_enabled(&self, project_id: &str, enabled: bool) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE projects SET orchestration_enabled=?2 WHERE id=?1",
            params![project_id, enabled],
        )?;
        Ok(())
    }

    // ---- runs (D7) ----

    /// Persist one run. `feature` is the bee feature the run was started
    /// for (board-run-actions D3) — `None` for every run that is not a
    /// board-triggered one (the MCP dispatch tool's own runs), which is
    /// also what every row written before the `feature` column existed
    /// reads back as.
    ///
    /// It is a separate argument rather than a `Run` field because
    /// [`Run`] is constructed by literal in several places this cell may
    /// not touch; see `Run`'s own doc for the note.
    pub fn insert_run(&self, r: &Run, feature: Option<&str>) -> Result<()> {
        self.insert_run_with_request(r, feature, None, None)
            .map(|_| ())
    }

    pub fn insert_run_with_request(
        &self,
        r: &Run,
        feature: Option<&str>,
        request_id: Option<&str>,
        caller: Option<&str>,
    ) -> Result<String> {
        self.insert_follow_up_run(r, feature, request_id, caller, None, false)
    }

    pub fn insert_follow_up_run(
        &self,
        r: &Run,
        feature: Option<&str>,
        request_id: Option<&str>,
        caller: Option<&str>,
        follow_up_of: Option<&str>,
        handoff: bool,
    ) -> Result<String> {
        let c = self.conn.lock().unwrap();
        let inserted = c.execute(
            "INSERT INTO runs(id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at,feature,request_id,caller,follow_up_of,handoff)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(project_id,request_id) WHERE request_id IS NOT NULL DO NOTHING",
            params![
                r.id,
                r.project_id,
                r.pane_id,
                r.preset_label,
                r.task,
                r.baseline,
                r.marker,
                r.status,
                r.created_at,
                r.updated_at,
                feature,
                request_id,
                caller,
                follow_up_of,
                handoff
            ],
        )?;
        if inserted == 1 {
            return Ok(r.id.clone());
        }
        Ok(c.query_row(
            "SELECT id FROM runs WHERE project_id=?1 AND request_id=?2",
            params![r.project_id, request_id],
            |row| row.get(0),
        )?)
    }

    pub fn run_by_request(&self, project_id: &str, request_id: &str) -> Result<Option<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE project_id=?1 AND request_id=?2",
        )?;
        let mut rows = stmt.query(params![project_id, request_id])?;
        Ok(rows.next()?.map(row_to_run))
    }

    pub fn run_follow_up_of(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT follow_up_of FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, Option<String>>(0).ok())
            .flatten())
    }

    pub fn run_is_handoff(&self, id: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT handoff FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, i64>(0).ok())
            .is_some_and(|h| h != 0))
    }

    pub fn runs_on_pane(&self, pane_id: &str) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE pane_id=?1 ORDER BY created_at DESC, rowid DESC",
        )?;
        let rows = stmt.query_map(params![pane_id], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn family_handoffs(&self, root: &str) -> Result<HashSet<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "WITH RECURSIVE fam(id) AS (
                SELECT ?1
                UNION
                SELECT r.id FROM runs r JOIN fam ON r.follow_up_of=fam.id
             )
             SELECT id FROM runs WHERE handoff=1 AND id IN (SELECT id FROM fam)",
        )?;
        let rows = stmt.query_map(params![root], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn run_request_id(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT request_id FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, Option<String>>(0).ok())
            .flatten())
    }

    pub fn follow_up_chain(&self, id: &str) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "WITH RECURSIVE up(id, parent, depth) AS (
                SELECT id, follow_up_of, 0 FROM runs WHERE id=?1
                UNION ALL
                SELECT r.id, r.follow_up_of, up.depth+1 FROM runs r JOIN up ON r.id=up.parent
                WHERE up.depth < 64
             )
             SELECT r.id,r.project_id,r.pane_id,r.preset_label,r.task,r.baseline,r.marker,r.status,r.created_at,r.updated_at
             FROM up JOIN runs r ON r.id=up.id ORDER BY up.depth DESC",
        )?;
        let rows = stmt.query_map(params![id], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn follow_up_family(&self, root: &str) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "WITH RECURSIVE fam(id) AS (
                SELECT ?1
                UNION
                SELECT r.id FROM runs r JOIN fam ON r.follow_up_of=fam.id
             )
             SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE id IN (SELECT id FROM fam) ORDER BY created_at ASC, rowid ASC",
        )?;
        let rows = stmt.query_map(params![root], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn leader_runs(
        &self,
        project_id: &str,
        pane_id: &str,
        limit: usize,
    ) -> Result<Vec<LeaderRun>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at,request_id,follow_up_of
             FROM runs WHERE project_id=?1 AND pane_id=?2
             ORDER BY created_at DESC, rowid DESC LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![project_id, pane_id, limit as i64], |r| {
            Ok(LeaderRun {
                run: row_to_run(r),
                request_id: r.get(10)?,
                follow_up_of: r.get(11)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn project_for_corr(&self, corr: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT project_id FROM runs
             WHERE request_id=?1 OR substr(request_id,1,length(?1)+1)=?1||':'
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![corr])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub fn run_caller(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT caller FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, Option<String>>(0).ok())
            .flatten())
    }

    pub fn run_attempts(&self, id: &str) -> Result<Option<i64>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT attempts FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows.next()?.and_then(|r| r.get::<_, i64>(0).ok()))
    }

    pub fn list_queued_paseo_runs(&self, project_id: &str) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE project_id=?1 AND status='queued' AND pane_id LIKE 'paseo:%'
             ORDER BY created_at ASC, rowid ASC",
        )?;
        let rows = stmt.query_map(params![project_id], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_projects_with_queued_paseo_runs(&self) -> Result<Vec<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT DISTINCT project_id FROM runs
             WHERE status='queued' AND pane_id LIKE 'paseo:%' ORDER BY project_id",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn claim_queued_run(&self, id: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        let changed = c.execute(
            "UPDATE runs SET status='working' WHERE id=?1 AND status='queued'",
            params![id],
        )?;
        Ok(changed == 1)
    }

    pub fn set_run_request(&self, id: &str, request_id: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET request_id=?2 WHERE id=?1",
            params![id, request_id],
        )?;
        Ok(())
    }

    pub fn set_run_session(&self, id: &str, session_id: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        let changed = c.execute(
            "UPDATE runs SET session_id=?2 WHERE id=?1 AND session_id IS NULL",
            params![id, session_id],
        )?;
        Ok(changed == 1)
    }

    pub fn run_session(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let found = c
            .query_row(
                "SELECT session_id FROM runs WHERE id=?1",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        Ok(found.flatten())
    }

    pub fn run_sessions(&self) -> Result<HashMap<String, crate::usage_rows::RunMeta>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT session_id, preset_label, task FROM runs
             WHERE session_id IS NOT NULL ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                crate::usage_rows::RunMeta {
                    preset_label: row.get(1)?,
                    task: row.get(2)?,
                },
            ))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn set_run_pane(&self, id: &str, pane_id: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET pane_id=?2 WHERE id=?1",
            params![id, pane_id],
        )?;
        Ok(())
    }

    pub fn requeue_run(&self, id: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        let changed = c.execute(
            "UPDATE runs SET status='queued', attempts=attempts+1 WHERE id=?1 AND status='working'",
            params![id],
        )?;
        Ok(changed == 1)
    }

    pub fn list_inflight_paseo_runs(&self) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE status IN ('working','blocked') AND pane_id LIKE 'paseo:%'
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_leader_waits(&self, lost_since: &str) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs
             WHERE pane_id LIKE 'paseo:%'
               AND (status='blocked' OR (status='lost' AND updated_at >= ?1))
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![lost_since], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Update a run's status (and its `updated_at` stamp). `baseline`/`marker`
    /// are optionally replaced too, for a re-send against the same run row.
    pub fn update_run_status(
        &self,
        id: &str,
        status: &str,
        updated_at: &str,
        baseline: Option<&str>,
        marker: Option<&str>,
    ) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET status=?2, updated_at=?3,
               baseline=COALESCE(?4, baseline),
               marker=COALESCE(?5, marker)
             WHERE id=?1",
            params![id, status, updated_at, baseline, marker],
        )?;
        Ok(())
    }

    pub fn get_run(&self, id: &str) -> Result<Option<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE id=?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows.next()?.map(row_to_run))
    }

    pub fn list_runs(&self, project_id: &str, limit: usize) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE project_id=?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit as i64], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Every run this project started for `feature` that is still `working`
    /// (board-run-actions D3, the per-feature run lock). Newest first.
    ///
    /// Deliberately only the `working` rows: a run that reached any terminal
    /// status has released the feature, and a run whose pane has since
    /// vanished is filtered out by the caller against a live herdr snapshot
    /// — this store cannot know that, so it never guesses. Rows written
    /// before the `feature` column existed carry SQL NULL and match no
    /// feature at all, which is the right answer: they were never board
    /// runs.
    pub fn list_live_runs_for_feature(&self, project_id: &str, feature: &str) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE project_id=?1 AND feature=?2 AND status='working'
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![project_id, feature], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Every still-`working` run waggledance itself spawned, across all
    /// projects — the reaper's sweep list (board-run-reaper D1,
    /// `eecfefeb`). Newest first.
    ///
    /// Two filters, both load-bearing:
    ///
    /// - `status='working'`: a run that already reached any terminal status
    ///   (`done`, `blocked`, `timeout`, `lost`) is finished business, and
    ///   re-awaiting it would re-read a pane that may belong to someone
    ///   else by now.
    /// - `preset_label IS NOT NULL`: waggledance spawned this pane. A
    ///   `DispatchTarget::Pane` run borrows a pane the user already owned
    ///   and leaves the column NULL exactly there — the same guard
    ///   `orchestrate::finish` reads before closing anything — so the
    ///   reaper never so much as looks at a human's own pane.
    ///
    /// Deliberately project-blind, unlike
    /// [`list_live_runs_for_feature`](Self::list_live_runs_for_feature):
    /// the leak D1 names is who-calls-await, not which project or which
    /// button dispatched, so the sweep covers board runs and abandoned MCP
    /// dispatches alike. Age is not filtered here — the caller's grace
    /// window is a policy the store has no business holding.
    pub fn list_unattended_working_runs(&self) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE status='working' AND preset_label IS NOT NULL
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_working_paseo_runs(&self) -> Result<Vec<Run>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,project_id,pane_id,preset_label,task,baseline,marker,status,created_at,updated_at
             FROM runs WHERE status='working' AND pane_id LIKE 'paseo:%'
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| Ok(row_to_run(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// The `feature` column of one run row, for the round-trip proof and for
    /// any caller that holds a [`Run`] and needs the column the struct does
    /// not carry. `Ok(None)` for both an unknown id and a NULL column.
    pub fn run_feature(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT feature FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, Option<String>>(0).ok())
            .flatten())
    }

    /// Record the transcript delta a run ended with.
    ///
    /// Written only once a run reaches a terminal status, so a non-NULL
    /// column means "this run is over, and this is what it left on screen"
    /// — which is what lets a second await answer from the ledger instead
    /// of re-reading a pane that may already have been closed.
    ///
    /// Column-only, like `feature`, and for the same reason: see [`Run`]'s
    /// own doc on why neither becomes a struct field.
    pub fn set_run_final_transcript(&self, id: &str, transcript: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET final_transcript=?2 WHERE id=?1",
            params![id, transcript],
        )?;
        Ok(())
    }

    /// The `final_transcript` column of one run row. `Ok(None)` for an
    /// unknown id, for a run that has not finished, and for every row
    /// written before the column existed — all three are honestly "no
    /// stored transcript", and the caller treats them alike.
    pub fn run_final_transcript(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT final_transcript FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, Option<String>>(0).ok())
            .flatten())
    }

    pub fn finish_run(
        &self,
        id: &str,
        status: &str,
        updated_at: &str,
        transcript: Option<&str>,
        notice: Option<(&str, &str)>,
    ) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET status=?2, updated_at=?3,
               final_transcript=COALESCE(?4, final_transcript),
               notice=COALESCE(?5, notice),
               notice_state=CASE WHEN ?5 IS NULL THEN notice_state ELSE 'pending' END,
               notice_until=COALESCE(?6, notice_until)
             WHERE id=?1",
            params![
                id,
                status,
                updated_at,
                transcript,
                notice.map(|(text, _)| text),
                notice.map(|(_, until)| until),
            ],
        )?;
        Ok(())
    }

    pub fn pending_notices(&self) -> Result<Vec<(String, String, String, String)>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, caller, notice, notice_until FROM runs
             WHERE notice_state='pending' AND caller IS NOT NULL AND notice IS NOT NULL
             ORDER BY updated_at ASC, rowid ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn set_notice_state(&self, run_id: &str, state: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET notice_state=?2 WHERE id=?1",
            params![run_id, state],
        )?;
        Ok(())
    }

    pub fn run_notice_state(&self, id: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT notice_state FROM runs WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows
            .next()?
            .and_then(|r| r.get::<_, Option<String>>(0).ok())
            .flatten())
    }

    pub fn insert_inbox_message(
        &self,
        recipient: &str,
        corr: &str,
        project_id: &str,
        kind: &str,
        body: &str,
        created_at: &str,
    ) -> Result<i64> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO inbox(recipient, corr, project_id, kind, body, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![recipient, corr, project_id, kind, body, created_at],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn next_inbox_message(&self, recipient: &str) -> Result<Option<InboxMessage>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT seq, recipient, corr, project_id, kind, body, created_at, acked_at,
                    attempts, dead_at
             FROM inbox
             WHERE recipient=?1 AND acked_at IS NULL AND dead_at IS NULL
             ORDER BY seq ASC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![recipient])?;
        Ok(rows.next()?.map(|r| InboxMessage {
            seq: r.get_unwrap(0),
            recipient: r.get_unwrap(1),
            corr: r.get_unwrap(2),
            project_id: r.get_unwrap(3),
            kind: r.get_unwrap(4),
            body: r.get_unwrap(5),
            created_at: r.get_unwrap(6),
            acked_at: r.get_unwrap(7),
            attempts: r.get_unwrap(8),
            dead_at: r.get_unwrap(9),
        }))
    }

    pub fn ack_inbox_message(&self, seq: i64, acked_at: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE inbox SET acked_at=?2 WHERE seq=?1",
            params![seq, acked_at],
        )?;
        Ok(())
    }

    pub fn fail_inbox_message(&self, seq: i64, dead_at_attempts: i64, now: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE inbox SET attempts=attempts+1,
                    dead_at=CASE WHEN attempts+1 >= ?2 THEN ?3 ELSE dead_at END
             WHERE seq=?1",
            params![seq, dead_at_attempts, now],
        )?;
        Ok(c.query_row(
            "SELECT dead_at IS NOT NULL FROM inbox WHERE seq=?1",
            params![seq],
            |r| r.get(0),
        )?)
    }

    pub fn prune_inbox(&self, acked_before: &str) -> Result<usize> {
        let c = self.conn.lock().unwrap();
        Ok(c.execute(
            "DELETE FROM inbox WHERE acked_at IS NOT NULL AND acked_at < ?1",
            params![acked_before],
        )?)
    }

    pub fn insert_task(&self, corr: &str, project_id: &str, at: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        Ok(c.execute(
            "INSERT OR IGNORE INTO tasks(corr, project_id, state, created_at, changed_at)
             VALUES(?1, ?2, 'open', ?3, ?3)",
            params![corr, project_id, at],
        )? == 1)
    }

    pub fn get_task(&self, corr: &str) -> Result<Option<WdTask>> {
        Ok(self.select_tasks("WHERE corr=?1", params![corr])?.pop())
    }

    pub fn list_open_tasks(&self) -> Result<Vec<WdTask>> {
        self.select_tasks(
            "WHERE state='open' ORDER BY created_at ASC, corr ASC",
            params![],
        )
    }

    fn select_tasks(&self, filter: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<WdTask>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(&format!(
            "SELECT corr, project_id, leader_id, state, created_at, last_delivered_at,
                    last_mail_at, nudged_at, progress_sent_at, changed_at, board_sent_at
             FROM tasks {filter}"
        ))?;
        let rows = stmt.query_map(args, |r| {
            Ok(WdTask {
                corr: r.get(0)?,
                project_id: r.get(1)?,
                leader_id: r.get(2)?,
                state: r.get(3)?,
                created_at: r.get(4)?,
                last_delivered_at: r.get(5)?,
                last_mail_at: r.get(6)?,
                nudged_at: r.get(7)?,
                progress_sent_at: r.get(8)?,
                changed_at: r.get(9)?,
                board_sent_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn update_task(&self, set: &str, corr: &str, args: &[&str]) -> Result<()> {
        let c = self.conn.lock().unwrap();
        let mut values: Vec<&dyn rusqlite::ToSql> = vec![&corr];
        values.extend(args.iter().map(|a| a as &dyn rusqlite::ToSql));
        c.execute(
            &format!("UPDATE tasks SET {set} WHERE corr=?1"),
            values.as_slice(),
        )?;
        Ok(())
    }

    pub fn mark_task_delivered(&self, corr: &str, leader_id: &str, at: &str) -> Result<()> {
        self.update_task(
            "leader_id=?2, last_delivered_at=?3, nudged_at=NULL, progress_sent_at=NULL,
             changed_at=?3",
            corr,
            &[leader_id, at],
        )
    }

    pub fn mark_task_mail(&self, corr: &str, at: &str) -> Result<()> {
        self.update_task("last_mail_at=?2, changed_at=?2", corr, &[at])
    }

    pub fn set_hold(&self, leader_id: &str, why: &str, set_at: &str, until: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO leader_holds(leader_id, why, set_at, until) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(leader_id) DO UPDATE SET why=?2, set_at=?3, until=?4",
            params![leader_id, why, set_at, until],
        )?;
        Ok(())
    }

    pub fn release_hold(&self, leader_id: &str) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        Ok(c.execute(
            "DELETE FROM leader_holds WHERE leader_id=?1",
            params![leader_id],
        )? == 1)
    }

    pub fn live_hold(&self, leader_id: &str, now: &str) -> Result<Option<LeaderHold>> {
        let c = self.conn.lock().unwrap();
        Ok(c.query_row(
            "SELECT leader_id, why, set_at, until FROM leader_holds
             WHERE leader_id=?1 AND until>?2",
            params![leader_id, now],
            |r| {
                Ok(LeaderHold {
                    leader_id: r.get(0)?,
                    why: r.get(1)?,
                    set_at: r.get(2)?,
                    until: r.get(3)?,
                })
            },
        )
        .optional()?)
    }

    pub fn stamp_late_reply(&self, run_id: &str, at: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE runs SET late_reply_at=?2 WHERE id=?1",
            params![run_id, at],
        )?;
        Ok(())
    }

    pub fn close_task(&self, corr: &str, at: &str) -> Result<()> {
        self.update_task("state='closed', changed_at=?2", corr, &[at])
    }

    pub fn set_task_nudged(&self, corr: &str, at: &str) -> Result<()> {
        self.update_task("nudged_at=?2", corr, &[at])
    }

    pub fn set_task_progress_sent(&self, corr: &str, at: &str) -> Result<()> {
        self.update_task("progress_sent_at=?2", corr, &[at])
    }

    pub fn stamp_board_sent(&self, at: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("UPDATE tasks SET board_sent_at=?1", params![at])?;
        Ok(())
    }

    pub fn insert_permission_request(&self, r: &PermissionRequest) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        Ok(c.execute(
            "INSERT OR IGNORE INTO permission_requests
                 (req_id, corr, leader_id, tool, description, seen_at, answered_at, closed_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                r.req_id,
                r.corr,
                r.leader_id,
                r.tool,
                r.description,
                r.seen_at,
                r.answered_at,
                r.closed_at
            ],
        )? == 1)
    }

    pub fn unanswered_permission_requests(&self, corr: &str) -> Result<Vec<PermissionRequest>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT req_id, corr, leader_id, tool, description, seen_at, answered_at, closed_at
             FROM permission_requests
             WHERE corr=?1 AND answered_at IS NULL AND closed_at IS NULL
             ORDER BY seen_at DESC, rowid DESC",
        )?;
        let rows = stmt.query_map(params![corr], |r| {
            Ok(PermissionRequest {
                req_id: r.get(0)?,
                corr: r.get(1)?,
                leader_id: r.get(2)?,
                tool: r.get(3)?,
                description: r.get(4)?,
                seen_at: r.get(5)?,
                answered_at: r.get(6)?,
                closed_at: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn answer_permission_request(&self, req_id: &str, at: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE permission_requests SET answered_at=?2 WHERE req_id=?1",
            params![req_id, at],
        )?;
        Ok(())
    }

    pub fn close_permission_request(&self, req_id: &str, at: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE permission_requests SET closed_at=?2 WHERE req_id=?1",
            params![req_id, at],
        )?;
        Ok(())
    }

    pub fn permission_request_ids(&self, corr: &str) -> Result<Vec<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT req_id FROM permission_requests WHERE corr=?1")?;
        let rows = stmt.query_map(params![corr], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn prune_tasks(&self, before: &str) -> Result<usize> {
        let c = self.conn.lock().unwrap();
        let tasks = c.execute(
            "DELETE FROM tasks WHERE state='closed' AND changed_at < ?1",
            params![before],
        )?;
        let requests = c.execute(
            "DELETE FROM permission_requests WHERE closed_at < ?1 OR answered_at < ?1",
            params![before],
        )?;
        Ok(tasks + requests)
    }

    // ---- files ----

    pub fn upsert_file(&self, f: &IndexedFile, content_hash: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO files(project_id,rel_path,abs_path,title,size_bytes,modified_at,path_hash,content_hash)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(project_id,rel_path) DO UPDATE SET
               abs_path=?3, title=?4, size_bytes=?5, modified_at=?6, path_hash=?7, content_hash=?8",
            params![
                f.project_id,
                f.rel_path,
                f.abs_path.to_string_lossy(),
                f.title,
                f.size_bytes as i64,
                f.modified_at,
                short_link::path_hash(&f.project_id, &f.rel_path),
                content_hash
            ],
        )?;
        Ok(())
    }

    pub fn delete_file(&self, project_id: &str, rel_path: &str) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "DELETE FROM files WHERE project_id=?1 AND rel_path=?2",
            params![project_id, rel_path],
        )?;
        c.execute(
            "DELETE FROM links WHERE project_id=?1 AND source_rel=?2",
            params![project_id, rel_path],
        )?;
        Ok(())
    }

    // ---- links / backlinks (FR-18) ----

    /// Replace the set of outgoing internal links for a source file.
    pub fn set_file_links(
        &self,
        project_id: &str,
        source_rel: &str,
        targets: &[String],
    ) -> Result<()> {
        let c = self.conn.lock().unwrap();
        let stored: HashSet<String> = {
            let mut stmt =
                c.prepare("SELECT target_rel FROM links WHERE project_id=?1 AND source_rel=?2")?;
            let rows = stmt.query_map(params![project_id, source_rel], |r| r.get(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        if stored.len() == targets.iter().collect::<HashSet<_>>().len()
            && targets.iter().all(|t| stored.contains(t))
        {
            return Ok(());
        }
        c.execute(
            "DELETE FROM links WHERE project_id=?1 AND source_rel=?2",
            params![project_id, source_rel],
        )?;
        for t in targets {
            c.execute(
                "INSERT OR IGNORE INTO links(project_id,source_rel,target_rel) VALUES(?1,?2,?3)",
                params![project_id, source_rel, t],
            )?;
        }
        Ok(())
    }

    /// Files that link *to* `target_rel` → (source_rel, title).
    pub fn backlinks(&self, project_id: &str, target_rel: &str) -> Result<Vec<(String, String)>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT l.source_rel, COALESCE(f.title, l.source_rel)
             FROM links l
             LEFT JOIN files f ON f.project_id = l.project_id AND f.rel_path = l.source_rel
             WHERE l.project_id = ?1 AND l.target_rel = ?2
             ORDER BY l.source_rel",
        )?;
        let rows = stmt.query_map(params![project_id, target_rel], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_file(&self, project_id: &str, rel_path: &str) -> Result<Option<IndexedFile>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT project_id,abs_path,rel_path,title,size_bytes,modified_at FROM files WHERE project_id=?1 AND rel_path=?2")?;
        let mut rows = stmt.query(params![project_id, rel_path])?;
        Ok(rows.next()?.map(row_to_file))
    }

    pub fn list_files(&self, project_id: &str) -> Result<Vec<IndexedFile>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT project_id,abs_path,rel_path,title,size_bytes,modified_at FROM files WHERE project_id=?1 ORDER BY rel_path")?;
        let rows = stmt.query_map(params![project_id], |r| Ok(row_to_file(r)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn file_hash(&self, project_id: &str, rel_path: &str) -> Result<Option<String>> {
        let c = self.conn.lock().unwrap();
        let mut stmt =
            c.prepare("SELECT content_hash FROM files WHERE project_id=?1 AND rel_path=?2")?;
        let mut rows = stmt.query(params![project_id, rel_path])?;
        Ok(rows.next()?.map(|r| r.get_unwrap::<_, String>(0)))
    }

    /// Absolute paths of every indexed file in a project — the link resolver index.
    pub fn file_abs_paths(&self, project_id: &str) -> Result<HashSet<PathBuf>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT abs_path FROM files WHERE project_id=?1")?;
        let rows = stmt.query_map(params![project_id], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).map(PathBuf::from).collect())
    }

    /// The file a short code points at, or `None` when nothing matches.
    ///
    /// The pattern is built in Rust and bound as one parameter. Concatenating in
    /// SQL (`path_hash GLOB ?1 || '*'`) returns the same rows but makes the
    /// right-hand side an expression, which disables SQLite's GLOB index
    /// optimisation and silently turns this into a full table scan — see
    /// `short_link::hash_prefix_pattern`.
    ///
    /// Two files sharing a 12-character prefix is ~1.8e-5 likely even at 100k
    /// files, so the tie-break only has to be *stable*, not clever: order by the
    /// primary key and take the first.
    pub fn find_by_hash_prefix(&self, code: &str) -> Result<Option<(String, String)>> {
        if code.is_empty() {
            return Ok(None);
        }
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT project_id, rel_path FROM files
             WHERE path_hash GLOB ?1
             ORDER BY project_id, rel_path
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![short_link::hash_prefix_pattern(code)])?;
        match rows.next()? {
            Some(r) => Ok(Some((r.get(0)?, r.get(1)?))),
            None => Ok(None),
        }
    }

    /// Query plan for [`find_by_hash_prefix`], so a test can prove it still uses
    /// the hash index rather than only proving it returns the right row.
    #[cfg(test)]
    fn hash_prefix_query_plan(&self, code: &str) -> Result<String> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "EXPLAIN QUERY PLAN
             SELECT project_id, rel_path FROM files
             WHERE path_hash GLOB ?1
             ORDER BY project_id, rel_path
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![short_link::hash_prefix_pattern(code)])?;
        let mut plan = String::new();
        while let Some(r) = rows.next()? {
            plan.push_str(&r.get::<_, String>(3)?);
            plan.push('\n');
        }
        Ok(plan)
    }

    pub fn file_count(&self, project_id: &str) -> Result<usize> {
        let c = self.conn.lock().unwrap();
        let n: i64 = c.query_row(
            "SELECT COUNT(*) FROM files WHERE project_id=?1",
            params![project_id],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }

    /// `(schema version, files still missing a short-link code)` — what `mdview
    /// doctor` reports so an operator can see whether an upgrade finished.
    pub fn schema_report(&self) -> Result<(i64, usize)> {
        let c = self.conn.lock().unwrap();
        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let unhashed: i64 =
            c.query_row("SELECT COUNT(*) FROM files WHERE path_hash=''", [], |r| {
                r.get(0)
            })?;
        Ok((version, unhashed as usize))
    }

    pub fn total_file_count(&self) -> Result<usize> {
        let c = self.conn.lock().unwrap();
        let n: i64 = c.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    // ---- search (title and path) ----

    pub fn search(
        &self,
        query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let terms: Vec<String> = query.split_whitespace().map(fold).collect();
        if terms.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(String, String, String)> = {
            let c = self.conn.lock().unwrap();
            let mut stmt = c.prepare(
                "SELECT project_id, rel_path, title FROM files WHERE (?1 IS NULL OR project_id = ?1)",
            )?;
            let rows = stmt.query_map(params![project_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let mut hits: Vec<(bool, String, String, String)> = rows
            .into_iter()
            .filter_map(|(project_id, rel_path, title)| {
                let folded_title = fold(&title);
                let folded_path = fold(&rel_path);
                let matched = terms
                    .iter()
                    .all(|t| folded_title.contains(t) || folded_path.contains(t));
                let in_title = terms.iter().all(|t| folded_title.contains(t));
                matched.then_some((!in_title, rel_path, project_id, title))
            })
            .collect();
        hits.sort();
        Ok(hits
            .into_iter()
            .take(limit)
            .map(|(_, rel_path, project_id, title)| SearchResult {
                url: format!("/p/{project_id}/{rel_path}"),
                project_id,
                rel_path,
                title,
                excerpt: String::new(),
                score: 0.0,
            })
            .collect())
    }

    pub fn reclaim_legacy_index(&self) -> Result<bool> {
        let c = self.conn.lock().unwrap();
        let populated: bool =
            c.query_row("SELECT EXISTS(SELECT 1 FROM files_fts_content)", [], |r| {
                r.get(0)
            })?;
        if !populated {
            return Ok(false);
        }
        for table in ["files_fts_content", "files_fts_docsize", "files_fts_data"] {
            let sql =
                format!("DELETE FROM {table} WHERE rowid IN (SELECT rowid FROM {table} LIMIT 200)");
            loop {
                c.execute_batch("BEGIN IMMEDIATE")?;
                let deleted = match c.execute(&sql, []) {
                    Ok(n) => n,
                    Err(e) => {
                        c.execute_batch("ROLLBACK").ok();
                        return Err(e.into());
                    }
                };
                c.execute_batch("COMMIT")?;
                if deleted == 0 {
                    break;
                }
            }
        }
        c.execute_batch(
            "DROP TABLE files_fts;
             CREATE VIRTUAL TABLE files_fts USING fts5(
                 project_id UNINDEXED,
                 rel_path UNINDEXED,
                 title,
                 content
             );
             VACUUM;",
        )?;
        c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(true)
    }
}

/// Ordered, append-only migration steps.
///
/// `SCHEMA` above only ever runs `CREATE TABLE IF NOT EXISTS`, so a database
/// created by an older build keeps its old columns forever — anything new has to
/// be added here instead. To add a migration, append one entry; never edit or
/// reorder an existing one, because databases in the field have already run it.
///
/// This is SQLite's own `PRAGMA user_version` convention, which is also what
/// crates like `rusqlite_migration` implement underneath. With a single step,
/// that crate would only wrap this list, so the dependency is not earned yet;
/// the shape here is deliberately the one it expects, so adopting it later is a
/// mechanical swap rather than a redesign.
type MigrationStep = (i64, fn(&Connection) -> Result<()>);
const MIGRATIONS: &[MigrationStep] = &[
    (1, migration_1_path_hash),
    (2, migration_2_orchestration_enabled),
    (3, migration_3_runs_feature),
    (4, migration_4_runs_final_transcript),
    (5, migration_5_runs_request),
    (6, migration_6_runs_notice),
    (7, migration_7_inbox),
    (8, migration_8_tasks),
    (9, migration_9_runs_session),
    (10, migration_10_runs_follow_up),
    (11, migration_11_runs_handoff),
    (12, migration_12_leader_holds),
    (13, migration_13_content_hash),
];

/// Schema version this build expects — the last entry in [`MIGRATIONS`].
pub const SCHEMA_VERSION: i64 = 13;

/// Bring an existing database up to [`SCHEMA_VERSION`].
///
/// Every step is additive (no row is dropped or rewritten) and stamps
/// `user_version` as soon as it finishes, so a run interrupted halfway resumes at
/// the next unfinished step instead of redoing completed ones, and running this
/// against an up-to-date database is a no-op.
fn migrate(conn: &Connection) -> Result<()> {
    let mut version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (target, step) in MIGRATIONS {
        if version >= *target {
            continue;
        }
        step(conn)?;
        conn.pragma_update(None, "user_version", target)?;
        version = *target;
    }
    Ok(())
}

/// v1 — short-link support. `path_hash` lets `/s/<code>` find a file without a
/// separate shortlink table, so a code's lifetime is its index row's lifetime.
fn migration_1_path_hash(conn: &Connection) -> Result<()> {
    // A database created by this build already has the column from SCHEMA; one
    // created by an older build does not, because `CREATE TABLE IF NOT EXISTS`
    // leaves an existing table alone. The index has to be created here rather
    // than in SCHEMA for the same reason: on a legacy database SCHEMA runs
    // first, while the column still does not exist.
    if !has_column(conn, "files", "path_hash")? {
        conn.execute(
            "ALTER TABLE files ADD COLUMN path_hash TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_files_hash ON files(path_hash)",
        [],
    )?;
    backfill_path_hash(conn)
}

/// v13 (registry-db-lock D3) — content hash for change detection. Not
/// backfilled: the migration reads no file, so the first change event for a
/// pre-upgrade file reports a change once.
fn migration_13_content_hash(conn: &Connection) -> Result<()> {
    if !has_column(conn, "files", "content_hash")? {
        conn.execute(
            "ALTER TABLE files ADD COLUMN content_hash TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    Ok(())
}

/// v2 (D6) — per-project orchestrator-dispatch opt-in. `runs` (D7) is a
/// brand-new table, so `SCHEMA`'s `CREATE TABLE IF NOT EXISTS` already covers
/// both fresh and legacy databases without a migration step; only the new
/// `projects` column needs one, for the same reason `path_hash` did.
fn migration_2_orchestration_enabled(conn: &Connection) -> Result<()> {
    if !has_column(conn, "projects", "orchestration_enabled")? {
        conn.execute(
            "ALTER TABLE projects ADD COLUMN orchestration_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

/// v3 (board-run-actions D3) — which bee feature a run was started for. The
/// per-feature run lock reads it, so it has to be a column the store can
/// filter on rather than something inferred from the task text. Nullable
/// with no default: a run that is not a board run genuinely has no feature,
/// and every row written before this column existed is exactly that.
fn migration_3_runs_feature(conn: &Connection) -> Result<()> {
    if !has_column(conn, "runs", "feature")? {
        conn.execute("ALTER TABLE runs ADD COLUMN feature TEXT", [])?;
    }
    Ok(())
}

/// v4 (dispatch-submit-and-reclaim P2-4) — the transcript a run finished
/// with. Once a finished run's pane can be closed, the pane is no longer
/// there to re-read, so the answer a repeat await gets has to be one the
/// store already holds. Nullable with no default, for the same reason
/// `feature` is: a run still working genuinely has no final transcript, and
/// so does every row written before this column existed.
fn migration_4_runs_final_transcript(conn: &Connection) -> Result<()> {
    if !has_column(conn, "runs", "final_transcript")? {
        conn.execute("ALTER TABLE runs ADD COLUMN final_transcript TEXT", [])?;
    }
    Ok(())
}

fn migration_5_runs_request(conn: &Connection) -> Result<()> {
    if !has_column(conn, "runs", "request_id")? {
        conn.execute("ALTER TABLE runs ADD COLUMN request_id TEXT", [])?;
    }
    if !has_column(conn, "runs", "caller")? {
        conn.execute("ALTER TABLE runs ADD COLUMN caller TEXT", [])?;
    }
    if !has_column(conn, "runs", "attempts")? {
        conn.execute(
            "ALTER TABLE runs ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_runs_request ON runs(project_id, request_id)
         WHERE request_id IS NOT NULL",
        [],
    )?;
    Ok(())
}

fn migration_6_runs_notice(conn: &Connection) -> Result<()> {
    if !has_column(conn, "runs", "notice")? {
        conn.execute("ALTER TABLE runs ADD COLUMN notice TEXT", [])?;
    }
    if !has_column(conn, "runs", "notice_state")? {
        conn.execute("ALTER TABLE runs ADD COLUMN notice_state TEXT", [])?;
    }
    if !has_column(conn, "runs", "notice_until")? {
        conn.execute("ALTER TABLE runs ADD COLUMN notice_until TEXT", [])?;
    }
    Ok(())
}

fn migration_7_inbox(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS inbox (
             seq INTEGER PRIMARY KEY,
             recipient TEXT NOT NULL,
             corr TEXT NOT NULL,
             project_id TEXT NOT NULL,
             kind TEXT NOT NULL,
             body TEXT NOT NULL,
             created_at TEXT NOT NULL,
             acked_at TEXT,
             attempts INTEGER NOT NULL DEFAULT 0,
             dead_at TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_inbox_pending ON inbox(recipient, acked_at, dead_at, seq);",
    )?;
    Ok(())
}

fn migration_9_runs_session(conn: &Connection) -> Result<()> {
    let runs: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='runs'",
        [],
        |r| r.get(0),
    )?;
    if runs == 1 && !has_column(conn, "runs", "session_id")? {
        conn.execute("ALTER TABLE runs ADD COLUMN session_id TEXT", [])?;
    }
    Ok(())
}

fn migration_10_runs_follow_up(conn: &Connection) -> Result<()> {
    let runs: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='runs'",
        [],
        |r| r.get(0),
    )?;
    if runs == 1 && !has_column(conn, "runs", "follow_up_of")? {
        conn.execute("ALTER TABLE runs ADD COLUMN follow_up_of TEXT", [])?;
    }
    Ok(())
}

fn migration_11_runs_handoff(conn: &Connection) -> Result<()> {
    let runs: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='runs'",
        [],
        |r| r.get(0),
    )?;
    if runs == 1 && !has_column(conn, "runs", "handoff")? {
        conn.execute(
            "ALTER TABLE runs ADD COLUMN handoff INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

fn migration_12_leader_holds(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS leader_holds (
             leader_id TEXT PRIMARY KEY,
             why TEXT NOT NULL,
             set_at TEXT NOT NULL,
             until TEXT NOT NULL
         );",
    )?;
    let runs: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='runs'",
        [],
        |r| r.get(0),
    )?;
    if runs == 1 && !has_column(conn, "runs", "late_reply_at")? {
        conn.execute("ALTER TABLE runs ADD COLUMN late_reply_at TEXT", [])?;
    }
    Ok(())
}

fn migration_8_tasks(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
             corr TEXT PRIMARY KEY,
             project_id TEXT NOT NULL,
             leader_id TEXT,
             state TEXT NOT NULL,
             created_at TEXT NOT NULL,
             last_delivered_at TEXT,
             last_mail_at TEXT,
             nudged_at TEXT,
             progress_sent_at TEXT,
             changed_at TEXT NOT NULL,
             board_sent_at TEXT
         );
         CREATE TABLE IF NOT EXISTS permission_requests (
             req_id TEXT PRIMARY KEY,
             corr TEXT NOT NULL,
             leader_id TEXT NOT NULL,
             tool TEXT NOT NULL,
             description TEXT NOT NULL,
             seen_at TEXT NOT NULL,
             answered_at TEXT,
             closed_at TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_permission_requests_corr ON permission_requests(corr);",
    )?;
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        if r.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Fill `path_hash` for every row still carrying the empty default.
///
/// Scoped to empty values rather than rewriting the whole table so an
/// interrupted run costs only what it did not finish, and so a re-run after a
/// crash is cheap rather than a full rewrite of 15k+ rows.
fn backfill_path_hash(conn: &Connection) -> Result<()> {
    let pending: Vec<(String, String)> = {
        let mut stmt =
            conn.prepare("SELECT project_id, rel_path FROM files WHERE path_hash = ''")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    if pending.is_empty() {
        return Ok(());
    }
    conn.execute_batch("BEGIN")?;
    for (project_id, rel_path) in &pending {
        conn.execute(
            "UPDATE files SET path_hash=?3 WHERE project_id=?1 AND rel_path=?2",
            params![
                project_id,
                rel_path,
                short_link::path_hash(project_id, rel_path)
            ],
        )?;
    }
    conn.execute_batch("COMMIT")?;
    Ok(())
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    orchestration_enabled INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS files (
    project_id TEXT NOT NULL,
    rel_path TEXT NOT NULL,
    abs_path TEXT NOT NULL,
    title TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    modified_at TEXT NOT NULL,
    path_hash TEXT NOT NULL DEFAULT '',
    PRIMARY KEY(project_id, rel_path)
);
CREATE INDEX IF NOT EXISTS idx_files_project ON files(project_id);
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    project_id UNINDEXED,
    rel_path UNINDEXED,
    title,
    content
);
CREATE TABLE IF NOT EXISTS links (
    project_id TEXT NOT NULL,
    source_rel TEXT NOT NULL,
    target_rel TEXT NOT NULL,
    PRIMARY KEY(project_id, source_rel, target_rel)
);
CREATE INDEX IF NOT EXISTS idx_links_target ON links(project_id, target_rel);
CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    pane_id TEXT NOT NULL,
    preset_label TEXT,
    task TEXT NOT NULL,
    baseline TEXT NOT NULL,
    marker TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    feature TEXT,
    final_transcript TEXT,
    request_id TEXT,
    caller TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    notice TEXT,
    notice_state TEXT,
    notice_until TEXT
);
CREATE INDEX IF NOT EXISTS idx_runs_project ON runs(project_id, created_at);
"#;

fn row_to_project(r: &rusqlite::Row) -> Project {
    Project {
        id: r.get_unwrap(0),
        name: r.get_unwrap(1),
        root_path: PathBuf::from(r.get_unwrap::<_, String>(2)),
        created_at: r.get_unwrap(3),
        last_seen_at: r.get_unwrap(4),
        orchestration_enabled: r.get_unwrap(5),
    }
}

fn row_to_run(r: &rusqlite::Row) -> Run {
    Run {
        id: r.get_unwrap(0),
        project_id: r.get_unwrap(1),
        pane_id: r.get_unwrap(2),
        preset_label: r.get_unwrap(3),
        task: r.get_unwrap(4),
        baseline: r.get_unwrap(5),
        marker: r.get_unwrap(6),
        status: r.get_unwrap(7),
        created_at: r.get_unwrap(8),
        updated_at: r.get_unwrap(9),
    }
}

fn row_to_file(r: &rusqlite::Row) -> IndexedFile {
    IndexedFile {
        project_id: r.get_unwrap(0),
        abs_path: PathBuf::from(r.get_unwrap::<_, String>(1)),
        rel_path: r.get_unwrap(2),
        title: r.get_unwrap(3),
        size_bytes: r.get_unwrap::<_, i64>(4) as u64,
        modified_at: r.get_unwrap(5),
    }
}

fn fold(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(|c| c.to_lowercase())
        .map(|c| if c == 'đ' { 'd' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{IndexedFile, Project, Run};

    fn sample_project() -> Project {
        Project {
            id: "p1".into(),
            name: "P1".into(),
            root_path: PathBuf::from("/proj"),
            created_at: "2026-07-15T00:00:00Z".into(),
            last_seen_at: "2026-07-15T00:00:00Z".into(),
            orchestration_enabled: false,
        }
    }

    fn sample_run() -> Run {
        Run {
            id: "r1".into(),
            project_id: "p1".into(),
            pane_id: "pane-1".into(),
            preset_label: Some("claude".into()),
            task: "do the thing".into(),
            baseline: "baseline text".into(),
            marker: "HERDR_DONE_abc123".into(),
            status: "pending".into(),
            created_at: "2026-08-16T00:00:00Z".into(),
            updated_at: "2026-08-16T00:00:00Z".into(),
        }
    }

    fn file(rel: &str, title: &str) -> IndexedFile {
        IndexedFile {
            project_id: "p1".into(),
            abs_path: PathBuf::from("/proj").join(rel),
            rel_path: rel.into(),
            title: title.into(),
            size_bytes: 10,
            modified_at: "2026-07-15T00:00:00Z".into(),
        }
    }

    /// A database as an older build left it: no `path_hash`, no `user_version`.
    fn legacy_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (
                 project_id TEXT NOT NULL,
                 rel_path TEXT NOT NULL,
                 abs_path TEXT NOT NULL,
                 title TEXT NOT NULL,
                 size_bytes INTEGER NOT NULL,
                 modified_at TEXT NOT NULL,
                 PRIMARY KEY(project_id, rel_path)
             );
             INSERT INTO files VALUES('mdview','docs/a.md','/x/docs/a.md','A',1,'t');
             INSERT INTO files VALUES('mdview','README.md','/x/README.md','R',1,'t');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migrate_backfills_a_legacy_database() {
        let store = SqliteStore::from_conn(legacy_conn()).unwrap();
        let c = store.conn.lock().unwrap();

        let version: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        let hash: String = c
            .query_row(
                "SELECT path_hash FROM files WHERE rel_path='docs/a.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hash, short_link::path_hash("mdview", "docs/a.md"));

        let unfilled: i64 = c
            .query_row("SELECT COUNT(*) FROM files WHERE path_hash=''", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(unfilled, 0, "every legacy row must be backfilled");
    }

    #[test]
    fn migrate_is_idempotent() {
        let store = SqliteStore::from_conn(legacy_conn()).unwrap();
        {
            let c = store.conn.lock().unwrap();
            // Second pass over an already-migrated database must change nothing
            // and must not fail on the column already existing.
            migrate(&c).unwrap();
            let version: i64 = c
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version, SCHEMA_VERSION);
        }
        assert_eq!(
            store
                .find_by_hash_prefix(&short_link::short_code(&short_link::path_hash(
                    "mdview",
                    "docs/a.md"
                )))
                .unwrap(),
            Some(("mdview".into(), "docs/a.md".into()))
        );
    }

    #[test]
    fn a_fresh_database_needs_no_alter_table() {
        // SCHEMA already carries path_hash, so migrate must recognise that and
        // still stamp the version rather than trying to add the column again.
        let store = SqliteStore::open_in_memory().unwrap();
        let c = store.conn.lock().unwrap();
        let version: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn upsert_file_records_the_path_hash() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Alpha"), "alpha").unwrap();

        let code = short_link::short_code(&short_link::path_hash("p1", "docs/a.md"));
        assert_eq!(
            s.find_by_hash_prefix(&code).unwrap(),
            Some(("p1".into(), "docs/a.md".into()))
        );
    }

    #[test]
    fn re_indexing_keeps_the_same_hash() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Alpha"), "first").unwrap();
        let code = short_link::short_code(&short_link::path_hash("p1", "docs/a.md"));

        // Same path, new content/title — the link handed out earlier must survive.
        let mut changed = file("docs/a.md", "Alpha v2");
        changed.size_bytes = 999;
        s.upsert_file(&changed, "second").unwrap();

        assert_eq!(
            s.find_by_hash_prefix(&code).unwrap(),
            Some(("p1".into(), "docs/a.md".into()))
        );
    }

    #[test]
    fn unknown_code_resolves_to_nothing() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Alpha"), "alpha").unwrap();

        assert_eq!(s.find_by_hash_prefix("ffffffffffff").unwrap(), None);
        assert_eq!(s.find_by_hash_prefix("").unwrap(), None);
    }

    /// Regression guard with teeth: a functional test passes whether or not the
    /// query uses the index, because both forms return the same rows. Only the
    /// query plan distinguishes the fast path from a silent full scan.
    #[test]
    fn prefix_lookup_uses_the_hash_index() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        for i in 0..200 {
            s.upsert_file(&file(&format!("docs/f{i}.md"), "T"), "body")
                .unwrap();
        }
        let plan = s.hash_prefix_query_plan("a3f9c1d20b74").unwrap();
        assert!(
            plan.contains("idx_files_hash"),
            "prefix lookup must hit idx_files_hash, got plan: {plan}"
        );
    }

    #[test]
    fn project_and_file_roundtrip() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Alpha"), "alpha content here")
            .unwrap();
        s.upsert_file(&file("src/b.md", "Beta"), "beta words")
            .unwrap();

        assert_eq!(s.file_count("p1").unwrap(), 2);
        assert_eq!(
            s.get_file("p1", "docs/a.md").unwrap().unwrap().title,
            "Alpha"
        );
        assert!(s
            .file_abs_paths("p1")
            .unwrap()
            .contains(&PathBuf::from("/proj/docs/a.md")));

        let found = s.find_project_by_root(Path::new("/proj")).unwrap();
        assert_eq!(found.unwrap().id, "p1");
    }

    #[test]
    fn delete_file_removes_the_row_and_its_search_hit() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/unique-token.md", "Alpha"), "h1")
            .unwrap();
        assert_eq!(s.search("unique-token", Some("p1"), 10).unwrap().len(), 1);
        s.delete_file("p1", "docs/unique-token.md").unwrap();
        assert_eq!(s.file_count("p1").unwrap(), 0);
        assert_eq!(s.search("unique-token", Some("p1"), 10).unwrap().len(), 0);
    }

    #[test]
    fn file_hash_reads_back_the_stored_hash_and_none_when_absent() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        assert_eq!(s.file_hash("p1", "docs/a.md").unwrap(), None);

        s.upsert_file(&file("docs/a.md", "Alpha"), "first").unwrap();
        assert_eq!(
            s.file_hash("p1", "docs/a.md").unwrap(),
            Some("first".to_string())
        );
        s.upsert_file(&file("docs/a.md", "Alpha"), "second")
            .unwrap();
        assert_eq!(
            s.file_hash("p1", "docs/a.md").unwrap(),
            Some("second".to_string())
        );
    }

    #[test]
    fn search_matches_titles_and_paths_with_every_term() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Deployment Guide"), "h")
            .unwrap();
        s.upsert_file(&file("ops/runbook.md", "Other"), "h")
            .unwrap();

        let by_title = s.search("DEPLOY guide", Some("p1"), 10).unwrap();
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].rel_path, "docs/a.md");
        assert_eq!(by_title[0].url, "/p/p1/docs/a.md");
        assert_eq!(by_title[0].excerpt, "");

        let by_path = s.search("runbook", None, 10).unwrap();
        assert_eq!(by_path.len(), 1);
        assert_eq!(by_path[0].rel_path, "ops/runbook.md");

        assert_eq!(s.search("deploy docs", None, 10).unwrap().len(), 1);
        assert!(s.search("deploy runbook", None, 10).unwrap().is_empty());
        assert!(s.search("deploy missing", None, 10).unwrap().is_empty());
        assert!(s.search("   ", None, 10).unwrap().is_empty());
    }

    #[test]
    fn search_ranks_title_hits_before_path_only_hits_and_filters_by_project() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("a/notes.md", "Other"), "h").unwrap();
        s.upsert_file(&file("z/x.md", "Notes"), "h").unwrap();
        let mut elsewhere = file("m/notes.md", "Notes");
        elsewhere.project_id = "p2".into();
        s.upsert_file(&elsewhere, "h").unwrap();

        let hits = s.search("notes", Some("p1"), 10).unwrap();
        let order: Vec<&str> = hits.iter().map(|h| h.rel_path.as_str()).collect();
        assert_eq!(order, vec!["z/x.md", "a/notes.md"]);
        assert_eq!(s.search("notes", None, 10).unwrap().len(), 3);
    }

    #[test]
    fn search_folds_diacritics_and_case() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/spec.md", "Đặc tả"), "h").unwrap();

        let hits = s.search("dac ta", Some("p1"), 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Đặc tả");
        assert_eq!(s.search("ĐẶC", None, 10).unwrap().len(), 1);
    }

    #[test]
    fn set_file_links_writes_only_when_the_targets_differ() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.set_file_links("p1", "a.md", &["b.md".into(), "c.md".into()])
            .unwrap();
        let rowid = |s: &SqliteStore| -> Vec<i64> {
            let c = s.conn.lock().unwrap();
            let mut stmt = c
                .prepare("SELECT rowid FROM links WHERE source_rel='a.md' ORDER BY target_rel")
                .unwrap();
            let rows = stmt.query_map([], |r| r.get(0)).unwrap();
            rows.map(|r| r.unwrap()).collect()
        };
        let before = rowid(&s);
        s.set_file_links("p1", "a.md", &["c.md".into(), "b.md".into()])
            .unwrap();
        assert_eq!(rowid(&s), before);

        s.set_file_links("p1", "a.md", &["b.md".into()]).unwrap();
        assert_eq!(s.backlinks("p1", "c.md").unwrap().len(), 0);
        assert_eq!(s.backlinks("p1", "b.md").unwrap().len(), 1);
    }

    #[test]
    fn reclaim_legacy_index_empties_the_old_table_and_keeps_the_rest() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Alpha"), "h").unwrap();
        s.insert_run(&sample_run(), None).unwrap();
        {
            let c = s.conn.lock().unwrap();
            for i in 0..450 {
                c.execute(
                    "INSERT INTO files_fts(project_id,rel_path,title,content) VALUES('p1',?1,'T','body')",
                    params![format!("docs/f{i}.md")],
                )
                .unwrap();
            }
        }
        let fts_rows = |s: &SqliteStore| -> i64 {
            let c = s.conn.lock().unwrap();
            c.query_row("SELECT COUNT(*) FROM files_fts", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(fts_rows(&s), 450);

        assert!(s.reclaim_legacy_index().unwrap());
        assert_eq!(fts_rows(&s), 0);
        assert_eq!(s.file_count("p1").unwrap(), 1);
        assert!(s.get_run("r1").unwrap().is_some());
        assert!(!s.reclaim_legacy_index().unwrap());
    }

    #[test]
    fn migration_13_adds_content_hash_without_reading_files() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.upsert_file(&file("docs/a.md", "Alpha"), "").unwrap();
        let c = s.conn.lock().unwrap();
        assert!(has_column(&c, "files", "content_hash").unwrap());
        assert_eq!(SCHEMA_VERSION, 13);
    }

    #[test]
    fn run_roundtrips_through_insert_get_list() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        let run = sample_run();
        s.insert_run(&run, None).unwrap();

        let got = s.get_run("r1").unwrap().unwrap();
        assert_eq!(got.id, run.id);
        assert_eq!(got.project_id, run.project_id);
        assert_eq!(got.pane_id, run.pane_id);
        assert_eq!(got.preset_label, run.preset_label);
        assert_eq!(got.task, run.task);
        assert_eq!(got.baseline, run.baseline);
        assert_eq!(got.marker, run.marker);
        assert_eq!(got.status, run.status);
        assert_eq!(got.created_at, run.created_at);
        assert_eq!(got.updated_at, run.updated_at);

        let listed = s.list_runs("p1", 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "r1");
    }

    #[test]
    fn list_runs_respects_project_scope_and_limit() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        let mut other = sample_project();
        other.id = "p2".into();
        s.upsert_project(&other).unwrap();

        for i in 0..3 {
            let mut r = sample_run();
            r.id = format!("r{i}");
            r.created_at = format!("2026-08-16T00:0{i}:00Z");
            s.insert_run(&r, None).unwrap();
        }
        let mut other_run = sample_run();
        other_run.id = "r-other".into();
        other_run.project_id = "p2".into();
        s.insert_run(&other_run, None).unwrap();

        assert_eq!(s.list_runs("p1", 10).unwrap().len(), 3);
        assert_eq!(s.list_runs("p1", 2).unwrap().len(), 2);
        assert_eq!(s.list_runs("p2", 10).unwrap().len(), 1);
    }

    #[test]
    fn update_run_status_bumps_status_and_timestamp_and_can_replace_baseline_marker() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.insert_run(&sample_run(), None).unwrap();

        s.update_run_status("r1", "working", "2026-08-16T00:05:00Z", None, None)
            .unwrap();
        let got = s.get_run("r1").unwrap().unwrap();
        assert_eq!(got.status, "working");
        assert_eq!(got.updated_at, "2026-08-16T00:05:00Z");
        // Not passed — baseline/marker survive untouched.
        assert_eq!(got.baseline, "baseline text");
        assert_eq!(got.marker, "HERDR_DONE_abc123");

        s.update_run_status(
            "r1",
            "pending",
            "2026-08-16T00:10:00Z",
            Some("new baseline"),
            Some("HERDR_DONE_xyz789"),
        )
        .unwrap();
        let got = s.get_run("r1").unwrap().unwrap();
        assert_eq!(got.baseline, "new baseline");
        assert_eq!(got.marker, "HERDR_DONE_xyz789");
    }

    /// board-run-actions D3: a run remembers which feature it was started
    /// for, and the lock's own query reads exactly the rows that still hold
    /// it — this feature's `working` rows and nothing else. A run for
    /// another feature, a finished run, and a featureless (MCP) run all fail
    /// to lock, which is the whole point of filtering here rather than in
    /// the caller.
    #[test]
    fn a_runs_feature_round_trips_and_only_working_rows_hold_the_lock() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();

        let mut working = sample_run();
        working.id = "r-working".into();
        working.status = "working".into();
        s.insert_run(&working, Some("board-run-actions")).unwrap();

        let mut done = sample_run();
        done.id = "r-done".into();
        done.status = "done".into();
        s.insert_run(&done, Some("board-run-actions")).unwrap();

        let mut other = sample_run();
        other.id = "r-other-feature".into();
        other.status = "working".into();
        s.insert_run(&other, Some("something-else")).unwrap();

        let mut featureless = sample_run();
        featureless.id = "r-mcp".into();
        featureless.status = "working".into();
        s.insert_run(&featureless, None).unwrap();

        assert_eq!(
            s.run_feature("r-working").unwrap().as_deref(),
            Some("board-run-actions"),
            "the feature a run was started for must survive the round trip"
        );
        assert_eq!(
            s.run_feature("r-mcp").unwrap(),
            None,
            "a run started with no feature reads back with none"
        );

        let live = s
            .list_live_runs_for_feature("p1", "board-run-actions")
            .unwrap();
        assert_eq!(
            live.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["r-working"],
            "only this feature's still-working run holds the lock: {live:?}"
        );
        assert!(
            s.list_live_runs_for_feature("p2", "board-run-actions")
                .unwrap()
                .is_empty(),
            "the lock is per project as well as per feature"
        );
    }

    /// board-run-reaper D1: the reaper's sweep list is every `working` run
    /// waggledance itself spawned, in every project. The four rows this test
    /// keeps out are the whole contract — a finished run, a reaped (`lost`)
    /// run, and a run in a pane the user already owned (NULL
    /// `preset_label`, the same guard `finish` reads before closing
    /// anything) must never reach a sweep, while a second project's working
    /// run must.
    #[test]
    fn the_sweep_list_is_every_waggledance_spawned_working_run_in_every_project() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();

        let mut working = sample_run();
        working.id = "r-working".into();
        working.status = "working".into();
        working.created_at = "2026-08-16T00:00:00Z".into();
        s.insert_run(&working, Some("board-run-reaper")).unwrap();

        // Another project entirely, and newer — the sweep is project-blind
        // and newest-first.
        let mut other_project = sample_run();
        other_project.id = "r-other-project".into();
        other_project.project_id = "p2".into();
        other_project.status = "working".into();
        other_project.created_at = "2026-08-17T00:00:00Z".into();
        s.insert_run(&other_project, None).unwrap();

        let mut done = sample_run();
        done.id = "r-done".into();
        done.status = "done".into();
        s.insert_run(&done, Some("board-run-reaper")).unwrap();

        let mut lost = sample_run();
        lost.id = "r-lost".into();
        lost.status = "lost".into();
        s.insert_run(&lost, Some("board-run-reaper")).unwrap();

        // Dispatched into a pane the user already owned: no preset label,
        // so nothing waggledance spawned, so not the reaper's business.
        let mut borrowed = sample_run();
        borrowed.id = "r-borrowed-pane".into();
        borrowed.status = "working".into();
        borrowed.preset_label = None;
        s.insert_run(&borrowed, Some("board-run-reaper")).unwrap();

        let sweep = s.list_unattended_working_runs().unwrap();
        assert_eq!(
            sweep.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["r-other-project", "r-working"],
            "only preset-labeled working rows, newest first: {sweep:?}"
        );

        // And a run the reaper caps leaves the list on the next sweep.
        s.update_run_status("r-working", "lost", "2026-08-18T00:00:00Z", None, None)
            .unwrap();
        let sweep = s.list_unattended_working_runs().unwrap();
        assert_eq!(
            sweep.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["r-other-project"],
            "a reaped run is not swept twice: {sweep:?}"
        );
    }

    #[test]
    fn working_paseo_runs_are_listed_apart_from_the_herdr_sweep() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        for (id, pane, status) in [
            ("r-paseo", "paseo:agent-1", "working"),
            ("r-paseo-done", "paseo:agent-2", "done"),
            ("r-herdr", "w1:p1", "working"),
        ] {
            let mut run = sample_run();
            run.id = id.into();
            run.pane_id = pane.into();
            run.status = status.into();
            run.preset_label = None;
            s.insert_run(&run, None).unwrap();
        }
        let paseo = s.list_working_paseo_runs().unwrap();
        assert_eq!(
            paseo.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["r-paseo"]
        );
        assert!(s.list_unattended_working_runs().unwrap().is_empty());
    }

    #[test]
    fn leader_waits_are_blocked_paseo_runs_and_only_recent_lost_ones() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        for (id, pane, status, updated_at) in [
            (
                "r-blocked",
                "paseo:agent-1",
                "blocked",
                "2026-10-16T09:00:00Z",
            ),
            (
                "r-lost-inside",
                "paseo:agent-2",
                "lost",
                "2026-10-16T10:30:00Z",
            ),
            (
                "r-lost-outside",
                "paseo:agent-3",
                "lost",
                "2026-10-16T08:00:00Z",
            ),
            (
                "r-working",
                "paseo:agent-4",
                "working",
                "2026-10-16T10:40:00Z",
            ),
            (
                "r-herdr-blocked",
                "w1:p1",
                "blocked",
                "2026-10-16T10:40:00Z",
            ),
        ] {
            let mut run = sample_run();
            run.id = id.into();
            run.pane_id = pane.into();
            run.status = status.into();
            run.updated_at = updated_at.into();
            s.insert_run(&run, None).unwrap();
        }
        let waits = s.list_leader_waits("2026-10-16T10:00:00Z").unwrap();
        assert_eq!(
            waits.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["r-lost-inside", "r-blocked"],
            "blocked always counts, lost only inside the window, and neither a working nor a \
             herdr-pane run is a leader wait: {waits:?}"
        );
    }

    /// A database whose `runs` table predates the `feature` column: SCHEMA's
    /// `CREATE TABLE IF NOT EXISTS` leaves it exactly as it was, so the
    /// migration step is the only thing that can add the column. The old row
    /// survives and reads back with no feature, which is true — it was never
    /// a board run.
    #[test]
    fn migrate_adds_the_runs_feature_column_to_an_older_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE runs (
                 id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 pane_id TEXT NOT NULL,
                 preset_label TEXT,
                 task TEXT NOT NULL,
                 baseline TEXT NOT NULL,
                 marker TEXT NOT NULL,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             INSERT INTO runs VALUES('r-old','p1','pane-1',NULL,'t','','m','working','t','t');",
        )
        .unwrap();

        let s = SqliteStore::from_conn(conn).unwrap();
        {
            let c = s.conn.lock().unwrap();
            let version: i64 = c
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version, SCHEMA_VERSION);
            assert!(
                has_column(&c, "runs", "feature").unwrap(),
                "the migration must add the column an older build never had"
            );
        }

        assert_eq!(
            s.get_run("r-old").unwrap().unwrap().status,
            "working",
            "the legacy row survives the migration"
        );
        assert_eq!(s.run_feature("r-old").unwrap(), None);
        assert!(
            s.list_live_runs_for_feature("p1", "anything")
                .unwrap()
                .is_empty(),
            "a legacy run locks no feature"
        );

        let mut fresh = sample_run();
        fresh.id = "r-new".into();
        fresh.project_id = "p1".into();
        fresh.status = "working".into();
        s.insert_run(&fresh, Some("feat-x")).unwrap();
        assert_eq!(
            s.list_live_runs_for_feature("p1", "feat-x").unwrap().len(),
            1,
            "the migrated database takes a feature-carrying run"
        );
    }

    /// dispatch-submit-and-reclaim P2-4: the transcript a run finished with
    /// is stored on its own row, so the answer survives the pane. A run
    /// still working has none — that is what makes a non-NULL column mean
    /// "over".
    #[test]
    fn a_runs_final_transcript_round_trips_and_is_empty_until_it_finishes() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.insert_run(&sample_run(), None).unwrap();

        assert_eq!(
            s.run_final_transcript("r1").unwrap(),
            None,
            "a run that has not finished carries no final transcript"
        );
        assert_eq!(
            s.run_final_transcript("r-unknown").unwrap(),
            None,
            "an unknown run reads back the same honest none"
        );

        s.set_run_final_transcript("r1", "the last delta\nHERDR_DONE_abc123")
            .unwrap();
        assert_eq!(
            s.run_final_transcript("r1").unwrap().as_deref(),
            Some("the last delta\nHERDR_DONE_abc123"),
            "the transcript a run ended with must survive the round trip"
        );
        assert_eq!(
            s.get_run("r1").unwrap().unwrap().status,
            "pending",
            "writing the transcript touches no other column"
        );
    }

    /// A database whose `runs` table predates `final_transcript`: as with
    /// `feature`, only the migration can add it, and running the migration
    /// twice (a second open of the same file) must stay a no-op.
    #[test]
    fn migrate_adds_the_runs_final_transcript_column_and_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE runs (
                 id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 pane_id TEXT NOT NULL,
                 preset_label TEXT,
                 task TEXT NOT NULL,
                 baseline TEXT NOT NULL,
                 marker TEXT NOT NULL,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             INSERT INTO runs VALUES('r-old','p1','pane-1',NULL,'t','','m','done','t','t');",
        )
        .unwrap();

        let s = SqliteStore::from_conn(conn).unwrap();
        {
            let c = s.conn.lock().unwrap();
            assert_eq!(
                c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
            assert!(
                has_column(&c, "runs", "final_transcript").unwrap(),
                "the migration must add the column an older build never had"
            );
            // A second open runs `migrate` again against the same database.
            // `has_column` guards the ALTER, and `user_version` already
            // stands at the target, so the step must not error or double-add.
            super::migrate(&c).unwrap();
            super::migration_4_runs_final_transcript(&c)
                .expect("re-running the step against a migrated database is a no-op");
            assert_eq!(
                c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
        }

        assert_eq!(
            s.run_final_transcript("r-old").unwrap(),
            None,
            "a row written before the column existed has no stored transcript"
        );
        s.set_run_final_transcript("r-old", "recovered").unwrap();
        assert_eq!(
            s.run_final_transcript("r-old").unwrap().as_deref(),
            Some("recovered"),
            "the migrated column is writable"
        );
    }

    fn queued_leader_run(id: &str, project_id: &str, created_at: &str) -> Run {
        let mut run = sample_run();
        run.id = id.into();
        run.project_id = project_id.into();
        run.pane_id = "paseo:pending".into();
        run.preset_label = None;
        run.status = "queued".into();
        run.created_at = created_at.into();
        run
    }

    #[test]
    fn migrate_adds_the_runs_request_columns_to_a_v4_database_keeping_its_runs() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE runs (
                 id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 pane_id TEXT NOT NULL,
                 preset_label TEXT,
                 task TEXT NOT NULL,
                 baseline TEXT NOT NULL,
                 marker TEXT NOT NULL,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 feature TEXT,
                 final_transcript TEXT
             );
             INSERT INTO runs VALUES('r-old','p1','paseo:a1',NULL,'t','','m','done','t','t',NULL,'fin');
             PRAGMA user_version = 4;",
        )
        .unwrap();

        let s = SqliteStore::from_conn(conn).unwrap();
        {
            let c = s.conn.lock().unwrap();
            assert_eq!(
                c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
            for column in ["request_id", "caller", "attempts"] {
                assert!(has_column(&c, "runs", column).unwrap(), "{column}");
            }
            super::migrate(&c).unwrap();
            super::migration_5_runs_request(&c).unwrap();
        }
        assert_eq!(s.get_run("r-old").unwrap().unwrap().status, "done");
        assert_eq!(
            s.run_final_transcript("r-old").unwrap().as_deref(),
            Some("fin")
        );
        assert_eq!(s.run_attempts("r-old").unwrap(), Some(0));
        assert_eq!(s.run_caller("r-old").unwrap(), None);
    }

    #[test]
    fn migrate_adds_the_runs_notice_columns_to_a_v5_database_keeping_its_runs() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE runs (
                 id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 pane_id TEXT NOT NULL,
                 preset_label TEXT,
                 task TEXT NOT NULL,
                 baseline TEXT NOT NULL,
                 marker TEXT NOT NULL,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 feature TEXT,
                 final_transcript TEXT,
                 request_id TEXT,
                 caller TEXT,
                 attempts INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO runs VALUES('r-old','p1','paseo:a1',NULL,'t','','m','done','t','t',NULL,'fin','req-1','caller',0);
             PRAGMA user_version = 5;",
        )
        .unwrap();

        let s = SqliteStore::from_conn(conn).unwrap();
        {
            let c = s.conn.lock().unwrap();
            assert_eq!(
                c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
            for column in ["notice", "notice_state", "notice_until"] {
                assert!(has_column(&c, "runs", column).unwrap(), "{column}");
            }
            super::migrate(&c).unwrap();
            super::migration_6_runs_notice(&c).unwrap();
        }
        assert_eq!(s.get_run("r-old").unwrap().unwrap().status, "done");
        assert_eq!(s.run_caller("r-old").unwrap().as_deref(), Some("caller"));
        assert!(
            s.pending_notices().unwrap().is_empty(),
            "a row written before the columns existed owes no notice"
        );
    }

    #[test]
    fn a_finished_run_keeps_its_caller_notice_until_it_is_sent_or_dropped() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.insert_run_with_request(&sample_run(), None, None, Some("caller"))
            .unwrap();

        assert!(s.pending_notices().unwrap().is_empty());
        s.finish_run(
            "r1",
            "done",
            "2026-10-16T11:00:00Z",
            Some("the reply"),
            Some(("notice one", "2026-10-16T12:00:00Z")),
        )
        .unwrap();

        let run = s.get_run("r1").unwrap().unwrap();
        assert_eq!(run.status, "done");
        assert_eq!(run.updated_at, "2026-10-16T11:00:00Z");
        assert_eq!(
            s.run_final_transcript("r1").unwrap().as_deref(),
            Some("the reply")
        );
        assert_eq!(
            s.pending_notices().unwrap(),
            vec![(
                "r1".to_string(),
                "caller".to_string(),
                "notice one".to_string(),
                "2026-10-16T12:00:00Z".to_string()
            )]
        );

        s.set_notice_state("r1", "sent").unwrap();
        assert!(s.pending_notices().unwrap().is_empty());

        s.set_notice_state("r1", "pending").unwrap();
        s.finish_run(
            "r1",
            "blocked",
            "2026-10-16T11:05:00Z",
            None,
            Some(("notice two", "2026-10-16T12:00:00Z")),
        )
        .unwrap();
        assert_eq!(
            s.pending_notices()
                .unwrap()
                .iter()
                .map(|n| n.2.as_str())
                .collect::<Vec<_>>(),
            vec!["notice two"],
            "a newer notice replaces the one still pending for the same run"
        );
        assert_eq!(
            s.run_final_transcript("r1").unwrap().as_deref(),
            Some("the reply"),
            "a finish with no transcript leaves the stored one alone"
        );
    }

    #[test]
    fn a_follow_up_run_stores_its_link_and_reads_its_chain_and_family() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        let root = queued_leader_run("r-root", "p1", "2026-08-16T00:00:00Z");
        s.insert_follow_up_run(&root, None, Some("req-root"), None, None, false)
            .unwrap();
        let first = queued_leader_run("r-f1", "p1", "2026-08-16T00:00:01Z");
        s.insert_follow_up_run(&first, None, None, None, Some("r-root"), false)
            .unwrap();
        let second = queued_leader_run("r-f2", "p1", "2026-08-16T00:00:02Z");
        s.insert_follow_up_run(&second, None, None, None, Some("r-f1"), false)
            .unwrap();
        let sibling = queued_leader_run("r-f3", "p1", "2026-08-16T00:00:03Z");
        s.insert_follow_up_run(&sibling, None, None, None, Some("r-root"), false)
            .unwrap();
        let stranger = queued_leader_run("r-x", "p1", "2026-08-16T00:00:04Z");
        s.insert_follow_up_run(&stranger, None, None, None, None, false)
            .unwrap();

        assert_eq!(s.run_follow_up_of("r-f2").unwrap().as_deref(), Some("r-f1"));
        assert_eq!(s.run_follow_up_of("r-root").unwrap(), None);
        assert_eq!(
            s.run_request_id("r-root").unwrap().as_deref(),
            Some("req-root")
        );
        let chain: Vec<String> = s
            .follow_up_chain("r-f2")
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(chain, ["r-root", "r-f1", "r-f2"]);
        let family: Vec<String> = s
            .follow_up_family("r-root")
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(family, ["r-root", "r-f1", "r-f2", "r-f3"]);
    }

    #[test]
    fn leader_runs_lists_the_runs_on_one_pane_newest_first() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        for (id, at, pane) in [
            ("r-1", "2026-08-16T00:00:00Z", "paseo:lead-a"),
            ("r-2", "2026-08-16T00:00:01Z", "paseo:lead-b"),
            ("r-3", "2026-08-16T00:00:02Z", "paseo:lead-a"),
        ] {
            let mut run = queued_leader_run(id, "p1", at);
            run.pane_id = pane.into();
            let follow = (id == "r-3").then_some("r-1");
            s.insert_follow_up_run(&run, None, Some(&format!("req-{id}")), None, follow, false)
                .unwrap();
        }
        let runs = s.leader_runs("p1", "paseo:lead-a", 5).unwrap();
        let rows: Vec<(&str, Option<&str>, Option<&str>)> = runs
            .iter()
            .map(|l| {
                (
                    l.run.id.as_str(),
                    l.request_id.as_deref(),
                    l.follow_up_of.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("r-3", Some("req-r-3"), Some("r-1")),
                ("r-1", Some("req-r-1"), None)
            ]
        );
        assert_eq!(s.leader_runs("p1", "paseo:lead-a", 1).unwrap().len(), 1);
    }

    #[test]
    fn handoff_flag_round_trips_and_runs_on_pane_lists_across_projects() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        for (id, at, handoff, of) in [
            ("h-root", "2026-08-16T00:00:00Z", false, None),
            ("h-hand", "2026-08-16T00:00:01Z", true, Some("h-root")),
        ] {
            let mut run = sample_run();
            run.id = id.into();
            run.pane_id = "paseo:lead-a".into();
            run.created_at = at.into();
            s.insert_follow_up_run(&run, None, None, None, of, handoff)
                .unwrap();
        }
        assert!(!s.run_is_handoff("h-root").unwrap());
        assert!(s.run_is_handoff("h-hand").unwrap());
        assert_eq!(
            s.family_handoffs("h-root").unwrap(),
            HashSet::from(["h-hand".to_string()])
        );
        let ids: Vec<String> = s
            .runs_on_pane("paseo:lead-a")
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, ["h-hand", "h-root"]);
    }

    #[test]
    fn migrate_adds_the_handoff_column_to_a_v10_database_keeping_its_runs() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (project_id TEXT NOT NULL, rel_path TEXT NOT NULL);
             CREATE TABLE runs (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, task TEXT NOT NULL);
             INSERT INTO runs(id, project_id, task) VALUES ('r-old', 'p1', 'old task');
             PRAGMA user_version = 10;",
        )
        .unwrap();
        super::migrate(&conn).unwrap();
        let handoff: i64 = conn
            .query_row("SELECT handoff FROM runs WHERE id='r-old'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(handoff, 0);
    }

    #[test]
    fn migration_12_adds_leader_holds_and_late_reply_at_keeping_runs() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (project_id TEXT NOT NULL, rel_path TEXT NOT NULL);
             CREATE TABLE runs (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, task TEXT NOT NULL);
             INSERT INTO runs(id, project_id, task) VALUES ('r-old', 'p1', 'old task');
             PRAGMA user_version = 11;",
        )
        .unwrap();
        super::migrate(&conn).unwrap();
        super::migrate(&conn).unwrap();
        assert!(has_column(&conn, "runs", "late_reply_at").unwrap());
        assert!(has_column(&conn, "leader_holds", "until").unwrap());
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let kept: i64 = conn
            .query_row("SELECT count(*) FROM runs WHERE id='r-old'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(kept, 1);
    }

    #[test]
    fn migrate_adds_the_follow_up_column_to_a_v9_database_keeping_its_runs() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (project_id TEXT NOT NULL, rel_path TEXT NOT NULL);
             CREATE TABLE runs (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, task TEXT NOT NULL);
             INSERT INTO runs(id, project_id, task) VALUES ('r-old', 'p1', 'old task');
             PRAGMA user_version = 9;",
        )
        .unwrap();
        super::migrate(&conn).unwrap();
        assert!(has_column(&conn, "runs", "follow_up_of").unwrap());
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let kept: Option<String> = conn
            .query_row("SELECT follow_up_of FROM runs WHERE id='r-old'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(kept, None);
    }

    #[test]
    fn runs_with_one_request_id_insert_once_per_project() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        let first = queued_leader_run("r-a", "p1", "2026-08-16T00:00:00Z");
        assert_eq!(
            s.insert_run_with_request(&first, None, Some("req-1"), Some("agent-x"))
                .unwrap(),
            "r-a"
        );
        let repeat = queued_leader_run("r-b", "p1", "2026-08-16T00:00:01Z");
        assert_eq!(
            s.insert_run_with_request(&repeat, None, Some("req-1"), None)
                .unwrap(),
            "r-a"
        );
        assert!(s.get_run("r-b").unwrap().is_none());
        assert_eq!(s.run_by_request("p1", "req-1").unwrap().unwrap().id, "r-a");
        assert_eq!(s.run_caller("r-a").unwrap().as_deref(), Some("agent-x"));

        let other = queued_leader_run("r-c", "p2", "2026-08-16T00:00:02Z");
        assert_eq!(
            s.insert_run_with_request(&other, None, Some("req-1"), None)
                .unwrap(),
            "r-c"
        );
        assert!(s.run_by_request("p1", "req-missing").unwrap().is_none());

        let mut plain = sample_run();
        plain.id = "r-d".into();
        s.insert_run(&plain, None).unwrap();
        plain.id = "r-e".into();
        s.insert_run(&plain, None).unwrap();
        assert!(s.insert_run(&plain, None).is_err());
        s.set_run_request("r-d", "req-2").unwrap();
        assert_eq!(
            s.run_by_request(&plain.project_id, "req-2")
                .unwrap()
                .unwrap()
                .id,
            "r-d"
        );
    }

    #[test]
    fn queued_paseo_runs_claim_once_and_requeue_with_an_attempt() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        for run in [
            queued_leader_run("r-late", "p1", "2026-08-16T00:00:02Z"),
            queued_leader_run("r-early", "p1", "2026-08-16T00:00:01Z"),
            queued_leader_run("r-p2", "p2", "2026-08-16T00:00:03Z"),
        ] {
            s.insert_run(&run, None).unwrap();
        }
        let mut herdr = queued_leader_run("r-herdr", "p1", "2026-08-16T00:00:00Z");
        herdr.pane_id = "w1:p1".into();
        s.insert_run(&herdr, None).unwrap();

        assert_eq!(
            s.list_queued_paseo_runs("p1")
                .unwrap()
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["r-early", "r-late"]
        );
        assert_eq!(
            s.list_projects_with_queued_paseo_runs().unwrap(),
            vec!["p1".to_string(), "p2".to_string()]
        );

        assert!(s.claim_queued_run("r-early").unwrap());
        assert!(!s.claim_queued_run("r-early").unwrap());
        let claimed = s.get_run("r-early").unwrap().unwrap();
        assert_eq!(claimed.status, "working");
        assert_eq!(claimed.pane_id, "paseo:pending");

        s.set_run_pane("r-early", "paseo:agent-1").unwrap();
        s.update_run_status("r-late", "blocked", "t", None, None)
            .unwrap();
        assert_eq!(
            s.list_inflight_paseo_runs()
                .unwrap()
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["r-early", "r-late"]
        );

        assert!(s.requeue_run("r-early").unwrap());
        assert!(!s.requeue_run("r-early").unwrap());
        assert_eq!(s.run_attempts("r-early").unwrap(), Some(1));
        assert_eq!(s.get_run("r-early").unwrap().unwrap().status, "queued");
        assert_eq!(s.run_attempts("r-unknown").unwrap(), None);
    }

    #[test]
    fn queued_runs_are_not_in_the_unattended_working_runs_sweep() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        let mut run = queued_leader_run("r-q", "p1", "2026-08-16T00:00:00Z");
        run.preset_label = Some("claude".into());
        s.insert_run(&run, None).unwrap();
        assert!(s.list_unattended_working_runs().unwrap().is_empty());
        assert!(s.list_working_paseo_runs().unwrap().is_empty());
    }

    #[test]
    fn set_orchestration_enabled_flips_the_flag_on_get_project() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        assert!(!s.get_project("p1").unwrap().unwrap().orchestration_enabled);

        s.set_orchestration_enabled("p1", true).unwrap();
        assert!(s.get_project("p1").unwrap().unwrap().orchestration_enabled);

        s.set_orchestration_enabled("p1", false).unwrap();
        assert!(!s.get_project("p1").unwrap().unwrap().orchestration_enabled);
    }

    /// A database as a pre-D6 build left it: `projects` exists but has no
    /// `orchestration_enabled` column.
    fn legacy_projects_conn() -> Connection {
        let conn = legacy_conn();
        conn.execute_batch(
            "CREATE TABLE projects (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 root_path TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 last_seen_at TEXT NOT NULL
             );
             INSERT INTO projects VALUES('p1','P1','/proj','2026-07-15T00:00:00Z','2026-07-15T00:00:00Z');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migration_applies_orchestration_enabled_column_to_a_legacy_database() {
        let store = SqliteStore::from_conn(legacy_projects_conn()).unwrap();
        let c = store.conn.lock().unwrap();

        let version: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(has_column(&c, "projects", "orchestration_enabled").unwrap());

        let flag: bool = c
            .query_row(
                "SELECT orchestration_enabled FROM projects WHERE id='p1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!flag, "existing rows must default the new column to off");
    }

    fn mail(s: &SqliteStore, project_id: &str, body: &str) -> i64 {
        s.insert_inbox_message(
            "wd",
            "corr-1",
            project_id,
            "result",
            body,
            "2026-10-22T00:00:00Z",
        )
        .unwrap()
    }

    #[test]
    fn migrate_adds_the_inbox_table_to_a_v6_database_keeping_its_runs() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE runs (
                 id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 pane_id TEXT NOT NULL,
                 preset_label TEXT,
                 task TEXT NOT NULL,
                 baseline TEXT NOT NULL,
                 marker TEXT NOT NULL,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 feature TEXT,
                 final_transcript TEXT,
                 request_id TEXT,
                 caller TEXT,
                 attempts INTEGER NOT NULL DEFAULT 0,
                 notice TEXT,
                 notice_state TEXT,
                 notice_until TEXT
             );
             INSERT INTO runs VALUES('r-a','p1','paseo:a1',NULL,'t','','m','done','t','t',NULL,'fin','req-1','caller',0,NULL,NULL,NULL);
             INSERT INTO runs VALUES('r-b','p1','paseo:a2',NULL,'t','','m','working','t','t',NULL,NULL,'req-2',NULL,1,NULL,NULL,NULL);
             PRAGMA user_version = 6;",
        )
        .unwrap();

        let s = SqliteStore::from_conn(conn).unwrap();
        {
            let c = s.conn.lock().unwrap();
            assert_eq!(
                c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
            assert_eq!(
                c.query_row::<i64, _, _>("SELECT COUNT(*) FROM runs", [], |r| r.get(0))
                    .unwrap(),
                2
            );
            super::migrate(&c).unwrap();
            super::migration_7_inbox(&c).unwrap();
        }
        assert_eq!(s.get_run("r-b").unwrap().unwrap().status, "working");
        let seq = mail(&s, "p1", "hello");
        assert_eq!(s.next_inbox_message("wd").unwrap().unwrap().seq, seq);
    }

    #[test]
    fn next_pending_inbox_message_is_the_lowest_seq_and_ack_hides_it() {
        let s = SqliteStore::open_in_memory().unwrap();
        assert!(s.next_inbox_message("wd").unwrap().is_none());
        let first = mail(&s, "p1", "one");
        let second = mail(&s, "p1", "two");
        s.insert_inbox_message("other", "c", "p1", "result", "x", "t")
            .unwrap();
        assert!(second > first);

        let next = s.next_inbox_message("wd").unwrap().unwrap();
        assert_eq!(next.seq, first);
        assert_eq!(next.recipient, "wd");
        assert_eq!(next.corr, "corr-1");
        assert_eq!(next.project_id, "p1");
        assert_eq!(next.kind, "result");
        assert_eq!(next.body, "one");
        assert_eq!(next.created_at, "2026-10-22T00:00:00Z");
        assert_eq!(next.acked_at, None);
        assert_eq!(next.attempts, 0);
        assert_eq!(next.dead_at, None);

        s.ack_inbox_message(first, "2026-10-22T00:01:00Z").unwrap();
        assert_eq!(s.next_inbox_message("wd").unwrap().unwrap().seq, second);
        s.ack_inbox_message(second, "2026-10-22T00:02:00Z").unwrap();
        assert!(s.next_inbox_message("wd").unwrap().is_none());
    }

    #[test]
    fn five_failed_attempts_mark_an_inbox_message_dead_and_next_pending_skips_it() {
        let s = SqliteStore::open_in_memory().unwrap();
        let poison = mail(&s, "p1", "poison");
        let after = mail(&s, "p1", "after");
        for attempt in 1..5 {
            assert!(!s
                .fail_inbox_message(poison, 5, "2026-10-22T00:05:00Z")
                .unwrap());
            let next = s.next_inbox_message("wd").unwrap().unwrap();
            assert_eq!((next.seq, next.attempts), (poison, attempt));
        }
        assert!(s
            .fail_inbox_message(poison, 5, "2026-10-22T00:05:00Z")
            .unwrap());
        assert_eq!(s.next_inbox_message("wd").unwrap().unwrap().seq, after);
    }

    #[test]
    fn prune_deletes_only_acked_inbox_rows_older_than_the_cutoff() {
        let s = SqliteStore::open_in_memory().unwrap();
        let old_acked = mail(&s, "p1", "old");
        let new_acked = mail(&s, "p1", "new");
        let pending = mail(&s, "p1", "pending");
        s.ack_inbox_message(old_acked, "2026-09-01T00:00:00Z")
            .unwrap();
        s.ack_inbox_message(new_acked, "2026-10-21T00:00:00Z")
            .unwrap();

        assert_eq!(s.prune_inbox("2026-09-22T00:00:00Z").unwrap(), 1);
        let c = s.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT seq FROM inbox ORDER BY seq").unwrap();
        let left: Vec<i64> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(left, vec![new_acked, pending]);
    }

    #[test]
    fn delete_project_removes_only_that_projects_inbox_rows() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        mail(&s, "p1", "gone");
        let kept = mail(&s, "p2", "kept");
        s.delete_project("p1").unwrap();
        let next = s.next_inbox_message("wd").unwrap().unwrap();
        assert_eq!((next.seq, next.project_id.as_str()), (kept, "p2"));
        s.ack_inbox_message(kept, "t").unwrap();
        assert!(s.next_inbox_message("wd").unwrap().is_none());
    }

    #[test]
    fn migrate_adds_the_tasks_and_permission_tables_to_a_v7_database_keeping_its_inbox() {
        let conn = Connection::open_in_memory().unwrap();
        super::migration_7_inbox(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO inbox(recipient, corr, project_id, kind, body, created_at)
             VALUES('wd','c-1','p1','result','kept','t');
             PRAGMA user_version = 7;",
        )
        .unwrap();

        let s = SqliteStore::from_conn(conn).unwrap();
        {
            let c = s.conn.lock().unwrap();
            assert_eq!(
                c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
            super::migrate(&c).unwrap();
            super::migration_8_tasks(&c).unwrap();
        }
        assert_eq!(s.next_inbox_message("wd").unwrap().unwrap().body, "kept");
        assert!(s.insert_task("c-1", "p1", "t0").unwrap());
        assert!(s
            .insert_permission_request(&request("q-1", "c-1", "t0"))
            .unwrap());
    }

    fn request(req_id: &str, corr: &str, seen_at: &str) -> PermissionRequest {
        PermissionRequest {
            req_id: req_id.into(),
            corr: corr.into(),
            leader_id: "lead".into(),
            tool: "Bash".into(),
            description: "rm -rf target".into(),
            seen_at: seen_at.into(),
            answered_at: None,
            closed_at: None,
        }
    }

    #[test]
    fn an_inserted_task_is_open_with_only_its_creation_stamps_and_inserts_once() {
        let s = SqliteStore::open_in_memory().unwrap();
        assert!(s.get_task("c-1").unwrap().is_none());
        assert!(s.insert_task("c-1", "p1", "t1").unwrap());
        assert!(!s.insert_task("c-1", "p2", "t2").unwrap());
        assert_eq!(
            s.get_task("c-1").unwrap().unwrap(),
            WdTask {
                corr: "c-1".into(),
                project_id: "p1".into(),
                leader_id: None,
                state: "open".into(),
                created_at: "t1".into(),
                last_delivered_at: None,
                last_mail_at: None,
                nudged_at: None,
                progress_sent_at: None,
                changed_at: "t1".into(),
                board_sent_at: None,
            }
        );
    }

    #[test]
    fn delivery_records_the_leader_and_clears_the_nudge_marks() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.insert_task("c-1", "p1", "t1").unwrap();
        s.set_task_nudged("c-1", "t2").unwrap();
        s.set_task_progress_sent("c-1", "t3").unwrap();
        let t = s.get_task("c-1").unwrap().unwrap();
        assert_eq!(
            (t.nudged_at.as_deref(), t.progress_sent_at.as_deref()),
            (Some("t2"), Some("t3"))
        );
        assert_eq!(t.changed_at, "t1");

        s.mark_task_delivered("c-1", "lead", "t4").unwrap();
        let t = s.get_task("c-1").unwrap().unwrap();
        assert_eq!(t.leader_id.as_deref(), Some("lead"));
        assert_eq!(t.last_delivered_at.as_deref(), Some("t4"));
        assert_eq!((t.nudged_at, t.progress_sent_at), (None, None));
        assert_eq!(t.changed_at, "t4");
    }

    #[test]
    fn mail_and_close_stamp_changed_at_and_close_drops_the_task_from_the_open_list() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.insert_task("c-1", "p1", "t1").unwrap();
        s.insert_task("c-2", "p2", "t2").unwrap();
        s.mark_task_mail("c-1", "t3").unwrap();
        let t = s.get_task("c-1").unwrap().unwrap();
        assert_eq!(
            (t.last_mail_at.as_deref(), t.changed_at.as_str()),
            (Some("t3"), "t3")
        );
        let open: Vec<String> = s
            .list_open_tasks()
            .unwrap()
            .into_iter()
            .map(|t| t.corr)
            .collect();
        assert_eq!(open, vec!["c-1", "c-2"]);

        s.close_task("c-1", "t4").unwrap();
        let t = s.get_task("c-1").unwrap().unwrap();
        assert_eq!((t.state.as_str(), t.changed_at.as_str()), ("closed", "t4"));
        let open: Vec<String> = s
            .list_open_tasks()
            .unwrap()
            .into_iter()
            .map(|t| t.corr)
            .collect();
        assert_eq!(open, vec!["c-2"]);
    }

    #[test]
    fn stamping_the_board_marks_every_task() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.insert_task("c-1", "p1", "t1").unwrap();
        s.insert_task("c-2", "p2", "t2").unwrap();
        s.close_task("c-2", "t3").unwrap();
        s.stamp_board_sent("t5").unwrap();
        for corr in ["c-1", "c-2"] {
            let t = s.get_task(corr).unwrap().unwrap();
            assert_eq!(t.board_sent_at.as_deref(), Some("t5"), "{corr}");
        }
        assert_eq!(s.get_task("c-1").unwrap().unwrap().changed_at, "t1");
    }

    #[test]
    fn permission_requests_insert_once_and_answer_or_close_hides_them() {
        let s = SqliteStore::open_in_memory().unwrap();
        assert!(s
            .insert_permission_request(&request("q-1", "c-1", "t1"))
            .unwrap());
        assert!(!s
            .insert_permission_request(&request("q-1", "c-1", "t9"))
            .unwrap());
        s.insert_permission_request(&request("q-2", "c-1", "t2"))
            .unwrap();
        s.insert_permission_request(&request("q-3", "c-1", "t3"))
            .unwrap();
        s.insert_permission_request(&request("q-4", "c-2", "t4"))
            .unwrap();

        let open = s.unanswered_permission_requests("c-1").unwrap();
        assert_eq!(open[0], request("q-3", "c-1", "t3"));
        let ids: Vec<&str> = open.iter().map(|r| r.req_id.as_str()).collect();
        assert_eq!(ids, vec!["q-3", "q-2", "q-1"]);

        s.answer_permission_request("q-3", "t5").unwrap();
        s.close_permission_request("q-1", "t6").unwrap();
        let ids: Vec<String> = s
            .unanswered_permission_requests("c-1")
            .unwrap()
            .into_iter()
            .map(|r| r.req_id)
            .collect();
        assert_eq!(ids, vec!["q-2"]);
        let mut known = s.permission_request_ids("c-1").unwrap();
        known.sort();
        assert_eq!(known, vec!["q-1", "q-2", "q-3"]);
    }

    #[test]
    fn prune_drops_old_closed_tasks_and_old_answered_or_closed_requests_only() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.insert_task("old-closed", "p1", "2026-08-01").unwrap();
        s.close_task("old-closed", "2026-08-02").unwrap();
        s.insert_task("new-closed", "p1", "2026-08-01").unwrap();
        s.close_task("new-closed", "2026-10-20").unwrap();
        s.insert_task("old-open", "p1", "2026-08-01").unwrap();
        for id in ["old-answered", "old-closed", "new-answered", "old-pending"] {
            s.insert_permission_request(&request(id, "c-1", "2026-08-01"))
                .unwrap();
        }
        s.answer_permission_request("old-answered", "2026-08-02")
            .unwrap();
        s.close_permission_request("old-closed", "2026-08-02")
            .unwrap();
        s.answer_permission_request("new-answered", "2026-10-20")
            .unwrap();

        assert_eq!(s.prune_tasks("2026-09-22").unwrap(), 3);
        assert!(s.get_task("old-closed").unwrap().is_none());
        assert!(s.get_task("new-closed").unwrap().is_some());
        assert!(s.get_task("old-open").unwrap().is_some());
        let mut known = s.permission_request_ids("c-1").unwrap();
        known.sort();
        assert_eq!(known, vec!["new-answered", "old-pending"]);
    }

    #[test]
    fn delete_project_removes_only_that_projects_tasks_and_their_requests() {
        let s = SqliteStore::open_in_memory().unwrap();
        s.upsert_project(&sample_project()).unwrap();
        s.insert_task("c-1", "p1", "t1").unwrap();
        s.insert_task("c-2", "p2", "t1").unwrap();
        s.insert_permission_request(&request("q-1", "c-1", "t1"))
            .unwrap();
        s.insert_permission_request(&request("q-2", "c-2", "t1"))
            .unwrap();
        s.delete_project("p1").unwrap();
        assert!(s.get_task("c-1").unwrap().is_none());
        assert!(s.get_task("c-2").unwrap().is_some());
        assert!(s.permission_request_ids("c-1").unwrap().is_empty());
        assert_eq!(s.permission_request_ids("c-2").unwrap(), vec!["q-2"]);
    }
}
