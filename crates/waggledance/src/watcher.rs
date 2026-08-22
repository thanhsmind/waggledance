//! Status watcher — turns herdr agent status into de-duplicated change
//! events (D1: ported from herdr-go's `watcher.rs`). Polling only: herdr's
//! socket API is request/response, so the cursor de-dup below is what keeps
//! a duplicate snapshot read (or a status that has not actually changed)
//! from surfacing as a fresh event.
//!
//! Wired in behind the D7 opt-in switch by `crate::TerminalBackground`
//! (`crates/waggledance/src/main.rs`): `reconcile` is the only place a
//! [`PollWatcher`] is ever run, and a switch left off drives it to poll
//! nothing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use waggledance_core::bee::{read_snapshot, BeeActivityState, BeeSnapshot};

use crate::herdr::{AgentStatus, Herdr};

/// A de-duplicated status change worth acting on (e.g. notifying a human).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusChange {
    pub pane_id: String,
    pub kind: String,
    pub status: AgentStatus,
}

/// Tracks the last status seen per pane so only real transitions surface, and
/// the same transition never surfaces twice.
#[derive(Default)]
pub struct StatusCursor {
    last: HashMap<String, AgentStatus>,
}

impl StatusCursor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a fresh snapshot's agent statuses; return only the changes not
    /// already seen. A pane whose status is unchanged yields nothing; a
    /// repeated duplicate event yields nothing.
    pub fn diff(&mut self, statuses: &[(String, String, AgentStatus)]) -> Vec<StatusChange> {
        let mut out = Vec::new();
        for (pane_id, kind, status) in statuses {
            match self.last.get(pane_id) {
                Some(prev) if prev == status => {} // unchanged / duplicate
                _ => {
                    self.last.insert(pane_id.clone(), *status);
                    out.push(StatusChange {
                        pane_id: pane_id.clone(),
                        kind: kind.clone(),
                        status: *status,
                    });
                }
            }
        }
        out
    }
}

/// Flatten a snapshot into (pane_id, agent_kind, status) triples.
pub fn statuses_from(snap: &crate::herdr::Snapshot) -> Vec<(String, String, AgentStatus)> {
    snap.agents
        .iter()
        .map(|a| (a.pane_id.clone(), a.kind.clone(), a.status))
        .collect()
}

/// A de-duplicated bee agent-activity transition worth a human's attention
/// (A5). Only two things ever surface: an *entry* into the need-you family
/// ({`blocked`, `waiting_input`}) from outside it, and the move to
/// `exited`. Escalation inside the family (`waiting_input` -> `blocked`) and
/// leaving it never do -- the human already knows they are needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTransition {
    pub session_id: String,
    /// The session's herdr pane when it runs in one (A2) -- what the notify
    /// path suppresses on and what the body names.
    pub pane: Option<String>,
    /// The state this cursor last saw for the session; `None` on first
    /// sight (a session already blocked when waggledance starts still
    /// counts as an entry and fires -- see this cell's trace).
    pub from: Option<BeeActivityState>,
    pub to: BeeActivityState,
}

/// Tracks the last bee activity state seen per session so only the
/// transitions [`ActivityTransition`] describes surface, and each surfaces
/// once. The bee-side twin of [`StatusCursor`], fed from the same 2 s tick
/// (A5) -- a separate cursor because it speaks a separate vocabulary (bee's
/// `activity.state`, not herdr's screen-derived status).
#[derive(Default)]
pub struct ActivityCursor {
    last: HashMap<String, BeeActivityState>,
}

