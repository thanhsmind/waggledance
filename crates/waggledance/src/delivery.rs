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
use crate::paseo::{self, ModeCheck, Paseo, PaseoError, TurnEnd};

pub const TICK: Duration = Duration::from_secs(2);
const MAX_ATTEMPTS: i64 = 5;
const NOTICE_WINDOW: Duration = Duration::from_secs(3600);
const NOTICE_GAP: Duration = Duration::from_secs(30);
const MAY_STILL_FINISH: &str = "the leader may still be finishing";
const RESTARTED: &str = "daemon restarted — the leader may still be finishing; check its chat";
const NO_REPLY: &str = "[waggledance] the leader ended its turn but left no reply to this task in its log — read its Paseo chat";
const LEADER_RUNNING: [&str; 2] = ["running", "initializing"];
const UNFINISHED: &str = "timeout";
const UNREAD_LIMIT: u32 = 3;

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
    sweeping: bool,
    headers: HashMap<String, String>,
    inflight: HashSet<String>,
    not_before: HashMap<String, Instant>,
    quiet_until: HashMap<String, Instant>,
}

struct Notice {
    run_id: String,
    text: String,
}

struct Follow {
    leader: Option<String>,
    prior: String,
    of: Option<String>,
}

enum Route {
    Wait,
    Follow(Follow),
}

const FAMILY_OPEN: [&str; 3] = ["queued", "working", "blocked"];

fn started_leader(run: &Run) -> Option<&str> {
    run.pane_id
        .strip_prefix(PASEO_PANE_PREFIX)
        .filter(|a| *a != "pending")
}

