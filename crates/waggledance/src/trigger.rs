//! The observer-tick trigger — the in-daemon task that wakes exactly one
//! cold `bee supervisor` observation tick per detected fleet transition
//! (feature `observer-tick-trigger`, D1 `45a554bb` / D2 / D3 / D5–D10).
//!
//! `orchestrator-dispatch` D1 is still the rule: waggledance never chooses
//! WHAT to dispatch through the MCP surface, an external agent does. This
//! task is the one narrow, logged exception to it (D1 above), and it is
//! narrow in a way the code has to keep: the task text is a single fixed
//! template with two values substituted into it — the transition kind and a
//! minimal evidence pointer — so this module can decide only *whether* to
//! wake an observer, never what to tell it. It states no finding, suggests
//! no action, and keeps no record of what it saw (D5): every write after a
//! tick fires belongs to the woken agent, inside the target repo.
//!
//! "trigger" is the Rust name; "observer-tick" is the human-facing one
//! (D2). Nothing here is called `supervisor` — `crates/waggledance/src/supervisor.rs`
//! is the unrelated herdr watchdog.
//!
//! # The shape
//!
//! One task, one poll tick ([`TRIGGER_POLL_INTERVAL`]), and one shared gate
//! ([`Trigger::maybe_dispatch`]) that every detector funnels through. D3's
//! "event-driven, never a timer" lives in what reaches that gate: a
//! transition, edge-detected by its own source, never "it is time again".
//!
//! All four detectors are wired. D4a, a run capped, is PUSHED: its source is the
//! reaper's own sweep verdict, arriving over the channel `Reaper::run` was
//! given, because the reaper is the only thing in the process that can tell a
//! run it capped apart from an ordinary healthy completion — from outside,
//! the ledger's status column reads identically for both. D4c, a run
//! overrun, is PULLED: the poll branch scans the same ledger list the reaper
//! sweeps and compares each row's age against
//! [`TRIGGER_OVERRUN_THRESHOLD`]. What keeps a pulled detector on the
//! event-driven side of D3 is its edge: a per-run seen-set, so an overrun
//! fires once when the row crosses the threshold and never again while it
//! stays across it. D4b, a run's pane entering `Blocked`, is PULLED the same
//! way, off this task's own herdr snapshot and its own
//! [`watcher::StatusCursor`](crate::watcher::StatusCursor) — the cursor is
//! its edge, so a pane that stays blocked is silent after the first poll.
//! D4d, a new escalation row, is PULLED as well, off each consenting
//! project's own `.bee/supervisor/interventions.jsonl` — the one and only
//! read this repo makes against that store (D5), never a write — with a
//! per-project row-count cursor as its edge.
//!
//! Wired in behind the terminal family switch AND `terminal.trigger_enabled`
//! by `crate::TerminalBackground` (`crates/waggledance/src/main.rs`), the same
//! slot/cancel-flag/tick-counter pattern the supervisor, notify and reaper
//! tasks already follow: `reconcile_trigger` is the only place a [`Trigger`]
//! is ever constructed, and either switch off drives it to spawn nothing.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::sync::mpsc::UnboundedReceiver;
use waggledance_core::domain::Project;
use waggledance_core::Engine;

use crate::herdr::{AgentStatus, Herdr};
use crate::orchestrate::{self, DispatchTarget, RunStatus};
use crate::reaper::Verdict;
use crate::watcher::{statuses_from, BeeRoots, StatusCursor};

/// D9's marker: the `feature` every tick this task dispatches is stamped
/// with, and the one thing that keeps the trigger from observing itself.
///
/// Without it, a dispatched tick's own completion is itself a "run capped"
/// transition in the same project, which would wake another tick pointed at
/// the first tick's run — a self-sustaining loop bounded only by D8's
/// cooldown and never terminating. Every detector drops a run or row
/// carrying this marker before treating it as a transition at all.
pub const TRIGGER_FEATURE_MARKER: &str = "observer-tick-trigger";

/// The preset label a dispatched tick is started from: the target project's
/// own `herding.agents` role for observation work.
///
/// Resolved for real through [`crate::mcp::resolve_preset`] on every
/// dispatch — never `preset_label: None`. Two independent reasons, either
/// one sufficient: `DispatchTarget::Spawn` cannot start a pane without a
/// resolved entry at all, and `list_unattended_working_runs` filters on
/// `preset_label IS NOT NULL`, so a label-less tick would be invisible to
/// the reaper (and so to this task's own D4a source) forever. A project
/// that declares no such role resolves nothing and is dispatched nothing —
/// the refusal is logged and the transition dropped, never guessed around.
pub const TRIGGER_PRESET_LABEL: &str = "supervisor";

/// The task's own poll cadence.
///
/// Set by politeness rather than responsiveness, exactly like
/// [`crate::reaper::SWEEP_INTERVAL`]: each tick costs at most one herdr
/// snapshot and one ledger read, and nothing this task watches is
/// latency-sensitive — a transition noticed 30 seconds late still wakes the
/// same observer with the same fixed text.
///
/// A poll cadence is not the timer D3 forbids. What D3 forbids is firing a
/// tick on every poll regardless of state — which is what
/// `bee herding control-loop --role supervisor` already does and what this
/// feature exists to avoid. Cheap freshness reads behind an edge-detecting
/// cursor are the same shape `watcher.rs` has always had.
pub const TRIGGER_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// D8: the minimum spacing between two ticks dispatched into the SAME
/// project.
///
/// Fifteen minutes, chosen against what a tick costs rather than against how
/// fast transitions can arrive: one tick is a whole cold agent session in
/// someone's repo, so a flapping run that produces a transition every few
/// seconds must not turn into a spawn every few seconds. Per project, so a
/// quiet project is never made to wait on a noisy one.
///
/// The window bounds the DISPATCH only, never the detection — see
/// [`Trigger::maybe_dispatch`] for the contract every detector follows.
pub const TRIGGER_DISPATCH_COOLDOWN: Duration = Duration::from_secs(900);

/// D4c: how long a still-`working` run must have sat untouched before this
/// task calls it overrun.
///
/// One hour, and the reasoning is entirely about what the reaper has already
/// ruled out by then. [`crate::reaper::GRACE_WINDOW`] (60s) plus its
/// `AWAIT_BUDGET` (5s) is the whole window in which a run is merely
/// *unattended*; past it the reaper judges the row on every
/// [`crate::reaper::SWEEP_INTERVAL`] (30s) and caps it the moment the pane is
/// gone or the marker lands. So a row still `working` an hour later is one
/// the reaper has looked at some 120 times and deliberately left alone every
/// time: the pane is alive, the agent has not declared done, and nothing
/// mechanical is going to resolve it. That is the condition worth waking an
/// observer for — sixty times the reaper's own window, so no ordinary slow
/// agent and no await race can reach it, and short enough that a wedged
/// session is noticed inside one working session rather than the next day.
///
/// D4's own wording is the constraint on how this is computed: an overrun is
/// "computed by waggledance from its own run ledger, never LLM-judged". It is
/// arithmetic on `updated_at`, and that is all it will ever be.
pub const TRIGGER_OVERRUN_THRESHOLD: Duration = Duration::from_secs(3600);

/// D4a's transition kind, as it is named to the woken observer.
///
/// The kind is a value substituted into [`TRIGGER_TASK_TEMPLATE`], never a
/// branch in it (D1). Each later detector adds its own constant beside this
/// one; none of them may add wording.
pub const TRANSITION_CAPPED: &str = "a run was capped";

/// D4c's transition kind. A value, exactly like [`TRANSITION_CAPPED`] — it
/// is substituted into [`TRIGGER_TASK_TEMPLATE`] and adds no wording of its
/// own to it.
pub const TRANSITION_OVERRUN: &str = "a run overran";

/// D4b's transition kind. A value like the two above it, on the same terms:
/// it is substituted into [`TRIGGER_TASK_TEMPLATE`] and contributes no
/// wording of its own to the template.
pub const TRANSITION_BLOCKED: &str = "a run's pane entered blocked";

/// D4d's transition kind, and the last of the four. A value on exactly the
/// same terms as the three above it: substituted into
/// [`TRIGGER_TASK_TEMPLATE`], contributing no wording of its own to it, and
/// saying nothing about what the escalation row actually said — this task
/// reads that a row appeared and nothing more (D5).
pub const TRANSITION_ESCALATION: &str = "a new escalation row appeared";

/// The whole task text, once, for every transition kind there is (D1).
///
/// Three placeholders, all pure substitution: `{transition}` is the kind's
/// own short name, `{run}` and `{project}` are the evidence pointer. Nothing
/// else varies, and in particular no wording anywhere in this template is
/// chosen by which transition fired — that is what keeps this task on the
/// "decides whether, never what" side of the D1 exception it was granted.
/// It names why the observer was woken and where to look, and stops:
/// no finding, no assessment, no suggested action.
pub const TRIGGER_TASK_TEMPLATE: &str = "\
Run one cold `bee supervisor` observation tick for this repository.

waggledance woke you because it detected one mechanical transition here: {transition}. \
Evidence pointer: run {run} in project {project}. \
waggledance has judged nothing about it and keeps no record of it — observe the \
current state yourself, and record whatever you find through `bee supervisor`'s own \
verbs. Nothing beyond that observation tick was asked of you.";

