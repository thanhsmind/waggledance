use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::Instant;
use waggledance_core::domain::Run;
use waggledance_core::indexer::now_rfc3339;
use waggledance_core::notify_store::NotifyStore;
use waggledance_core::Engine;

use crate::discovery::{self, Bounds};
use crate::orchestrate::{self, PASEO_PANE_PREFIX};
use crate::paseo::{self, Leader, LeaderLock, ModeCheck, Paseo, PaseoError, TurnEnd};

pub const TICK: Duration = Duration::from_secs(2);
const MAX_ATTEMPTS: i64 = 5;
const NOTICE_WINDOW: Duration = Duration::from_secs(3600);
const NOTICE_GAP: Duration = Duration::from_secs(30);
const MAY_STILL_FINISH: &str = "the leader may still be finishing";
const RESTARTED: &str = "daemon restarted — the leader may still be finishing; check its chat";

pub type NotifySlot = Arc<Mutex<Option<Arc<NotifyStore>>>>;

#[derive(Clone)]
pub struct Delivery {
    paseo: Arc<dyn Paseo>,
    engine: Arc<Engine>,
    data_dir: PathBuf,
    notify: NotifySlot,
    state: Arc<Mutex<State>>,
    backoff: [Duration; 4],
    permission_poll: Duration,
    mode_settle: Duration,
    mode_poll: Duration,
    bounds: Bounds,
}

#[derive(Default)]
struct State {
    recovered: bool,
    discovering: bool,
    headers: HashMap<String, String>,
    busy: HashSet<String>,
    held: HashSet<String>,
    not_before: HashMap<String, Instant>,
    quiet_until: HashMap<String, Instant>,
}

struct Notice {
    run_id: String,
    text: String,
}