fn gone_note(of: &str, leader: Option<&str>) -> String {
    match leader {
        Some(leader) => format!(
            "[waggledance] follow-up of run {of}: its leader {leader} was gone (archived or \
             unreadable), so a new leader carries the original task"
        ),
        None => format!(
            "[waggledance] follow-up of run {of}: no leader was ever started for it, so a new \
             leader carries the original task"
        ),
    }
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

fn settled(state: &paseo::AgentState) -> bool {
    !state.archived
        && state.pending_permissions == 0
        && !LEADER_RUNNING.contains(&state.status.as_str())
}

fn idle_for(
    state: &paseo::AgentState,
    now: time::OffsetDateTime,
    limit: time::Duration,
) -> Option<time::Duration> {
    if !settled(state) {
        return None;
    }
    let updated = time::OffsetDateTime::parse(
        state.updated_at.as_deref()?,
        &time::format_description::well_known::Rfc3339,
    )
    .ok()?;
    let idle = now - updated;
    (idle > limit).then_some(idle)
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
                _ = discoveries.tick() => {
                    self.discover(&mut deliveries);
                    self.sweep_idle_leaders(&mut deliveries);
                }
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

    fn sweep_idle_leaders(&self, tasks: &mut JoinSet<()>) {
        if std::mem::replace(&mut self.state.lock().unwrap().sweeping, true) {
            return;
        }
        let this = self.clone();
        tasks.spawn(async move {
            let now = time::OffsetDateTime::now_utc();
            this.sweep_leaders(now).await;
            this.state.lock().unwrap().sweeping = false;
        });
    }

    async fn sweep_leaders(&self, now: time::OffsetDateTime) {
        let leaders = match self.paseo.all_leaders().await {
            Ok(leaders) => leaders,
            Err(e) => {
                tracing::warn!("idle sweep could not list the leaders: {e}");
                return;
            }
        };
        for id in leaders {
            let state = match self.paseo.inspect(&id).await {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!("idle sweep could not inspect leader {id}: {e}");
                    continue;
                }
            };
            let carried = self.carried_runs(&id);
            let limit = self.idle_limit(&carried);
            let Some(idle) = idle_for(&state, now, limit) else {
                continue;
            };
            if self.family_open(&carried) {
                continue;
            }
            match self.paseo.archive(&id).await {
                Ok(()) => tracing::info!(
                    leader = %id,
                    idle_minutes = idle.whole_minutes(),
                    "idle sweep archived leader {id}"
                ),
                Err(e) => {
                    tracing::warn!("idle sweep could not archive leader {id}: {e}")
                }
            }
        }
    }

    fn carried_runs(&self, leader: &str) -> Vec<Run> {
        self.engine
            .runs_on_pane(&pane(Some(leader)))
            .unwrap_or_else(|e| {
                tracing::warn!("delivery could not list leader {leader}'s runs: {e}");
                Vec::new()
            })
    }

    fn idle_limit(&self, carried: &[Run]) -> time::Duration {
        let own = carried
            .first()
            .and_then(|run| self.engine.get_project(&run.project_id).ok().flatten())
            .and_then(|p| self.engine.project_settings(&p).0.paseo.leader_idle_minutes);
        let minutes = own.unwrap_or(self.engine.config.paseo.leader_idle_minutes);
        time::Duration::minutes(i64::try_from(minutes).unwrap_or(i64::MAX / 120))
    }

    fn family_open(&self, carried: &[Run]) -> bool {
        carried.iter().any(|run| {
            if FAMILY_OPEN.contains(&run.status.as_str()) {
                return true;
            }
            match self.family(&run.id) {
                Some((_, family)) => family
                    .iter()
                    .any(|r| FAMILY_OPEN.contains(&r.status.as_str())),
                None => true,
            }
        })
    }

    fn family(&self, of: &str) -> Option<(Vec<Run>, Vec<Run>)> {
        let chain = self
            .engine
            .follow_up_chain(of)
            .map_err(|e| tracing::warn!("delivery could not read run {of}'s chain: {e}"))
            .ok()
            .filter(|chain| !chain.is_empty())?;
        let family = self
            .engine
            .follow_up_family(&chain[0].id)
            .map_err(|e| tracing::warn!("delivery could not read run {of}'s family: {e}"))
            .ok()?;
        Some((chain, family))
    }

    fn handoff_of(&self, run: &Run, of: &str) -> (String, Vec<String>) {
        let mut olds: Vec<String> = Vec::new();
        if let Some((_, family)) = self.family(of) {
            for leader in family
                .iter()
                .rev()
                .filter(|r| r.id != run.id)
                .filter_map(|r| started_leader(r))
            {
                if !olds.iter().any(|o| o == leader) {
                    olds.push(leader.to_string());
                }
            }
        }
        let takes = olds
            .first()
            .map(|old| format!(": takes over from leader {old}"))
            .unwrap_or_default();
        (format!("[waggledance] handoff of run {of}{takes}"), olds)
    }

    fn retire(&self, old: String, deadline: Instant) {
        let this = self.clone();
        tokio::spawn(async move {
            while Instant::now() < deadline {
                match this.paseo.inspect(&old).await {
                    Ok(state) if state.archived => return,
                    Ok(state) if settled(&state) && !this.carries_open(&old) => {
                        match this.paseo.archive(&old).await {
                            Ok(()) => {
                                tracing::info!(leader = %old, "handoff archived leader {old}")
                            }
                            Err(e) => tracing::warn!("handoff could not archive leader {old}: {e}"),
                        }
                        return;
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("handoff could not inspect leader {old}: {e}"),
                }
                tokio::time::sleep(this.permission_poll).await;
            }
        });
    }

    fn carries_open(&self, leader: &str) -> bool {
        self.carried_runs(leader)
            .iter()
            .any(|r| FAMILY_OPEN.contains(&r.status.as_str()))
    }

    async fn tick(&self, deliveries: &mut JoinSet<()>) {
        self.beat();
        let first = !std::mem::replace(&mut self.state.lock().unwrap().recovered, true);
        if first {
            self.recover(deliveries);
        }
        self.flush_notices().await;
        let projects = self
            .engine
            .list_projects_with_queued_paseo_runs()
            .unwrap_or_else(|e| {
                tracing::warn!("delivery could not list queued leader runs: {e}");
                Vec::new()
            });
        for project_id in projects {
            for run in self.deliverable(&project_id) {
                {
                    let mut s = self.state.lock().unwrap();
                    if !s.inflight.insert(run.id.clone()) {
                        continue;
                    }
                }
                let this = self.clone();
                deliveries.spawn(async move {
                    this.deliver(&run).await;
                    this.state.lock().unwrap().inflight.remove(&run.id);
                });
            }
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

    fn recover(&self, deliveries: &mut JoinSet<()>) {
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
            let Some(agent) = agent.map(str::to_string) else {
                self.end(&self.name_of(&run), &run, "lost", Some(RESTARTED), None);
                continue;
            };
            if !self.state.lock().unwrap().inflight.insert(run.id.clone()) {
                continue;
            }
            let this = self.clone();
            deliveries.spawn(async move {
                let name = this.name_of(&run);
                let unseen = Ok(TurnEnd::Other {
                    status: UNFINISHED.to_string(),
                    message: "daemon restarted before the leader's turn end was seen".to_string(),
                });
                this.follow_turn(&name, &run, &agent, unseen, &run.task)
                    .await;
                this.state.lock().unwrap().inflight.remove(&run.id);
            });
        }
    }

    fn deliverable(&self, project_id: &str) -> Vec<Run> {
        let runs = self
            .engine
            .list_queued_paseo_runs(project_id)
            .map_err(|e| tracing::warn!("delivery could not list queued runs: {e}"))
            .unwrap_or_default();
        let s = self.state.lock().unwrap();
        runs.into_iter()
            .filter(|run| {
                s.not_before
                    .get(&run.id)
                    .is_none_or(|at| Instant::now() >= *at)
                    && !s.inflight.contains(&run.id)
            })
            .collect()
    }

    fn name_of(&self, run: &Run) -> String {
        match self.engine.get_project(&run.project_id) {
            Ok(Some(p)) => p.name,
            _ => run.project_id.clone(),
        }
    }

    async fn deliver(&self, run: &Run) {
        match self.engine.claim_queued_run(&run.id) {
            Ok(true) => {}
            Ok(false) => return,
            Err(e) => {
                tracing::warn!("delivery could not claim run {}: {e}", run.id);
                return;
            }
        }
        let name = self.name_of(run);
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
        let mode = paseo::leader_mode(config, &settings.paseo);
        let handoff = self.engine.run_is_handoff(&run.id).unwrap_or(false);
        let mut note = None;
        let mut olds = Vec::new();
        let route = match self.engine.run_follow_up_of(&run.id) {
            Ok(Some(of)) if handoff => {
                let (line, leaders) = self.handoff_of(run, &of);
                note = Some(line);
                olds = leaders;
                None
            }
            Ok(Some(of)) => self.follow_up_of(run, &of),
            Ok(None) => wd.then(|| self.follow_up(run)).flatten().map(Route::Follow),
            Err(e) => {
                tracing::warn!(
                    "delivery could not read run {}'s follow-up link: {e}",
                    run.id
                );
                wd.then(|| self.follow_up(run)).flatten().map(Route::Follow)
            }
        };
        let prompt = match route {
            Some(Route::Wait) => {
                self.set_status(&run.id, "queued");
                return;
            }
            Some(Route::Follow(follow)) => {
                let state = match follow.leader.as_deref() {
                    Some(leader) => Some(self.paseo.inspect(leader).await),
                    None => None,
                };
                match (follow.leader.as_deref(), state) {
                    (Some(_), Some(Ok(state)))
                        if !state.archived && LEADER_RUNNING.contains(&state.status.as_str()) =>
                    {
                        self.set_status(&run.id, "queued");
                        return;
                    }
                    (Some(leader), Some(Ok(state))) if !state.archived => {
                        return self.follow_on(&name, run, leader, &mode, timeout, wd).await;
                    }
                    (leader, _) if paseo::is_steer(&run.task) => {
                        let of = follow.of.as_deref().unwrap_or("its family");
                        let why = format!(
                            "steer note for run {of} not delivered: its leader {} is gone, and a \
                             steer never opens a leader",
                            leader.unwrap_or("was never started")
                        );
                        return self.lose(&name, run, None, &why);
                    }
                    (leader, _) => {
                        note = follow.of.as_deref().map(|of| gone_note(of, leader));
                        format!("{}\n\n{}", follow.prior, run.task)
                    }
                }
            }
            None => run.task.clone(),
        };
        let start = match paseo::resolve_leader(
            self.paseo.as_ref(),
            config,
            &settings.paseo,
            &project,
            &run.id,
            wd,
        )
        .await
        {
            Ok(start) => start,
            Err(e) => return self.presend_failed(&name, run, &e.to_string()),
        };
        let sent_at = now_rfc3339();
        tracing::info!(
            run = %run.id,
            workspace = %start.workspace,
            reason = start.reason,
            "delivery starts the {name} leader"
        );
        let mut header = format!(
            "[waggledance] leader started in Paseo workspace {} ({})",
            start.workspace, start.reason
        );
        if let Some(note) = note {
            header.push('\n');
            header.push_str(&note);
        }
        self.state
            .lock()
            .unwrap()
            .headers
            .insert(run.id.clone(), header);
        let spec = start.spec(&prompt);
        let started = self.paseo.run_and_wait(&spec, timeout);
        let retired = std::sync::atomic::AtomicBool::new(false);
        let found: Mutex<Option<String>> = Mutex::new(None);
        let finder = async {
            let id = self.new_leader(&project.id, &run.id).await;
            *found.lock().unwrap() = Some(id.clone());
            if !retired.swap(true, std::sync::atomic::Ordering::SeqCst) {
                olds.iter()
                    .for_each(|old| self.retire(old.clone(), deadline));
            }
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
        let (agent, outcome) = match result {
            Ok((id, end)) => {
                if !checked {
                    if !retired.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        olds.iter()
                            .for_each(|old| self.retire(old.clone(), deadline));
                    }
                    self.check_mode(run, &id, &mode).await;
                }
                self.sent(run, &id, wd, &sent_at);
                (Some(id), Ok(end))
            }
            Err(e @ (PaseoError::Spawn { .. } | PaseoError::PromptFile(_))) => (None, Err(e)),
            Err(e) => {
                let found = found.lock().unwrap().clone();
                let id = match found {
                    Some(id) => Some(id),
                    None => self
                        .paseo
                        .run_leaders(&project.id, &run.id)
                        .await
                        .ok()
                        .and_then(|ids| ids.into_iter().next()),
                };
                if let Some(id) = &id {
                    self.sent(run, id, wd, &sent_at);
                }
                (id, Err(e))
            }
        };
        let run = Run {
            pane_id: pane(agent.as_deref()),
            ..run.clone()
        };
        self.settle(&name, &run, agent.as_deref(), outcome, &prompt)
            .await;
    }

    async fn settle(
        &self,
        name: &str,
        run: &Run,
        agent: Option<&str>,
        end: paseo::Result<TurnEnd>,
        sent: &str,
    ) {
        match (agent, end) {
            (_, Err(e @ (PaseoError::Spawn { .. } | PaseoError::PromptFile(_)))) => {
                self.presend_failed(name, run, &e.to_string())
            }
            (Some(agent), end) => self.follow_turn(name, run, agent, end, sent).await,
            (None, Ok(TurnEnd::Completed)) => self.done(name, run, None, sent).await,
            (None, Ok(TurnEnd::Permission)) => self.lose(name, run, None, "permission requested"),
            (None, Ok(TurnEnd::Other { status, message })) => self.lose(
                name,
                run,
                None,
                &format!("paseo reported {status}: {message}"),
            ),
            (None, Err(e)) => self.lose(name, run, None, &e.to_string()),
        }
    }

    async fn follow_turn(
        &self,
        name: &str,
        run: &Run,
        agent: &str,
        mut end: paseo::Result<TurnEnd>,
        sent: &str,
    ) {
        let timeout = Duration::from_secs(self.engine.config.paseo.turn_timeout_secs);
        let mut unread = 0;
        loop {
            let why = match end {
                Ok(TurnEnd::Completed) => return self.done(name, run, Some(agent), sent).await,
                Ok(TurnEnd::Other { status, .. }) if status == "idle" => {
                    return self.done(name, run, Some(agent), sent).await
                }
                Ok(TurnEnd::Permission) => {
                    if let Some(why) = self.hold_for_permission(name, run, agent).await {
                        return self.gone(name, run, agent, &why);
                    }
                    end = self.paseo.wait(agent, timeout).await;
                    continue;
                }
                Ok(TurnEnd::Other { status, message }) if status != UNFINISHED => {
                    let why = format!("paseo reported {status}: {message}");
                    return self.lose(name, run, Some(agent), &why);
                }
                Ok(TurnEnd::Other { status, message }) => {
                    format!("paseo reported {status}: {message}")
                }
                Err(e) => e.to_string(),
            };
            match self.paseo.inspect(agent).await {
                Ok(state) if state.archived => {
                    return self.gone(name, run, agent, &format!("archived; {why}"))
                }
                Ok(_) => unread = 0,
                Err(e) => {
                    unread += 1;
                    if unread >= UNREAD_LIMIT {
                        let why = format!("{why}; its state could not be read: {e}");
                        return self.gone(name, run, agent, &why);
                    }
                }
            }
            tracing::info!(
                run = %run.id,
                leader = agent,
                "delivery keeps run {} working: its leader is still at work ({why})",
                run.id
            );
            tokio::time::sleep(self.permission_poll).await;
            end = self.paseo.wait(agent, timeout).await;
        }
    }

    async fn hold_for_permission(&self, name: &str, run: &Run, agent: &str) -> Option<String> {
        self.end(name, run, "blocked", None, Some(agent));
        let mut unread = 0;
        loop {
            tokio::time::sleep(self.permission_poll).await;
            match self.paseo.inspect(agent).await {
                Ok(s) if s.archived => {
                    return Some("archived while a permission was pending".to_string())
                }
                Ok(s) if s.pending_permissions == 0 => return None,
                Ok(_) => unread = 0,
                Err(e) => {
                    tracing::warn!("delivery could not inspect leader {agent}: {e}");
                    unread += 1;
                    if unread >= UNREAD_LIMIT {
                        return Some(format!("its state could not be read: {e}"));
                    }
                }
            }
        }
    }

    fn follow_up(&self, run: &Run) -> Option<Follow> {
        let corr = paseo::wd_corr(&run.task)?;
        let original = self
            .engine
            .run_by_request(&run.project_id, corr)
            .ok()
            .flatten()?;
        if original.id == run.id {
            return None;
        }
        let leader = self.engine.get_task(corr).ok().flatten()?.leader_id?;
        Some(Follow {
            leader: Some(leader),
            prior: original.task,
            of: None,
        })
    }

    fn follow_up_of(&self, run: &Run, of: &str) -> Option<Route> {
        let (chain, family) = self.family(of)?;
        let handoffs = self
            .engine
            .family_handoffs(&chain[0].id)
            .unwrap_or_else(|e| {
                tracing::warn!("delivery could not read run {of}'s handoffs: {e}");
                HashSet::new()
            });
        let mut before = true;
        let mut candidates: Vec<&Run> = Vec::new();
        for r in &family {
            if r.id == run.id {
                before = false;
            } else if before || handoffs.contains(&r.id) {
                candidates.push(r);
            }
        }
        if candidates
            .iter()
            .any(|r| FAMILY_OPEN.contains(&r.status.as_str()))
        {
            return Some(Route::Wait);
        }
        let leader = candidates.iter().rev().find_map(|r| started_leader(r));
        let prior = chain
            .iter()
            .map(|r| r.task.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        Some(Route::Follow(Follow {
            leader: leader.map(str::to_string),
            prior,
            of: Some(of.to_string()),
        }))
    }

    async fn follow_on(
        &self,
        name: &str,
        run: &Run,
        leader: &str,
        mode: &str,
        timeout: Duration,
        wd: bool,
    ) {
        self.check_mode(run, leader, mode).await;
        self.sent(run, leader, wd, &now_rfc3339());
        let run = Run {
            pane_id: pane(Some(leader)),
            ..run.clone()
        };
        let end = self.paseo.send_and_wait(leader, &run.task, timeout).await;
        self.settle(name, &run, Some(leader), end, &run.task).await;
    }

    async fn new_leader(&self, project_id: &str, run_id: &str) -> String {
        loop {
            if let Ok(ids) = self.paseo.run_leaders(project_id, run_id).await {
                if let Some(id) = ids.into_iter().next() {
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

    async fn done(&self, name: &str, run: &Run, agent: Option<&str>, sent: &str) {
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
        let reply = leader_reply(&logs, sent);
        let reply = if reply.trim().is_empty() {
            format!("{NO_REPLY} (leader {})", agent.unwrap_or("not started"))
        } else {
            reply
        };
        self.end(name, run, "done", Some(&reply), agent);
    }

    fn gone(&self, name: &str, run: &Run, agent: &str, why: &str) {
        let transcript = format!("the leader {agent} is gone without a result ({why})");
        self.end(name, run, "lost", Some(&transcript), Some(agent));
    }

    fn lose(&self, name: &str, run: &Run, agent: Option<&str>, why: &str) {
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

        async fn sweep_at(&self, now: time::OffsetDateTime) {
            self.delivery.sweep_leaders(now).await;
        }

        fn started_tasks(&self) -> Vec<String> {
            self.fake
                .calls()
                .into_iter()
                .filter_map(|c| match c {
                    Call::Run(spec) => Some(spec.task),
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
    async fn delivery_starts_one_leader_per_queued_run_in_the_same_tick() {
        let r = rig();
        root_workspace(&r);
        r.queue_at("proj-1", "run-a", "first", None, "2026-10-14T14:00:01Z");
        r.queue_at("proj-1", "run-b", "second", None, "2026-10-14T14:00:02Z");
        r.queue_at("proj-1", "run-c", "third", None, "2026-10-14T14:00:03Z");
        r.tick().await;
        let mut tasks: Vec<String> = r
            .fake
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::Run(spec) => Some(spec.task),
                _ => None,
            })
            .collect();
        tasks.sort();
        assert_eq!(tasks, vec!["first", "second", "third"]);
        assert_eq!(
            (r.status("run-a"), r.status("run-b"), r.status("run-c")),
            ("done".into(), "done".into(), "done".into())
        );
        assert!(std::fs::metadata(paseo::delivery_heartbeat_path(&r.delivery.data_dir)).is_ok());
    }

    #[tokio::test]
    async fn delivery_serves_two_projects_in_the_same_tick() {
        let r = rig();
        r.fake.add_workspace(
            "wks-1",
            "local",
            &r.engine.get_project("proj-1").unwrap().unwrap().root_path,
        );
        r.fake.add_workspace(
            "wks-2",
            "local",
            &r.engine.get_project("proj-2").unwrap().unwrap().root_path,
        );
        r.queue("proj-1", "run-1", "one", None);
        r.queue("proj-2", "run-2", "two", None);
        r.tick().await;
        assert_eq!(
            (r.status("run-1"), r.status("run-2")),
            ("done".into(), "done".into())
        );
    }

    #[tokio::test]
    async fn delivery_never_sends_to_a_leader_that_existed_before_the_run() {
        let r = rig();
        root_workspace(&r);
        r.fake
            .add_run_leader("proj-1", "run-0", "idle-lead", "idle");
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        assert_eq!(r.status("run-1"), "done");
        let run = r.engine.get_run("run-1").unwrap().unwrap();
        assert_ne!(run.pane_id, "paseo:idle-lead");
        assert!(r
            .fake
            .calls()
            .iter()
            .all(|c| !matches!(c, Call::Inspect(_))));
        assert!(
            !r.fake.archived(agent_of(&run)),
            "a done run keeps its leader"
        );
    }

    #[tokio::test]
    async fn a_run_whose_delivery_is_in_flight_is_not_delivered_twice() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "one", None);
        r.delivery
            .state
            .lock()
            .unwrap()
            .inflight
            .insert("run-1".into());
        r.tick().await;
        assert!(r.fake.calls().iter().all(|c| !matches!(c, Call::Run(_))));
        assert_eq!(r.status("run-1"), "queued");
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

    fn agent_of(run: &Run) -> &str {
        run.pane_id
            .strip_prefix(orchestrate::PASEO_PANE_PREFIX)
            .unwrap_or_default()
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
    async fn the_mode_check_reads_the_leader_started_for_that_run() {
        let r = rig();
        root_workspace(&r);
        r.fake
            .add_run_leader("proj-1", "run-0", "idle-lead", "idle");
        r.queue("proj-1", "run-1", "one", None);
        r.tick().await;
        let run = r.engine.get_run("run-1").unwrap().unwrap();
        let agent = agent_of(&run).to_string();
        assert_ne!(agent, "idle-lead");
        let calls = r.fake.calls();
        assert!(calls.contains(&Call::RunLeaders("proj-1".into(), "run-1".into())));
        assert!(calls.contains(&Call::RuntimeMode(agent.clone())));
        assert!(!calls.contains(&Call::RuntimeMode("idle-lead".into())));
        assert!(set_modes(&r).is_empty());
        assert_eq!(r.status("run-1"), "done");
        assert!(!r.fake.archived(&agent), "a done run keeps its leader");
    }

    #[tokio::test]
    async fn an_empty_global_mode_starts_without_a_mode_and_checks_nothing() {
        let mut config = Config::default();
        config.paseo.mode = String::new();
        let r = rig_with(config);
        root_workspace(&r);
        r.fake.add_workspace(
            "wks-2",
            "local",
            &r.engine.get_project("proj-2").unwrap().unwrap().root_path,
        );
        r.fake.script_modes("agent-1", &[Some("default")], true);
        r.fake.script_modes("agent-2", &[Some("default")], true);
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
        root_workspace(&r);
        r.fake.add_leader("caller", "idle", false);
        r.fake.set_permissions("agent-1", &["Write"]);
        r.fake.push_turn_end(TurnEnd::Permission);
        r.fake.push_turn_end(TurnEnd::Other {
            status: "idle".into(),
            message: "Agent is idle.".into(),
        });
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
        let run = r.engine.get_run("run-1").unwrap().unwrap();
        let agent = agent_of(&run).to_string();
        r.set_logs(&agent, "[User] one\nwrote it\n");
        assert!(!r.fake.archived(&agent), "a blocked run keeps its leader");
        r.fake.set_permissions(&agent, &[]);
        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
        assert_eq!(r.status("run-1"), "done");
        assert!(
            r.transcript("run-1").ends_with("wrote it"),
            "{}",
            r.transcript("run-1")
        );
        assert!(r.fake.calls().contains(&Call::Wait(agent.clone())));
        assert!(!r.fake.archived(&agent), "a done run keeps its leader");
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
    async fn an_unfinished_turn_waits_on_a_live_leader_and_is_lost_only_once_it_is_gone() {
        let r = rig();
        root_workspace(&r);
        r.fake.push_turn_end(TurnEnd::Other {
            status: "timeout".into(),
            message: "still running".into(),
        });
        r.queue_at("proj-1", "run-1", "one", None, "2026-10-14T14:00:01Z");
        r.tick().await;
        assert_eq!(r.status("run-1"), "done");
        let first = leader_of(&r, "run-1");
        assert!(r.fake.calls().contains(&Call::Wait(first.clone())));
        assert!(!r.fake.archived(&first), "a done run keeps its leader");

        r.fake.state.lock().unwrap().gone_after_run = true;
        r.fake.push_turn_end(TurnEnd::Other {
            status: "timeout".into(),
            message: "still running".into(),
        });
        r.queue_at("proj-1", "run-2", "two", None, "2026-10-14T14:00:02Z");
        r.tick().await;
        assert_eq!(r.status("run-2"), "lost");
        let gone = leader_of(&r, "run-2");
        assert_ne!(first, gone, "each run has its own leader");
        assert!(
            r.transcript("run-2")
                .contains(&format!("the leader {gone} is gone without a result")),
            "{}",
            r.transcript("run-2")
        );
    }

    #[tokio::test]
    async fn a_wrapper_timeout_with_a_live_leader_is_not_lost_and_ends_on_the_leaders_turn_end() {
        let r = rig();
        root_workspace(&r);
        r.fake.add_leader("caller-1", "idle", false);
        r.fake.push_timeout();
        r.fake.push_timeout();
        r.set_logs("agent-2", "[User] build it\nmerged and released v2.82.0\n");
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        assert_eq!(r.status("run-1"), "done", "{}", r.transcript("run-1"));
        assert_eq!(leader_of(&r, "run-1"), "agent-2");
        let transcript = r.transcript("run-1");
        assert!(
            transcript.ends_with("merged and released v2.82.0"),
            "{transcript}"
        );
        assert!(!transcript.contains(MAY_STILL_FINISH), "{transcript}");
        let waits = r
            .fake
            .calls()
            .into_iter()
            .filter(|c| *c == Call::Wait("agent-2".into()))
            .count();
        assert_eq!(
            waits, 2,
            "the leader is waited on until its turn really ends"
        );
        r.tick().await;
        let notices = r.notices();
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(notices[0].1.contains("(done)"), "{notices:?}");

        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(
            r.started_tasks().len(),
            1,
            "the follow-up starts no second leader"
        );
        assert_eq!(
            send_waits(&r),
            vec![("agent-2".to_string(), "add a test".to_string())]
        );
    }

    #[tokio::test]
    async fn a_wrapper_timeout_whose_leader_is_gone_ends_lost_and_still_names_its_leader() {
        let r = rig();
        root_workspace(&r);
        r.fake.state.lock().unwrap().gone_after_run = true;
        r.fake.push_timeout();
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        assert_eq!(r.status("run-1"), "lost");
        assert_eq!(leader_of(&r, "run-1"), "agent-1");
        assert!(
            r.transcript("run-1").contains("agent-1"),
            "{}",
            r.transcript("run-1")
        );

        r.fake.state.lock().unwrap().gone_after_run = false;
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        let transcript = r.transcript("run-2");
        assert!(
            transcript.contains("follow-up of run run-1: its leader agent-1 was gone"),
            "{transcript}"
        );
        assert!(
            !transcript.contains("no leader was ever started"),
            "{transcript}"
        );
    }

    #[tokio::test]
    async fn a_done_run_whose_leader_left_no_reply_says_so() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", None);
        r.tick().await;
        assert_eq!(r.status("run-1"), "done");
        let transcript = r.transcript("run-1");
        assert!(transcript.contains(NO_REPLY), "{transcript}");
        assert!(transcript.contains("agent-1"), "{transcript}");
    }

    #[tokio::test]
    async fn delivery_requeues_a_pre_send_failure_and_loses_it_on_the_fifth() {
        let r = rig();
        r.fake.state.lock().unwrap().workspaces_fail = true;
        r.queue("proj-1", "run-1", "one", Some("caller"));
        r.fake.add_leader("caller", "idle", false);
        for attempt in 1..5 {
            r.tick().await;
            assert_eq!(r.status("run-1"), "queued");
            assert_eq!(r.engine.run_attempts("run-1").unwrap(), Some(attempt));
        }
        r.tick().await;
        assert_eq!(r.status("run-1"), "lost");
        assert!(r.transcript("run-1").contains("`fake workspace` exited"));
        assert!(r.fake.calls().iter().all(|c| !matches!(c, Call::Run(_))));
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
    async fn delivery_restart_requeues_unsent_runs_and_keeps_watching_sent_ones_without_resending()
    {
        let r = rig();
        root_workspace(&r);
        r.fake.add_leader("running-lead", "running", false);
        r.fake.add_leader("caller", "idle", false);
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
        r.engine
            .set_run_pane("run-sent", "paseo:running-lead")
            .unwrap();

        r.tick().await;
        assert_eq!(r.status("run-pending"), "done");
        assert_eq!(r.status("run-sent"), "done");
        assert!(r.fake.calls().contains(&Call::Wait("running-lead".into())));
        assert!(r.fake.calls().iter().all(|c| !matches!(
            c,
            Call::SendAndWait(id, _) if id == "running-lead"
        )));
        assert!(r.fake.calls().iter().all(|c| !matches!(
            c,
            Call::RunDone(id) if id == "running-lead"
        )));
        assert!(!r.fake.archived("running-lead"));
        r.tick().await;
        assert_eq!(r.notices().len(), 1);
        assert_eq!(r.notice_state("run-sent").as_deref(), Some("sent"));
    }

    #[tokio::test]
    async fn delivery_notifies_a_caller_once_and_only_while_idle() {
        let r = rig();
        root_workspace(&r);
        r.fake.add_leader("caller", "running", false);
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
        root_workspace(&r);
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
        root_workspace(&r);
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
        root_workspace(&r);
        r.fake.add_leader("caller", "running", false);
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
        root_workspace(&r);
        r.fake.add_leader("caller", "idle", false);
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
        r.fake.add_leader("plain", "idle", false);
        root_workspace(&r);
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
        assert!(r
            .fake
            .calls()
            .iter()
            .all(|c| !matches!(c, Call::Inspect(_))));
        assert!(r.fake.calls().iter().all(|c| !matches!(
            c,
            Call::RunDone(id) if id == "plain"
        )));
        assert_eq!(r.status("run-1"), "done");
        let agent = agent_of(&r.engine.get_run("run-1").unwrap().unwrap()).to_string();
        assert!(!r.fake.archived(&agent), "a wd run keeps its leader");
        assert!(inbox(&r.engine).is_empty());
        r.tick().await;
        assert!(r.notices().is_empty());
    }

    #[tokio::test]
    async fn delivering_a_wd_run_records_its_leader_and_clears_the_nudge_marks() {
        let r = rig();
        root_workspace(&r);
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
        assert_eq!(r.started_tasks(), vec![task]);
        let delivered_to = agent_of(&r.engine.get_run("run-1").unwrap().unwrap()).to_string();
        let t = r.engine.get_task("corr-4").unwrap().unwrap();
        assert_eq!(t.leader_id.as_deref(), Some(delivered_to.as_str()));
        assert!(
            !r.fake.archived(&delivered_to),
            "a wd run keeps its leader for reply and redo"
        );
        let delivered = t.last_delivered_at.unwrap();
        assert!(delivered >= before, "{delivered} < {before}");
        assert_eq!(t.changed_at, delivered);
        assert_eq!((t.nudged_at, t.progress_sent_at), (None, None));
    }

    #[tokio::test]
    async fn delivering_a_non_wd_run_touches_no_task() {
        let r = rig();
        root_workspace(&r);
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

    fn hand_off(r: &Rig, id: &str, corr: &str, text: &str) -> String {
        let task = paseo::wd_task(corr, text);
        r.engine
            .insert_task(corr, "proj-1", "2026-10-14T13:00:00Z")
            .unwrap();
        r.queue("proj-1", id, &task, Some(paseo::WD_CALLER));
        r.engine.set_run_request(id, corr).unwrap();
        id.to_string()
    }

    fn follow_up(r: &Rig, id: &str, corr: &str, n: usize, text: &str) {
        let task = paseo::wd_task(corr, &format!("Reply from the user:\n{text}"));
        r.queue("proj-1", id, &task, Some(paseo::WD_CALLER));
        r.engine
            .set_run_request(id, &format!("{corr}:{n}"))
            .unwrap();
    }

    fn send_waits(r: &Rig) -> Vec<(String, String)> {
        r.fake
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::SendAndWait(id, text) => Some((id, text)),
                _ => None,
            })
            .collect()
    }

    fn queue_follow(r: &Rig, id: &str, of: &str, text: &str, at: &str) {
        let run = Run {
            id: id.into(),
            project_id: "proj-1".into(),
            pane_id: "paseo:pending".into(),
            preset_label: None,
            task: text.into(),
            baseline: at.into(),
            marker: String::new(),
            status: "queued".into(),
            created_at: at.into(),
            updated_at: at.into(),
        };
        r.engine
            .insert_follow_up_run(&run, None, Some("caller-1"), Some(of), false)
            .unwrap();
    }

    fn leader_of(r: &Rig, id: &str) -> String {
        agent_of(&r.engine.get_run(id).unwrap().unwrap()).to_string()
    }

    #[tokio::test]
    async fn a_follow_up_of_goes_to_the_idle_leader_of_the_run_it_names() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let leader = leader_of(&r, "run-1");
        r.fake.set_status(&leader, "idle");
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(r.started_tasks(), vec!["build it".to_string()]);
        assert_eq!(
            send_waits(&r),
            vec![(leader.clone(), "add a test".to_string())]
        );
        assert_eq!(r.status("run-2"), "done");
        assert_eq!(leader_of(&r, "run-2"), leader);
        assert!(!r.fake.archived(&leader));
    }

    #[tokio::test]
    async fn a_follow_up_of_a_running_leader_waits_queued_and_sends_once_idle() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let leader = leader_of(&r, "run-1");
        r.fake.set_status(&leader, "running");
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        assert!(send_waits(&r).is_empty(), "no send into a running turn");
        assert_eq!(r.status("run-2"), "queued");
        assert_eq!(r.started_tasks().len(), 1, "no second leader");
        r.fake.set_status(&leader, "idle");
        r.tick().await;
        assert_eq!(send_waits(&r).len(), 1);
        assert_eq!(r.status("run-2"), "done");
        assert_eq!(r.started_tasks().len(), 1);
    }

    #[tokio::test]
    async fn a_follow_up_of_an_archived_leader_starts_one_carrying_the_original_task() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let old = leader_of(&r, "run-1");
        r.fake.archive(&old).await.unwrap();
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        let started = r.started_tasks();
        assert_eq!(started, vec!["build it", "build it\n\nadd a test"]);
        assert!(send_waits(&r).is_empty());
        let fresh = leader_of(&r, "run-2");
        assert_ne!(fresh, old);
        assert_eq!(r.status("run-2"), "done");
        let transcript = r.transcript("run-2");
        assert!(
            transcript.contains(&format!(
                "follow-up of run run-1: its leader {old} was gone"
            )),
            "{transcript}"
        );
    }

    #[tokio::test]
    async fn a_steer_to_an_archived_leader_ends_lost_and_starts_no_leader() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let old = leader_of(&r, "run-1");
        r.fake.archive(&old).await.unwrap();
        let note = paseo::steer_task("new leader run-9 works nearby");
        queue_follow(&r, "run-2", "run-1", &note, "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(r.started_tasks(), vec!["build it"]);
        assert!(send_waits(&r).is_empty());
        assert_eq!(r.status("run-2"), "lost");
        let transcript = r.transcript("run-2");
        assert!(
            transcript.contains("steer note for run run-1 not delivered"),
            "{transcript}"
        );
    }

    #[tokio::test]
    async fn a_steer_to_an_idle_leader_is_sent_to_it() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let leader = leader_of(&r, "run-1");
        r.fake.set_status(&leader, "idle");
        let note = paseo::steer_task("new leader run-9 works nearby");
        queue_follow(&r, "run-2", "run-1", &note, "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(r.started_tasks(), vec!["build it"]);
        assert_eq!(leader_of(&r, "run-2"), leader);
        assert_eq!(r.status("run-2"), "done");
    }

    #[tokio::test]
    async fn a_chain_of_follow_ups_follows_the_newest_leader() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let first = leader_of(&r, "run-1");
        r.fake.archive(&first).await.unwrap();
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        let second = leader_of(&r, "run-2");
        assert_ne!(second, first);
        r.fake.set_status(&second, "idle");
        queue_follow(&r, "run-3", "run-1", "and a doc", "2026-10-14T14:00:09Z");
        r.tick().await;
        assert_eq!(r.status("run-3"), "done");
        r.fake.set_status(&second, "idle");
        queue_follow(
            &r,
            "run-4",
            "run-2",
            "and a changelog",
            "2026-10-14T14:00:10Z",
        );
        r.tick().await;
        assert_eq!(r.status("run-4"), "done");
        assert_eq!(
            send_waits(&r),
            vec![
                (second.clone(), "and a doc".to_string()),
                (second.clone(), "and a changelog".to_string())
            ]
        );
        assert_eq!(r.started_tasks().len(), 2);
        assert_eq!(leader_of(&r, "run-4"), second);
    }

    #[tokio::test]
    async fn a_follow_up_waits_while_an_earlier_run_of_its_family_is_still_open() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let leader = leader_of(&r, "run-1");
        r.fake.set_status(&leader, "idle");
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.engine
            .update_run_status("run-2", "working", &now_rfc3339(), None, None)
            .unwrap();
        queue_follow(&r, "run-3", "run-2", "and a doc", "2026-10-14T14:00:09Z");
        r.tick().await;
        assert_eq!(r.status("run-3"), "queued");
        assert!(send_waits(&r).is_empty(), "no send while run-2 is open");
        r.engine
            .finish_run("run-2", "done", &now_rfc3339(), Some("ok"), None)
            .unwrap();
        r.tick().await;
        assert_eq!(r.status("run-3"), "done");
        assert_eq!(send_waits(&r), vec![(leader, "and a doc".to_string())]);
    }

    #[tokio::test]
    async fn a_follow_up_of_a_run_whose_leader_never_started_starts_one_with_both_tasks() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.engine
            .finish_run("run-1", "lost", &now_rfc3339(), Some("never sent"), None)
            .unwrap();
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(r.started_tasks(), vec!["build it\n\nadd a test"]);
        assert!(r
            .transcript("run-2")
            .contains("no leader was ever started for it"));
    }

    #[tokio::test]
    async fn a_fresh_leader_carrying_both_tasks_reports_its_reply() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.engine
            .finish_run("run-1", "lost", &now_rfc3339(), Some("never sent"), None)
            .unwrap();
        r.set_logs(
            "agent-1",
            "[User] build it\n\nadd a test\nbuilt it and added the test\n",
        );
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(r.status("run-2"), "done");
        let transcript = r.transcript("run-2");
        assert!(
            transcript.ends_with("built it and added the test"),
            "{transcript}"
        );
    }

    #[tokio::test]
    async fn a_reply_goes_to_the_tasks_idle_leader_and_starts_no_new_one() {
        let r = rig();
        root_workspace(&r);
        hand_off(&r, "run-1", "corr-7", "build it");
        r.tick().await;
        let leader = agent_of(&r.engine.get_run("run-1").unwrap().unwrap()).to_string();
        r.fake.set_status(&leader, "idle");
        follow_up(&r, "run-2", "corr-7", 1, "add a test");
        r.tick().await;
        let started = r.started_tasks();
        assert_eq!(started, vec![paseo::wd_task("corr-7", "build it")]);
        let sent = send_waits(&r);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert_eq!(sent[0].0, leader);
        assert!(sent[0].1.ends_with("Reply from the user:\nadd a test"));
        assert_eq!(r.status("run-2"), "done");
        assert_eq!(
            r.engine.get_run("run-2").unwrap().unwrap().pane_id,
            format!("paseo:{leader}")
        );
        assert!(
            !r.fake.archived(&leader),
            "a wd leader stays for the next reply"
        );
        assert_eq!(
            r.engine
                .get_task("corr-7")
                .unwrap()
                .unwrap()
                .leader_id
                .as_deref(),
            Some(leader.as_str())
        );
    }

    #[tokio::test]
    async fn a_reply_to_a_running_leader_waits_queued_until_it_is_idle() {
        let r = rig();
        root_workspace(&r);
        hand_off(&r, "run-1", "corr-8", "build it");
        r.tick().await;
        let leader = agent_of(&r.engine.get_run("run-1").unwrap().unwrap()).to_string();
        r.fake.set_status(&leader, "running");
        follow_up(&r, "run-2", "corr-8", 1, "add a test");
        r.tick().await;
        assert!(send_waits(&r).is_empty(), "no send mid-turn");
        assert_eq!(r.status("run-2"), "queued");
        assert_eq!(r.started_tasks().len(), 1, "no second leader");
        r.fake.set_status(&leader, "idle");
        r.tick().await;
        let sent = send_waits(&r);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert_eq!(sent[0].0, leader);
        assert_eq!(r.status("run-2"), "done");
        assert_eq!(r.started_tasks().len(), 1);
    }

    #[tokio::test]
    async fn a_reply_whose_leader_is_archived_starts_one_that_carries_the_original_task() {
        let r = rig();
        root_workspace(&r);
        hand_off(&r, "run-1", "corr-9", "build it");
        r.tick().await;
        let old = agent_of(&r.engine.get_run("run-1").unwrap().unwrap()).to_string();
        r.fake.archive(&old).await.unwrap();
        follow_up(&r, "run-2", "corr-9", 1, "add a test");
        r.tick().await;
        let started = r.started_tasks();
        assert_eq!(started.len(), 2, "{started:?}");
        let prompt = &started[1];
        let original = paseo::wd_task("corr-9", "build it");
        assert!(prompt.starts_with(&original), "{prompt}");
        assert!(
            prompt.contains("\n\nReply from the user:\nadd a test"),
            "{prompt}"
        );
        assert!(send_waits(&r).is_empty());
        let fresh = agent_of(&r.engine.get_run("run-2").unwrap().unwrap()).to_string();
        assert_ne!(fresh, old);
        assert_eq!(
            r.engine
                .get_task("corr-9")
                .unwrap()
                .unwrap()
                .leader_id
                .as_deref(),
            Some(fresh.as_str())
        );
    }

    #[tokio::test]
    async fn a_wd_run_that_is_not_a_follow_up_still_starts_its_own_leader() {
        let r = rig();
        root_workspace(&r);
        r.fake.set_status("idle-lead", "idle");
        hand_off(&r, "run-1", "corr-10", "build it");
        r.engine
            .mark_task_delivered("corr-10", "idle-lead", "2026-10-14T13:00:00Z")
            .unwrap();
        r.tick().await;
        let started = r.started_tasks();
        assert_eq!(started, vec![paseo::wd_task("corr-10", "build it")]);
        assert!(send_waits(&r).is_empty());
        let fresh = agent_of(&r.engine.get_run("run-1").unwrap().unwrap()).to_string();
        assert_ne!(fresh, "idle-lead");
        assert!(r
            .fake
            .calls()
            .iter()
            .all(|c| !matches!(c, Call::Inspect(_))));
    }

    #[tokio::test]
    async fn a_blocked_wd_run_mails_no_permission_and_sends_no_notice() {
        let r = rig();
        root_workspace(&r);
        r.fake.set_permissions("agent-1", &["Write"]);
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
        r.fake.set_permissions("agent-1", &[]);
        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
        assert_eq!(r.status("run-1"), "done");
        assert!(!r.fake.archived("agent-1"));
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
        root_workspace(&r);
        r.fake.state.lock().unwrap().gone_after_run = true;
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
        assert!(
            mail[0].body.contains("gone without a result"),
            "{}",
            mail[0].body
        );
        r.tick().await;
        assert!(r.notices().is_empty());
    }

    #[test]
    fn delivery_reply_is_the_leader_text_after_the_last_task_line() {
        let logs = "[User] one\nold\n[User] one\nsecond line\nnew reply\n[User] other\n";
        assert_eq!(leader_reply(logs, "one\nsecond line"), "new reply");
        assert_eq!(leader_reply("[User] two\nx\n", "one"), "");
    }

    fn clock(text: &str) -> time::OffsetDateTime {
        time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339).unwrap()
    }

    const NOW: &str = "2026-11-02T12:00:00Z";

    #[tokio::test]
    async fn the_idle_sweep_archives_a_leader_idle_for_over_a_day() {
        let r = rig();
        r.fake
            .add_listed_leader("stale-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.sweep_at(clock(NOW)).await;
        assert!(r.fake.archived("stale-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_keeps_a_leader_updated_within_the_limit() {
        let r = rig();
        r.fake
            .add_listed_leader("fresh-lead", "idle", Some("2026-11-02T11:45:00Z"));
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived("fresh-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_archives_at_31_minutes_and_not_at_29() {
        let r = rig();
        r.fake
            .add_listed_leader("fresh-lead", "idle", Some("2026-11-02T11:31:00Z"));
        r.fake
            .add_listed_leader("stale-lead", "idle", Some("2026-11-02T11:29:00Z"));
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived("fresh-lead"));
        assert!(r.fake.archived("stale-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_keeps_a_leader_whose_run_is_blocked() {
        let r = rig();
        root_workspace(&r);
        r.fake
            .add_listed_leader("blocked-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.queue("proj-1", "run-1", "build it", None);
        r.engine
            .set_run_pane("run-1", &format!("{PASEO_PANE_PREFIX}blocked-lead"))
            .unwrap();
        r.engine.claim_queued_run("run-1").unwrap();
        r.engine
            .update_run_status("run-1", "blocked", &now_rfc3339(), None, None)
            .unwrap();
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived("blocked-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_keeps_a_leader_whose_family_has_a_queued_run() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let leader = leader_of(&r, "run-1");
        r.fake
            .add_listed_leader(&leader, "idle", Some("2026-11-01T11:00:00Z"));
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived(&leader));
        r.engine
            .finish_run("run-2", "done", &now_rfc3339(), Some("ok"), None)
            .unwrap();
        r.sweep_at(clock(NOW)).await;
        assert!(r.fake.archived(&leader));
    }

    fn queue_handoff(r: &Rig, id: &str, of: &str, text: &str, at: &str) {
        let run = Run {
            id: id.into(),
            project_id: "proj-1".into(),
            pane_id: "paseo:pending".into(),
            preset_label: None,
            task: text.into(),
            baseline: at.into(),
            marker: String::new(),
            status: "queued".into(),
            created_at: at.into(),
            updated_at: at.into(),
        };
        r.engine
            .insert_follow_up_run(&run, None, Some("caller-1"), Some(of), true)
            .unwrap();
    }

    async fn settle_archive(r: &Rig, leader: &str) -> bool {
        for _ in 0..200 {
            if r.fake.archived(leader) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        false
    }

    #[tokio::test]
    async fn a_handoff_archives_the_old_leader_only_once_it_is_idle() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let old = leader_of(&r, "run-1");
        r.fake.set_status(&old, "running");
        queue_handoff(&r, "run-2", "run-1", "carry on", "2026-10-14T14:00:05Z");
        r.tick().await;
        assert_eq!(r.started_tasks(), vec!["build it", "carry on"]);
        assert!(send_waits(&r).is_empty());
        let fresh = leader_of(&r, "run-2");
        assert_ne!(fresh, old);
        assert!(r.transcript("run-2").contains(&format!(
            "handoff of run run-1: takes over from leader {old}"
        )));
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!r.fake.archived(&old), "a running old leader is kept");
        r.fake.set_status(&old, "idle");
        assert!(settle_archive(&r, &old).await);
        assert!(!r.fake.archived(&fresh));
    }

    #[tokio::test]
    async fn a_follow_up_of_an_old_run_goes_to_the_successor_even_when_queued_before_the_handoff() {
        let r = rig();
        root_workspace(&r);
        r.queue("proj-1", "run-1", "build it", Some("caller-1"));
        r.tick().await;
        let old = leader_of(&r, "run-1");
        r.fake.set_status(&old, "idle");
        queue_follow(&r, "run-2", "run-1", "add a test", "2026-10-14T14:00:05Z");
        queue_handoff(&r, "run-3", "run-1", "carry on", "2026-10-14T14:00:06Z");
        r.engine
            .update_run_status("run-3", "working", &now_rfc3339(), None, None)
            .unwrap();
        r.tick().await;
        assert_eq!(r.status("run-2"), "queued", "waits for the open handoff");
        r.engine
            .set_run_pane("run-3", &format!("{PASEO_PANE_PREFIX}successor"))
            .unwrap();
        r.fake.add_leader("successor", "idle", false);
        r.engine
            .finish_run("run-3", "done", &now_rfc3339(), Some("ok"), None)
            .unwrap();
        r.tick().await;
        assert_eq!(
            send_waits(&r),
            vec![("successor".to_string(), "add a test".to_string())]
        );
        queue_follow(&r, "run-4", "run-1", "and a doc", "2026-10-14T14:00:09Z");
        r.tick().await;
        assert_eq!(send_waits(&r).len(), 2);
        assert_eq!(send_waits(&r)[1].0, "successor");
    }

    #[tokio::test]
    async fn the_idle_sweep_keeps_a_leader_that_is_still_running() {
        let r = rig();
        r.fake
            .add_listed_leader("busy-lead", "running", Some("2026-10-31T00:00:00Z"));
        r.fake.add_listed_leader(
            "starting-lead",
            "initializing",
            Some("2026-10-31T00:00:00Z"),
        );
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived("busy-lead"));
        assert!(!r.fake.archived("starting-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_keeps_a_leader_with_no_timestamp_or_an_unreadable_one() {
        let r = rig();
        r.fake.add_listed_leader("undated-lead", "idle", None);
        r.fake
            .add_listed_leader("garbled-lead", "idle", Some("yesterday"));
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived("undated-lead"));
        assert!(!r.fake.archived("garbled-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_archives_a_via_wd_leader_too() {
        let r = rig();
        r.fake
            .add_listed_leader("wd-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.fake.label("wd-lead", paseo::WD_LABEL);
        r.sweep_at(clock(NOW)).await;
        assert!(r.fake.archived("wd-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_carries_on_past_a_leader_it_could_not_archive() {
        let r = rig();
        r.fake
            .add_listed_leader("stuck-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.fake
            .add_listed_leader("next-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.fake.fail_archive("stuck-lead");
        r.sweep_at(clock(NOW)).await;
        assert!(!r.fake.archived("stuck-lead"));
        assert!(r.fake.archived("next-lead"));
    }

    #[tokio::test]
    async fn the_idle_sweep_keeps_the_leader_of_a_run_in_flight() {
        let r = rig();
        root_workspace(&r);
        r.fake
            .add_listed_leader("working-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.queue("proj-1", "run-1", "build it", None);
        r.engine
            .set_run_pane("run-1", &format!("{PASEO_PANE_PREFIX}working-lead"))
            .unwrap();
        r.engine.claim_queued_run("run-1").unwrap();
        r.sweep_at(clock(NOW)).await;
        assert!(
            !r.fake.archived("working-lead"),
            "a leader behind a live run is not archived"
        );
    }

    #[tokio::test]
    async fn the_idle_sweep_skips_a_leader_it_could_not_inspect_and_sweeps_the_rest() {
        let r = rig();
        r.fake
            .add_listed_leader("ghost-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.fake
            .add_listed_leader("next-lead", "idle", Some("2026-11-01T11:00:00Z"));
        r.fake.state.lock().unwrap().agents.remove("ghost-lead");
        r.sweep_at(clock(NOW)).await;
        assert!(r.fake.archived("next-lead"));
    }

    #[tokio::test]
    async fn a_running_delivery_loop_archives_a_leader_left_idle_for_a_day() {
        let r = rig();
        let stale = (time::OffsetDateTime::now_utc() - time::Duration::hours(25))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        r.fake
            .add_listed_leader("forgotten-lead", "idle", Some(&stale));
        let task = tokio::spawn(r.delivery.clone().run());
        let mut archived = false;
        for _ in 0..200 {
            archived = r.fake.archived("forgotten-lead");
            if archived {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        task.abort();
        assert!(
            archived,
            "the delivery loop archives an idle leader with no dispatch and no OpenClaw"
        );
    }

    #[tokio::test]
    async fn the_idle_sweep_never_overlaps_itself() {
        let r = rig();
        let stale = (time::OffsetDateTime::now_utc() - time::Duration::hours(25))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        r.fake.add_listed_leader("stale-lead", "idle", Some(&stale));
        let mut set = JoinSet::new();
        r.delivery.sweep_idle_leaders(&mut set);
        r.delivery.sweep_idle_leaders(&mut set);
        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
        assert!(r.fake.archived("stale-lead"));
        assert!(
            r.fake
                .calls()
                .iter()
                .filter(|c| **c == Call::Archive("stale-lead".into()))
                .count()
                == 1,
            "the second sweep waits instead of archiving twice"
        );
        assert!(!r.delivery.state.lock().unwrap().sweeping);
    }
}