/// The fixed text for one transition — [`TRIGGER_TASK_TEMPLATE`] with its
/// three values substituted and nothing else.
///
/// `run` is `None` for a transition that is not about a run (D4d's
/// escalation rows); it reads as `-`, which is a missing pointer, not a
/// different message.
pub fn task_text(kind: &str, project: &Project, run: Option<&str>) -> String {
    TRIGGER_TASK_TEMPLATE
        .replace("{transition}", kind)
        .replace("{run}", run.unwrap_or("-"))
        .replace("{project}", &project.id)
}

/// What the shared gate decided about one transition.
///
/// Every variant means the same thing to a detector: **this transition is
/// finished with**. A detector advances its own cursor/seen-set past a
/// transition on any of these, dispatched or not — D8's suppression drops a
/// transition, it never queues or retries one, and neither does a refusal.
/// Reported back so a caller (and this module's tests) can see the decision
/// itself rather than infer it from the ledger afterwards, the same
/// contract [`crate::reaper::Verdict`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateOutcome {
    /// D9: the transition is about this task's own dispatched tick.
    SelfExcluded,
    /// D7/D6: the project has not opted into orchestration, or the terminal
    /// family switch is off.
    NotConsented,
    /// D8: inside this project's cooldown window.
    CooledDown,
    /// D10: `terminal.trigger_dry_run` is on — logged, never dispatched.
    DryRun,
    /// The task was switched off between the decision and the call.
    Cancelled,
    /// One tick was dispatched.
    Dispatched,
    /// A dispatch was attempted and refused (no resolvable preset, no
    /// destination, herdr down). Logged, and the transition is dropped like
    /// any other — a refusal is never retried into a spawn storm.
    Refused,
}

/// The trigger task itself. Construction is `TerminalBackground`'s alone.
pub struct Trigger {
    herdr: Arc<dyn Herdr>,
    engine: Arc<Engine>,
    interval: Duration,
    cooldown: Duration,
    /// D10. Every gate that decides whether a transition is real (D9, D7/D6)
    /// still runs above it; the one external call is replaced by the log line
    /// describing it, and D8's rate limit — which exists only to bound that
    /// call — is not applied to a run that makes no call.
    dry_run: bool,
    /// D8's state, and the only state this task keeps: when each project was
    /// last dispatched into. Not a record of what was observed (D5) —
    /// nothing about the transition itself is stored, only that one happened.
    last_dispatch: Mutex<HashMap<String, Instant>>,
    /// D4c's edge, and the reason a poll is not a timer: the run ids this
    /// task has already reported as overrun. A row stays overrun for as long
    /// as it stays stuck, so without this set every poll would re-fire the
    /// same transition — which is exactly the "fires because time passed"
    /// shape D3 forbids. Not a record of anything observed (D5): ids only,
    /// in memory, and a restart legitimately starts over.
    seen_overrun: Mutex<HashSet<String>>,
    /// D4b's edge: this task's OWN [`StatusCursor`], fed from this task's own
    /// herdr snapshot on every poll.
    ///
    /// Its own, deliberately, on two counts. The type is reused verbatim from
    /// `watcher.rs` — a second cursor type speaking the same vocabulary would
    /// be a fork, not a feature — but the *instance* is this task's, because
    /// `watcher.rs`'s cursor lives inside the notify watcher and only runs
    /// when `terminal.notify_enabled` is on. Sharing it would make
    /// `trigger_enabled` silently depend on a different opt-in switch, which
    /// is exactly what D6's own reasoning forbids: each of these switches
    /// means what it says on its own.
    ///
    /// Not a record of anything observed (D5): last-status per pane, in
    /// memory, and a restart legitimately starts over.
    blocked_cursor: Mutex<StatusCursor>,
    /// D4d's source of project roots to read escalation mailboxes from.
    ///
    /// [`watcher::BeeRoots`](crate::watcher::BeeRoots) itself, not a second
    /// port speaking the same sentence: it already means exactly "the roots
    /// to read `.bee/` from", it is already re-asked on every tick (so a
    /// project registered while the daemon runs is picked up with no
    /// restart), and a parallel `TriggerRoots` would be a fork of that
    /// meaning rather than a new one.
    ///
    /// Built from the engine here rather than passed in, so this task's
    /// owner keeps the constructor it already calls; the port stays the port
    /// because what it answers is a policy question ("which roots?") that a
    /// test or a later caller can answer differently.
    roots: Arc<dyn BeeRoots>,
    /// D4d's edge: how many escalation rows this task has already accounted
    /// for in each project's mailbox, keyed by project id.
    ///
    /// A count works because the mailbox is append-only and
    /// [`read_escalations`](waggledance_core::bee::read_escalations) answers
    /// in file order: qualifying rows only ever arrive at the end, so
    /// everything past the mark is new and everything before it is history.
    ///
    /// The first read of a project PRIMES this mark and dispatches nothing —
    /// the one place this detector deliberately differs from
    /// [`Trigger::scan_blocked`], whose first sight of an already-blocked
    /// pane IS an entry. The difference is what the two sources are: a herdr
    /// snapshot is current state, so a pane blocked right now is blocked
    /// right now; a mailbox is history, and a project with fifty escalations
    /// filed last month has had no transition at all — waking an observer
    /// for them on daemon start would be a restart storm dressed as an event.
    ///
    /// Not a record of anything observed (D5): a count per project, in
    /// memory, and a restart legitimately starts over.
    seen_escalations: Mutex<HashMap<String, usize>>,
    /// Flipped by the owner (`TerminalBackground`) the moment either switch
    /// is turned off. Checked immediately before the one call with an
    /// irreversible external side effect (`dispatch_run`, which spawns an
    /// agent), for the same reason `reaper.rs` and `supervisor.rs` check
    /// theirs: cancelling the task only lands at its next `.await` point,
    /// which can be after the decision to act but before the act.
    cancelled: Arc<AtomicBool>,
}

impl Trigger {
    pub fn with_cancel_flag(
        herdr: Arc<dyn Herdr>,
        engine: Arc<Engine>,
        interval: Duration,
        cooldown: Duration,
        dry_run: bool,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        // The same closure `main.rs` already hands the notify watcher, for
        // the same reason: the registry IS the list of roots, and asking it
        // per tick is what keeps a mid-run registration visible.
        let registry = engine.clone();
        let roots: Arc<dyn BeeRoots> = Arc::new(move || -> Vec<std::path::PathBuf> {
            registry
                .list_projects()
                .map(|ps| ps.into_iter().map(|p| p.root_path).collect())
                .unwrap_or_default()
        });
        Trigger {
            herdr,
            engine,
            interval,
            cooldown,
            dry_run,
            last_dispatch: Mutex::new(HashMap::new()),
            seen_overrun: Mutex::new(HashSet::new()),
            blocked_cursor: Mutex::new(StatusCursor::new()),
            roots,
            seen_escalations: Mutex::new(HashMap::new()),
            cancelled,
        }
    }