fn notice_until() -> String {
    let until =
        time::OffsetDateTime::now_utc() + time::Duration::seconds(NOTICE_WINDOW.as_secs() as i64);
    until
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn notice_expired(until: &str) -> bool {
    match time::OffsetDateTime::parse(until, &time::format_description::well_known::Rfc3339) {
        Ok(at) => at <= time::OffsetDateTime::now_utc(),
        Err(e) => {
            tracing::warn!("delivery could not read notice expiry {until}: {e}");
            false
        }
    }
}

fn pane(agent: Option<&str>) -> String {
    format!("{PASEO_PANE_PREFIX}{}", agent.unwrap_or("pending"))
}

fn notice_text(project: &str, run_id: &str, status: &str) -> String {
    if status == "blocked" {
        format!(
            "[waggledance] {project} leader is waiting for a permission in its Paseo chat \
             (run {run_id}) — approve or deny it there."
        )
    } else {
        format!(
            "[waggledance] {project} leader finished run {run_id} ({status}) — informational, \
             no action needed; read it with waggledance_await {{\"run_id\": \"{run_id}\"}}."
        )
    }
}

fn leader_reply(logs: &str, task: &str) -> String {
    let head = format!("[User] {}", task.lines().next().unwrap_or_default());
    let Some(at) = logs.rfind(&head) else {
        return String::new();
    };
    let after = &logs[at..];
    let rest = after
        .strip_prefix(&format!("[User] {task}"))
        .unwrap_or(&after[head.len()..]);
    rest.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("[User]"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Delivery {
    pub fn new(
        paseo: Arc<dyn Paseo>,
        engine: Arc<Engine>,
        data_dir: PathBuf,
        notify: NotifySlot,
    ) -> Self {
        Self {
            paseo,
            engine,
            data_dir,
            notify,
            state: Arc::default(),
            backoff: [10, 30, 60, 300].map(Duration::from_secs),
            permission_poll: Duration::from_secs(10),
            mode_settle: Duration::from_secs(10),
            mode_poll: Duration::from_secs(1),
            bounds: Bounds::host(),
        }
    }

    pub async fn run(self) {
        let mut deliveries = JoinSet::new();
        let mut ticks = tokio::time::interval(TICK);
        let mut discoveries = tokio::time::interval(discovery::EVERY);
        loop {
            tokio::select! {
                _ = ticks.tick() => {
                    while deliveries.try_join_next().is_some() {}
                    self.tick(&mut deliveries).await;
                }
                _ = discoveries.tick() => self.discover(&mut deliveries),
            }
        }
    }

    fn discover(&self, tasks: &mut JoinSet<()>) {
        if std::mem::replace(&mut self.state.lock().unwrap().discovering, true) {
            return;
        }
        let this = self.clone();
        tasks.spawn(async move {
            discovery::discover(this.paseo.as_ref(), &this.engine, &this.bounds).await;
            this.state.lock().unwrap().discovering = false;
        });
    }

    async fn tick(&self, deliveries: &mut JoinSet<()>) {
        self.beat();
        let first = !std::mem::replace(&mut self.state.lock().unwrap().recovered, true);
        if first {
            self.recover();
        }
        self.flush_notices().await;
        let held: Vec<String> = self.state.lock().unwrap().held.iter().cloned().collect();
        for project_id in held {
            if self.leader_idle(&project_id).await {
                self.state.lock().unwrap().held.remove(&project_id);
            }
        }
        let projects = self
            .engine
            .list_projects_with_queued_paseo_runs()
            .unwrap_or_else(|e| {
                tracing::warn!("delivery could not list queued leader runs: {e}");
                Vec::new()
            });
        for project_id in projects {
            let Some(run) = self.next_run(&project_id) else {
                continue;
            };
            {
                let mut s = self.state.lock().unwrap();
                if s.busy.contains(&project_id) || s.held.contains(&project_id) {
                    continue;
                }
                s.busy.insert(project_id.clone());
            }
            let this = self.clone();
            deliveries.spawn(async move {
                this.deliver(&run).await;
                this.state.lock().unwrap().busy.remove(&run.project_id);
            });
        }
    }

    fn beat(&self) {
        let path = paseo::delivery_heartbeat_path(&self.data_dir);
        if let Err(e) = std::fs::create_dir_all(&self.data_dir)
            .and_then(|()| std::fs::write(&path, now_rfc3339()))
        {
            tracing::warn!("delivery heartbeat {} failed: {e}", path.display());
        }
    }

    fn recover(&self) {
        let runs = self.engine.list_inflight_paseo_runs().unwrap_or_else(|e| {
            tracing::warn!("delivery could not list in-flight leader runs: {e}");
            Vec::new()
        });
        for run in runs {
            let agent = run
                .pane_id
                .strip_prefix(PASEO_PANE_PREFIX)
                .filter(|a| *a != "pending");
            if agent.is_none() && run.status == "working" {
                self.set_status(&run.id, "queued");
                continue;
            }
            self.state
                .lock()
                .unwrap()
                .held
                .insert(run.project_id.clone());
            self.end(&self.name_of(&run), &run, "lost", Some(RESTARTED), agent);
        }
    }

    fn next_run(&self, project_id: &str) -> Option<Run> {
        let run = self
            .engine
            .list_queued_paseo_runs(project_id)
            .map_err(|e| tracing::warn!("delivery could not list queued runs: {e}"))
            .ok()?
            .into_iter()
            .next()?;
        let s = self.state.lock().unwrap();
        match s.not_before.get(&run.id) {
            Some(at) if Instant::now() < *at => None,
            _ => Some(run),
        }
    }

    async fn leader_idle(&self, project_id: &str) -> bool {
        let Ok(ids) = self.paseo.leaders(project_id).await else {
            return false;
        };
        for id in ids {
            match self.paseo.inspect(&id).await {
                Ok(s) if s.archived => {}
                Ok(s)
                    if s.pending_permissions == 0
                        && !matches!(s.status.as_str(), "running" | "initializing") => {}
                _ => return false,
            }
        }
        true
    }

    fn name_of(&self, run: &Run) -> String {
        match self.engine.get_project(&run.project_id) {
            Ok(Some(p)) => p.name,
            _ => run.project_id.clone(),
        }
    }

    async fn deliver(&self, run: &Run) {
        let lock = LeaderLock::acquire(&self.data_dir, &run.project_id).await;
        match self.engine.claim_queued_run(&run.id) {
            Ok(true) => {}
            Ok(false) => return,
            Err(e) => {
                tracing::warn!("delivery could not claim run {}: {e}", run.id);
                return;
            }
        }
        let name = self.name_of(run);
        let _lock = match lock {
            Ok(lock) => lock,
            Err(e) => return self.presend_failed(&name, run, &e.to_string()),
        };
        let project = match self.engine.get_project(&run.project_id) {
            Ok(Some(p)) => p,
            Ok(None) => {
                let error = format!("project {} is not registered", run.project_id);
                return self.presend_failed(&name, run, &error);
            }
            Err(e) => return self.presend_failed(&name, run, &e.to_string()),
        };
        let config = &self.engine.config.paseo;
        let timeout = Duration::from_secs(config.turn_timeout_secs);
        let deadline = Instant::now() + timeout;
        let (settings, warnings) = self.engine.project_settings(&project);
        for warning in warnings {
            tracing::warn!("delivery for {name}: {warning}");
        }
        let wd = matches!(self.engine.run_caller(&run.id), Ok(Some(c)) if c == paseo::WD_CALLER);
        let paseo = self.paseo.as_ref();
        let leader = match paseo::resolve_leader(paseo, config, &settings.paseo, &project, wd).await
        {
            Ok(leader) => leader,
            Err(e) => return self.presend_failed(&name, run, &e.to_string()),
        };
        let mode = paseo::leader_mode(config, &settings.paseo);
        let sent_at = now_rfc3339();
        let (agent, outcome) = match leader {
            Leader::Existing { busy: true, .. } => {
                self.set_status(&run.id, "queued");
                self.state.lock().unwrap().held.insert(project.id);
                return;
            }
            Leader::Existing { id, .. } => {
                self.check_mode(run, &id, &mode).await;
                self.sent(run, &id, wd, &sent_at);
                let outcome = self.paseo.send_and_wait(&id, &run.task, timeout).await;
                (Some(id), outcome)
            }
            Leader::Start(start) => {
                tracing::info!(
                    run = %run.id,
                    workspace = %start.workspace,
                    reason = start.reason,
                    "delivery starts the {name} leader"
                );
                self.state.lock().unwrap().headers.insert(
                    run.id.clone(),
                    format!(
                        "[waggledance] leader started in Paseo workspace {} ({})",
                        start.workspace, start.reason
                    ),
                );
                let known = paseo::leader_ids(paseo, &project.id, wd)
                    .await
                    .unwrap_or_default();
                let spec = start.spec(&run.task);
                let started = paseo.run_and_wait(&spec, timeout);
                let finder = async {
                    let id = self.new_leader(&project.id, wd, &known).await;
                    self.check_mode(run, &id, &mode).await;
                };
                tokio::pin!(started, finder);
                let mut checked = false;
                let result = loop {
                    tokio::select! {
                        result = &mut started => break result,
                        () = &mut finder, if !checked => checked = true,
                    }
                };
                match result {
                    Ok((id, end)) => {
                        if !checked {
                            self.check_mode(run, &id, &mode).await;
                        }
                        self.sent(run, &id, wd, &sent_at);
                        (Some(id), Ok(end))
                    }
                    Err(e) => (None, Err(e)),
                }
            }
        };
        let run = Run {
            pane_id: pane(agent.as_deref()),
            ..run.clone()
        };
        let agent = agent.as_deref();
        match outcome {
            Err(e @ (PaseoError::Spawn { .. } | PaseoError::PromptFile(_))) => {
                self.presend_failed(&name, &run, &e.to_string())
            }
            Err(e) => self.lose(&name, &run, agent, &e.to_string()),
            Ok(TurnEnd::Completed) => self.done(&name, &run, agent).await,
            Ok(TurnEnd::Permission) => match agent {
                Some(agent) => self.blocked_then_done(&name, &run, agent, deadline).await,
                None => self.lose(&name, &run, None, "permission requested"),
            },
            Ok(TurnEnd::Other { status, message }) => self.lose(
                &name,
                &run,
                agent,
                &format!("paseo reported {status}: {message}"),
            ),
        }
    }

    async fn new_leader(&self, project_id: &str, wd: bool, known: &[String]) -> String {
        loop {
            if let Ok(ids) = paseo::leader_ids(self.paseo.as_ref(), project_id, wd).await {
                if let Some(id) = ids.into_iter().find(|id| !known.contains(id)) {
                    return id;
                }
            }
            tokio::time::sleep(self.mode_poll).await;
        }
    }

    async fn check_mode(&self, run: &Run, id: &str, mode: &str) {
        let check = paseo::ensure_mode(
            self.paseo.as_ref(),
            id,
            mode,
            self.mode_settle,
            self.mode_poll,
        )
        .await;
        let ModeCheck::Mismatch { runtime } = check else {
            return;
        };
        let runtime = runtime.as_deref().unwrap_or("unknown");
        tracing::warn!(run = %run.id, agent = id, asked = mode, runtime, "delivery leader runs in the wrong mode");
        let line = format!(
            "[waggledance] leader {id} runs in mode {runtime}, asked {mode}; set it in Paseo"
        );
        let mut s = self.state.lock().unwrap();
        match s.headers.get_mut(&run.id) {
            Some(header) => {
                header.push('\n');
                header.push_str(&line);
            }
            None => {
                s.headers.insert(run.id.clone(), line);
            }
        }
    }

    async fn blocked_then_done(&self, name: &str, run: &Run, agent: &str, deadline: Instant) {
        loop {
            self.end(name, run, "blocked", None, Some(agent));
            loop {
                if Instant::now() >= deadline {
                    return self.lose(name, run, Some(agent), "permission still pending");
                }
                tokio::time::sleep(self.permission_poll).await;
                match self.paseo.inspect(agent).await {
                    Ok(s) if s.pending_permissions == 0 => break,
                    Ok(_) => {}
                    Err(e) => tracing::warn!("delivery could not inspect leader {agent}: {e}"),
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.paseo.wait(agent, remaining).await {
                Ok(TurnEnd::Completed) => return self.done(name, run, Some(agent)).await,
                Ok(TurnEnd::Other { status, .. }) if status == "idle" => {
                    return self.done(name, run, Some(agent)).await
                }
                Ok(TurnEnd::Permission) => {}
                Ok(TurnEnd::Other { status, message }) => {
                    let why = format!("paseo reported {status}: {message}");
                    return self.lose(name, run, Some(agent), &why);
                }
                Err(e) => return self.lose(name, run, Some(agent), &e.to_string()),
            }
        }
    }

    fn mail(&self, run: &Run, to: &str, kind: &str, body: &str) {
        let corr = paseo::wd_corr(&run.task).unwrap_or(&run.id);
        let now = now_rfc3339();
        if let Err(e) =
            self.engine
                .insert_inbox_message(to, corr, &run.project_id, kind, body, &now)
        {
            tracing::warn!("delivery could not mail {to} about run {}: {e}", run.id);
        }
    }

    async fn done(&self, name: &str, run: &Run, agent: Option<&str>) {
        let logs = match agent {
            Some(agent) => self
                .paseo
                .logs_since(agent, &run.baseline)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("delivery could not read leader {agent} logs: {e}");
                    String::new()
                }),
            None => String::new(),
        };
        let reply = leader_reply(&logs, &run.task);
        self.end(name, run, "done", Some(&reply), agent);
    }

    fn lose(&self, name: &str, run: &Run, agent: Option<&str>, why: &str) {
        self.state
            .lock()
            .unwrap()
            .held
            .insert(run.project_id.clone());
        let transcript = format!("{MAY_STILL_FINISH} ({why})");
        self.end(name, run, "lost", Some(&transcript), agent);
    }

    fn presend_failed(&self, name: &str, run: &Run, error: &str) {
        self.set_pane(&run.id, None);
        let run = Run {
            pane_id: pane(None),
            ..run.clone()
        };
        let attempts = self
            .engine
            .run_attempts(&run.id)
            .ok()
            .flatten()
            .unwrap_or(0)
            + 1;
        if attempts >= MAX_ATTEMPTS {
            return self.end(name, &run, "lost", Some(error), None);
        }
        tracing::warn!("delivery of run {} failed before the send: {error}", run.id);
        if let Err(e) = self.engine.requeue_run(&run.id) {
            tracing::warn!("delivery could not requeue run {}: {e}", run.id);
        }
        let wait = self.backoff[(attempts as usize - 1).min(self.backoff.len() - 1)];
        self.state
            .lock()
            .unwrap()
            .not_before
            .insert(run.id, Instant::now() + wait);
    }

    fn end(
        &self,
        name: &str,
        run: &Run,
        status: &str,
        transcript: Option<&str>,
        leader: Option<&str>,
    ) {
        let first = transcript
            .and_then(|t| t.lines().next())
            .unwrap_or_default();
        let transcript = transcript.map(|text| {
            let header = self.state.lock().unwrap().headers.remove(&run.id);
            header.map_or_else(|| text.to_string(), |h| format!("{h}\n\n{text}"))
        });
        let store = self.notify.lock().unwrap().clone();
        if let Some(store) = store {
            orchestrate::enqueue_run_notice(&store, run, status);
        }
        let caller = match self.engine.run_caller(&run.id) {
            Ok(Some(caller)) if Some(caller.as_str()) != leader => Some(caller),
            _ => None,
        };
        let mailbox = caller.as_deref().and_then(|c| c.strip_prefix("openclaw:"));
        if let (Some(to), "lost") = (mailbox, status) {
            let leader = leader.unwrap_or("(not started)");
            let body = format!("{name} leader {leader} lost run {}: {first}", run.id);
            self.mail(run, to, "error", &body);
        }
        let caller = caller.filter(|c| !c.starts_with("openclaw:"));
        let until = notice_until();
        let notice = caller.map(|_| (notice_text(name, &run.id, status), until));
        let notice = notice
            .as_ref()
            .map(|(text, until)| (text.as_str(), until.as_str()));
        if let Err(e) = self.engine.finish_run(
            &run.id,
            status,
            &now_rfc3339(),
            transcript.as_deref(),
            notice,
        ) {
            tracing::warn!("delivery could not end run {}: {e}", run.id);
        }
    }

    async fn flush_notices(&self) {
        let pending = self.engine.pending_notices().unwrap_or_else(|e| {
            tracing::warn!("delivery could not read pending notices: {e}");
            Vec::new()
        });
        let mut batches: Vec<(String, Vec<Notice>)> = Vec::new();
        for (run_id, caller, text, until) in pending {
            if notice_expired(&until) {
                tracing::warn!("delivery dropped a notice to {caller} after 1h");
                if let Err(e) = self.engine.set_notice_state(&run_id, "dropped") {
                    tracing::warn!("delivery could not mark run {run_id}'s notice dropped: {e}");
                }
                continue;
            }
            if caller.starts_with("openclaw:") {
                continue;
            }
            match batches.iter_mut().find(|(c, _)| *c == caller) {
                Some((_, batch)) => batch.push(Notice { run_id, text }),
                None => batches.push((caller, vec![Notice { run_id, text }])),
            }
        }
        for (caller, batch) in batches {
            let quiet = self.state.lock().unwrap().quiet_until.get(&caller).copied();
            let ready = quiet.is_none_or(|at| Instant::now() >= at)
                && match self.paseo.inspect(&caller).await {
                    Ok(s) => s.status == "idle",
                    Err(e) => {
                        tracing::warn!("delivery could not inspect caller {caller}: {e}");
                        false
                    }
                };
            if ready {
                let text = batch
                    .iter()
                    .map(|n| n.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                match self.paseo.send(&caller, &text).await {
                    Ok(()) => {
                        self.state
                            .lock()
                            .unwrap()
                            .quiet_until
                            .insert(caller, Instant::now() + NOTICE_GAP);
                        for notice in &batch {
                            if let Err(e) = self.engine.set_notice_state(&notice.run_id, "sent") {
                                tracing::warn!(
                                    "delivery could not mark run {}'s notice sent: {e}",
                                    notice.run_id
                                );
                            }
                        }
                        continue;
                    }
                    Err(e) => tracing::warn!("delivery could not notify caller {caller}: {e}"),
                }
            }
        }
    }

    fn set_status(&self, run_id: &str, status: &str) {
        if let Err(e) = self
            .engine
            .update_run_status(run_id, status, &now_rfc3339(), None, None)
        {
            tracing::warn!("delivery could not mark run {run_id} {status}: {e}");
        }
    }

    fn sent(&self, run: &Run, agent: &str, wd: bool, at: &str) {
        self.set_pane(&run.id, Some(agent));
        let Some(corr) = wd.then(|| paseo::wd_corr(&run.task)).flatten() else {
            return;
        };
        if let Err(e) = self.engine.mark_task_delivered(corr, agent, at) {
            tracing::warn!("delivery could not record task {corr}'s leader: {e}");
        }
    }

    fn set_pane(&self, run_id: &str, agent: Option<&str>) {
        if let Err(e) = self.engine.set_run_pane(run_id, &pane(agent)) {
            tracing::warn!("delivery could not record run {run_id}'s leader: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paseo::fake::{Call, FakePaseo};
    use waggledance_core::domain::Project;
    use waggledance_core::{Config, SqliteStore};

    struct Rig {
        fake: Arc<FakePaseo>,
        engine: Arc<Engine>,
        delivery: Delivery,
        _dir: tempfile::TempDir,
    }

    fn rig() -> Rig {
        rig_with(Config::default())
    }

    fn rig_with(mut config: Config) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        config.paseo.enabled = Some(true);
        let engine = Arc::new(Engine::new(SqliteStore::open_in_memory().unwrap(), config));
        for id in ["proj-1", "proj-2"] {
            let root = dir.path().join(id);
            std::fs::create_dir_all(&root).unwrap();
            engine
                .store
                .upsert_project(&Project {
                    id: id.into(),
                    name: format!("{id}-name"),
                    root_path: root,
                    created_at: now_rfc3339(),
                    last_seen_at: now_rfc3339(),
                    orchestration_enabled: true,
                })
                .unwrap();
        }
        let fake = Arc::new(FakePaseo::default());
        let mut delivery = Delivery::new(
            fake.clone(),
            engine.clone(),
            dir.path().join("data"),
            Arc::default(),
        );
        delivery.backoff = [Duration::ZERO; 4];
        delivery.permission_poll = Duration::from_millis(1);
        delivery.mode_settle = Duration::from_millis(10);
        delivery.mode_poll = Duration::from_millis(1);
        Rig {
            fake,
            engine,
            delivery,
            _dir: dir,
        }
    }

    impl Rig {
        fn queue(&self, project: &str, id: &str, task: &str, caller: Option<&str>) {
            self.queue_at(project, id, task, caller, "2026-10-14T14:00:00Z");
        }

        fn queue_at(&self, project: &str, id: &str, task: &str, caller: Option<&str>, at: &str) {
            let run = Run {
                id: id.into(),
                project_id: project.into(),
                pane_id: "paseo:pending".into(),
                preset_label: None,
                task: task.into(),
                baseline: at.into(),
                marker: String::new(),
                status: "queued".into(),
                created_at: at.into(),
                updated_at: at.into(),
            };
            self.engine
                .insert_run_with_request(&run, None, None, caller)
                .unwrap();
        }

        fn status(&self, id: &str) -> String {
            self.engine.get_run(id).unwrap().unwrap().status
        }

        fn transcript(&self, id: &str) -> String {
            self.engine
                .run_final_transcript(id)
                .unwrap()
                .unwrap_or_default()
        }

        async fn tick(&self) {
            let mut set = JoinSet::new();
            self.delivery.tick(&mut set).await;
            while let Some(r) = set.join_next().await {
                r.unwrap();
            }
        }

        fn sends_to(&self, id: &str) -> Vec<String> {
            self.fake
                .calls()
                .into_iter()
                .filter_map(|c| match c {
                    Call::SendAndWait(to, text) if to == id => Some(text),
                    _ => None,
                })
                .collect()
        }

        fn notices(&self) -> Vec<(String, String)> {
            self.fake
                .calls()
                .into_iter()
                .filter_map(|c| match c {
                    Call::Send(to, text) => Some((to, text)),
                    _ => None,
                })
                .collect()
        }

        fn notice_state(&self, id: &str) -> Option<String> {
            self.engine.run_notice_state(id).unwrap()
        }

        fn set_logs(&self, id: &str, logs: &str) {
            self.fake
                .state
                .lock()
                .unwrap()
                .logs
                .insert(id.into(), logs.into());
        }
    }

    #[tokio::test]
    async fn delivery_sends_one_run_per_project_per_tick_in_order_and_marks_done() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.queue_at("proj-1", "run-a", "first", None, "2026-10-14T14:00:01Z");
        r.queue_at("proj-1", "run-b", "second", None, "2026-10-14T14:00:02Z");
        r.queue_at("proj-1", "run-c", "third", None, "2026-10-14T14:00:03Z");
        r.set_logs("lead", "[User] first\nreply one\n");

        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec!["first"]);
        assert_eq!(
            (r.status("run-a"), r.status("run-b"), r.status("run-c")),
            ("done".into(), "queued".into(), "queued".into())
        );
        assert_eq!(r.transcript("run-a"), "reply one");
        assert_eq!(
            r.engine.get_run("run-a").unwrap().unwrap().pane_id,
            "paseo:lead"
        );
        r.tick().await;
        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec!["first", "second", "third"]);
        assert_eq!(r.status("run-c"), "done");
        assert!(std::fs::metadata(paseo::delivery_heartbeat_path(&r.delivery.data_dir)).is_ok());
    }

    #[tokio::test]
    async fn delivery_serves_two_projects_in_the_same_tick() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead-1", "idle", false);
        r.fake.add_leader("proj-2", "lead-2", "idle", false);
        r.queue("proj-1", "run-1", "one", None);
        r.queue("proj-2", "run-2", "two", None);
        r.tick().await;
        assert_eq!(r.sends_to("lead-1"), vec!["one"]);
        assert_eq!(r.sends_to("lead-2"), vec!["two"]);
    }

    #[tokio::test]
    async fn delivery_skips_a_project_with_a_delivery_in_flight() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.queue("proj-1", "run-1", "one", None);
        r.delivery
            .state
            .lock()
            .unwrap()
            .busy
            .insert("proj-1".into());
        r.tick().await;
        assert!(r.sends_to("lead").is_empty());
        assert_eq!(r.status("run-1"), "queued");
    }

    #[tokio::test]
    async fn delivery_never_sends_to_a_running_leader_and_waits_for_idle() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "running", false);
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        r.tick().await;
        assert!(r.sends_to("lead").is_empty());
        assert_eq!(r.status("run-1"), "queued");
        assert_eq!(r.engine.run_attempts("run-1").unwrap(), Some(0));
        r.fake.set_status("lead", "idle");
        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec!["one"]);
        assert_eq!(r.status("run-1"), "done");
    }

    #[tokio::test]
    async fn delivery_starts_a_missing_leader_with_the_task() {
        let r = rig();
        r.queue("proj-1", "run-1", "build it", None);
        r.fake.add_workspace(
            "wks",
            "local",
            &r.engine.get_project("proj-1").unwrap().unwrap().root_path,
        );
        r.tick().await;
        let runs: Vec<_> = r
            .fake
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::Run(spec) => Some(spec.task),
                _ => None,
            })
            .collect();
        assert_eq!(runs, vec!["build it"]);
        let run = r.engine.get_run("run-1").unwrap().unwrap();
        assert_eq!(run.status, "done");
        assert_eq!(run.pane_id, "paseo:agent-1");
        assert!(r
            .transcript("run-1")
            .starts_with("[waggledance] leader started in Paseo workspace wks (lowest-id)"));
    }

    fn root_workspace(r: &Rig) {
        r.fake.add_workspace(
            "wks",
            "local",
            &r.engine.get_project("proj-1").unwrap().unwrap().root_path,
        );
    }

    fn set_modes(r: &Rig) -> Vec<(String, String)> {
        r.fake
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::SetMode(id, mode) => Some((id, mode)),
                _ => None,
            })
            .collect()
    }

    fn at(r: &Rig, call: &Call) -> usize {
        r.fake
            .calls()
            .iter()
            .position(|c| c == call)
            .unwrap_or_else(|| panic!("{call:?} never called"))
    }

    #[tokio::test]
    async fn a_new_leader_still_in_the_wrong_mode_is_named_in_the_reply_and_the_run_ends_done() {
        let r = rig();
        root_workspace(&r);
        r.fake.script_modes("agent-1", &[Some("default")], true);
        r.set_logs("agent-1", "[User] build it\nbuilt\n");
        r.queue("proj-1", "run-1", "build it", None);
        r.tick().await;
        let spec = r
            .fake
            .calls()
            .into_iter()
            .find_map(|c| match c {
                Call::Run(spec) => Some(spec),
                _ => None,
            })
            .unwrap();
        assert_eq!(spec.mode, "bypassPermissions");
        assert_eq!(
            set_modes(&r),
            [("agent-1".to_string(), "bypassPermissions".to_string())]
        );
        assert_eq!(r.status("run-1"), "done");
        assert_eq!(
            r.transcript("run-1"),
            "[waggledance] leader started in Paseo workspace wks (lowest-id)\n\
             [waggledance] leader agent-1 runs in mode default, asked bypassPermissions; \
             set it in Paseo\n\nbuilt"
        );
    }

    #[tokio::test]
    async fn a_new_leader_that_appears_late_is_repaired_while_its_first_turn_runs() {
        let r = rig();
        root_workspace(&r);
        {
            let mut s = r.fake.state.lock().unwrap();
            s.appear_after = Duration::from_millis(20);
            s.run_after = Duration::from_millis(200);
        }
        r.fake.script_modes("agent-1", &[Some("default")], false);
        r.queue("proj-1", "run-1", "build it", None);
        r.tick().await;
        let set = Call::SetMode("agent-1".into(), "bypassPermissions".into());
        assert_eq!(set_modes(&r).len(), 1);
        assert!(at(&r, &set) < at(&r, &Call::RunDone("agent-1".into())));
        assert_eq!(r.status("run-1"), "done");
        assert!(!r.transcript("run-1").contains("runs in mode"));
    }

    #[tokio::test]
    async fn an_existing_leader_is_set_to_the_asked_mode_before_the_task_is_sent() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.fake.script_modes("lead", &[Some("default")], false);
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        let set = Call::SetMode("lead".into(), "bypassPermissions".into());
        assert_eq!(set_modes(&r).len(), 1);
        assert!(at(&r, &set) < at(&r, &Call::SendAndWait("lead".into(), "one".into())));
        assert_eq!(r.status("run-1"), "done");
    }

    #[tokio::test]
    async fn an_empty_global_mode_starts_without_a_mode_and_checks_nothing() {
        let mut config = Config::default();
        config.paseo.mode = String::new();
        let r = rig_with(config);
        root_workspace(&r);
        r.fake.add_leader("proj-2", "lead", "idle", false);
        r.fake.script_modes("lead", &[Some("default")], true);
        r.queue("proj-1", "run-1", "build it", None);
        r.queue("proj-2", "run-2", "two", None);
        r.tick().await;
        assert_eq!(
            (r.status("run-1"), r.status("run-2")),
            ("done".into(), "done".into())
        );
        assert!(r.fake.calls().iter().all(|c| match c {
            Call::Run(spec) => spec.mode.is_empty(),
            Call::RuntimeMode(_) | Call::SetMode(..) => false,
            _ => true,
        }));
    }

    #[tokio::test]
    async fn delivery_discovers_paseo_workspaces_when_it_starts() {
        let mut r = rig();
        r.delivery.bounds = Bounds {
            home: None,
            tmp: PathBuf::from("/nonexistent-tmp"),
        };
        let repo = std::fs::canonicalize(r._dir.path()).unwrap().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        r.fake.add_workspace("wks_repo", "local", &repo);
        let task = tokio::spawn(r.delivery.clone().run());
        let mut found = false;
        for _ in 0..200 {
            found = r
                .engine
                .list_projects()
                .unwrap()
                .iter()
                .any(|p| p.root_path == repo);
            if found {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        task.abort();
        assert!(
            found,
            "the delivery task registers a Paseo workspace root at start"
        );
    }

    #[tokio::test]
    async fn delivery_marks_a_permission_blocked_then_done_once_idle() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.fake.add_leader("proj-9", "caller", "idle", false);
        r.fake.set_permissions("lead", &["Write"]);
        r.fake.push_turn_end(TurnEnd::Permission);
        r.fake.push_turn_end(TurnEnd::Other {
            status: "idle".into(),
            message: "Agent is idle.".into(),
        });
        r.set_logs("lead", "[User] one\nwrote it\n");
        r.queue("proj-1", "run-1", "one", Some("caller"));

        let mut set = JoinSet::new();
        r.delivery.tick(&mut set).await;
        for _ in 0..200 {
            if r.status("run-1") == "blocked" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(r.status("run-1"), "blocked");
        r.fake.set_permissions("lead", &[]);
        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
        assert_eq!(r.status("run-1"), "done");
        assert_eq!(r.transcript("run-1"), "wrote it");
        assert!(r.fake.calls().contains(&Call::Wait("lead".into())));
        r.tick().await;
        let notices = r.notices();
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(
            notices[0].1.contains("finished run run-1 (done)"),
            "the blocked notice is replaced by the newer done one: {notices:?}"
        );
        assert!(!notices[0].1.contains("waiting for a permission"));
        assert_eq!(r.notice_state("run-1").as_deref(), Some("sent"));
    }

    #[tokio::test]
    async fn delivery_marks_an_unfinished_turn_lost_and_holds_the_project_until_idle() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.fake.push_turn_end(TurnEnd::Other {
            status: "timeout".into(),
            message: "still running".into(),
        });
        r.queue_at("proj-1", "run-1", "one", None, "2026-10-14T14:00:01Z");
        r.queue_at("proj-1", "run-2", "two", None, "2026-10-14T14:00:02Z");
        r.fake.set_status("lead", "idle");
        r.tick().await;
        assert_eq!(r.status("run-1"), "lost");
        assert!(r.transcript("run-1").contains(MAY_STILL_FINISH));
        assert!(r.delivery.state.lock().unwrap().held.contains("proj-1"));
        r.fake.set_status("lead", "running");
        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec!["one"]);
        r.fake.set_status("lead", "idle");
        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec!["one", "two"]);
        assert_eq!(r.status("run-2"), "done");
    }

    #[tokio::test]
    async fn delivery_requeues_a_pre_send_failure_and_loses_it_on_the_fifth() {
        let r = rig();
        r.fake.state.lock().unwrap().workspaces_fail = true;
        r.queue("proj-1", "run-1", "one", Some("caller"));
        r.fake.add_leader("proj-9", "caller", "idle", false);
        for attempt in 1..5 {
            r.tick().await;
            assert_eq!(r.status("run-1"), "queued");
            assert_eq!(r.engine.run_attempts("run-1").unwrap(), Some(attempt));
        }
        r.tick().await;
        assert_eq!(r.status("run-1"), "lost");
        assert!(r.transcript("run-1").contains("`fake workspace` exited"));
        assert!(r
            .fake
            .calls()
            .iter()
            .all(|c| !matches!(c, Call::Run(_) | Call::SendAndWait(..))));
        r.tick().await;
        let notices = r.notices();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].1.contains("(lost)"));
        assert_eq!(r.notice_state("run-1").as_deref(), Some("sent"));
    }

    #[tokio::test]
    async fn delivery_requeues_when_no_provider_is_available_and_never_runs() {
        let r = rig();
        r.fake.state.lock().unwrap().no_providers = true;
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        assert_eq!(r.status("run-1"), "queued");
        assert_eq!(r.engine.run_attempts("run-1").unwrap(), Some(1));
        assert!(r.fake.calls().iter().all(|c| !matches!(c, Call::Run(_))));
    }

    #[tokio::test]
    async fn delivery_waits_out_the_backoff_before_a_retry() {
        let mut r = rig();
        r.delivery.backoff = [Duration::from_secs(600); 4];
        r.fake.state.lock().unwrap().workspaces_fail = true;
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        r.tick().await;
        assert_eq!(r.engine.run_attempts("run-1").unwrap(), Some(1));
    }

    #[tokio::test]
    async fn delivery_restart_requeues_unsent_runs_and_loses_sent_ones_without_resending() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "running", false);
        r.fake.add_leader("proj-9", "caller", "idle", false);
        r.queue_at("proj-1", "run-pending", "one", None, "2026-10-14T14:00:01Z");
        r.queue_at(
            "proj-1",
            "run-sent",
            "two",
            Some("caller"),
            "2026-10-14T14:00:02Z",
        );
        r.engine.claim_queued_run("run-pending").unwrap();
        r.engine.claim_queued_run("run-sent").unwrap();
        r.engine.set_run_pane("run-sent", "paseo:lead").unwrap();

        r.tick().await;
        assert_eq!(r.status("run-pending"), "queued");
        assert_eq!(r.status("run-sent"), "lost");
        assert_eq!(r.transcript("run-sent"), RESTARTED);
        assert!(r.sends_to("lead").is_empty());
        r.fake.set_status("lead", "idle");
        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec!["one"]);
        assert_eq!(r.notices().len(), 1);
        assert_eq!(r.notice_state("run-sent").as_deref(), Some("sent"));
    }

    #[tokio::test]
    async fn delivery_notifies_a_caller_once_and_only_while_idle() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.fake.add_leader("proj-9", "caller", "running", false);
        r.queue("proj-1", "run-1", "one", Some("caller"));
        r.tick().await;
        r.tick().await;
        assert!(r.notices().is_empty());
        r.fake.set_status("caller", "idle");
        r.tick().await;
        r.tick().await;
        assert_eq!(
            r.notices(),
            vec![(
                "caller".to_string(),
                "[waggledance] proj-1-name leader finished run run-1 (done) — informational, \
                 no action needed; read it with waggledance_await {\"run_id\": \"run-1\"}."
                    .to_string()
            )]
        );
    }

    #[tokio::test]
    async fn delivery_never_notifies_the_leader_about_its_own_run() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.queue("proj-1", "run-1", "one", Some("lead"));
        r.tick().await;
        r.tick().await;
        assert_eq!(r.status("run-1"), "done");
        assert!(r.notices().is_empty());
    }

    #[tokio::test]
    async fn delivery_enqueues_terminal_states_on_the_notify_path() {
        let r = rig();
        let store = Arc::new(NotifyStore::open_in_memory().unwrap());
        *r.delivery.notify.lock().unwrap() = Some(store.clone());
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        let kinds: Vec<String> = store
            .undelivered()
            .unwrap()
            .into_iter()
            .map(|n| n.kind)
            .collect();
        assert_eq!(kinds, vec!["done"]);
    }

    #[tokio::test]
    async fn a_pending_notice_survives_a_restart_and_sends_on_the_first_idle_tick() {
        let mut r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.fake.add_leader("proj-9", "caller", "running", false);
        r.queue("proj-1", "run-1", "one", Some("caller"));
        r.tick().await;
        assert_eq!(r.status("run-1"), "done");
        assert_eq!(r.notice_state("run-1").as_deref(), Some("pending"));
        assert!(r.notices().is_empty());

        let dir = r.delivery.data_dir.clone();
        r.delivery = Delivery::new(r.fake.clone(), r.engine.clone(), dir, Arc::default());
        r.fake.set_status("caller", "idle");
        r.tick().await;
        assert_eq!(
            r.notices().len(),
            1,
            "the restarted delivery sends the stored notice"
        );
        assert!(r.notices()[0].1.contains("finished run run-1 (done)"));
        assert_eq!(r.notice_state("run-1").as_deref(), Some("sent"));
    }

    #[tokio::test]
    async fn an_expired_notice_is_dropped_and_never_sent() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.fake.add_leader("proj-9", "caller", "idle", false);
        r.queue("proj-1", "run-1", "one", Some("caller"));
        r.engine
            .finish_run(
                "run-1",
                "done",
                "2026-10-16T11:00:00Z",
                Some("the reply"),
                Some(("a stale notice", "2020-01-01T00:00:00Z")),
            )
            .unwrap();
        r.tick().await;
        assert!(
            r.notices().is_empty(),
            "an expired notice is never delivered"
        );
        assert_eq!(r.notice_state("run-1").as_deref(), Some("dropped"));
    }

    fn inbox(engine: &Engine) -> Vec<waggledance_core::domain::InboxMessage> {
        let mut got = Vec::new();
        while let Some(m) = engine.next_inbox_message("wd").unwrap() {
            engine.ack_inbox_message(m.seq, &now_rfc3339()).unwrap();
            got.push(m);
        }
        got
    }

    #[tokio::test]
    async fn a_wd_run_starts_a_via_wd_leader_and_never_uses_a_plain_one() {
        let r = rig();
        r.fake.add_leader("proj-1", "plain", "idle", false);
        r.fake.add_workspace(
            "wks",
            "local",
            &r.engine.get_project("proj-1").unwrap().unwrap().root_path,
        );
        let task = paseo::wd_task("corr-1", "build it");
        r.queue("proj-1", "run-1", &task, Some(paseo::WD_CALLER));
        r.tick().await;
        let labels: Vec<Vec<String>> = r
            .fake
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::Run(spec) => Some(spec.labels),
                _ => None,
            })
            .collect();
        assert_eq!(labels.len(), 1, "{labels:?}");
        assert!(labels[0].iter().any(|l| l == paseo::WD_LABEL));
        assert!(r.sends_to("plain").is_empty());
        assert_eq!(r.status("run-1"), "done");
        assert!(inbox(&r.engine).is_empty());
        r.tick().await;
        assert!(r.notices().is_empty());
    }

    #[tokio::test]
    async fn delivering_a_wd_run_records_its_leader_and_clears_the_nudge_marks() {
        let r = rig();
        r.fake.add_wd_leader("proj-1", "lead", "idle");
        r.engine
            .insert_task("corr-4", "proj-1", "2026-10-14T13:00:00Z")
            .unwrap();
        r.engine
            .set_task_nudged("corr-4", "2026-10-14T13:10:00Z")
            .unwrap();
        r.engine
            .set_task_progress_sent("corr-4", "2026-10-14T13:20:00Z")
            .unwrap();
        let before = now_rfc3339();
        let task = paseo::wd_task("corr-4", "one");
        r.queue("proj-1", "run-1", &task, Some(paseo::WD_CALLER));
        r.tick().await;
        assert_eq!(r.sends_to("lead"), vec![task]);
        let t = r.engine.get_task("corr-4").unwrap().unwrap();
        assert_eq!(t.leader_id.as_deref(), Some("lead"));
        let delivered = t.last_delivered_at.unwrap();
        assert!(delivered >= before, "{delivered} < {before}");
        assert_eq!(t.changed_at, delivered);
        assert_eq!((t.nudged_at, t.progress_sent_at), (None, None));
    }

    #[tokio::test]
    async fn delivering_a_non_wd_run_touches_no_task() {
        let r = rig();
        r.fake.add_leader("proj-1", "lead", "idle", false);
        r.engine
            .insert_task("corr-5", "proj-1", "2026-10-14T13:00:00Z")
            .unwrap();
        r.queue("proj-1", "run-1", &paseo::wd_task("corr-5", "one"), None);
        r.tick().await;
        assert_eq!(r.status("run-1"), "done");
        let t = r.engine.get_task("corr-5").unwrap().unwrap();
        assert_eq!((t.leader_id, t.last_delivered_at), (None, None));
        assert_eq!(t.changed_at, "2026-10-14T13:00:00Z");
    }

    #[tokio::test]
    async fn a_blocked_wd_run_mails_no_permission_and_sends_no_notice() {
        let r = rig();
        r.fake.add_wd_leader("proj-1", "lead", "idle");
        r.fake.set_permissions("lead", &["Write"]);
        r.fake.push_turn_end(TurnEnd::Permission);
        r.fake.push_turn_end(TurnEnd::Completed);
        let task = paseo::wd_task("corr-2", "one");
        r.queue("proj-1", "run-1", &task, Some(paseo::WD_CALLER));

        let mut set = JoinSet::new();
        r.delivery.tick(&mut set).await;
        for _ in 0..200 {
            if r.status("run-1") == "blocked" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        r.fake.set_permissions("lead", &[]);
        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
        assert_eq!(r.status("run-1"), "done");
        let mail = inbox(&r.engine);
        assert!(
            mail.is_empty(),
            "the permission watcher is the one writer: {mail:?}"
        );
        r.tick().await;
        assert!(r.notices().is_empty());
    }

    #[tokio::test]
    async fn a_lost_wd_run_mails_one_error_and_sends_no_notice() {
        let r = rig();
        r.fake.add_wd_leader("proj-1", "lead", "idle");
        r.fake.push_turn_end(TurnEnd::Other {
            status: "timeout".into(),
            message: "still running".into(),
        });
        let task = paseo::wd_task("corr-3", "one");
        r.queue("proj-1", "run-1", &task, Some(paseo::WD_CALLER));
        r.tick().await;
        assert_eq!(r.status("run-1"), "lost");
        let mail = inbox(&r.engine);
        assert_eq!(mail.len(), 1, "{mail:?}");
        assert_eq!(
            (mail[0].kind.as_str(), mail[0].corr.as_str()),
            ("error", "corr-3")
        );
        assert!(mail[0].body.contains(MAY_STILL_FINISH), "{}", mail[0].body);
        r.tick().await;
        assert!(r.notices().is_empty());
    }

    #[test]
    fn delivery_reply_is_the_leader_text_after_the_last_task_line() {
        let logs = "[User] one\nold\n[User] one\nsecond line\nnew reply\n[User] other\n";
        assert_eq!(leader_reply(logs, "one\nsecond line"), "new reply");
        assert_eq!(leader_reply("[User] two\nx\n", "one"), "");
    }
}