impl ActivityCursor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a fresh batch of `(session_id, pane, state)`; return only the
    /// notifiable transitions. An unchanged state yields nothing (neither
    /// rule can fire when `from == to`), so a repeated snapshot read is
    /// silent without a separate dedup branch.
    pub fn diff(
        &mut self,
        states: &[(String, Option<String>, BeeActivityState)],
    ) -> Vec<ActivityTransition> {
        let mut out = Vec::new();
        for (session_id, pane, to) in states {
            let from = self.last.get(session_id).cloned();
            let entered_need_you = to.needs_you() && !from.as_ref().is_some_and(|f| f.needs_you());
            let just_exited =
                *to == BeeActivityState::Exited && from.as_ref() != Some(&BeeActivityState::Exited);
            self.last.insert(session_id.clone(), to.clone());
            if entered_need_you || just_exited {
                out.push(ActivityTransition {
                    session_id: session_id.clone(),
                    pane: pane.clone(),
                    from,
                    to: to.clone(),
                });
            }
        }
        out
    }
}

/// Flatten a bee snapshot into `(session_id, pane, state)` triples -- live
/// sessions carrying an activity record, which is the only population the
/// notifier speaks for (a dead session's state is history, not news).
pub fn activity_states_from(snap: &BeeSnapshot) -> Vec<(String, Option<String>, BeeActivityState)> {
    snap.sessions
        .iter()
        .filter(|s| s.live)
        .filter_map(|s| {
            s.activity
                .as_ref()
                .map(|a| (s.id.clone(), a.pane.clone(), a.state.clone()))
        })
        .collect()
}

/// A port for asking which project roots to read bee activity from, so the
/// watcher never takes a dependency on the registry (the same hexagonal
/// shape `notify::RunOwnership` uses for run ownership). Re-asked every
/// tick, so a project registered while the watcher runs is picked up
/// without a restart.
pub trait BeeRoots: Send + Sync {
    fn roots(&self) -> Vec<PathBuf>;
}

impl<F> BeeRoots for F
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    fn roots(&self) -> Vec<PathBuf> {
        self()
    }
}

/// One tick's worth of news, from either cursor -- a single event type so
/// the loop keeps one handler while carrying two vocabularies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    Status(StatusChange),
    Activity(ActivityTransition),
}

/// Poll-based event source. Emits de-duplicated status changes on `sink`.
pub struct PollWatcher {
    control: Arc<dyn Herdr>,
    interval: Duration,
    /// `None` (the default) polls herdr only -- bee activity needs a root
    /// source, and without one the activity cursor is simply never fed.
    bee_roots: Option<Arc<dyn BeeRoots>>,
}

impl PollWatcher {
    pub fn new(control: Arc<dyn Herdr>, interval: Duration) -> Self {
        PollWatcher {
            control,
            interval,
            bee_roots: None,
        }
    }

    /// Also poll bee agent activity across `roots`, on the same tick.
    pub fn with_bee_roots(mut self, roots: Arc<dyn BeeRoots>) -> Self {
        self.bee_roots = Some(roots);
        self
    }

