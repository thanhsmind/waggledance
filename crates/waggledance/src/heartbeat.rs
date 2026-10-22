use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::{interval_at, Instant, MissedTickBehavior};
use waggledance_core::domain::{PermissionRequest, WdTask};
use waggledance_core::indexer::now_rfc3339;
use waggledance_core::Engine;

use crate::paseo::{AgentState, LeaderLock, Paseo, Permit};

const RECIPIENT: &str = "wd";
const WATCH_EVERY: Duration = Duration::from_secs(30);

pub struct Heartbeat {
    paseo: Arc<dyn Paseo>,
    engine: Arc<Engine>,
    data_dir: PathBuf,
}

fn idle(state: &AgentState) -> bool {
    state.pending_permissions == 0 && !matches!(state.status.as_str(), "running" | "initializing")
}

fn silent(task: &WdTask) -> bool {
    match (&task.last_mail_at, &task.last_delivered_at) {
        (None, _) => true,
        (Some(mail), Some(delivered)) => mail < delivered,
        (Some(_), None) => false,
    }
}

fn owner<'a>(tasks: &'a [WdTask], agent_id: &str) -> Option<&'a WdTask> {
    tasks
        .iter()
        .filter(|t| t.leader_id.as_deref() == Some(agent_id))
        .max_by(|a, b| a.last_delivered_at.cmp(&b.last_delivered_at))
}

fn nudge_text(corr: &str) -> String {
    format!(
        "waggledance#{corr}: you are idle and wd has no report on this task since its last \
         delivery. If it is finished, run `waggledance mail send --to wd --corr {corr} --kind \
         result` with the body on stdin, carrying four parts: `Done:`, `Proof:`, `Left:`, \
         `Where:`. If you need a decision, mail `--kind question`. If work remains, continue it \
         and mail the result when done."
    )
}

impl Heartbeat {
    pub fn new(paseo: Arc<dyn Paseo>, engine: Arc<Engine>, data_dir: PathBuf) -> Self {
        Self {
            paseo,
            engine,
            data_dir,
        }
    }