    /// The one gate every detector funnels a transition through, in this
    /// order and deliberately: D9 self-exclusion, D7/D6 consent, D10
    /// dry-run, D8 cooldown, the cancel flag, then the dispatch.
    ///
    /// The order is the point. Self-exclusion runs first so a tick's own
    /// run can never even consume a cooldown slot; consent runs before
    /// anything is spent on a project that declined orchestration; D10's
    /// dry run sits ahead of D8 so the log reports EVERY real transition
    /// rather than one per cooldown window — measuring volume is the whole
    /// reason D10 exists, and a dry run consumes no window because it spawns
    /// nothing; and the cancel flag is last, immediately before the one call
    /// that spawns an agent, which is the same placement `reaper.rs` and
    /// `supervisor.rs` give theirs.
    ///
    /// `kind` names the transition ([`TRANSITION_CAPPED`] and its siblings);
    /// `run` is the evidence pointer, `None` for a transition that is not
    /// about a run. A detector whose transitions carry no run id owes D9 its
    /// own filter before calling — the self-exclusion below can only answer
    /// for a run it can look up.
    ///
    /// **Contract for every detector**: whatever comes back, the transition
    /// is finished with. Advance the cursor or seen-set past it. D8
    /// suppresses the dispatch, never the detection.
    pub async fn maybe_dispatch(
        &self,
        project: &Project,
        kind: &str,
        run: Option<&str>,
    ) -> GateOutcome {
        // (1) D9 — before anything else, including consent and the cooldown.
        if let Some(run_id) = run {
            let own = self
                .engine
                .run_feature(run_id)
                .ok()
                .flatten()
                .is_some_and(|f| f == TRIGGER_FEATURE_MARKER);
            if own {
                tracing::debug!(
                    run = %run_id,
                    project = %project.id,
                    "observer tick skipped: the transition is about a tick this task dispatched"
                );
                return GateOutcome::SelfExcluded;
            }
        }

        // (2) D7 + D6 — the per-project opt-in and the global family switch,
        // both false by default, both re-read on every dispatch rather than
        // captured once at construction.
        if !self.engine.orchestration_allowed(project) || !self.engine.config.terminal.enabled {
            tracing::debug!(
                project = %project.id,
                "observer tick skipped: this project has not opted into orchestration"
            );
            return GateOutcome::NotConsented;
        }

        // (3) D10 — ahead of the cooldown, deliberately. D10 exists so an
        // operator can MEASURE real transition volume before arming
        // autonomous dispatch; behind D8 it would report at most one line per
        // cooldown window per project, which is the measurement filtered
        // through the very rate limit the operator is trying to size. A dry
        // run also spends nothing — no spawn, no cooldown slot — so there is
        // nothing for the window to protect here. Everything that decides
        // whether a transition is REAL (D9's self-exclusion, D7/D6's consent)
        // still runs above, so a tick's own run or a project that declined
        // orchestration stays silent in dry run too.
        if self.dry_run {
            tracing::info!(
                project = %project.id,
                transition = kind,
                run = run.unwrap_or("-"),
                preset = TRIGGER_PRESET_LABEL,
                feature = TRIGGER_FEATURE_MARKER,
                "observer tick DRY RUN: would dispatch one tick for this transition and did not"
            );
            return GateOutcome::DryRun;
        }

        // (4) D8 — the stamp is taken HERE, the moment the decision to act is
        // made, not after a successful dispatch: a refusal that left the
        // window open would let a flapping run retry a failing spawn every
        // few seconds, which is the storm this window exists to bound.
        {
            let mut seen = self.last_dispatch.lock().unwrap();
            let now = Instant::now();
            if let Some(last) = seen.get(&project.id) {
                if now.duration_since(*last) < self.cooldown {
                    tracing::debug!(
                        project = %project.id,
                        transition = kind,
                        "observer tick suppressed by the per-project cooldown; the transition is \
                         dropped, never queued"
                    );
                    return GateOutcome::CooledDown;
                }
            }
            seen.insert(project.id.clone(), now);
        }

        let task = task_text(kind, project, run);

        // (5) The cancel flag, last possible moment before the one call that
        // spawns an agent (`reaper.rs` and `supervisor.rs` take the same
        // check in the same place, for the same reason).
        if self.cancelled.load(Ordering::SeqCst) {
            return GateOutcome::Cancelled;
        }

        // (6) The dispatch, through a preset resolved for real.
        let (preset, entry) =
            match crate::mcp::resolve_preset(&self.engine, project, TRIGGER_PRESET_LABEL) {
                Ok(resolved) => resolved,
                Err(e) => {
                    tracing::warn!(
                        project = %project.id,
                        preset = TRIGGER_PRESET_LABEL,
                        "observer tick not dispatched -- preset unresolved: {e}"
                    );
                    return GateOutcome::Refused;
                }
            };
        match orchestrate::dispatch_run(
            self.herdr.as_ref(),
            &self.engine,
            project,
            DispatchTarget::Spawn { entry, cwd: None },
            &task,
            Some(TRIGGER_FEATURE_MARKER),
            Some(preset.label),
        )
        .await
        {
            Ok(dispatched) => {
                for warning in &dispatched.warnings {
                    tracing::warn!(
                        project = %project.id,
                        "observer tick dispatched with a warning: {warning}"
                    );
                }
                tracing::info!(
                    project = %project.id,
                    transition = kind,
                    run = run.unwrap_or("-"),
                    tick = %dispatched.run.id,
                    "observer tick dispatched"
                );
                GateOutcome::Dispatched
            }
            Err(e) => {
                tracing::warn!(
                    project = %project.id,
                    transition = kind,
                    "observer tick refused by the dispatch path: {e}"
                );
                GateOutcome::Refused
            }
        }
    }

    /// D4a: one reaper sweep verdict, judged.
    ///
    /// Only a CAP is this transition: `Lost` (the pane vanished and the row
    /// was capped from the ledger) and `Awaited(Done | Timeout)` (the reaper
    /// called `await_run` and it finished). `Awaited(Blocked)` is
    /// deliberately excluded — a blocked run is D4b's transition and D4b's
    /// alone, and [`Trigger::scan_blocked`] is now here to take it, so
    /// letting it through as well would double-detect the same event.
    /// `LeftAlone` and `TooYoung` are not transitions at all: nothing
    /// changed.
    pub async fn on_verdict(&self, run_id: &str, verdict: Verdict) -> Option<GateOutcome> {
        let capped = matches!(
            verdict,
            Verdict::Lost
                | Verdict::Awaited(RunStatus::Done)
                | Verdict::Awaited(RunStatus::Timeout)
        );
        if !capped {
            return None;
        }
        // Fail-closed on state that will not read: a transition whose own
        // project cannot be resolved yields no tick, never a guessed one.
        let project = match self.engine.get_run(run_id) {
            Ok(Some(run)) => match self.engine.get_project(&run.project_id) {
                Ok(Some(project)) => project,
                Ok(None) => return None,
                Err(e) => {
                    tracing::warn!("observer tick could not read run {run_id}'s project: {e}");
                    return None;
                }
            },
            Ok(None) => return None,
            Err(e) => {
                tracing::warn!("observer tick could not read run {run_id}: {e}");
                return None;
            }
        };
        Some(
            self.maybe_dispatch(&project, TRANSITION_CAPPED, Some(run_id))
                .await,
        )
    }