    /// Run one bee-activity poll cycle against a cursor. Returns nothing
    /// when no root source is configured or no root is registered.
    /// `read_snapshot` is synchronous file I/O, so the reads go through
    /// `spawn_blocking` -- the same rule `server::cross_project_rollup`
    /// already follows for this reader.
    pub async fn poll_activity_once(&self, cursor: &mut ActivityCursor) -> Vec<ActivityTransition> {
        let Some(source) = self.bee_roots.clone() else {
            return Vec::new();
        };
        let roots = source.roots();
        if roots.is_empty() {
            return Vec::new();
        }
        let states = tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            for root in roots {
                out.extend(activity_states_from(&read_snapshot(&root)));
            }
            out
        })
        .await
        // A join failure yields nothing -- never a spurious transition.
        .unwrap_or_default();
        cursor.diff(&states)
    }

    /// Run one poll cycle against a cursor, returning fresh changes. Extracted
    /// so tests drive it deterministically without sleeping.
    pub async fn poll_once(&self, cursor: &mut StatusCursor) -> Vec<StatusChange> {
        match self.control.snapshot().await {
            Ok(snap) => cursor.diff(&statuses_from(&snap)),
            // A failed snapshot yields nothing — never a spurious change.
            Err(_) => Vec::new(),
        }
    }

    /// Run the poll loop forever, invoking `on_change` for each fresh change.
    /// Superseded in production by [`run_async`](Self::run_async), which
    /// also threads a tick counter for observability — kept as the simpler
    /// public entry point this type's own doc promises, not test-exercised
    /// today.
    #[allow(dead_code)]
    pub async fn run<F>(self, mut on_change: F)
    where
        F: FnMut(StatusChange) + Send,
    {
        let mut cursor = StatusCursor::new();
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            for change in self.poll_once(&mut cursor).await {
                on_change(change);
            }
        }
    }

    /// Like [`run`](Self::run) but awaits an async handler for each event —
    /// used to feed the notify service (record → drain) per event. Both
    /// cursors run on this one tick (A5): herdr status first, then bee
    /// agent activity.
    /// `ticks` counts every completed poll cycle — a real,
    /// externally-observable side effect of the loop still running, so a
    /// caller proving "switched off stops the task" has something to
    /// observe beyond its own bookkeeping (`crate::TerminalBackground`,
    /// this cell's trace).
    pub async fn run_async<F, Fut>(self, ticks: Arc<AtomicU64>, mut on_event: F)
    where
        F: FnMut(WatchEvent) -> Fut + Send,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let mut cursor = StatusCursor::new();
        let mut activity = ActivityCursor::new();
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            for change in self.poll_once(&mut cursor).await {
                on_event(WatchEvent::Status(change)).await;
            }
            for transition in self.poll_activity_once(&mut activity).await {
                on_event(WatchEvent::Activity(transition)).await;
            }
            ticks.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;

    #[test]
    fn cursor_emits_only_real_changes_and_dedups() {
        let mut cursor = StatusCursor::new();
        let batch = vec![
            ("p1".to_string(), "claude".to_string(), AgentStatus::Working),
            ("p2".to_string(), "codex".to_string(), AgentStatus::Idle),
        ];
        // First observation: both are new.
        assert_eq!(cursor.diff(&batch).len(), 2);
        // Same batch again (duplicate events / replay): nothing.
        assert_eq!(cursor.diff(&batch).len(), 0);
        // p1 transitions to blocked: exactly one change.
        let batch2 = vec![
            ("p1".to_string(), "claude".to_string(), AgentStatus::Blocked),
            ("p2".to_string(), "codex".to_string(), AgentStatus::Idle),
        ];
        let changes = cursor.diff(&batch2);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, AgentStatus::Blocked);
    }

    fn batch(
        rows: &[(&str, Option<&str>, BeeActivityState)],
    ) -> Vec<(String, Option<String>, BeeActivityState)> {
        rows.iter()
            .map(|(id, pane, st)| (id.to_string(), pane.map(|p| p.to_string()), st.clone()))
            .collect()
    }

    /// A5's whole transition rule in one table: entry into the need-you
    /// family fires once, escalation *inside* it fires nothing, leaving it
    /// fires nothing, re-entry fires again, and `exited` fires exactly once.
    #[test]
    fn activity_cursor_fires_on_entry_and_exit_only() {
        let mut cursor = ActivityCursor::new();
        let s = |st: BeeActivityState| batch(&[("s1", Some("w1:p1"), st)]);

        // working -> waiting_input: an entry into the family.
        assert_eq!(cursor.diff(&s(BeeActivityState::Working)).len(), 0);
        let entered = cursor.diff(&s(BeeActivityState::WaitingInput));
        assert_eq!(entered.len(), 1);
        assert_eq!(entered[0].session_id, "s1");
        assert_eq!(entered[0].pane.as_deref(), Some("w1:p1"));
        assert_eq!(entered[0].from, Some(BeeActivityState::Working));
        assert_eq!(entered[0].to, BeeActivityState::WaitingInput);
        // Repeat of the same state: nothing (dedup).
        assert_eq!(cursor.diff(&s(BeeActivityState::WaitingInput)).len(), 0);
        // waiting_input -> blocked: escalation inside the family, not an entry.
        assert_eq!(cursor.diff(&s(BeeActivityState::Blocked)).len(), 0);
        // blocked -> idle: leaving the family is never news.
        assert_eq!(cursor.diff(&s(BeeActivityState::Idle)).len(), 0);
        // idle -> blocked: an entry again.
        assert_eq!(cursor.diff(&s(BeeActivityState::Blocked)).len(), 1);
        // -> exited fires once; exited -> exited nothing.
        let exited = cursor.diff(&s(BeeActivityState::Exited));
        assert_eq!(exited.len(), 1);
        assert_eq!(exited[0].to, BeeActivityState::Exited);
        assert_eq!(cursor.diff(&s(BeeActivityState::Exited)).len(), 0);
    }

    /// First sight of a session already needing a human counts as an entry
    /// (`from == None`) -- otherwise a session that went blocked before
    /// waggledance started would never be announced at all.
    #[test]
    fn first_sight_of_a_blocked_session_fires() {
        let mut cursor = ActivityCursor::new();
        let fired = cursor.diff(&batch(&[("s1", None, BeeActivityState::Blocked)]));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].from, None);
        assert_eq!(fired[0].pane, None);
        // First sight of a working session is not news.
        let mut other = ActivityCursor::new();
        assert!(other
            .diff(&batch(&[("s2", None, BeeActivityState::Working)]))
            .is_empty());
    }

    /// Sessions are tracked independently -- one session's entry never
    /// suppresses another's.
    #[test]
    fn cursor_tracks_each_session_separately() {
        let mut cursor = ActivityCursor::new();
        cursor.diff(&batch(&[
            ("s1", None, BeeActivityState::Working),
            ("s2", None, BeeActivityState::Working),
        ]));
        let fired = cursor.diff(&batch(&[
            ("s1", None, BeeActivityState::Blocked),
            ("s2", None, BeeActivityState::Working),
        ]));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].session_id, "s1");
    }

    /// Without a root source the activity cursor is never fed -- the herdr
    /// path is unchanged for a deployment with no registry behind it.
    #[tokio::test]
    async fn activity_poll_without_roots_yields_nothing() {
        let watcher = PollWatcher::new(Arc::new(FakeHerdr::new()), Duration::from_millis(500));
        let mut cursor = ActivityCursor::new();
        assert!(watcher.poll_activity_once(&mut cursor).await.is_empty());
    }

    /// A root with no `.bee/` reads as an absent snapshot, not an error,
    /// and yields no transition.
    #[tokio::test]
    async fn activity_poll_over_a_beeless_root_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let watcher = PollWatcher::new(Arc::new(FakeHerdr::new()), Duration::from_millis(500))
            .with_bee_roots(Arc::new(move || vec![root.clone()]));
        let mut cursor = ActivityCursor::new();
        assert!(watcher.poll_activity_once(&mut cursor).await.is_empty());
    }

    #[tokio::test]
    async fn poll_once_reports_driven_transition_once() {
        let fake = Arc::new(FakeHerdr::new());
        let watcher = PollWatcher::new(fake.clone(), Duration::from_millis(500));
        let mut cursor = StatusCursor::new();
        // First poll: seeds all four seeded panes as "new".
        let first = watcher.poll_once(&mut cursor).await;
        assert_eq!(first.len(), 4);
        // No change → no events.
        assert_eq!(watcher.poll_once(&mut cursor).await.len(), 0);
        // Drive the idle agent → done.
        fake.set_status("w2:p4", AgentStatus::Done).await.unwrap();
        let changes = watcher.poll_once(&mut cursor).await;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].pane_id, "w2:p4");
        assert_eq!(changes[0].status, AgentStatus::Done);
        // Polling again with no change → still nothing (dedup).
        assert_eq!(watcher.poll_once(&mut cursor).await.len(), 0);
    }
}