    pub async fn run(self) {
        let every = Duration::from_secs(self.engine.config.openclaw.heartbeat_secs.max(1));
        let watching = async {
            let mut ticks = tokio::time::interval(WATCH_EVERY);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                ticks.tick().await;
                self.watch().await;
            }
        };
        let beating = async {
            let mut ticks = interval_at(Instant::now() + every, every);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                ticks.tick().await;
                self.beat().await;
            }
        };
        tokio::join!(watching, beating);
    }

    fn project_name(&self, project_id: &str) -> String {
        match self.engine.get_project(project_id) {
            Ok(Some(p)) => p.name,
            _ => project_id.to_string(),
        }
    }

    fn mail(&self, corr: &str, project_id: &str, kind: &str, body: &str) -> bool {
        match self.engine.insert_inbox_message(
            RECIPIENT,
            corr,
            project_id,
            kind,
            body,
            &now_rfc3339(),
        ) {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!("heartbeat could not mail wd a {kind} about {corr}: {e}");
                false
            }
        }
    }

    async fn watch(&self) {
        let tasks = match self.engine.list_open_tasks() {
            Ok(tasks) => tasks,
            Err(e) => return tracing::warn!("permission watcher could not list wd tasks: {e}"),
        };
        if !tasks.iter().any(|t| t.leader_id.is_some()) {
            return;
        }
        let rows = match self.paseo.permits().await {
            Ok(rows) => rows,
            Err(e) => return tracing::warn!("permission watcher could not list permits: {e}"),
        };
        for row in &rows {
            if let Some(task) = owner(&tasks, &row.agent_id) {
                self.seen(task, row);
            }
        }
        for task in &tasks {
            let open = match self.engine.unanswered_permission_requests(&task.corr) {
                Ok(open) => open,
                Err(e) => {
                    tracing::warn!("permission watcher could not read {}: {e}", task.corr);
                    continue;
                }
            };
            for request in open {
                if !rows.iter().any(|row| row.id == request.req_id) {
                    self.vanished(task, &request);
                }
            }
        }
    }

    fn seen(&self, task: &WdTask, row: &Permit) {
        let request = PermissionRequest {
            req_id: row.id.clone(),
            corr: task.corr.clone(),
            leader_id: row.agent_id.clone(),
            tool: row.name.clone(),
            description: row.description.clone(),
            seen_at: now_rfc3339(),
            answered_at: None,
            closed_at: None,
        };
        match self.engine.insert_permission_request(&request) {
            Ok(true) => {
                let body = format!(
                    "{} leader {} asks to use {}: {} — req_id {}; answer with permit allow|deny \
                     {} {}",
                    self.project_name(&task.project_id),
                    row.agent_id,
                    row.name,
                    row.description,
                    row.id,
                    task.corr,
                    row.id
                );
                self.mail(&task.corr, &task.project_id, "permission", &body);
            }
            Ok(false) => {}
            Err(e) => tracing::warn!("permission watcher could not record {}: {e}", row.id),
        }
    }

    fn vanished(&self, task: &WdTask, request: &PermissionRequest) {
        if let Err(e) = self
            .engine
            .close_permission_request(&request.req_id, &now_rfc3339())
        {
            return tracing::warn!("permission watcher could not close {}: {e}", request.req_id);
        }
        let body = format!(
            "permission request {} ({}: {}) from leader {} vanished unanswered",
            request.req_id, request.tool, request.description, request.leader_id
        );
        self.mail(&task.corr, &task.project_id, "closed", &body);
    }

    async fn beat(&self) {
        let tasks = match self.engine.list_open_tasks() {
            Ok(tasks) => tasks,
            Err(e) => return tracing::warn!("heartbeat could not list wd tasks: {e}"),
        };
        let mut board = Vec::new();
        for task in &tasks {
            let state = match &task.leader_id {
                Some(leader) => self.paseo.inspect(leader).await.ok(),
                None => None,
            };
            if let (Some(leader), Some(state)) = (&task.leader_id, &state) {
                if task.nudged_at.is_none() {
                    if idle(state) && silent(task) {
                        self.nudge(task, leader).await;
                    }
                } else if task.progress_sent_at.is_none() && silent(task) {
                    self.progress(task, leader, state);
                }
            }
            let shown = match (&task.leader_id, state) {
                (None, _) => "no leader yet".to_string(),
                (Some(_), None) => "unknown".to_string(),
                (Some(_), Some(s)) if s.pending_permissions > 0 => {
                    format!("{} (permission pending)", s.status)
                }
                (Some(_), Some(s)) => s.status,
            };
            board.push(format!(
                "{} {} {shown}",
                task.corr,
                self.project_name(&task.project_id)
            ));
        }
        let changed = tasks.iter().any(|t| {
            t.board_sent_at
                .as_ref()
                .is_none_or(|sent| t.changed_at > *sent)
        });
        if changed && self.mail("board", "all", "board", &board.join("\n")) {
            if let Err(e) = self.engine.stamp_board_sent(&now_rfc3339()) {
                tracing::warn!("heartbeat could not stamp the board: {e}");
            }
        }
    }

    async fn nudge(&self, task: &WdTask, leader: &str) {
        let _lock = match LeaderLock::acquire(&self.data_dir, &task.project_id).await {
            Ok(lock) => lock,
            Err(e) => return tracing::warn!("heartbeat could not lock {}: {e}", task.project_id),
        };
        let fresh = match self.engine.get_task(&task.corr) {
            Ok(Some(fresh)) => fresh,
            _ => return,
        };
        if fresh.nudged_at.is_some()
            || fresh.state != "open"
            || fresh.leader_id.as_deref() != Some(leader)
            || !silent(&fresh)
        {
            return;
        }
        match self.paseo.inspect(leader).await {
            Ok(state) if idle(&state) => {}
            _ => return,
        }
        if let Err(e) = self.paseo.send(leader, &nudge_text(&task.corr)).await {
            return tracing::warn!("heartbeat could not nudge leader {leader}: {e}");
        }
        if let Err(e) = self.engine.set_task_nudged(&task.corr, &now_rfc3339()) {
            tracing::warn!("heartbeat could not record the nudge on {}: {e}", task.corr);
        }
    }

    fn progress(&self, task: &WdTask, leader: &str, state: &AgentState) {
        let body = format!(
            "{} leader {leader} was nudged at {} and has still sent no report; its status is {}",
            self.project_name(&task.project_id),
            task.nudged_at.as_deref().unwrap_or_default(),
            state.status
        );
        if self.mail(&task.corr, &task.project_id, "progress", &body) {
            if let Err(e) = self
                .engine
                .set_task_progress_sent(&task.corr, &now_rfc3339())
            {
                tracing::warn!("heartbeat could not record progress on {}: {e}", task.corr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paseo::fake::{Call, FakePaseo};
    use waggledance_core::domain::{InboxMessage, Project};
    use waggledance_core::{Config, SqliteStore};

    const AT: &str = "2026-10-22T10:00:00Z";

    struct Rig {
        fake: Arc<FakePaseo>,
        engine: Arc<Engine>,
        heartbeat: Heartbeat,
        _dir: tempfile::TempDir,
    }

    fn rig() -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let engine = Arc::new(Engine::new(
            SqliteStore::open_in_memory().unwrap(),
            Config::default(),
        ));
        engine
            .store
            .upsert_project(&Project {
                id: "proj-1".into(),
                name: "waggledance".into(),
                root_path: dir.path().join("proj-1"),
                created_at: AT.into(),
                last_seen_at: AT.into(),
                orchestration_enabled: true,
            })
            .unwrap();
        let fake = Arc::new(FakePaseo::default());
        fake.add_wd_leader("proj-1", "lead", "idle");
        let heartbeat = Heartbeat::new(fake.clone(), engine.clone(), dir.path().join("data"));
        Rig {
            fake,
            engine,
            heartbeat,
            _dir: dir,
        }
    }

    impl Rig {
        fn task(&self, corr: &str, delivered: &str) {
            self.engine.insert_task(corr, "proj-1", AT).unwrap();
            self.engine
                .mark_task_delivered(corr, "lead", delivered)
                .unwrap();
        }

        fn permits(&self, rows: &[(&str, &str)]) {
            self.fake.state.lock().unwrap().permits = rows
                .iter()
                .map(|(id, agent)| Permit {
                    id: id.to_string(),
                    agent_id: agent.to_string(),
                    name: "Bash".into(),
                    description: "rm -rf build".into(),
                })
                .collect();
        }

        fn inbox(&self) -> Vec<InboxMessage> {
            let mut got = Vec::new();
            while let Some(m) = self.engine.next_inbox_message(RECIPIENT).unwrap() {
                self.engine
                    .ack_inbox_message(m.seq, &now_rfc3339())
                    .unwrap();
                got.push(m);
            }
            got
        }

        fn kinds(&self, kind: &str) -> Vec<InboxMessage> {
            self.inbox()
                .into_iter()
                .filter(|m| m.kind == kind)
                .collect()
        }

        fn nudges(&self) -> Vec<String> {
            self.fake
                .calls()
                .into_iter()
                .filter_map(|c| match c {
                    Call::Send(to, text) if to == "lead" => Some(text),
                    _ => None,
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn one_watcher_pass_writes_one_permission_message_per_new_request_and_a_second_none() {
        let r = rig();
        r.task("c-old", "2026-10-22T10:01:00Z");
        r.task("c-new", "2026-10-22T10:02:00Z");
        r.permits(&[("req-1", "lead"), ("req-2", "someone-else")]);
        r.heartbeat.watch().await;
        let mail = r.inbox();
        assert_eq!(mail.len(), 1, "{mail:?}");
        let m = &mail[0];
        assert_eq!((m.kind.as_str(), m.corr.as_str()), ("permission", "c-new"));
        for part in ["waggledance", "Bash", "rm -rf build", "req-1", "c-new"] {
            assert!(m.body.contains(part), "{part} missing from {}", m.body);
        }
        r.heartbeat.watch().await;
        assert!(r.inbox().is_empty());
        assert!(!r.fake.calls().iter().any(|c| matches!(c, Call::Permit(..))));
    }

    #[tokio::test]
    async fn a_vanished_unanswered_request_writes_one_closed_message() {
        let r = rig();
        r.task("c-1", "2026-10-22T10:01:00Z");
        r.permits(&[("req-1", "lead"), ("req-2", "lead")]);
        r.heartbeat.watch().await;
        assert_eq!(r.inbox().len(), 2);
        r.engine.answer_permission_request("req-2", AT).unwrap();
        r.permits(&[]);
        r.heartbeat.watch().await;
        let mail = r.inbox();
        assert_eq!(mail.len(), 1, "{mail:?}");
        assert_eq!(
            (mail[0].kind.as_str(), mail[0].corr.as_str()),
            ("closed", "c-1")
        );
        assert!(mail[0].body.contains("req-1"), "{}", mail[0].body);
        r.heartbeat.watch().await;
        assert!(r.inbox().is_empty());
    }

    #[tokio::test]
    async fn a_silent_idle_leader_is_nudged_once_then_wd_hears_progress_once() {
        let r = rig();
        r.task("c-1", "2026-10-22T10:01:00Z");
        r.heartbeat.beat().await;
        let nudges = r.nudges();
        assert_eq!(nudges.len(), 1, "{nudges:?}");
        assert!(nudges[0].contains("waggledance mail send --to wd --corr c-1 --kind result"));
        assert!(r
            .engine
            .get_task("c-1")
            .unwrap()
            .unwrap()
            .nudged_at
            .is_some());
        assert!(r.kinds("progress").is_empty());

        r.heartbeat.beat().await;
        assert_eq!(r.nudges().len(), 1);
        let progress = r.kinds("progress");
        assert_eq!(progress.len(), 1, "{progress:?}");
        assert_eq!(progress[0].corr, "c-1");

        r.heartbeat.beat().await;
        assert_eq!(r.nudges().len(), 1);
        assert!(r.kinds("progress").is_empty());
    }

    #[tokio::test]
    async fn a_busy_or_reporting_leader_is_never_nudged() {
        let r = rig();
        r.task("c-1", "2026-10-22T10:01:00Z");
        r.fake.set_status("lead", "running");
        r.heartbeat.beat().await;
        r.fake.set_status("lead", "idle");
        r.engine
            .mark_task_mail("c-1", "2026-10-22T10:05:00Z")
            .unwrap();
        r.heartbeat.beat().await;
        r.heartbeat.beat().await;
        assert!(r.nudges().is_empty());
        assert!(r.kinds("progress").is_empty());
    }

    #[tokio::test]
    async fn a_board_goes_out_only_when_a_task_changed() {
        let r = rig();
        r.task("c-1", "2026-10-22T10:01:00Z");
        r.engine.insert_task("c-2", "proj-1", AT).unwrap();
        r.heartbeat.beat().await;
        let boards = r.kinds("board");
        assert_eq!(boards.len(), 1, "{boards:?}");
        assert_eq!(
            (boards[0].corr.as_str(), boards[0].project_id.as_str()),
            ("board", "all")
        );
        assert_eq!(
            boards[0].body,
            "c-1 waggledance idle\nc-2 waggledance no leader yet"
        );

        r.heartbeat.beat().await;
        assert!(r.kinds("board").is_empty());

        r.engine.mark_task_mail("c-1", &now_rfc3339()).unwrap();
        r.heartbeat.beat().await;
        assert_eq!(r.kinds("board").len(), 1);
    }
}
