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
//! Two detectors are wired. D4a, a run capped, is PUSHED: its source is the
//! reaper's own sweep verdict, arriving over the channel `Reaper::run` was
//! given, because the reaper is the only thing in the process that can tell a
//! run it capped apart from an ordinary healthy completion — from outside,
//! the ledger's status column reads identically for both. D4c, a run
//! overrun, is PULLED: the poll branch scans the same ledger list the reaper
//! sweeps and compares each row's age against
//! [`TRIGGER_OVERRUN_THRESHOLD`]. What keeps a pulled detector on the
//! event-driven side of D3 is its edge: a per-run seen-set, so an overrun
//! fires once when the row crosses the threshold and never again while it
//! stays across it. The remaining two D4 detectors (blocked, escalation row)
//! hang off the same loop and reuse the same gate.
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

use crate::herdr::Herdr;
use crate::orchestrate::{self, DispatchTarget, RunStatus};
use crate::reaper::Verdict;

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
    /// D10. Every gate above it still runs; only the one external call is
    /// replaced by the log line describing it.
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
        Trigger {
            herdr,
            engine,
            interval,
            cooldown,
            dry_run,
            last_dispatch: Mutex::new(HashMap::new()),
            seen_overrun: Mutex::new(HashSet::new()),
            cancelled,
        }
    }

    /// The one gate every detector funnels a transition through, in this
    /// order and deliberately: D9 self-exclusion, D7/D6 consent, D8
    /// cooldown, D10 dry-run, the cancel flag, then the dispatch.
    ///
    /// The order is the point. Self-exclusion runs first so a tick's own
    /// run can never even consume a cooldown slot; consent runs before
    /// anything is spent on a project that declined orchestration; and the
    /// cancel flag is last, immediately before the one call that spawns an
    /// agent, which is the same placement `reaper.rs` and `supervisor.rs`
    /// give theirs.
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

        // (3) D8 — the stamp is taken HERE, the moment the decision to act is
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

        // (4) D10 — everything above ran exactly as it would in production;
        // only the spawn is replaced by the description of it.
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
    /// alone, and letting it through here would double-detect it the moment
    /// that detector lands. `LeftAlone` and `TooYoung` are not transitions
    /// at all: nothing changed.
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
    /// scan runs there, and the counter advances after it, so a tick means a
    /// poll that completed its scan. D4a takes the other arm — its source
    /// pushes, so it needs no polling at all.
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
        let run = Run {
            id: id.into(),
            project_id: "proj-1".into(),
            pane_id: "w9:p9".into(),
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
