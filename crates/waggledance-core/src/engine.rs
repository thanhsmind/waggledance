use crate::config::Config;
use crate::domain::{InboxMessage, LeaderHold, PermissionRequest, Project, Run, WdTask};
use crate::error::{Error, Result};
use crate::intake::{IntakeItem, IntakeVerdict};
use crate::project_settings::{self, ProjectSettings};
use crate::repository::SqliteStore;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub struct Engine {
    pub store: SqliteStore,
    pub config: Config,
    settings_cache: Mutex<HashMap<PathBuf, CachedSettings>>,
}

type CachedSettings = (Option<SystemTime>, ProjectSettings, Vec<String>);

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

fn slug_from_root(root: &Path) -> String {
    let base = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let mut out = String::new();
    for ch in base.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' || ch == '.' {
            out.push('-');
        }
    }
    let s = out.trim_matches('-').to_string();
    if s.is_empty() {
        "project".into()
    } else {
        s
    }
}

impl Engine {
    pub fn new(store: SqliteStore, config: Config) -> Self {
        Self {
            store,
            config,
            settings_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Canonicalize when possible; otherwise fall back to the given path.
    fn canonical(root: &Path) -> PathBuf {
        std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
    }

    /// Find the project owning `root`, or record it (implicit registration).
    pub fn ensure_project(&self, root: &Path, name: Option<&str>) -> Result<Project> {
        let root = Self::canonical(root);
        if let Some(mut p) = self.store.find_project_by_root(&root)? {
            p.last_seen_at = now_rfc3339();
            self.store.upsert_project(&p)?;
            return Ok(self.with_consent(p));
        }
        let id = self.unique_id(&slug_from_root(&root))?;
        let name = name.map(|s| s.to_string()).unwrap_or_else(|| {
            root.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&id)
                .to_string()
        });
        let now = now_rfc3339();
        let project = Project {
            id,
            name,
            root_path: root,
            created_at: now.clone(),
            last_seen_at: now,
            orchestration_enabled: false,
        };
        self.store.upsert_project(&project)?;
        Ok(self.with_consent(project))
    }

    fn unique_id(&self, base: &str) -> Result<String> {
        if self.store.get_project(base)?.is_none() {
            return Ok(base.to_string());
        }
        for n in 2..10_000 {
            let cand = format!("{base}-{n}");
            if self.store.get_project(&cand)?.is_none() {
                return Ok(cand);
            }
        }
        Err(Error::Other("could not allocate project id".into()))
    }

    /// Register a project explicitly (CLI). Same as ensure_project + optional name.
    pub fn register(&self, root: &Path, name: Option<&str>) -> Result<Project> {
        self.ensure_project(root, name)
    }

    pub fn unregister(&self, project_id: &str) -> Result<()> {
        self.store.delete_project(project_id)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        Ok(self
            .store
            .list_projects()?
            .into_iter()
            .map(|p| self.with_consent(p))
            .collect())
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        Ok(self.store.get_project(id)?.map(|p| self.with_consent(p)))
    }

    fn with_consent(&self, mut project: Project) -> Project {
        project.orchestration_enabled = self.orchestration_allowed(&project);
        project
    }

    pub fn set_orchestration_enabled(&self, project_id: &str, enabled: bool) -> Result<()> {
        self.store.set_orchestration_enabled(project_id, enabled)
    }

    pub fn orchestration_allowed(&self, project: &Project) -> bool {
        let settings = self.project_settings(project).0;
        settings.unreadable.is_none() && settings.orchestration != Some(false)
    }

    pub fn project_settings(&self, project: &Project) -> (ProjectSettings, Vec<String>) {
        let path = project_settings::settings_path(&project.root_path);
        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let mut cache = self
            .settings_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some((cached, settings, warnings)) = cache.get(&path) {
            if *cached == mtime {
                return (settings.clone(), warnings.clone());
            }
        }
        let (settings, warnings) = project_settings::load(&project.root_path);
        cache.insert(path, (mtime, settings.clone(), warnings.clone()));
        (settings, warnings)
    }

    // ---- runs (D7) ----

    /// Persist one run, stamped with the bee feature it was started for
    /// (board-run-actions D3) — `None` for a run with no feature behind it.
    pub fn insert_run(&self, run: &Run, feature: Option<&str>) -> Result<()> {
        self.store.insert_run(run, feature)
    }

    pub fn update_run_status(
        &self,
        id: &str,
        status: &str,
        updated_at: &str,
        baseline: Option<&str>,
        marker: Option<&str>,
    ) -> Result<()> {
        self.store
            .update_run_status(id, status, updated_at, baseline, marker)
    }

    pub fn get_run(&self, id: &str) -> Result<Option<Run>> {
        self.store.get_run(id)
    }

    pub fn list_runs(&self, project_id: &str, limit: usize) -> Result<Vec<Run>> {
        self.store.list_runs(project_id, limit)
    }

    /// The still-`working` runs this project started for `feature` — the
    /// store half of the board's per-feature run lock (board-run-actions
    /// D3). A caller decides whether each one's pane is still alive; this
    /// answers only what the ledger holds.
    pub fn list_live_runs_for_feature(&self, project_id: &str, feature: &str) -> Result<Vec<Run>> {
        self.store.list_live_runs_for_feature(project_id, feature)
    }

    /// The feature a run was started for, read back off its own row.
    pub fn run_feature(&self, id: &str) -> Result<Option<String>> {
        self.store.run_feature(id)
    }

    /// Record the transcript a run ended with, on its own row. Called once
    /// a run reaches a terminal status, so the answer outlives the pane it
    /// was read from.
    pub fn set_run_final_transcript(&self, id: &str, transcript: &str) -> Result<()> {
        self.store.set_run_final_transcript(id, transcript)
    }

    pub fn finish_run(
        &self,
        id: &str,
        status: &str,
        updated_at: &str,
        transcript: Option<&str>,
        notice: Option<(&str, &str)>,
    ) -> Result<()> {
        self.store
            .finish_run(id, status, updated_at, transcript, notice)
    }

    pub fn pending_notices(&self) -> Result<Vec<(String, String, String, String)>> {
        self.store.pending_notices()
    }

    pub fn set_notice_state(&self, run_id: &str, state: &str) -> Result<()> {
        self.store.set_notice_state(run_id, state)
    }

    pub fn run_notice_state(&self, id: &str) -> Result<Option<String>> {
        self.store.run_notice_state(id)
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
        self.store
            .insert_inbox_message(recipient, corr, project_id, kind, body, created_at)
    }

    pub fn next_inbox_message(&self, recipient: &str) -> Result<Option<InboxMessage>> {
        self.store.next_inbox_message(recipient)
    }

    pub fn ack_inbox_message(&self, seq: i64, acked_at: &str) -> Result<()> {
        self.store.ack_inbox_message(seq, acked_at)
    }

    pub fn fail_inbox_message(&self, seq: i64, dead_at_attempts: i64, now: &str) -> Result<bool> {
        self.store.fail_inbox_message(seq, dead_at_attempts, now)
    }

    pub fn prune_inbox(&self, acked_before: &str) -> Result<usize> {
        self.store.prune_inbox(acked_before)
    }

    pub fn insert_task(&self, corr: &str, project_id: &str, at: &str) -> Result<bool> {
        self.store.insert_task(corr, project_id, at)
    }

    pub fn get_task(&self, corr: &str) -> Result<Option<WdTask>> {
        self.store.get_task(corr)
    }

    pub fn list_open_tasks(&self) -> Result<Vec<WdTask>> {
        self.store.list_open_tasks()
    }

    pub fn insert_intake_item(&self, item: &IntakeItem) -> Result<bool> {
        self.store.insert_intake_item(item)
    }

    pub fn list_intake_items(&self, verdict: IntakeVerdict) -> Result<Vec<IntakeItem>> {
        self.store.list_intake_items(verdict)
    }

    pub fn mark_intake_notified(&self, keys: &[String]) -> Result<()> {
        self.store.mark_intake_notified(keys)
    }

    pub fn mark_task_delivered(&self, corr: &str, leader_id: &str, at: &str) -> Result<()> {
        self.store.mark_task_delivered(corr, leader_id, at)
    }

    pub fn mark_task_mail(&self, corr: &str, at: &str) -> Result<()> {
        self.store.mark_task_mail(corr, at)
    }

    pub fn set_hold(
        &self,
        leader_id: &str,
        why: &str,
        minutes: Option<u64>,
        now: &str,
    ) -> Result<LeaderHold> {
        let max = self.config.paseo.hold_max_minutes;
        let minutes = minutes.unwrap_or(max).min(max);
        let start = OffsetDateTime::parse(now, &Rfc3339)
            .map_err(|e| Error::Other(format!("bad timestamp `{now}`: {e}")))?;
        let until = (start + time::Duration::minutes(minutes as i64))
            .format(&Rfc3339)
            .map_err(|e| Error::Other(e.to_string()))?;
        self.store.set_hold(leader_id, why, now, &until)?;
        Ok(LeaderHold {
            leader_id: leader_id.into(),
            why: why.into(),
            set_at: now.into(),
            until,
        })
    }

    pub fn release_hold(&self, leader_id: &str) -> Result<bool> {
        self.store.release_hold(leader_id)
    }

    pub fn live_hold(&self, leader_id: &str, now: &str) -> Result<Option<LeaderHold>> {
        self.store.live_hold(leader_id, now)
    }

    pub fn stamp_late_reply(&self, run_id: &str, at: &str) -> Result<()> {
        self.store.stamp_late_reply(run_id, at)
    }

    pub fn close_task(&self, corr: &str, at: &str) -> Result<()> {
        self.store.close_task(corr, at)
    }

    pub fn set_task_nudged(&self, corr: &str, at: &str) -> Result<()> {
        self.store.set_task_nudged(corr, at)
    }

    pub fn set_task_progress_sent(&self, corr: &str, at: &str) -> Result<()> {
        self.store.set_task_progress_sent(corr, at)
    }

    pub fn stamp_board_sent(&self, at: &str) -> Result<()> {
        self.store.stamp_board_sent(at)
    }

    pub fn insert_permission_request(&self, request: &PermissionRequest) -> Result<bool> {
        self.store.insert_permission_request(request)
    }

    pub fn unanswered_permission_requests(&self, corr: &str) -> Result<Vec<PermissionRequest>> {
        self.store.unanswered_permission_requests(corr)
    }

    pub fn answer_permission_request(&self, req_id: &str, at: &str) -> Result<()> {
        self.store.answer_permission_request(req_id, at)
    }

    pub fn close_permission_request(&self, req_id: &str, at: &str) -> Result<()> {
        self.store.close_permission_request(req_id, at)
    }

    pub fn permission_request_ids(&self, corr: &str) -> Result<Vec<String>> {
        self.store.permission_request_ids(corr)
    }

    pub fn prune_tasks(&self, before: &str) -> Result<usize> {
        self.store.prune_tasks(before)
    }

    /// The transcript a finished run ended with — `None` while it is still
    /// working, and for rows written before the column existed.
    pub fn run_final_transcript(&self, id: &str) -> Result<Option<String>> {
        self.store.run_final_transcript(id)
    }

    pub fn insert_run_with_request(
        &self,
        run: &Run,
        feature: Option<&str>,
        request_id: Option<&str>,
        caller: Option<&str>,
    ) -> Result<String> {
        self.store
            .insert_run_with_request(run, feature, request_id, caller)
    }

    pub fn insert_follow_up_run(
        &self,
        run: &Run,
        request_id: Option<&str>,
        caller: Option<&str>,
        follow_up_of: Option<&str>,
        handoff: bool,
    ) -> Result<String> {
        self.store
            .insert_follow_up_run(run, None, request_id, caller, follow_up_of, handoff)
    }

    pub fn run_is_handoff(&self, id: &str) -> Result<bool> {
        self.store.run_is_handoff(id)
    }

    pub fn runs_on_pane(&self, pane_id: &str) -> Result<Vec<Run>> {
        self.store.runs_on_pane(pane_id)
    }

    pub fn family_handoffs(&self, root: &str) -> Result<std::collections::HashSet<String>> {
        self.store.family_handoffs(root)
    }

    pub fn run_follow_up_of(&self, id: &str) -> Result<Option<String>> {
        self.store.run_follow_up_of(id)
    }

    pub fn run_request_id(&self, id: &str) -> Result<Option<String>> {
        self.store.run_request_id(id)
    }

    pub fn follow_up_chain(&self, id: &str) -> Result<Vec<Run>> {
        self.store.follow_up_chain(id)
    }

    pub fn follow_up_family(&self, root: &str) -> Result<Vec<Run>> {
        self.store.follow_up_family(root)
    }

    pub fn leader_runs(
        &self,
        project_id: &str,
        pane_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::domain::LeaderRun>> {
        self.store.leader_runs(project_id, pane_id, limit)
    }

    pub fn run_by_request(&self, project_id: &str, request_id: &str) -> Result<Option<Run>> {
        self.store.run_by_request(project_id, request_id)
    }

    pub fn project_for_corr(&self, corr: &str) -> Result<Option<String>> {
        self.store.project_for_corr(corr)
    }

    pub fn run_caller(&self, id: &str) -> Result<Option<String>> {
        self.store.run_caller(id)
    }

    pub fn run_attempts(&self, id: &str) -> Result<Option<i64>> {
        self.store.run_attempts(id)
    }

    pub fn list_queued_paseo_runs(&self, project_id: &str) -> Result<Vec<Run>> {
        self.store.list_queued_paseo_runs(project_id)
    }

    pub fn list_projects_with_queued_paseo_runs(&self) -> Result<Vec<String>> {
        self.store.list_projects_with_queued_paseo_runs()
    }

    pub fn claim_queued_run(&self, id: &str) -> Result<bool> {
        self.store.claim_queued_run(id)
    }

    pub fn set_run_request(&self, id: &str, request_id: &str) -> Result<()> {
        self.store.set_run_request(id, request_id)
    }

    pub fn set_run_pane(&self, id: &str, pane_id: &str) -> Result<()> {
        self.store.set_run_pane(id, pane_id)
    }

    pub fn set_run_session(&self, id: &str, session_id: &str) -> Result<bool> {
        self.store.set_run_session(id, session_id)
    }

    pub fn run_session(&self, id: &str) -> Result<Option<String>> {
        self.store.run_session(id)
    }

    pub fn run_sessions(
        &self,
    ) -> Result<std::collections::HashMap<String, crate::usage_rows::RunMeta>> {
        self.store.run_sessions()
    }

    pub fn requeue_run(&self, id: &str) -> Result<bool> {
        self.store.requeue_run(id)
    }

    pub fn list_inflight_paseo_runs(&self) -> Result<Vec<Run>> {
        self.store.list_inflight_paseo_runs()
    }

    pub fn list_leader_waits(&self, lost_since: &str) -> Result<Vec<Run>> {
        self.store.list_leader_waits(lost_since)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registering_a_project_records_only_its_project_row() {
        let dir = std::env::temp_dir().join(format!("waggledance-reg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# Hi").unwrap();
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());

        let project = engine.register(&dir, Some("demo")).unwrap();
        let again = engine.register(&dir, None).unwrap();

        assert_eq!(project.id, again.id);
        assert_eq!(engine.list_projects().unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    fn sample_run(project_id: &str) -> Run {
        Run {
            id: "r1".into(),
            project_id: project_id.into(),
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

    #[test]
    fn run_crud_accessors_pass_through_to_the_store() {
        let dir = std::env::temp_dir().join(format!("waggledance-runs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();

        engine.insert_run(&sample_run(&project.id), None).unwrap();
        let got = engine.get_run("r1").unwrap().unwrap();
        assert_eq!(got.status, "pending");

        engine
            .update_run_status("r1", "done", "2026-08-16T00:05:00Z", None, None)
            .unwrap();
        assert_eq!(engine.get_run("r1").unwrap().unwrap().status, "done");

        let listed = engine.list_runs(&project.id, 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "r1");

        assert_eq!(engine.run_final_transcript("r1").unwrap(), None);
        engine
            .set_run_final_transcript("r1", "final delta")
            .unwrap();
        assert_eq!(
            engine.run_final_transcript("r1").unwrap().as_deref(),
            Some("final delta")
        );

        engine
            .finish_run(
                "r1",
                "done",
                "2026-08-16T00:06:00Z",
                Some("final delta"),
                Some(("notice text", "2026-08-16T01:00:00Z")),
            )
            .unwrap();
        assert_eq!(engine.get_run("r1").unwrap().unwrap().status, "done");
        assert_eq!(
            engine.get_run("r1").unwrap().unwrap().updated_at,
            "2026-08-16T00:06:00Z"
        );
        assert!(
            engine.pending_notices().unwrap().is_empty(),
            "a run with no caller owes nobody a notice"
        );
        engine.set_notice_state("r1", "dropped").unwrap();
        assert_eq!(
            engine.run_notice_state("r1").unwrap().as_deref(),
            Some("dropped")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    fn write_bumped(file: &Path, text: &str, secs: u64) {
        std::fs::write(file, text).unwrap();
        std::fs::File::options()
            .write(true)
            .open(file)
            .unwrap()
            .set_modified(SystemTime::now() + std::time::Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn orchestration_is_on_without_a_project_file_whatever_the_stored_flag() {
        let dir = std::env::temp_dir().join(format!("waggledance-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        assert!(engine.orchestration_allowed(&project));
        assert!(project.orchestration_enabled);

        engine
            .set_orchestration_enabled(&project.id, false)
            .unwrap();
        let reloaded = engine.get_project(&project.id).unwrap().unwrap();
        assert!(reloaded.orchestration_enabled);
        assert!(engine.orchestration_allowed(&reloaded));
        assert!(engine.list_projects().unwrap()[0].orchestration_enabled);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn orchestration_is_off_only_when_the_project_file_says_false_or_is_broken() {
        let dir =
            std::env::temp_dir().join(format!("waggledance-gate-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(project_settings::FILE_NAME);

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        engine.set_orchestration_enabled(&project.id, true).unwrap();

        write_bumped(&file, "orchestration = false\n", 5);
        assert!(!engine.orchestration_allowed(&project));
        assert!(
            !engine
                .get_project(&project.id)
                .unwrap()
                .unwrap()
                .orchestration_enabled
        );
        assert!(!engine.list_projects().unwrap()[0].orchestration_enabled);
        assert!(
            !engine
                .ensure_project(&dir, None)
                .unwrap()
                .orchestration_enabled
        );
        assert!(!engine.register(&dir, None).unwrap().orchestration_enabled);

        write_bumped(&file, "orchestration = true\n", 10);
        assert!(engine.orchestration_allowed(&project));
        assert!(
            engine
                .get_project(&project.id)
                .unwrap()
                .unwrap()
                .orchestration_enabled
        );

        write_bumped(&file, "[paseo]\nprovider = \"claude\"\n", 15);
        assert!(engine.orchestration_allowed(&project));

        write_bumped(&file, "orchestration = tru\n", 20);
        assert!(!engine.orchestration_allowed(&project));
        assert!(
            !engine
                .get_project(&project.id)
                .unwrap()
                .unwrap()
                .orchestration_enabled
        );
        let (settings, warnings) = engine.project_settings(&project);
        let unreadable = settings.unreadable.expect("a broken file is named");
        assert!(unreadable.contains("failed to parse"), "{unreadable}");
        assert_eq!(warnings, vec![unreadable]);

        std::fs::remove_dir_all(&dir).ok();
    }
}