    /// D4c: one overrun scan, over every still-`working` run waggledance
    /// spawned. Returns the outcome for each run it actually took to the
    /// gate, in the order decided.
    ///
    /// The source is `list_unattended_working_runs` — the reaper's own sweep
    /// list, project-blind and age-blind, and the only source this detector
    /// has. No new store method exists for it: the threshold below is
    /// policy, and policy stays out of the store exactly as that method's own
    /// doc asks.
    ///
    /// Four filters in this order, each cheap before the one after it:
    ///
    /// 1. D9 — a row carrying [`TRIGGER_FEATURE_MARKER`] is a tick this task
    ///    dispatched, and a wedged tick must never wake a tick about itself.
    ///    [`Trigger::maybe_dispatch`] would catch it too; catching it here
    ///    keeps a stuck tick out of the seen-set as well.
    /// 2. Age — [`TRIGGER_OVERRUN_THRESHOLD`] against `updated_at`, the same
    ///    fail-closed parse the reaper's own age check uses.
    /// 3. The project, resolved. Fail-closed and *without* marking the run
    ///    seen: a row whose project will not read is undecided, not
    ///    finished with, so a later poll may still judge it.
    /// 4. The seen-set — fire once per run id, forever. It is set before the
    ///    gate is called, because the gate's contract is that every outcome
    ///    means this transition is finished with.
    pub async fn scan_overruns(&self) -> Vec<(String, GateOutcome)> {
        let runs = match self.engine.store.list_unattended_working_runs() {
            Ok(runs) => runs,
            Err(e) => {
                tracing::warn!("observer tick could not list working runs: {e}");
                return Vec::new();
            }
        };
        let now = OffsetDateTime::now_utc();
        let mut outcomes: Vec<(String, GateOutcome)> = Vec::new();
        for run in runs {
            // (1) D9.
            let own = self
                .engine
                .run_feature(&run.id)
                .ok()
                .flatten()
                .is_some_and(|f| f == TRIGGER_FEATURE_MARKER);
            if own {
                continue;
            }
            // (2) Age.
            if !older_than(&run.updated_at, now, TRIGGER_OVERRUN_THRESHOLD) {
                continue;
            }
            // (3) The project.
            let project = match self.engine.get_project(&run.project_id) {
                Ok(Some(project)) => project,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("observer tick could not read run {}'s project: {e}", run.id);
                    continue;
                }
            };
            // (4) Fire once per run, whatever the gate then decides.
            {
                let mut seen = self.seen_overrun.lock().unwrap();
                if !seen.insert(run.id.clone()) {
                    continue;
                }
            }
            let outcome = self
                .maybe_dispatch(&project, TRANSITION_OVERRUN, Some(&run.id))
                .await;
            outcomes.push((run.id, outcome));
        }
        outcomes
    }

    /// D4b: one blocked scan, over this task's own herdr snapshot. Returns the
    /// outcome for each run it actually took to the gate, in the order
    /// decided.
    ///
    /// The source is a fresh snapshot read through this task's own [`Herdr`]
    /// handle, fed into this task's own [`StatusCursor`] — never the notify
    /// watcher's, see [`Trigger::blocked_cursor`] for why that independence is
    /// the point rather than an accident.
    ///
    /// The cursor is the edge, and the filter is this detector's own work:
    /// `diff` surfaces EVERY status change it sees (`Working` → `Done` just
    /// as much as anything → `Blocked`), so the entry-into-`Blocked` filter
    /// below is applied to its output, here, rather than expected from it. A
    /// pane that stays `Blocked` across polls is not a change and never
    /// reaches this filter at all; a pane already `Blocked` on the first
    /// snapshot is a first sight, which the cursor reports and this detector
    /// treats as an entry — the same answer `watcher.rs`'s own cursors give,
    /// and the right one: waggledance restarting does not un-block an agent.
    ///
    /// A pane is not a run. The ledger's still-`working` waggledance-spawned
    /// rows — `list_unattended_working_runs`, the same list D4c scans — are
    /// what turns one back into a run and so into a project: a blocked pane
    /// that belongs to no such row is a human's own agent, and this task has
    /// nothing to say about it (D7's consent is about projects waggledance
    /// dispatches into, and a borrowed pane's row carries no `preset_label`
    /// to be found by anyway).
    ///
    /// Then D9, before the gate: a pane belonging to a tick this task
    /// dispatched is never a transition — a tick that blocks waiting on its
    /// own input must not wake a tick pointed at itself. The gate would catch
    /// it too; catching it here keeps the log honest about what was dropped
    /// and why.
    pub async fn scan_blocked(&self) -> Vec<(String, GateOutcome)> {
        let snapshot = match self.herdr.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(e) => {
                // The cursor is deliberately NOT advanced here: nothing was
                // observed, so nothing is finished with, and the next poll
                // judges the same panes fresh.
                tracing::debug!("observer tick could not read a herdr snapshot: {e}");
                return Vec::new();
            }
        };
        let statuses = statuses_from(&snapshot);
        // Lock, diff, filter, unlock — all before the first `.await` below.
        let entered_blocked: Vec<String> = {
            let mut cursor = self.blocked_cursor.lock().unwrap();
            cursor
                .diff(&statuses)
                .into_iter()
                .filter(|change| change.status == AgentStatus::Blocked)
                .map(|change| change.pane_id)
                .collect()
        };
        if entered_blocked.is_empty() {
            return Vec::new();
        }
        let runs = match self.engine.store.list_unattended_working_runs() {
            Ok(runs) => runs,
            Err(e) => {
                tracing::warn!("observer tick could not list working runs: {e}");
                return Vec::new();
            }
        };
        let mut outcomes: Vec<(String, GateOutcome)> = Vec::new();
        for pane_id in entered_blocked {
            // (1) The pane, resolved back to the run that owns it.
            let Some(run) = runs.iter().find(|run| run.pane_id == pane_id) else {
                continue;
            };
            // (2) D9.
            let own = self
                .engine
                .run_feature(&run.id)
                .ok()
                .flatten()
                .is_some_and(|f| f == TRIGGER_FEATURE_MARKER);
            if own {
                tracing::debug!(
                    run = %run.id,
                    pane = %pane_id,
                    "observer tick skipped: the blocked pane belongs to a tick this task dispatched"
                );
                continue;
            }
            // (3) The project.
            let project = match self.engine.get_project(&run.project_id) {
                Ok(Some(project)) => project,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("observer tick could not read run {}'s project: {e}", run.id);
                    continue;
                }
            };
            let outcome = self
                .maybe_dispatch(&project, TRANSITION_BLOCKED, Some(&run.id))
                .await;
            outcomes.push((run.id.clone(), outcome));
        }
        outcomes
    }

    /// D4d: one escalation scan, over every registered, consenting project's
    /// own `.bee/supervisor/interventions.jsonl` mailbox. Returns the outcome
    /// for each row it actually took to the gate, keyed by row id, in the
    /// order decided.
    ///
    /// This is the only place in waggledance that touches a project's
    /// `.bee/supervisor/` store at all, and it only ever reads it (D5):
    /// [`read_escalations`](waggledance_core::bee::read_escalations) opens
    /// one file, and this task calls no `bee supervisor` write verb —
    /// `record`, `mark-delivered`, `away`, `back` or any other — from
    /// anywhere. Every write after a tick fires belongs to the woken agent,
    /// inside the target repo.
    ///
    /// Nothing a row SAYS is read, kept or forwarded. The gate is told a row
    /// appeared and where; the row's own question never enters the task text
    /// (that would be this module choosing what to say, which is exactly what
    /// the D1 exception does not cover), and nothing about it is stored
    /// beyond [`Trigger::seen_escalations`]'s per-project count.
    ///
    /// Four steps, in this order:
    ///
    /// 1. Roots, from the port ([`Trigger::roots`]), re-asked this tick.
    /// 2. The project behind each root, and D7's consent — checked HERE,
    ///    before the file is opened, not only at the gate: a project that
    ///    declined orchestration should not have its supervisor mailbox read
    ///    at all, and the cheapest way to honour that is to never open it.
    /// 3. The read itself, off the async worker
    ///    ([`tokio::task::spawn_blocking`]) — synchronous file I/O, the same
    ///    rule `watcher::PollWatcher::poll_activity_once` already follows for
    ///    this crate's other `.bee/` reads.
    /// 4. The cursor, then D9, then the gate. A row this task's own tick
    ///    could have filed is dropped before the gate, because the gate's own
    ///    D9 check can only answer for a transition that names a run and this
    ///    one names none. It is a no-op filter today — per D5 this task never
    ///    calls `bee supervisor record`, so no row in any mailbox can be its
    ///    own — and it is here so that stays true if that ever changes.
    pub async fn scan_escalations(&self) -> Vec<(String, GateOutcome)> {
        // (1) + (2) — the roots, narrowed to the projects that consented.
        let projects: Vec<Project> = {
            let roots = self.roots.roots();
            if roots.is_empty() {
                return Vec::new();
            }
            let registered = match self.engine.list_projects() {
                Ok(projects) => projects,
                Err(e) => {
                    tracing::warn!("observer tick could not list projects: {e}");
                    return Vec::new();
                }
            };
            roots
                .into_iter()
                .filter_map(|root| registered.iter().find(|p| p.root_path == root).cloned())
                .filter(|p| self.engine.orchestration_allowed(p))
                .collect()
        };
        if projects.is_empty() {
            return Vec::new();
        }

        // (3) The reads, all of them, off the async worker.
        let to_read: Vec<(String, std::path::PathBuf)> = projects
            .iter()
            .map(|p| (p.id.clone(), p.root_path.clone()))
            .collect();
        let mailboxes = tokio::task::spawn_blocking(move || {
            to_read
                .into_iter()
                .map(|(id, root)| (id, waggledance_core::bee::read_escalations(&root)))
                .collect::<Vec<_>>()
        })
        .await
        // A join failure yields nothing — never a spurious transition, and
        // never a cursor advanced past rows that were not read.
        .unwrap_or_default();

        let mut outcomes: Vec<(String, GateOutcome)> = Vec::new();
        for (project_id, rows) in mailboxes {
            let Some(project) = projects.iter().find(|p| p.id == project_id) else {
                continue;
            };
            // (4) The cursor. Everything past the mark is new; the mark moves
            // to the end of the file whatever the gate then decides about the
            // rows in between, which is the gate's own contract.
            let fresh: Vec<waggledance_core::bee::BeeEscalation> = {
                let mut seen = self.seen_escalations.lock().unwrap();
                match seen.insert(project_id.clone(), rows.len()) {
                    // First read of this project: prime and say nothing. A
                    // mailbox is history, not current state — see
                    // `seen_escalations` for why this differs from
                    // `scan_blocked`'s first-sight-is-an-entry.
                    None => continue,
                    // The file shrank (rotated, truncated, replaced). Nothing
                    // was appended, so nothing transitioned; re-baseline
                    // rather than re-fire everything that is left.
                    Some(mark) if mark > rows.len() => continue,
                    Some(mark) => rows[mark..].to_vec(),
                }
            };
            for row in fresh {
                // D9, this detector's own — the gate cannot do it for a
                // transition that names no run.
                if row.point_key.contains(TRIGGER_FEATURE_MARKER)
                    || row.target_session.contains(TRIGGER_FEATURE_MARKER)
                {
                    tracing::debug!(
                        row = %row.id,
                        project = %project.id,
                        "observer tick skipped: the escalation row is about this task's own tick"
                    );
                    continue;
                }
                let outcome = self
                    .maybe_dispatch(project, TRANSITION_ESCALATION, None)
                    .await;
                outcomes.push((row.id, outcome));
            }
        }
        outcomes
    }

    /// Run the trigger loop. `ticks` counts every completed poll — a real,
    /// externally observable side effect of the loop still running, which is
    /// what proves a switch-off actually stopped the task rather than merely
    /// emptying its owner's slot (the same contract the reaper's, the
    /// supervisor's and the notify watcher's tick counters hold).
    ///
    /// `verdicts` is D4a's source: the channel `Reaper::run` sends every
    /// sweep decision down. It is `Option` because the reaper is
    /// independently disableable — with no reaper running there is no
    /// channel, and the loop still polls (and still ticks), it just has one
    /// fewer source. A closed channel is treated the same way: the source is
    /// dropped and the loop carries on rather than spinning on a dead
    /// receiver.
    ///
    /// The poll branch is where the pulling detectors live: D4c's overrun
    /// scan, D4b's blocked scan and D4d's escalation scan all run there, in
    /// that order, and the counter advances after all three — so a tick means
    /// a poll that completed every scan it owns, never a poll that merely
    /// started. D4a takes the other arm — its source pushes, so it needs no
    /// polling at all.
    pub async fn run(
        self,
        ticks: Arc<AtomicU64>,
        mut verdicts: Option<UnboundedReceiver<(String, Verdict)>>,
    ) {
        let mut poll = tokio::time::interval(self.interval);
        // A verdict burst must never make the loop "catch up" on the ticks it
        // spent handling them — the counter measures liveness, not backlog.
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = poll.tick() => {
                    self.scan_overruns().await;
                    self.scan_blocked().await;
                    self.scan_escalations().await;
                    ticks.fetch_add(1, Ordering::Relaxed);
                }
                received = next_verdict(&mut verdicts) => {
                    match received {
                        Some((run_id, verdict)) => {
                            self.on_verdict(&run_id, verdict).await;
                        }
                        None => {
                            // The reaper went away. Drop the source so the
                            // next select does not poll a closed receiver
                            // that returns instantly, forever.
                            verdicts = None;
                        }
                    }
                }
            }
        }
    }
}

/// The next verdict, or a future that never resolves when there is no
/// source — so [`Trigger::run`]'s `select!` has one arm shape whether the
/// reaper is running or not.
async fn next_verdict(
    verdicts: &mut Option<UnboundedReceiver<(String, Verdict)>>,
) -> Option<(String, Verdict)> {
    match verdicts {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

/// Whether `updated_at` is at least `threshold` old as of `now` — D4c's whole
/// arithmetic.
///
/// Fail-closed in both odd directions, for the same reason
/// `reaper::older_than` is (and deliberately the same shape as it): a
/// timestamp that will not parse, and one stamped in the future (clock skew),
/// both answer `false`. An age this task cannot prove is not an overrun, and
/// waking a cold agent about a row on the strength of an unparseable
/// timestamp is exactly the kind of noise D3 and D8 exist to keep out.
fn older_than(updated_at: &str, now: OffsetDateTime, threshold: Duration) -> bool {
    let Ok(stamped) = OffsetDateTime::parse(updated_at, &Rfc3339) else {
        return false;
    };
    let threshold = time::Duration::try_from(threshold).unwrap_or(time::Duration::ZERO);
    now - stamped >= threshold
}

/// The configuration combination that leaves a dispatched tick unreclaimable,
/// as the warning to log — or `None` when the pair is sound.
///
/// D8 bounds how OFTEN a tick is dispatched, never how long one lives. The
/// only thing that reclaims a tick whose pane wedged is the reaper's own
/// sweep, and the reaper is a separately disableable switch: arming the
/// trigger with the reaper off is a legal configuration that leaks a stuck
/// pane per stuck tick, forever. Not refused (it is the owner's machine),
/// but never silent either.
pub fn lifecycle_warning(cfg: &waggledance_core::config::TerminalConfig) -> Option<&'static str> {
    (cfg.trigger_enabled && !cfg.reaper_enabled).then_some(
        "terminal.trigger_enabled is on with terminal.reaper_enabled off: the reaper is the \
         only thing that reclaims a dispatched observation tick whose pane wedges, so a stuck \
         tick will stay open forever. Turn reaper_enabled back on.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;
    use waggledance_core::domain::Run;
    use waggledance_core::indexer::now_rfc3339;
    use waggledance_core::{Config, SqliteStore};

    /// A directory that exists on disk (a `Boundary` resolves against the
    /// real filesystem, so a spawn destination cannot be invented) carrying
    /// a `.bee/config.json` that declares the observation role this task
    /// dispatches through.
    fn temp_root(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "waggledance-trigger-{tag}-{}-{:016x}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(root.join(".bee")).unwrap();
        std::fs::write(
            root.join(".bee").join("config.json"),
            format!(
                r#"{{"herding":{{"agents":{{"{TRIGGER_PRESET_LABEL}":["claude","--role","{TRIGGER_PRESET_LABEL}"]}}}}}}"#
            ),
        )
        .unwrap();
        root
    }

    fn test_engine(consented: bool, root: &std::path::Path) -> Arc<Engine> {
        let mut config = Config::default();
        config.terminal.enabled = true;
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        engine
            .store
            .upsert_project(&Project {
                id: "proj-1".into(),
                name: "test-proj".into(),
                root_path: root.to_path_buf(),
                created_at: now_rfc3339(),
                last_seen_at: now_rfc3339(),
                orchestration_enabled: consented,
            })
            .unwrap();
        Arc::new(engine)
    }

    /// Point the fake's own w2 panes at the project root so a `Spawn`
    /// dispatch has a destination that survives boundary validation.
    async fn spawnable_herdr(root: &std::path::Path) -> Arc<FakeHerdr> {
        let herdr = Arc::new(FakeHerdr::new());
        let dir = root.to_string_lossy().into_owned();
        for pane in ["w2:p3", "w2:p4", "w2:p5"] {
            herdr
                .set_pane_dirs(pane, Some(&dir), Some(&dir))
                .await
                .unwrap();
        }
        herdr
    }

    /// A long-dead RFC3339 stamp — any row carrying it is past
    /// [`TRIGGER_OVERRUN_THRESHOLD`] by years, whatever the threshold's value
    /// is later tuned to.
    const LONG_AGO: &str = "2020-01-01T00:00:00Z";

    /// A run row the reaper would have swept: waggledance-spawned
    /// (`preset_label` present), stamped with `feature`.
    fn seed_run(engine: &Engine, id: &str, feature: Option<&str>) {
        seed_run_at(engine, id, feature, &now_rfc3339());
    }

    /// The same row with `updated_at` chosen — D4c reads nothing else, so
    /// this is the whole of "a run that has been sitting for N".
    fn seed_run_at(engine: &Engine, id: &str, feature: Option<&str>, updated_at: &str) {
        seed_run_on(engine, id, feature, updated_at, "w9:p9");
    }

    /// The same row with its `pane_id` chosen — D4b's only join between a
    /// herdr snapshot and the ledger, so this is the whole of "this run owns
    /// that pane".
    fn seed_run_on_pane(engine: &Engine, id: &str, feature: Option<&str>, pane_id: &str) {
        seed_run_on(engine, id, feature, &now_rfc3339(), pane_id);
    }

    fn seed_run_on(
        engine: &Engine,
        id: &str,
        feature: Option<&str>,
        updated_at: &str,
        pane_id: &str,
    ) {
        let run = Run {
            id: id.into(),
            project_id: "proj-1".into(),
            pane_id: pane_id.into(),
            preset_label: Some("claude".into()),
            task: "do the thing".into(),
            baseline: "before".into(),
            marker: "HERDR_DONE_seed".into(),
            status: "working".into(),
            created_at: now_rfc3339(),
            updated_at: updated_at.to_string(),
        };
        engine.insert_run(&run, feature).unwrap();
    }

    fn trigger_for(
        herdr: Arc<FakeHerdr>,
        engine: Arc<Engine>,
        dry_run: bool,
        cooldown: Duration,
    ) -> (Trigger, Arc<AtomicBool>) {
        let cancelled = Arc::new(AtomicBool::new(false));
        let trigger = Trigger::with_cancel_flag(
            herdr,
            engine,
            TRIGGER_POLL_INTERVAL,
            cooldown,
            dry_run,
            cancelled.clone(),
        );
        (trigger, cancelled)
    }

    /// Every run row this task's own ticks created, newest first.
    fn dispatched_ticks(engine: &Engine) -> Vec<Run> {
        engine
            .list_runs("proj-1", 50)
            .unwrap()
            .into_iter()
            .filter(|r| {
                engine.run_feature(&r.id).unwrap().as_deref() == Some(TRIGGER_FEATURE_MARKER)
            })
            .collect()
    }

    /// The whole path, on the cheapest detector: a reaper cap verdict for a
    /// consented project spawns exactly one tick, and that tick is a real,
    /// reaper-visible run — a resolved preset label (never `None`, which
    /// `list_unattended_working_runs` filters out) carrying D9's marker.
    #[tokio::test]
    async fn a_capped_verdict_dispatches_exactly_one_marked_tick() {
        let root = temp_root("capped");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-capped", None);
        let (trigger, _cancel) =
            trigger_for(herdr, engine.clone(), false, TRIGGER_DISPATCH_COOLDOWN);

        let outcome = trigger.on_verdict("run-capped", Verdict::Lost).await;

        assert_eq!(outcome, Some(GateOutcome::Dispatched));
        let ticks = dispatched_ticks(&engine);
        assert_eq!(ticks.len(), 1, "exactly one tick per transition");
        assert_eq!(
            ticks[0].preset_label.as_deref(),
            Some(TRIGGER_PRESET_LABEL),
            "a tick must carry a resolved preset label or the reaper can never reclaim it"
        );
        assert!(
            ticks[0].task.contains("run-capped") && ticks[0].task.contains("proj-1"),
            "the tick names the evidence pointer it was woken for: {:?}",
            ticks[0].task
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// D4a's closed list: `Awaited(Done)` and `Awaited(Timeout)` are caps,
    /// `Awaited(Blocked)` is D4b's transition and never this one, and
    /// `LeftAlone`/`TooYoung` are not transitions at all.
    #[tokio::test]
    async fn only_a_cap_is_a_transition() {
        let root = temp_root("verdicts");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-x", None);
        // A cooldown of zero so each verdict below is judged on its own
        // merits rather than on the one before it, and a dry run because
        // this test is about which verdicts pass, not about spawning.
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), true, Duration::ZERO);

        for capped in [
            Verdict::Lost,
            Verdict::Awaited(RunStatus::Done),
            Verdict::Awaited(RunStatus::Timeout),
        ] {
            assert_eq!(
                trigger.on_verdict("run-x", capped).await,
                Some(GateOutcome::DryRun),
                "{capped:?} is a cap"
            );
        }
        for quiet in [
            Verdict::Awaited(RunStatus::Blocked),
            Verdict::Awaited(RunStatus::Working),
            Verdict::LeftAlone,
            Verdict::TooYoung,
        ] {
            assert_eq!(
                trigger.on_verdict("run-x", quiet).await,
                None,
                "{quiet:?} is not a D4a transition"
            );
        }
        std::fs::remove_dir_all(&root).ok();
    }

    /// D9, the loop-breaker: the exact same verdict, for a run this task
    /// itself dispatched, wakes nothing at all.
    #[tokio::test]
    async fn a_verdict_about_the_tasks_own_tick_dispatches_nothing() {
        let root = temp_root("self");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-own-tick", Some(TRIGGER_FEATURE_MARKER));
        let (trigger, _cancel) =
            trigger_for(herdr, engine.clone(), false, TRIGGER_DISPATCH_COOLDOWN);

        let outcome = trigger.on_verdict("run-own-tick", Verdict::Lost).await;

        assert_eq!(outcome, Some(GateOutcome::SelfExcluded));
        assert!(
            dispatched_ticks(&engine).len() == 1,
            "the seeded tick is the only marked run -- no second one was spawned"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// D7: a project that declined orchestration is not a second door in.
    #[tokio::test]
    async fn a_non_consenting_project_dispatches_nothing() {
        let root = temp_root("unconsented");
        let engine = test_engine(false, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-capped", None);
        let (trigger, _cancel) =
            trigger_for(herdr, engine.clone(), false, TRIGGER_DISPATCH_COOLDOWN);

        let outcome = trigger.on_verdict("run-capped", Verdict::Lost).await;

        assert_eq!(outcome, Some(GateOutcome::NotConsented));
        assert!(dispatched_ticks(&engine).is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    /// D8: a burst inside one window fires once. The second transition is
    /// DROPPED, not queued — a later poll must not deliver it late, which is
    /// what the third verdict here proves.
    #[tokio::test]
    async fn a_burst_inside_the_cooldown_window_dispatches_once() {
        let root = temp_root("cooldown");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-a", None);
        seed_run(&engine, "run-b", None);
        seed_run(&engine, "run-c", None);
        let (trigger, _cancel) =
            trigger_for(herdr, engine.clone(), false, TRIGGER_DISPATCH_COOLDOWN);

        assert_eq!(
            trigger.on_verdict("run-a", Verdict::Lost).await,
            Some(GateOutcome::Dispatched)
        );
        assert_eq!(
            trigger.on_verdict("run-b", Verdict::Lost).await,
            Some(GateOutcome::CooledDown)
        );
        // A later poll, nothing new: the suppressed transition is gone, and
        // the window is still closed.
        assert_eq!(
            trigger.on_verdict("run-c", Verdict::Lost).await,
            Some(GateOutcome::CooledDown)
        );
        assert_eq!(
            dispatched_ticks(&engine).len(),
            1,
            "one window, one tick -- and no retry of the two it swallowed"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// D10: the whole gate runs, the log says what would have happened, and
    /// nothing is spawned.
    #[tokio::test]
    async fn dry_run_logs_the_would_be_dispatch_and_spawns_nothing() {
        let root = temp_root("dryrun");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-capped", None);
        let (trigger, _cancel) = trigger_for(
            herdr.clone(),
            engine.clone(),
            true,
            TRIGGER_DISPATCH_COOLDOWN,
        );

        let logs = CapturedLogs::new();
        let outcome = {
            let _guard = logs.attach();
            trigger.on_verdict("run-capped", Verdict::Lost).await
        };

        assert_eq!(outcome, Some(GateOutcome::DryRun));
        assert!(
            dispatched_ticks(&engine).is_empty(),
            "a dry run calls dispatch_run zero times"
        );
        assert!(
            herdr.sent_text_log("w2:p5").await.is_empty(),
            "a dry run sends nothing to any pane either"
        );
        let text = logs.text();
        assert!(
            text.contains("DRY RUN") && text.contains("run-capped") && text.contains("proj-1"),
            "the dry run must log the transition and the dispatch it withheld: {text}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// D10 ahead of D8: a dry run measures VOLUME, so every transition inside
    /// what would be one cooldown window gets its own line. Behind the
    /// cooldown, the operator sizing the window would see exactly one line per
    /// window — the measurement filtered through the limit being measured.
    #[tokio::test]
    async fn a_dry_run_logs_every_transition_inside_one_cooldown_window() {
        let root = temp_root("dryrun-burst");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-a", None);
        seed_run(&engine, "run-b", None);
        seed_run(&engine, "run-c", None);
        // The real window, so the second and third verdicts land well inside
        // it — under the old order they would have come back `CooledDown`.
        let (trigger, _cancel) =
            trigger_for(herdr, engine.clone(), true, TRIGGER_DISPATCH_COOLDOWN);

        let logs = CapturedLogs::new();
        {
            let _guard = logs.attach();
            for run in ["run-a", "run-b", "run-c"] {
                assert_eq!(
                    trigger.on_verdict(run, Verdict::Lost).await,
                    Some(GateOutcome::DryRun),
                    "{run} is a real transition and a dry run reports every one of them"
                );
            }
        }

        let text = logs.text();
        assert_eq!(
            text.matches("DRY RUN").count(),
            3,
            "three transitions, three dry-run lines -- volume is what D10 measures: {text}"
        );
        for run in ["run-a", "run-b", "run-c"] {
            assert!(text.contains(run), "the dry run names {run}: {text}");
        }
        assert!(
            dispatched_ticks(&engine).is_empty(),
            "a dry run still calls dispatch_run zero times, burst or not"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The gates D10 still sits BEHIND: a transition about this task's own
    /// tick (D9), or one in a project that declined orchestration (D7), is
    /// silent in dry run too. A dry run reports real transitions, not noise.
    #[tokio::test]
    async fn a_dry_run_stays_silent_for_self_excluded_and_non_consenting_transitions() {
        let root = temp_root("dryrun-silent");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-own-tick", Some(TRIGGER_FEATURE_MARKER));
        let (trigger, _cancel) = trigger_for(
            herdr.clone(),
            engine.clone(),
            true,
            TRIGGER_DISPATCH_COOLDOWN,
        );

        let logs = CapturedLogs::new();
        let own = {
            let _guard = logs.attach();
            trigger.on_verdict("run-own-tick", Verdict::Lost).await
        };
        assert_eq!(own, Some(GateOutcome::SelfExcluded));
        assert!(
            !logs.text().contains("DRY RUN"),
            "D9 runs before D10: a tick's own run is not a transition to report: {}",
            logs.text()
        );

        let quiet_root = temp_root("dryrun-silent-unconsented");
        let quiet_engine = test_engine(false, &quiet_root);
        let quiet_herdr = spawnable_herdr(&quiet_root).await;
        seed_run(&quiet_engine, "run-capped", None);
        let (quiet_trigger, _quiet_cancel) = trigger_for(
            quiet_herdr,
            quiet_engine.clone(),
            true,
            TRIGGER_DISPATCH_COOLDOWN,
        );

        let quiet_logs = CapturedLogs::new();
        let declined = {
            let _guard = quiet_logs.attach();
            quiet_trigger.on_verdict("run-capped", Verdict::Lost).await
        };
        assert_eq!(declined, Some(GateOutcome::NotConsented));
        assert!(
            !quiet_logs.text().contains("DRY RUN"),
            "D7 runs before D10: a project that declined orchestration is not observed at all: {}",
            quiet_logs.text()
        );
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&quiet_root).ok();
    }

    /// The cancel flag is checked after every other gate and immediately
    /// before the spawn — a task switched off mid-decision spawns nothing.
    #[tokio::test]
    async fn a_cancelled_task_spawns_nothing() {
        let root = temp_root("cancelled");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-capped", None);
        let (trigger, cancel) =
            trigger_for(herdr, engine.clone(), false, TRIGGER_DISPATCH_COOLDOWN);
        cancel.store(true, Ordering::SeqCst);

        assert_eq!(
            trigger.on_verdict("run-capped", Verdict::Lost).await,
            Some(GateOutcome::Cancelled)
        );
        assert!(dispatched_ticks(&engine).is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    /// D4c's whole path, and its edge: a still-`working` row past the
    /// threshold wakes exactly one tick, and the SAME row — still working,
    /// still overrun — wakes nothing on the next poll. An overrun is a
    /// transition, not a condition that keeps being true at you.
    #[tokio::test]
    async fn an_overrun_run_dispatches_once_and_not_again_on_the_next_poll() {
        let root = temp_root("overrun");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run_at(&engine, "run-stuck", None, LONG_AGO);
        // A cooldown of zero so a second dispatch would be free to happen —
        // this test must prove the seen-set stopped it, never D8's window.
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        let first = trigger.scan_overruns().await;
        assert_eq!(
            first,
            vec![("run-stuck".to_string(), GateOutcome::Dispatched)],
            "one overrun row, one dispatch"
        );
        let ticks = dispatched_ticks(&engine);
        assert_eq!(ticks.len(), 1);
        assert!(
            ticks[0].task.contains(TRANSITION_OVERRUN) && ticks[0].task.contains("run-stuck"),
            "the tick names the transition and its evidence pointer: {:?}",
            ticks[0].task
        );

        let second = trigger.scan_overruns().await;
        assert!(
            second.is_empty(),
            "a still-overrunning row is not a second transition: {second:?}"
        );
        assert_eq!(
            dispatched_ticks(&engine).len(),
            1,
            "fire once per run, forever"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// D9 in the pulling detector: a tick this task dispatched, wedged and
    /// long overdue, is never an overrun transition. Without this a stuck
    /// tick would wake a tick about itself on the very next poll.
    #[tokio::test]
    async fn the_tasks_own_stuck_tick_is_never_an_overrun() {
        let root = temp_root("overrun-self");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run_at(
            &engine,
            "run-own-tick",
            Some(TRIGGER_FEATURE_MARKER),
            LONG_AGO,
        );
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        assert!(
            trigger.scan_overruns().await.is_empty(),
            "the task's own row is dropped before it is ever a transition"
        );
        assert_eq!(
            dispatched_ticks(&engine).len(),
            1,
            "the seeded tick is the only marked run -- no second one was spawned"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The other side of the threshold, and the fail-closed parse with it: a
    /// fresh row is not overrun, and neither is one whose age cannot be read.
    #[tokio::test]
    async fn a_run_younger_than_the_threshold_dispatches_nothing() {
        let root = temp_root("overrun-young");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run(&engine, "run-fresh", None);
        seed_run_at(&engine, "run-unreadable", None, "whenever");
        seed_run_at(&engine, "run-skewed", None, "2099-01-01T00:00:00Z");
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        assert!(trigger.scan_overruns().await.is_empty());
        assert!(dispatched_ticks(&engine).is_empty());

        let now = OffsetDateTime::now_utc();
        assert!(older_than(LONG_AGO, now, TRIGGER_OVERRUN_THRESHOLD));
        assert!(!older_than(&now_rfc3339(), now, TRIGGER_OVERRUN_THRESHOLD));
        std::fs::remove_dir_all(&root).ok();
    }

    /// D4b's whole path and its whole table, in the order a live fleet
    /// produces them: a pane that is not blocked yet, the entry into
    /// `Blocked` (one tick), the same pane still blocked on the next poll
    /// (nothing — a block is a transition, not a condition that keeps being
    /// true at you), and the way back out (nothing — leaving is a change, but
    /// not a change INTO `Blocked`). The same table `watcher.rs`'s own
    /// `StatusCursor` tests hold, judged here through the dispatch gate.
    #[tokio::test]
    async fn a_pane_entering_blocked_dispatches_once_and_not_while_it_stays_blocked() {
        let root = temp_root("blocked");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run_on_pane(&engine, "run-on-p3", None, "w2:p3");
        // A cooldown of zero so every later poll below is free to dispatch —
        // whatever stops them must be the cursor, never D8's window.
        let (trigger, _cancel) = trigger_for(herdr.clone(), engine.clone(), false, Duration::ZERO);

        // The independence this detector owes D6: the notify watcher's switch
        // is off, and this scan is about to work anyway.
        assert!(
            !engine.config.terminal.notify_enabled,
            "this detector must not need notify_enabled to be on"
        );

        // First poll: w2:p3 is not blocked. The fake's w1:p2 IS blocked and is
        // a first sight, but it belongs to no waggledance-spawned run — a
        // human's own agent is not this task's business.
        assert!(
            trigger.scan_blocked().await.is_empty(),
            "nothing entered blocked, and a blocked pane with no run behind it is not a transition"
        );
        assert!(dispatched_ticks(&engine).is_empty());

        herdr
            .set_status("w2:p3", AgentStatus::Blocked)
            .await
            .unwrap();
        assert_eq!(
            trigger.scan_blocked().await,
            vec![("run-on-p3".to_string(), GateOutcome::Dispatched)],
            "the entry into blocked is the transition"
        );
        let ticks = dispatched_ticks(&engine);
        assert_eq!(ticks.len(), 1, "exactly one tick per transition");
        assert!(
            ticks[0].task.contains(TRANSITION_BLOCKED) && ticks[0].task.contains("run-on-p3"),
            "the tick names the transition and its evidence pointer: {:?}",
            ticks[0].task
        );

        assert!(
            trigger.scan_blocked().await.is_empty(),
            "a pane that is still blocked is not a second transition"
        );
        herdr
            .set_status("w2:p3", AgentStatus::Working)
            .await
            .unwrap();
        assert!(
            trigger.scan_blocked().await.is_empty(),
            "leaving blocked is a status change, but it is not D4b's transition"
        );
        assert_eq!(
            dispatched_ticks(&engine).len(),
            1,
            "one entry into blocked, one tick"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The restart case, pinned deliberately: a run whose pane is ALREADY
    /// blocked when this task first looks fires, because a first sight is an
    /// entry as far as the cursor is concerned — and that is the right
    /// answer. waggledance restarting does not un-block an agent, and the
    /// human waiting on that pane is no less stuck for it.
    #[tokio::test]
    async fn a_pane_already_blocked_at_the_first_snapshot_is_an_entry() {
        let root = temp_root("blocked-first-sight");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        // The fake seeds w1:p2 blocked from the start.
        seed_run_on_pane(&engine, "run-on-p2", None, "w1:p2");
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        assert_eq!(
            trigger.scan_blocked().await,
            vec![("run-on-p2".to_string(), GateOutcome::Dispatched)]
        );
        assert_eq!(dispatched_ticks(&engine).len(), 1);

        assert!(
            trigger.scan_blocked().await.is_empty(),
            "and once seen, it is finished with"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// D9 in the blocked detector: a tick this task dispatched, sitting
    /// blocked on its own prompt, is never a transition. Without this the
    /// next poll would wake a tick about the first tick's own pane.
    #[tokio::test]
    async fn a_blocked_pane_belonging_to_the_tasks_own_tick_is_never_a_transition() {
        let root = temp_root("blocked-self");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        seed_run_on_pane(
            &engine,
            "run-own-tick",
            Some(TRIGGER_FEATURE_MARKER),
            "w2:p4",
        );
        let (trigger, _cancel) = trigger_for(herdr.clone(), engine.clone(), false, Duration::ZERO);

        assert!(trigger.scan_blocked().await.is_empty());
        herdr
            .set_status("w2:p4", AgentStatus::Blocked)
            .await
            .unwrap();

        assert!(
            trigger.scan_blocked().await.is_empty(),
            "the task's own pane is dropped before it is ever a transition"
        );
        assert_eq!(
            dispatched_ticks(&engine).len(),
            1,
            "the seeded tick is the only marked run -- no second one was spawned"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The threshold is chosen against the reaper's own windows, not picked
    /// out of the air — a value inside them would fire on runs the reaper is
    /// still in the middle of judging.
    #[test]
    fn the_overrun_threshold_sits_well_past_what_the_reaper_handles() {
        assert!(
            TRIGGER_OVERRUN_THRESHOLD > crate::reaper::GRACE_WINDOW * 10,
            "an overrun must be a run the reaper has repeatedly left alone, \
             not one it has not looked at yet"
        );
    }

    /// D1's discipline, made mechanical: the task text is ONE template with
    /// three values substituted into it. Whatever the transition kind is,
    /// the wording around it is byte-identical — no branch anywhere chooses
    /// different words for a different transition.
    #[test]
    fn the_task_text_never_branches_on_the_transition_kind() {
        let project = Project {
            id: "proj-1".into(),
            name: "p".into(),
            root_path: std::path::PathBuf::from("/tmp/p"),
            created_at: now_rfc3339(),
            last_seen_at: now_rfc3339(),
            orchestration_enabled: true,
        };
        // Every kind the four D4 detectors will ever name, plus one nobody
        // will: the template cannot be treating any of them specially.
        for kind in [
            TRANSITION_CAPPED,
            "a run entered blocked",
            "a run overran",
            "a new escalation row appeared",
            "something nobody has named yet",
        ] {
            for run in [Some("run-7"), None] {
                let produced = task_text(kind, &project, run);
                let expected = TRIGGER_TASK_TEMPLATE
                    .replace("{transition}", kind)
                    .replace("{run}", run.unwrap_or("-"))
                    .replace("{project}", &project.id);
                assert_eq!(
                    produced, expected,
                    "the wording must be pure substitution for kind {kind:?}"
                );
            }
        }
        // And the shared skeleton really is shared: strip each kind's own
        // name back out and every message is the same message.
        let skeleton =
            |kind: &str| task_text(kind, &project, Some("run-7")).replace(kind, "<kind>");
        assert_eq!(skeleton(TRANSITION_CAPPED), skeleton("a run overran"));
        assert!(
            !TRIGGER_TASK_TEMPLATE.contains("should")
                && !TRIGGER_TASK_TEMPLATE.contains("recommend"),
            "the template names why and where, never what to conclude (D1)"
        );
    }

    /// The accepted risk that must never be silent: an armed trigger with
    /// the reaper off leaks a stuck pane per wedged tick.
    #[test]
    fn arming_the_trigger_without_the_reaper_is_warned_about() {
        use waggledance_core::config::TerminalConfig;
        let both_on = TerminalConfig {
            trigger_enabled: true,
            reaper_enabled: true,
            ..Default::default()
        };
        assert!(lifecycle_warning(&both_on).is_none());
        assert!(lifecycle_warning(&TerminalConfig::default()).is_none());

        let leaky = TerminalConfig {
            trigger_enabled: true,
            reaper_enabled: false,
            ..Default::default()
        };
        let warning = lifecycle_warning(&leaky).expect("this pair must never be silent");
        assert!(warning.contains("reaper_enabled"), "{warning}");
    }

    // --- D4d: the escalation mailbox ---

    /// One mailbox row in the bytes `bee supervisor record` actually writes
    /// (generated against a scratch store on 2026-08-31 and locked in
    /// `waggledance_core::bee`'s own tests — this is the same shape, kept
    /// here so a trigger test reads like the file it is standing in for).
    fn mailbox_row(id: &str, kind: &str, point_key: &str, session: &str) -> String {
        format!(
            r#"{{"ts":"2026-08-31T10:17:47.459Z","event":"record","id":"{id}","kind":"{kind}","signal":"none","point_key":"{point_key}","question":"Is the row shape stable?","target_session":"{session}","tick":null,"queued":false}}"#
        )
    }

    /// Replace a project's whole mailbox with `lines`. Append-only in
    /// production; a test writes the file it wants to have been appended to.
    fn write_mailbox(root: &std::path::Path, lines: &[String]) {
        let dir = root.join(".bee").join("supervisor");
        std::fs::create_dir_all(&dir).unwrap();
        let mut body = lines.join("\n");
        body.push('\n');
        std::fs::write(dir.join("interventions.jsonl"), body).unwrap();
    }

    /// The whole D4d path: a row appended to a consenting project's mailbox
    /// since the last poll wakes exactly one tick, and only one.
    #[tokio::test]
    async fn a_new_escalation_row_dispatches_exactly_one_tick() {
        let root = temp_root("escalation-new");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        let (trigger, _cancel) =
            trigger_for(herdr, engine.clone(), false, TRIGGER_DISPATCH_COOLDOWN);

        // The first poll baselines the mailbox (empty here) and says nothing.
        assert!(trigger.scan_escalations().await.is_empty());

        write_mailbox(&root, &[mailbox_row("esc-1", "escalation", "p-one", "s1")]);
        let outcomes = trigger.scan_escalations().await;

        assert_eq!(
            outcomes,
            vec![("esc-1".to_string(), GateOutcome::Dispatched)],
            "the one appended row is the one transition"
        );
        assert_eq!(dispatched_ticks(&engine).len(), 1);

        // And it never fires again while it sits there.
        assert!(
            trigger.scan_escalations().await.is_empty(),
            "a row already accounted for is not a transition on the next poll"
        );
        assert_eq!(dispatched_ticks(&engine).len(), 1);
        std::fs::remove_dir_all(&root).ok();
    }

    /// D4d's closed kind set: `urgent` is a transition too, and
    /// `intervention` — the third mailbox kind — is not.
    #[tokio::test]
    async fn only_escalation_and_urgent_rows_are_transitions() {
        let root = temp_root("escalation-kinds");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        // Dry run with no cooldown: this test is about which kinds pass the
        // detector, not about spawning or about D8.
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), true, Duration::ZERO);
        assert!(trigger.scan_escalations().await.is_empty());

        write_mailbox(
            &root,
            &[mailbox_row("int-1", "intervention", "p-one", "s1")],
        );
        assert!(
            trigger.scan_escalations().await.is_empty(),
            "an ordinary intervention row is not a D4d transition"
        );

        write_mailbox(
            &root,
            &[
                mailbox_row("int-1", "intervention", "p-one", "s1"),
                mailbox_row("urg-1", "urgent", "p-two", "s2"),
            ],
        );
        assert_eq!(
            trigger.scan_escalations().await,
            vec![("urg-1".to_string(), GateOutcome::DryRun)],
            "urgent is the danger class and fires like an escalation"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A project that has never run a supervisor tick has no store at all.
    /// That is a normal, expected shape — not an error, and not a transition.
    #[tokio::test]
    async fn a_missing_supervisor_store_reads_as_empty() {
        let root = temp_root("escalation-absent");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        assert!(trigger.scan_escalations().await.is_empty());
        assert!(trigger.scan_escalations().await.is_empty());

        assert_eq!(dispatched_ticks(&engine).len(), 0);
        assert!(
            !root.join(".bee").join("supervisor").exists(),
            "D5: reading an absent supervisor store must never create one"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A mailbox already full of history is not a burst of transitions the
    /// moment the daemon starts — the first read baselines it.
    #[tokio::test]
    async fn the_first_read_of_a_mailbox_primes_the_cursor_and_dispatches_nothing() {
        let root = temp_root("escalation-prime");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        write_mailbox(
            &root,
            &[
                mailbox_row("old-1", "escalation", "p-one", "s1"),
                mailbox_row("old-2", "urgent", "p-two", "s2"),
                mailbox_row("old-3", "escalation", "p-three", "s3"),
            ],
        );
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        assert!(
            trigger.scan_escalations().await.is_empty(),
            "history is not news: a restart must not wake a tick per filed row"
        );
        assert_eq!(dispatched_ticks(&engine).len(), 0);
        std::fs::remove_dir_all(&root).ok();
    }

    /// D5/D7: a project that declined orchestration has its mailbox left
    /// alone entirely — the transition never reaches the gate at all.
    #[tokio::test]
    async fn a_non_consenting_projects_mailbox_dispatches_nothing() {
        let root = temp_root("escalation-unconsented");
        let engine = test_engine(false, &root);
        let herdr = spawnable_herdr(&root).await;
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), false, Duration::ZERO);

        assert!(trigger.scan_escalations().await.is_empty());
        write_mailbox(&root, &[mailbox_row("esc-1", "escalation", "p-one", "s1")]);
        assert!(trigger.scan_escalations().await.is_empty());

        assert_eq!(dispatched_ticks(&engine).len(), 0);
        std::fs::remove_dir_all(&root).ok();
    }

    /// D9, on the one detector whose transitions name no run: a row this
    /// task's own tick could have filed is dropped before the gate. A no-op
    /// filter today (D5 means this task files no rows at all) and here so it
    /// stays a no-op if that ever changes.
    #[tokio::test]
    async fn a_row_this_tasks_own_tick_could_have_filed_is_never_a_transition() {
        let root = temp_root("escalation-self");
        let engine = test_engine(true, &root);
        let herdr = spawnable_herdr(&root).await;
        let (trigger, _cancel) = trigger_for(herdr, engine.clone(), true, Duration::ZERO);
        assert!(trigger.scan_escalations().await.is_empty());

        write_mailbox(
            &root,
            &[
                mailbox_row("mine-1", "escalation", TRIGGER_FEATURE_MARKER, "s1"),
                mailbox_row("mine-2", "urgent", "p-two", TRIGGER_FEATURE_MARKER),
                mailbox_row("theirs", "escalation", "p-three", "s3"),
            ],
        );

        assert_eq!(
            trigger.scan_escalations().await,
            vec![("theirs".to_string(), GateOutcome::DryRun)],
            "only the row that is not this task's own is a transition"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// The reader's one deliberate departure from `bee.rs`'s silent
    /// precedent: a malformed line is named by its 1-indexed line number
    /// before it is skipped, and the rows around it still read. Asserted
    /// against the reader directly — `scan_escalations` runs it on a blocking
    /// worker thread, which a thread-local test subscriber cannot see.
    #[test]
    fn a_malformed_mailbox_line_warns_by_line_number_and_is_skipped() {
        let root = temp_root("escalation-malformed");
        write_mailbox(
            &root,
            &[
                mailbox_row("first", "escalation", "p-one", "s1"),
                "{not json at all".to_string(),
                mailbox_row("third", "urgent", "p-three", "s3"),
            ],
        );

        let logs = CapturedLogs::new();
        let rows = {
            let _guard = logs.attach();
            waggledance_core::bee::read_escalations(&root)
        };

        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["first", "third"],
            "a bad line never aborts the rows around it"
        );
        let text = logs.text();
        assert!(text.contains("WARN"), "the skip is never silent: {text}");
        assert!(
            text.contains("line=2"),
            "the warning names the line it skipped: {text}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A `tracing` subscriber that writes into a buffer this test can read —
    /// the only way to assert on a log line that IS the feature (D10's whole
    /// output is a log line).
    struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

    impl CapturedLogs {
        fn new() -> Self {
            CapturedLogs(Arc::new(Mutex::new(Vec::new())))
        }

        fn attach(&self) -> tracing::subscriber::DefaultGuard {
            let buffer = self.0.clone();
            tracing::subscriber::set_default(
                tracing_subscriber::fmt()
                    .with_writer(move || BufferWriter(buffer.clone()))
                    .with_ansi(false)
                    .finish(),
            )
        }

        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
