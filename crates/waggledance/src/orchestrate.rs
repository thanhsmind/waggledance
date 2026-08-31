//! Protocol engine — the mechanical Herdr protocol (D1/D5): preflight a
//! target pane, mint a fresh split marker, capture a pre-send baseline,
//! send the task, and poll for completion, all as functions over `&dyn
//! Herdr` so the whole protocol is testable against `FakeHerdr` with no
//! live herdr and no MCP process (`waggledance mcp`, a later slice, calls
//! these functions; it owns no protocol logic of its own — see
//! `docs/history/orchestrator-dispatch/plan.md`'s phase table).
//!
//! Fail-closed throughout (D5): a send is refused before any pane write
//! unless the target is verifiably `Idle`/`Done`/`Unknown`; a snapshot
//! failure is treated as unverifiable, never as "probably fine". Completion
//! is proven only by a *fresh* split marker — present in a current read and
//! absent from the run's own pre-send baseline — never by a marker string
//! that merely happens to already be on screen (a prior run's own echo, or
//! an agent quoting the instruction back). `Unknown` status (no agent
//! status this app trusts) falls back to three consecutive unchanged
//! `ansi::revision_of` reads as a settled-content proxy for completion.
//!
//! Every item here is exercised by this module's own `#[cfg(test)]` suite
//! (against `FakeHerdr`); its production caller is `mcp.rs`'s
//! `waggledance_dispatch`/`waggledance_await` tools (phase 3, per
//! `docs/history/orchestrator-dispatch/plan.md`'s phase table).

use std::time::Duration;

use waggledance_core::ansi;
use waggledance_core::domain::Run;
use waggledance_core::indexer::now_rfc3339;
use waggledance_core::notify_store::NotifyStore;
use waggledance_core::paths_boundary::Boundary;
use waggledance_core::Engine;

use crate::herdr::{self, AgentStatus, Herdr, HerdrError, ReadSource};
use crate::notify;

/// Herdr's own hard cap on a `recent` read (mirrors `pane_scroller.rs`'s own
/// local copy of the same constant, `pane_scroller.rs:52`) — baseline/delta
/// reads use `Recent`, capped the same way.
pub(crate) const RECENT_LINES_CAP: usize = 1000;

/// Hard cap on `await_run`'s wait (D4) — a caller-requested longer timeout
/// is silently clamped, never honored, never an error.
pub const MAX_AWAIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Production poll cadence for `await_run`'s status/content loop — long
/// enough not to hammer herdr with `snapshot`+`read_pane` calls every tick,
/// short enough not to waste much of a 60s budget on coarse granularity.
const AWAIT_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Consecutive unchanged `ansi::revision_of` reads required before an
/// `Unknown`-status pane is treated as settled (D5 content-stability
/// fallback).
const STABILITY_READS: u32 = 3;

/// The split marker's fixed half — never sent to a pane joined with its
/// suffix (see `Marker`'s own doc for why).
const MARKER_PREFIX: &str = "HERDR_DONE_";

/// Combines every failure `await_run` can hit: a herdr transport/protocol
/// failure, or a run-state persistence failure through the cell-1
/// repository methods.
#[derive(Debug, thiserror::Error)]
pub enum OrchestrateError {
    #[error(transparent)]
    Herdr(#[from] HerdrError),
    #[error("run persistence failed: {0}")]
    Store(#[from] waggledance_core::Error),
}

/// Why `preflight` refused a send, before any pane write happened (D5
/// fail-closed).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DispatchRefusal {
    #[error("pane {0} is working -- refusing to interrupt an in-flight run")]
    Working(String),
    #[error("pane {0} is blocked -- refusing to send while it waits on a human")]
    Blocked(String),
    #[error("herdr snapshot unavailable -- pane {pane_id} status is unverifiable: {reason}")]
    Unverifiable { pane_id: String, reason: String },
    #[error("no such pane: {0}")]
    NoSuchPane(String),
    #[error("pane {pane_id} is not inside project {project_id}'s own root -- refusing to dispatch across project boundaries")]
    OutsideBoundary { pane_id: String, project_id: String },
    /// herdr could not be read at all, so nothing about the destination is
    /// verifiable. Carries the transport error's own words, never a
    /// generic failure -- the caller shows them.
    #[error("herdr snapshot failed: {0}")]
    SnapshotFailed(String),
    /// No herdr workspace resolves inside the project's own root, so there
    /// is nowhere this project may legally start an agent. Never a fallback
    /// to some other directory.
    #[error("project {project_id} destination unresolved: {reason}")]
    DestinationUnresolved { project_id: String, reason: String },
    #[error("agent start failed: {0}")]
    AgentStartFailed(String),
    #[error("herdr read failed: {0}")]
    BaselineFailed(String),
    #[error("herdr send failed: {0}")]
    SendFailed(String),
    #[error("run persistence failed: {0}")]
    PersistenceFailed(String),
}

/// Where [`dispatch_run`] puts a task: into a pane that already exists, or
/// into one it starts for this run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchTarget {
    /// An existing pane, ALREADY contained to the caller's own project by
    /// the caller ([`verify_pane_in_boundary`] for a caller-supplied id,
    /// or a pane list that was itself built from a validated boundary).
    /// [`dispatch_run`] still preflights its status; containment is the one
    /// thing it trusts the caller for, because the two callers contain
    /// differently -- the MCP tool against the project root, the board
    /// against the feature's own granted worktree, which is a sibling
    /// directory OUTSIDE that root.
    Pane(String),
    /// Start a fresh agent pane from a project's own declaration. `entry`
    /// carries the command AND the conditions declared around it -- an `env`
    /// map to export, a trust store to seed -- because a `herding.agents`
    /// entry is more than an argv and honouring only the argv starts an agent
    /// that then stalls at a prompt nobody sees. `cwd` is the directory to
    /// start it in; `None` means the project's own resolved destination
    /// ([`resolve_spawn_destination`]), the only directory this module will
    /// ever choose on a caller's behalf.
    Spawn {
        entry: waggledance_core::bee::BeeHerdingEntry,
        cwd: Option<String>,
    },
}

/// Confirm `pane_id` names a pane whose own folder resolves **inside**
/// `boundary` (D6 per-project containment): the dispatch destination is only
/// ever a pane the calling project already owns, never one enumerated off
/// another project on the same host. Mirrors `server.rs`'s `project_panes`
/// containment rule exactly — a pane is in-project when its `cwd`, or failing
/// that its `foreground_cwd`, `validate_existing`s under the boundary — which
/// is the same check every sibling pane-scoped write route runs through
/// `project_and_verify_pane_in_boundary`. The pane-spawn branch of
/// [`run_dispatch`] never needs this: it *creates* the pane under a
/// boundary-validated destination, so containment is structural there;
/// the caller-supplied-`pane_id` branch is the one that must prove it.
///
/// `project_id` is carried only for the refusal message. A `pane_id` absent
/// from the snapshot's `panes[]`, or present but with no folder resolving
/// inside the boundary, both refuse — the second is the cross-project attack
/// this closes, the first is a stale/unknown pane, and neither is ever
/// treated as "probably fine".
pub fn verify_pane_in_boundary(
    snapshot: &herdr::Snapshot,
    boundary: &Boundary,
    pane_id: &str,
    project_id: &str,
) -> Result<(), DispatchRefusal> {
    let contained = snapshot
        .panes
        .iter()
        .filter(|pane| pane.pane_id == pane_id)
        .any(|pane| {
            pane.cwd
                .as_deref()
                .and_then(|raw| boundary.validate_existing(std::path::Path::new(raw)).ok())
                .or_else(|| {
                    pane.foreground_cwd
                        .as_deref()
                        .and_then(|raw| boundary.validate_existing(std::path::Path::new(raw)).ok())
                })
                .is_some()
        });
    if contained {
        Ok(())
    } else {
        Err(DispatchRefusal::OutsideBoundary {
            pane_id: pane_id.to_string(),
            project_id: project_id.to_string(),
        })
    }
}

/// Resolve `pane_id`'s current [`AgentStatus`] from a fresh snapshot and
/// gate the send on it (D5): only `Idle`/`Done`/`Unknown` are sendable.
/// `Working`/`Blocked` refuse outright, and a snapshot failure (transport
/// down, protocol mismatch, any `HerdrError`) is treated as unverifiable and
/// refuses fail-closed — never "probably fine".
pub async fn preflight(herdr: &dyn Herdr, pane_id: &str) -> Result<AgentStatus, DispatchRefusal> {
    let snapshot = herdr
        .snapshot()
        .await
        .map_err(|e| DispatchRefusal::Unverifiable {
            pane_id: pane_id.to_string(),
            reason: e.to_string(),
        })?;
    let agent = snapshot
        .agents
        .iter()
        .find(|a| a.pane_id == pane_id)
        .ok_or_else(|| DispatchRefusal::NoSuchPane(pane_id.to_string()))?;
    match agent.status {
        AgentStatus::Working => Err(DispatchRefusal::Working(pane_id.to_string())),
        AgentStatus::Blocked => Err(DispatchRefusal::Blocked(pane_id.to_string())),
        AgentStatus::Done | AgentStatus::Idle | AgentStatus::Unknown => Ok(agent.status),
    }
}

/// A freshly-minted split completion marker (D5). [`Marker::joined`] is the
/// exact string a completed run must print — the value `await_run` searches
/// for — and it is never sent to the pane as a contiguous substring:
/// [`Marker::instruction`] spells the two halves as two separate quoted
/// tokens the agent is told to concatenate itself. Sending the joined
/// string directly would echo it straight into the pane's own screen the
/// moment it's typed, and `await_run`'s "present in a current read" check
/// would then see it before the agent had done anything at all — a
/// same-poll false completion, not a marker the agent actually produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    suffix: String,
}

impl Marker {
    /// The exact string a completed run prints — what `await_run` searches
    /// a current read for, and what a run's baseline is checked against for
    /// staleness.
    pub fn joined(&self) -> String {
        format!("{MARKER_PREFIX}{}", self.suffix)
    }

    /// The instruction appended to the task text — spells the marker as two
    /// separate tokens (never joined) so the send itself never echoes the
    /// completion string onto the pane's own screen.
    pub fn instruction(&self) -> String {
        format!(
            "When (and only when) the task above is fully complete, print the string \"{MARKER_PREFIX}\" immediately followed by \"{}\" -- concatenate the two with no space, no punctuation, and no line break between them -- on a line by itself.",
            self.suffix
        )
    }
}

/// Mint a fresh, unique split marker for one run (D5): a random 64-bit
/// suffix, formatted as hex.
pub fn mint_marker() -> Marker {
    Marker {
        suffix: format!("{:016x}", rand::random::<u64>()),
    }
}

/// Capture `pane_id`'s pre-send transcript — D5's baseline. A `Recent` read
/// is the only legal source for a scrollback-spanning capture (`Visible`
/// only sees the current on-screen rows). A run's baseline is compared
/// against every later `Recent` read to prove marker *freshness* and to
/// compute the transcript delta.
pub async fn capture_baseline(herdr: &dyn Herdr, pane_id: &str) -> herdr::Result<String> {
    let read = herdr
        .read_pane(pane_id, ReadSource::Recent, RECENT_LINES_CAP)
        .await?;
    Ok(read.text)
}

/// The observed agent states [`send_task`] accepts as proof the submit
/// actually landed. `Working` alone is too narrow: a short turn can go
/// `Working` → `Idle` between the daemon's own observer samples and never
/// match, so a perfectly good send would report as a failure
/// (dispatch-submit-and-reclaim plan, "the decisive finding"). Each of the
/// three is a state the agent could only be observed in after taking input.
const SUBMIT_UNTIL: &[AgentStatus] = &[AgentStatus::Working, AgentStatus::Idle, AgentStatus::Done];

/// [`send_task`]'s wait budget, deliberately ABOVE the daemon's own ~5000ms
/// change-detection window. A genuine no-change then surfaces as
/// [`HerdrError::AgentPromptStalled`] — the answer dispatch needs — instead
/// of being masked as a plain [`HerdrError::Timeout`], which means something
/// else entirely here (see [`send_task`]).
const SUBMIT_TIMEOUT_MS: u64 = 8_000;

/// Send `task` into `pane_id`, followed by `marker`'s instruction, as one
/// submitted reply — the only pane write this module performs, and only
/// ever after a caller has already run [`preflight`] and captured a
/// baseline.
///
/// Routed through [`Herdr::agent_prompt`], never `send_input(.., true)`:
/// that older path fires its Enter blind, on a 1500ms settle heuristic that
/// reports nothing back, so a cold-starting agent swallowed the keystroke
/// and the run hung forever against a byte-identical delta
/// (dispatch-submit-and-reclaim defect A). A timing heuristic cannot know
/// whether a keystroke was accepted; only the agent's own observed state
/// can. Both dispatch targets reach this one call site, and both are
/// agent-tracked by the time they do — a `Pane` target was preflighted
/// through [`preflight`], which refuses `NoSuchPane` for a pane absent from
/// the snapshot's agents, and a `Spawn` target's `agent.start` only returns
/// once the agent is registered and ready for input.
///
/// The three failure shapes are not symmetric, and the asymmetry is the
/// point:
///
/// - [`HerdrError::AgentBlocked`] — nothing was submitted at all;
/// - [`HerdrError::AgentPromptStalled`] — the text WAS submitted and the
///   agent never visibly reacted;
/// - [`HerdrError::Timeout`] — a state change WAS observed first, so the
///   text landed and merely did not reach a state in [`SUBMIT_UNTIL`]
///   within the budget. "The text went in" is the whole question dispatch
///   asks, so this is **success**, not a failed send.
///
/// A stall is never retried. The text is already in the composer, so a
/// second send would re-type the task on top of itself
/// (dispatch-submit-and-reclaim P2-3); a stall is reported, never resent.
pub async fn send_task(
    herdr: &dyn Herdr,
    pane_id: &str,
    task: &str,
    marker: &Marker,
) -> herdr::Result<()> {
    let text = format!("{task}\n\n{}", marker.instruction());
    match herdr
        .agent_prompt(pane_id, &text, SUBMIT_UNTIL, SUBMIT_TIMEOUT_MS)
        .await
    {
        Ok(_) => Ok(()),
        Err(HerdrError::Timeout(_)) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Dispatch one task and record the run it started (D5's whole sequence in
/// one call): resolve the destination pane, preflight it, capture the
/// pre-send baseline, mint a fresh split marker, send, and persist a
/// `working` [`Run`]. Returns that run.
///
/// The ONE dispatch path in this process. Both callers -- the MCP
/// `waggledance_dispatch` tool and the board's run actions
/// (board-run-actions D1/D2) -- go through here, so a run started from a
/// card is the same kind of object, with the same baseline and marker
/// discipline, as one started from the tool; `await_run` cannot tell them
/// apart, which is exactly the point.
///
/// What each caller keeps for itself is what it alone knows: which pane or
/// argv to use (`target`), how the run is labelled (`preset_label`), and
/// which bee feature it belongs to (`feature`, the board's per-feature run
/// lock, `Engine::list_live_runs_for_feature`).
///
/// Nothing is persisted before the send succeeds: a refused or failed
/// dispatch leaves no run row behind to lock a feature forever.
pub async fn dispatch_run(
    herdr: &dyn Herdr,
    engine: &Engine,
    project: &waggledance_core::domain::Project,
    target: DispatchTarget,
    task: &str,
    feature: Option<&str>,
    preset_label: Option<String>,
) -> Result<Dispatched, DispatchRefusal> {
    let mut warnings: Vec<String> = Vec::new();
    let pane_id = match target {
        DispatchTarget::Pane(pane_id) => {
            preflight(herdr, &pane_id).await?;
            pane_id
        }
        DispatchTarget::Spawn { entry, cwd } => {
            let snapshot = herdr
                .snapshot()
                .await
                .map_err(|e| DispatchRefusal::SnapshotFailed(e.to_string()))?;
            let boundary = Boundary::new(vec![project.root_path.clone()]).map_err(|e| {
                DispatchRefusal::DestinationUnresolved {
                    project_id: project.id.clone(),
                    reason: e.to_string(),
                }
            })?;
            let (workspace_id, anchor) = resolve_spawn_destination(&snapshot, &boundary)
                .ok_or_else(|| DispatchRefusal::DestinationUnresolved {
                    project_id: project.id.clone(),
                    reason: "no herdr workspace has a resolved working directory under this \
                             project's own root; refusing to start an agent in an arbitrary \
                             directory"
                        .to_string(),
                })?;
            // The workspace is the placement anchor; the cwd is where the
            // agent actually lands. They differ for a board spawn into a
            // feature's granted worktree, which is a sibling of the root
            // the workspace resolved under.
            let dir = cwd.unwrap_or(anchor);
            // Protocol 20 split this in two: `agent.start` attaches to a
            // pane that already exists, so the pane has to be made first.
            // The shared helper owns that hop — including its refusal when
            // the new tab yields no pane, which must never be softened into
            // "use another pane" here.
            // D3: `dir` is the destination this function already validated
            // against the project's boundary, and it is the only directory
            // that reaches the trust seeding.
            let outcome = herdr::start_declared_agent(herdr, &workspace_id, Some(&dir), &entry)
                .await
                .map_err(|e| DispatchRefusal::AgentStartFailed(e.to_string()))?;
            warnings.extend(outcome.warnings);
            outcome.started.pane_id
        }
    };

    let baseline = capture_baseline(herdr, &pane_id)
        .await
        .map_err(|e| DispatchRefusal::BaselineFailed(e.to_string()))?;
    let marker = mint_marker();
    // A confirming submit ([`send_task`]): a stalled or blocked one refuses
    // the whole dispatch through `SendFailed`, carrying herdr's own words so
    // the refusal names the stall rather than saying "send failed". Nothing
    // below this line runs for a refused send, which is what keeps a wedged
    // dispatch from leaving a `working` run row behind forever.
    send_task(herdr, &pane_id, task, &marker)
        .await
        .map_err(|e| DispatchRefusal::SendFailed(e.to_string()))?;

    let now = now_rfc3339();
    let run = Run {
        id: format!("run-{:016x}", rand::random::<u64>()),
        project_id: project.id.clone(),
        pane_id,
        preset_label,
        task: task.to_string(),
        baseline,
        marker: marker.joined(),
        status: "working".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    engine
        .insert_run(&run, feature)
        .map_err(|e| DispatchRefusal::PersistenceFailed(e.to_string()))?;
    Ok(Dispatched { run, warnings })
}

/// What a dispatch produced: the durable run, plus anything that went wrong
/// on the way which did not stop it (herding-entry-conditions D9).
///
/// `warnings` exists so a fail-open step cannot go silent. A trust seeding
/// that fails is not fatal — but the operator must learn it in the same
/// answer that says the agent started, or the first they know of it is a pane
/// sitting at a prompt with nothing to attribute it to.
#[derive(Debug, Clone)]
pub struct Dispatched {
    pub run: Run,
    pub warnings: Vec<String>,
}

/// The transcript delta versus baseline — everything a `Recent` read has
/// gained since the run's own pre-send capture. `current` is expected to
/// carry `baseline` as a prefix (scrollback only grows forward); when it
/// doesn't (the buffer rotated past 1000 lines, dropping the baseline's own
/// oldest rows), the whole current read is returned rather than guessing at
/// an offset — a superset delta is safe, a wrong one is not.
fn delta_from_baseline(baseline: &str, current: &str) -> String {
    match current.strip_prefix(baseline) {
        Some(rest) => rest.to_string(),
        None => current.to_string(),
    }
}

/// Whether `current` shows `marker` as a marker the agent printed for THIS
/// run — the one completion signal that may retire a pane (D5), and the
/// single home of that rule.
///
/// Two halves, both required:
///
/// 1. `baseline` — the run's own pre-send capture — must NOT already carry
///    the joined marker. A marker minted for this run but already sitting in
///    its own baseline can only mean the string reached the pane some other
///    way (a leaked instruction, an unrelated echo); it can never be
///    evidence of completion, so staleness is decided against the run's own
///    fixed baseline, never re-derived from a later read.
/// 2. `current` — a fresh `Recent` read — must carry it.
///
/// [`await_run`] and the reaper's pre-check
/// (`crates/waggledance/src/reaper.rs`) both ask this question, and a second
/// copy of the rule is exactly how the two would drift into disagreeing
/// about which runs are finished — so there is one function and both call
/// it.
pub(crate) fn marker_is_fresh(baseline: &str, current: &str, marker: &str) -> bool {
    !baseline.contains(marker) && current.contains(marker)
}

/// Resolve the herdr workspace/cwd destination for spawning a fresh agent
/// pane (D3's preset-spawn path): the first workspace in `snapshot` whose D2
/// anchor (`Snapshot::anchor_cwd_for_workspace`) validates against
/// `boundary`. Mirrors `server.rs`'s `project_creation_destination` exactly
/// (same containment rule, same "first workspace that validates" scan) —
/// that function is private and `AppState`-typed, so the MCP dispatch path
/// (which has neither) gets its own copy here rather than a shared one.
/// `None` when no workspace qualifies: the caller refuses with a named
/// "destination unresolved" error rather than ever falling back to another
/// directory, and in particular never reaches [`Herdr::agent_start`]'s own
/// documented `cwd: None` fallback (herdr's own process directory) — every
/// caller of this function passes the resolved `cwd` as `Some`, never
/// `None`.
/// # Why the anchor alone is not enough
///
/// That anchor is the pane a human happens to have FOCUSED right now
/// (workspace → active tab → that tab's layout → its focused pane,
/// `herdr/wire.rs`), so on its own it makes a project dispatchable or not
/// depending on where the cursor sits. Observed live on 2026-08-25: beehive
/// had two agent panes whose folders resolve under its own root — `ask_state`
/// listed them for that project — while the workspace LABELLED `beehive` held
/// panes belonging to waggledance, so no anchor landed inside beehive and a
/// fully resolved preset label still refused with "destination unresolved".
///
/// So a second pass runs, and only when the first finds nothing: the first
/// workspace holding ANY pane whose folder validates against `boundary`,
/// using that pane's own resolved folder. Additive by construction — a
/// project that resolves today keeps the exact destination it has, and only
/// one that refuses today can begin resolving (spawn-destination-fallback
/// D1). Fail-closed is untouched: both passes validate through the same
/// `Boundary`, so a returned destination is always a directory this project
/// owns, and `None` still means the caller refuses rather than picking one.
pub fn resolve_spawn_destination(
    snapshot: &herdr::Snapshot,
    boundary: &Boundary,
) -> Option<(String, String)> {
    let by_anchor = snapshot.workspaces.iter().find_map(|w| {
        let anchor = snapshot.anchor_cwd_for_workspace(&w.workspace_id)?;
        let resolved = boundary
            .validate_existing(std::path::Path::new(&anchor))
            .ok()?;
        Some((
            w.workspace_id.clone(),
            resolved.to_string_lossy().into_owned(),
        ))
    });
    if by_anchor.is_some() {
        return by_anchor;
    }

    // D2: the first workspace holding an in-boundary pane, and that pane's own
    // folder. The snapshot's order is the order — picking a *better* pane
    // (least busy, most recent, matching a feature) is ranking, a different
    // decision that nothing here needs yet.
    snapshot
        .panes
        .iter()
        .find_map(|pane| Some((pane.workspace_id.clone(), pane_folder(pane, boundary)?)))
}

/// One pane's folder, validated against `boundary` — `cwd` first, then
/// `foreground_cwd` (D4).
///
/// Deliberately the same two steps `server.rs`'s `project_panes` takes to
/// decide which project a pane belongs to, so "where is this pane" has one
/// answer: a pane this resolver would spawn beside is exactly a pane
/// `ask_state` would list for that project. Two readers of that question
/// would drift, and the drift would read as a project whose panes are
/// visible but whose dispatch refuses.
fn pane_folder(pane: &herdr::wire::Pane, boundary: &Boundary) -> Option<String> {
    pane.cwd
        .as_deref()
        .and_then(|raw| boundary.validate_existing(std::path::Path::new(raw)).ok())
        .or_else(|| {
            pane.foreground_cwd
                .as_deref()
                .and_then(|raw| boundary.validate_existing(std::path::Path::new(raw)).ok())
        })
        .map(|p| p.to_string_lossy().into_owned())
}

/// Clamp a caller-requested await timeout to [`MAX_AWAIT_TIMEOUT`] (D4) — a
/// longer request is silently capped, never honored, never an error.
fn clamp_timeout(timeout: Duration) -> Duration {
    timeout.min(MAX_AWAIT_TIMEOUT)
}

/// A run's terminal-for-this-call status, as `await_run` determines it.
/// [`RunStatus::as_str`] is exactly what gets written through
/// `Engine::update_run_status` (`Run::status`'s own doc names the same
/// vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Still going: no fresh marker, not blocked, and (when status is
    /// `Unknown`) content hasn't settled yet. Also the timeout case for a
    /// pane with a trustworthy non-`Unknown` status — the wait's own
    /// deadline elapsed with the pane still reading as working, so the run
    /// stays open for a later `await_run` call.
    Working,
    /// A fresh marker was found, or (status `Unknown`) the screen settled
    /// for `STABILITY_READS` consecutive polls.
    Done,
    /// The pane's own `AgentStatus` reports `Blocked` — waiting on a human,
    /// never resolved by more polling.
    Blocked,
    /// The wait's own deadline elapsed while the pane's status stayed
    /// `Unknown` (or missing from the snapshot entirely) and never settled
    /// — distinct from `Working`'s timeout case: here there was never a
    /// trustworthy signal to fall back on at all, not even "known still
    /// working".
    Timeout,
    /// board-run-reaper D2 (`4047ca75`): the run's pane is gone from the
    /// herdr snapshot entirely, so there is nothing left to poll and
    /// nothing left to read. Terminal, and deliberately **row-only** — a
    /// vanished pane has no process to protect and no screen to store, so
    /// this status is written by `Engine::update_run_status` alone and
    /// never travels through [`finish`], which would try to close a pane
    /// that no longer exists. `await_run` never returns it either: it is
    /// the reaper's verdict about a pane's absence, not a wait's outcome.
    Lost,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Working => "working",
            RunStatus::Done => "done",
            RunStatus::Blocked => "blocked",
            RunStatus::Timeout => "timeout",
            RunStatus::Lost => "lost",
        }
    }

    /// Whether this status ends the run. `Working` is the one open
    /// status — its own doc says the run "stays open for a later
    /// `await_run` call" — and the store agrees: the per-feature run lock
    /// holds exactly the `working` rows and calls every other status
    /// terminal.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, RunStatus::Working)
    }

    /// The terminal status a stored `Run::status` string names, or `None`
    /// when the row is still open.
    ///
    /// Derived from [`RunStatus::as_str`] so the two spellings cannot
    /// drift, and deliberately a whitelist: `working` reads as open by
    /// name, and so does any other string — a `pending` row, or a status a
    /// future build wrote that this one does not know. An unrecognized
    /// value costs one poll of a live pane, never a wrong answer from the
    /// ledger.
    pub fn terminal_from_stored(status: &str) -> Option<RunStatus> {
        [
            RunStatus::Done,
            RunStatus::Blocked,
            RunStatus::Timeout,
            RunStatus::Lost,
        ]
        .into_iter()
        .find(|s| s.as_str() == status)
    }
}

/// `await_run`'s result: the terminal-for-this-call status plus the
/// transcript delta versus the run's own baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwaitOutcome {
    pub status: RunStatus,
    pub delta: String,
}

/// Bounded poll for `run`'s completion (D4/D5): `timeout` is clamped to
/// [`MAX_AWAIT_TIMEOUT`] and the loop never waits past its own deadline
/// regardless of poll cadence. Status-preferred: a `Blocked` pane returns
/// immediately; a fresh marker (present in a current `Recent` read, absent
/// from `run.baseline` — the staleness check that keeps a marker already
/// sitting in the baseline from ever counting) returns `Done` regardless of
/// status; an `Unknown`-status pane with no fresh marker falls back to
/// content stability, `STABILITY_READS` consecutive unchanged
/// `ansi::revision_of` reads. Every terminal-for-this-call transition is
/// persisted through `Engine::update_run_status` (cell-1's repository
/// methods) before returning, so a restarted orchestrator recovers the
/// fleet by reading run state (D7) instead of needing this call to have
/// been the one that saw it.
/// `notify_store: None` runs the same poll/persist loop with no alert path
/// configured -- a caller with no notification store simply raises nothing
/// (dbn-2's own contract).
pub async fn await_run(
    herdr: &dyn Herdr,
    engine: &Engine,
    run: &Run,
    timeout: Duration,
    notify_store: Option<&NotifyStore>,
) -> Result<AwaitOutcome, OrchestrateError> {
    await_run_with_poll_interval(
        herdr,
        engine,
        run,
        timeout,
        AWAIT_POLL_INTERVAL,
        notify_store,
    )
    .await
}

/// `await_run`'s real loop, parameterized on poll cadence — production
/// always goes through `await_run` and gets `AWAIT_POLL_INTERVAL`; tests
/// substitute a millisecond-scale interval so the loop's real timeout/
/// stability/marker logic runs against `FakeHerdr` without spending real
/// wall-clock time up to the 60s cap (same test-seam shape as
/// `SocketHerdr::with_settle_durations_for_test`).
async fn await_run_with_poll_interval(
    herdr: &dyn Herdr,
    engine: &Engine,
    run: &Run,
    timeout: Duration,
    poll_interval: Duration,
    notify_store: Option<&NotifyStore>,
) -> Result<AwaitOutcome, OrchestrateError> {
    // A run the ledger already records as finished is answered from the
    // ledger (P2-4). Once a finished run's pane can be closed, re-reading
    // it would turn a repeat await into a propagated pane error, and even
    // an open pane has moved on since -- so the status and the transcript
    // `finish` stored are the honest answer. Nothing on this path touches
    // herdr at all: no snapshot, no `read_pane`.
    if let Some(status) = RunStatus::terminal_from_stored(&run.status) {
        let delta = engine.run_final_transcript(&run.id)?.unwrap_or_default();
        return Ok(AwaitOutcome { status, delta });
    }

    let deadline = tokio::time::Instant::now() + clamp_timeout(timeout);
    let mut stable_reads: u32 = 0;
    let mut last_revision: Option<u64> = None;

    loop {
        let snapshot = herdr.snapshot().await?;
        let status = snapshot
            .agents
            .iter()
            .find(|a| a.pane_id == run.pane_id)
            .map(|a| a.status);
        let read = herdr
            .read_pane(&run.pane_id, ReadSource::Recent, RECENT_LINES_CAP)
            .await?;
        let delta = delta_from_baseline(&run.baseline, &read.text);

        if status == Some(AgentStatus::Blocked) {
            return finish(
                herdr,
                engine,
                run,
                RunStatus::Blocked,
                Completion::Observed,
                delta,
                notify_store,
            )
            .await;
        }

        if marker_is_fresh(&run.baseline, &read.text, run.marker.as_str()) {
            // The one declared completion in this loop: the agent's own
            // marker, freshly printed by the agent itself.
            return finish(
                herdr,
                engine,
                run,
                RunStatus::Done,
                Completion::Declared,
                delta,
                notify_store,
            )
            .await;
        }

        if status == Some(AgentStatus::Unknown) || status.is_none() {
            let revision = ansi::revision_of(&read.text);
            if last_revision == Some(revision) {
                stable_reads += 1;
            } else {
                last_revision = Some(revision);
                stable_reads = 1;
            }
            if stable_reads >= STABILITY_READS {
                // `Done` by inference, from a screen that stopped moving --
                // equally true of an agent paused on a tool call, so this
                // one never closes a pane (D2).
                return finish(
                    herdr,
                    engine,
                    run,
                    RunStatus::Done,
                    Completion::Observed,
                    delta,
                    notify_store,
                )
                .await;
            }
        } else {
            stable_reads = 0;
            last_revision = None;
        }

        let now = tokio::time::Instant::now();
        if now >= deadline {
            let timed_out_status = if status.is_none() || status == Some(AgentStatus::Unknown) {
                RunStatus::Timeout
            } else {
                RunStatus::Working
            };
            return finish(
                herdr,
                engine,
                run,
                timed_out_status,
                Completion::Observed,
                delta,
                notify_store,
            )
            .await;
        }
        let remaining = deadline.saturating_duration_since(now);
        tokio::time::sleep(poll_interval.min(remaining)).await;
    }
}

/// How this run's completion was learned -- the ONE thing the pane-close
/// guard is allowed to read about it.
///
/// D2: completion is an explicit declaration, never an inferred state. A
/// pane's observed state cannot tell a finished agent from one working
/// quietly in the background, so only the agent's own word may retire its
/// pane. This is a WHITELIST of one producer, deliberately not a blacklist
/// of bad states: a future `RunStatus`, or a second code path that decides
/// a run is `Done`, gets `Observed` by default and cannot silently acquire
/// the right to kill an agent process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Completion {
    /// The agent itself printed `HERDR_DONE_<nonce>` into its pane -- it
    /// said it was finished, in its own output. The only value that may
    /// close a pane.
    Declared,
    /// Everything else waggledance concluded by looking: the content
    /// stability fallback, the blocked-status read, the await deadline.
    /// Every one of these can be true of an agent that is still working.
    Observed,
}

/// Persist `run`'s terminal-for-this-call status transition (D7) and, when
/// the status is one a human must clear (D1) and a notification store is
/// configured, enqueue exactly one run-aware alert through dbn-1's
/// `enqueue_run_notification` -- the body names only project, pane and run
/// id (D4), and the store's own `(run_id, kind)` uniqueness constraint
/// makes a repeat enqueue for an already-notified status a no-op (D5).
/// Nothing is sent from here: the alert lands in the outbox and the
/// existing drain delivers it, so the opt-in switch (D6) keeps governing
/// delivery untouched. `notify_store: None` still persists the status --
/// it just raises nothing.
///
/// Last, and only under all three guards below, the run's pane is closed
/// (D1 -- close the pane on completion; defect B: every spawn-dispatch
/// used to leak a live agent process):
///
/// 1. `completion` is [`Completion::Declared`] -- the agent printed its own
///    marker. NOT "status == Done": the content-stability fallback in
///    `await_run_with_poll_interval` also returns `Done`, after ~1.5s of
///    static screen for a pane whose agent status is `Unknown` or missing
///    entirely, which is equally true of an agent paused on a tool call.
/// 2. `run.preset_label.is_some()` -- waggledance spawned this pane.
///    `DispatchTarget::Pane` dispatches into a pre-existing pane the user
///    owns and leaves `preset_label` `None` exactly there, and D1's
///    rationale reaches only what waggledance made.
/// 3. The final transcript was stored -- otherwise closing the pane would
///    destroy the only remaining record of what the run did.
///
/// Nothing else is read: no `agent_status`, no pane liveness, no screen
/// stability. The honest cost is that a run finishing without printing its
/// marker keeps its pane -- the leak is narrowed, not eliminated, which is
/// the correct trade when the alternative risks killing a working agent.
/// A close failure never changes the run's status: the work finished, the
/// pane is bookkeeping.
async fn finish(
    herdr: &dyn Herdr,
    engine: &Engine,
    run: &Run,
    status: RunStatus,
    completion: Completion,
    delta: String,
    notify_store: Option<&NotifyStore>,
) -> Result<AwaitOutcome, OrchestrateError> {
    // Transcript first, status second: a row that reads terminal then
    // always carries the transcript it was capped with, which is exactly
    // what the short-circuit above hands a second await. A failed write
    // leaves the run open and re-awaitable rather than finished with
    // nothing to show. A still-`Working` run has no final delta to store
    // -- its screen is not final -- so the column staying NULL is what
    // makes "has a transcript" mean "is over".
    let transcript_stored = if status.is_terminal() {
        engine.set_run_final_transcript(&run.id, &delta)?;
        true
    } else {
        false
    };
    engine.update_run_status(&run.id, status.as_str(), &now_rfc3339(), None, None)?;
    if let Some(store) = notify_store {
        if notify::is_run_notifiable(status) {
            let body = format!("{} {} {}", run.project_id, run.pane_id, run.id);
            if let Err(e) = store.enqueue_run_notification(
                &run.id,
                &run.project_id,
                &run.pane_id,
                status.as_str(),
                &body,
            ) {
                tracing::warn!("failed to enqueue run notification for {}: {e}", run.id);
            }
        }
    }
    if completion == Completion::Declared && run.preset_label.is_some() && transcript_stored {
        if let Err(e) = herdr.close_pane(&run.pane_id).await {
            // The run is over either way -- a pane that outlives it costs
            // machine performance, and reporting the run as anything but
            // finished would cost the result the agent already produced.
            tracing::warn!("failed to close pane {} for run {}: {e}", run.pane_id, run.id);
        }
    }
    Ok(AwaitOutcome { status, delta })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;
    use waggledance_core::{Config, SqliteStore};

    fn test_engine() -> Engine {
        Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default())
    }

    fn build_run(id: &str, pane_id: &str, baseline: &str, marker: &str) -> Run {
        let now = now_rfc3339();
        Run {
            id: id.to_string(),
            project_id: "proj-1".to_string(),
            pane_id: pane_id.to_string(),
            preset_label: None,
            task: "do the thing".to_string(),
            baseline: baseline.to_string(),
            marker: marker.to_string(),
            status: "working".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn preflight_refuses_a_working_pane() {
        let herdr = FakeHerdr::new();
        // w1:p1 is seeded Working (see FakeHerdr::new's doc).
        let err = preflight(&herdr, "w1:p1").await.unwrap_err();
        assert_eq!(err, DispatchRefusal::Working("w1:p1".to_string()));
    }

    #[tokio::test]
    async fn preflight_refuses_a_blocked_pane() {
        let herdr = FakeHerdr::new();
        // w1:p2 is seeded Blocked.
        let err = preflight(&herdr, "w1:p2").await.unwrap_err();
        assert_eq!(err, DispatchRefusal::Blocked("w1:p2".to_string()));
    }

    #[tokio::test]
    async fn preflight_refuses_when_herdr_is_unavailable() {
        let herdr = FakeHerdr::new();
        herdr.set_available(false);
        let err = preflight(&herdr, "w1:p1").await.unwrap_err();
        match err {
            DispatchRefusal::Unverifiable { pane_id, .. } => assert_eq!(pane_id, "w1:p1"),
            other => panic!("expected Unverifiable, got {other:?}"),
        }
    }

    /// What `agent.prompt` answered, scripted per test. `FakeHerdr` alone
    /// cannot express two of these live: it has no clock, so it never
    /// returns the daemon's `timeout`, and it starts every spawned agent
    /// `Idle` — a state `SUBMIT_UNTIL` accepts — so a spawn-path stall or
    /// block is unreachable through it.
    #[derive(Clone, Copy)]
    enum SubmitOutcome {
        /// A matching state was observed: an ordinary good send.
        Accepted,
        /// The text went in and the agent never visibly reacted.
        Stalled,
        /// Refused BEFORE any input was sent.
        Blocked,
        /// A state change WAS observed, then the budget ran out. The text
        /// landed, which is all dispatch asks.
        TimedOutAfterChange,
    }

    /// A `Herdr` that delegates every method to a real [`FakeHerdr`] except
    /// `agent_prompt`, which answers a scripted [`SubmitOutcome`] and
    /// records the call. The recording is what lets a test assert the
    /// dispatch send went through `agent_prompt` — and went through it
    /// exactly once, since a stall must never be retried — instead of
    /// assuming it from the source.
    struct ScriptedSubmit {
        inner: FakeHerdr,
        outcome: SubmitOutcome,
        prompts: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl ScriptedSubmit {
        fn new(outcome: SubmitOutcome) -> Self {
            Self {
                inner: FakeHerdr::new(),
                outcome,
                prompts: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn prompts(&self) -> Vec<(String, String)> {
            self.prompts.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl Herdr for ScriptedSubmit {
        async fn snapshot(&self) -> herdr::Result<herdr::Snapshot> {
            self.inner.snapshot().await
        }
        async fn ping(&self) -> herdr::Result<herdr::ProtocolInfo> {
            self.inner.ping().await
        }
        async fn read_pane(
            &self,
            pane_id: &str,
            source: ReadSource,
            lines: usize,
        ) -> herdr::Result<herdr::ScreenRead> {
            self.inner.read_pane(pane_id, source, lines).await
        }
        async fn send_input(&self, pane_id: &str, text: &str, submit: bool) -> herdr::Result<()> {
            self.inner.send_input(pane_id, text, submit).await
        }
        async fn agent_prompt(
            &self,
            pane_id: &str,
            text: &str,
            until: &[AgentStatus],
            timeout_ms: u64,
        ) -> herdr::Result<AgentStatus> {
            self.prompts
                .lock()
                .unwrap()
                .push((pane_id.to_string(), text.to_string()));
            assert_eq!(
                until,
                SUBMIT_UNTIL,
                "the dispatch send must wait on every state that proves the text landed"
            );
            assert_eq!(
                timeout_ms, SUBMIT_TIMEOUT_MS,
                "the budget must stay above the daemon's own change-detection window, \
                 or a stall comes back disguised as a timeout"
            );
            match self.outcome {
                // Blocked is the one outcome that withholds the input.
                SubmitOutcome::Blocked => Err(HerdrError::AgentBlocked(format!(
                    "agent on {pane_id} is blocked"
                ))),
                other => {
                    self.inner.send_input(pane_id, text, true).await?;
                    match other {
                        SubmitOutcome::Accepted => Ok(AgentStatus::Working),
                        SubmitOutcome::Stalled => Err(HerdrError::AgentPromptStalled(format!(
                            "no state change observed on {pane_id}"
                        ))),
                        SubmitOutcome::TimedOutAfterChange => Err(HerdrError::Timeout(
                            "state changed, then the budget ran out".to_string(),
                        )),
                        SubmitOutcome::Blocked => unreachable!("handled above"),
                    }
                }
            }
        }
        async fn agent_wait(
            &self,
            pane_id: &str,
            until: &[AgentStatus],
            timeout_ms: u64,
        ) -> herdr::Result<AgentStatus> {
            self.inner.agent_wait(pane_id, until, timeout_ms).await
        }
        async fn send_text(&self, pane_id: &str, bytes: &str) -> herdr::Result<()> {
            self.inner.send_text(pane_id, bytes).await
        }
        async fn send_keys(&self, pane_id: &str, keys: &[String]) -> herdr::Result<()> {
            self.inner.send_keys(pane_id, keys).await
        }
        async fn tab_create(
            &self,
            workspace_id: &str,
            cwd: Option<&str>,
        ) -> herdr::Result<herdr::TabCreated> {
            self.inner.tab_create(workspace_id, cwd).await
        }
        async fn agent_start(
            &self,
            pane_id: &str,
            argv: &[String],
        ) -> herdr::Result<herdr::AgentStarted> {
            self.inner.agent_start(pane_id, argv).await
        }
        async fn close_pane(&self, pane_id: &str) -> herdr::Result<()> {
            self.inner.close_pane(pane_id).await
        }
    }

    fn test_project(root: &std::path::Path) -> waggledance_core::domain::Project {
        let now = now_rfc3339();
        waggledance_core::domain::Project {
            id: "proj-1".to_string(),
            name: "proj".to_string(),
            root_path: root.to_path_buf(),
            created_at: now.clone(),
            last_seen_at: now,
            orchestration_enabled: true,
        }
    }

    /// A directory that exists on disk, because `Boundary` resolves against
    /// the real filesystem — a spawn destination cannot be invented.
    fn temp_root(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "waggledance-orchestrate-{tag}-{}-{:016x}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// Point the fake's own w2 panes at a real directory so a `Spawn`
    /// dispatch has a destination that survives boundary validation, and
    /// hand back the project rooted there.
    async fn spawnable_project(
        herdr: &ScriptedSubmit,
        root: &std::path::Path,
    ) -> waggledance_core::domain::Project {
        let dir = root.to_string_lossy().into_owned();
        for pane in ["w1:p1", "w2:p3", "w2:p4", "w2:p5"] {
            herdr
                .inner
                .set_pane_dirs(pane, Some(&dir), Some(&dir))
                .await
                .unwrap();
        }
        test_project(root)
    }

    /// The defect this feature exists for: the task used to go out as
    /// `send_input(.., submit = true)`, a blind Enter behind a 1500ms settle
    /// heuristic that reported nothing back. It goes through `agent_prompt`
    /// now, so a submit the agent never reacted to is an ERROR the caller
    /// can see rather than a run that hangs forever.
    #[tokio::test]
    async fn send_task_reports_a_stall_instead_of_firing_a_blind_enter() {
        let herdr = ScriptedSubmit::new(SubmitOutcome::Stalled);
        let marker = mint_marker();
        let err = send_task(&herdr, "w2:p4", "do the thing", &marker)
            .await
            .unwrap_err();
        assert!(
            matches!(err, HerdrError::AgentPromptStalled(_)),
            "a stall must surface as a stall, not as a silent success: {err:?}"
        );
        let prompts = herdr.prompts();
        assert_eq!(prompts.len(), 1, "the send goes through agent.prompt, once");
        assert_eq!(prompts[0].0, "w2:p4");
        assert!(
            prompts[0].1.contains("do the thing") && prompts[0].1.contains(&marker.suffix),
            "task and marker instruction travel as one submitted reply"
        );
    }

    #[tokio::test]
    async fn dispatch_run_into_a_pane_records_the_run_when_the_submit_is_confirmed() {
        let herdr = ScriptedSubmit::new(SubmitOutcome::Accepted);
        let engine = test_engine();
        let root = temp_root("dispatch-pane");
        let project = test_project(&root);

        // w2:p4 is seeded Idle -- a legal send target.
        let dispatched = dispatch_run(
            &herdr,
            &engine,
            &project,
            DispatchTarget::Pane("w2:p4".to_string()),
            "do the thing",
            None,
            None,
        )
        .await
        .expect("a confirmed submit dispatches");

        assert_eq!(dispatched.run.pane_id, "w2:p4");
        assert_eq!(dispatched.run.status, "working");
        assert_eq!(
            herdr.prompts().len(),
            1,
            "the pane target sends through agent.prompt"
        );
        let stored = engine.get_run(&dispatched.run.id).unwrap();
        assert!(stored.is_some(), "a dispatched run is persisted");
        std::fs::remove_dir_all(&root).ok();
    }

    /// The refusal that keeps a wedged dispatch from poisoning the store: a
    /// stalled submit must leave NOTHING behind, or a `working` row nobody
    /// can complete holds a feature's run lock forever.
    #[tokio::test]
    async fn dispatch_run_refuses_a_stalled_submit_and_stores_no_run() {
        let herdr = ScriptedSubmit::new(SubmitOutcome::Stalled);
        let engine = test_engine();
        let root = temp_root("dispatch-stalled");
        let project = test_project(&root);

        let refusal = dispatch_run(
            &herdr,
            &engine,
            &project,
            DispatchTarget::Pane("w2:p4".to_string()),
            "do the thing",
            None,
            None,
        )
        .await
        .expect_err("a stalled submit must refuse the dispatch");

        match &refusal {
            DispatchRefusal::SendFailed(msg) => assert!(
                msg.contains("stalled"),
                "the refusal must name the stall, not just 'send failed': {msg}"
            ),
            other => panic!("expected SendFailed, got {other:?}"),
        }
        assert!(
            engine.list_runs(&project.id, 10).unwrap().is_empty(),
            "a refused send must insert no run row"
        );
        assert_eq!(
            herdr.prompts().len(),
            1,
            "a stall is never retried -- the text is already in the composer"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A `timeout` is not a failed send: the daemon only reports it once it
    /// has already observed a state change, so the text landed. Dispatch
    /// asks nothing more than that.
    #[tokio::test]
    async fn dispatch_run_treats_a_timeout_after_an_observed_change_as_a_good_send() {
        let herdr = ScriptedSubmit::new(SubmitOutcome::TimedOutAfterChange);
        let engine = test_engine();
        let root = temp_root("dispatch-timeout");
        let project = test_project(&root);

        let dispatched = dispatch_run(
            &herdr,
            &engine,
            &project,
            DispatchTarget::Pane("w2:p4".to_string()),
            "do the thing",
            None,
            None,
        )
        .await
        .expect("a timeout after an observed change still means the text went in");

        assert_eq!(dispatched.run.status, "working");
        assert!(engine.get_run(&dispatched.run.id).unwrap().is_some());
        std::fs::remove_dir_all(&root).ok();
    }

    /// The spawn target takes the same confirming send as the pane target —
    /// the two branches meet at one `send_task` call, and neither is allowed
    /// its own blind Enter.
    #[tokio::test]
    async fn dispatch_run_into_a_spawned_pane_sends_through_the_confirming_submit() {
        let herdr = ScriptedSubmit::new(SubmitOutcome::Accepted);
        let engine = test_engine();
        let root = temp_root("dispatch-spawn");
        let project = spawnable_project(&herdr, &root).await;

        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: vec!["claude".to_string()],
            env: Vec::new(),
            workspace_trust: None,
        };
        let dispatched = dispatch_run(
            &herdr,
            &engine,
            &project,
            DispatchTarget::Spawn {
                entry,
                cwd: Some(root.to_string_lossy().into_owned()),
            },
            "do the thing",
            None,
            None,
        )
        .await
        .expect("a spawn dispatch with a confirmed submit returns a run");

        let prompts = herdr.prompts();
        assert_eq!(
            prompts.len(),
            1,
            "the spawn target sends through agent.prompt too"
        );
        assert_eq!(
            prompts[0].0, dispatched.run.pane_id,
            "the prompt goes to the pane the run records"
        );
        assert!(engine.get_run(&dispatched.run.id).unwrap().is_some());
        std::fs::remove_dir_all(&root).ok();
    }

    /// `agent.prompt` refuses a blocked agent before sending anything. The
    /// spawn path is where dispatch can meet one — a `Pane` target was
    /// already refused by `preflight` — and it must refuse rather than fall
    /// back to typing into a pane that is waiting on a human.
    #[tokio::test]
    async fn dispatch_run_refuses_a_blocked_submit_and_stores_no_run() {
        let herdr = ScriptedSubmit::new(SubmitOutcome::Blocked);
        let engine = test_engine();
        let root = temp_root("dispatch-blocked");
        let project = spawnable_project(&herdr, &root).await;

        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: vec!["claude".to_string()],
            env: Vec::new(),
            workspace_trust: None,
        };
        let refusal = dispatch_run(
            &herdr,
            &engine,
            &project,
            DispatchTarget::Spawn {
                entry,
                cwd: Some(root.to_string_lossy().into_owned()),
            },
            "do the thing",
            None,
            None,
        )
        .await
        .expect_err("a blocked agent must refuse the dispatch");

        match &refusal {
            DispatchRefusal::SendFailed(msg) => assert!(
                msg.contains("blocked"),
                "the refusal must name the block: {msg}"
            ),
            other => panic!("expected SendFailed, got {other:?}"),
        }
        assert!(
            engine.list_runs(&project.id, 10).unwrap().is_empty(),
            "a refused send must insert no run row"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn await_run_completes_on_a_fresh_marker() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        // w2:p4 is seeded Idle -- a legal send target.
        let pane = "w2:p4";
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        let run = build_run("run-fresh", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        send_task(&herdr, pane, &run.task, &marker).await.unwrap();
        // The agent's own later output prints the joined marker -- the only
        // way the joined string reaches the pane in this test.
        herdr
            .send_input(pane, &marker.joined(), false)
            .await
            .unwrap();

        let store = NotifyStore::open_in_memory().unwrap();
        let outcome = await_run(&herdr, &engine, &run, Duration::from_secs(5), Some(&store))
            .await
            .unwrap();
        assert_eq!(outcome.status, RunStatus::Done);
        assert!(
            outcome.delta.contains(&marker.joined()),
            "delta must carry the marker that proved completion: {:?}",
            outcome.delta
        );

        let stored = engine.get_run(&run.id).unwrap().unwrap();
        assert_eq!(
            stored.status, "done",
            "the transition must persist through the cell-1 repository methods"
        );
        assert!(
            store.undelivered().unwrap().is_empty(),
            "Done never notifies (D1)"
        );
    }

    #[tokio::test]
    async fn await_run_ignores_a_marker_already_present_in_the_baseline() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4"; // Idle -- never transitions to Blocked in this test.
        let marker = mint_marker();
        let joined = marker.joined();
        // The baseline itself already carries the joined marker -- a stale
        // sighting, never fresh -- and the pane's current content matches it
        // exactly (seeded to the same text), so the only way `await_run`
        // could ever call this Done is by skipping the staleness guard.
        let baseline = format!("earlier output\n{joined}\nmore earlier output");
        herdr.seed_scroll_pane(pane, &baseline, &baseline, None);
        let run = build_run("run-stale", pane, &baseline, &joined);
        engine.insert_run(&run, None).unwrap();

        let outcome = await_run_with_poll_interval(
            &herdr,
            &engine,
            &run,
            Duration::from_millis(20),
            Duration::from_millis(5),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            outcome.status,
            RunStatus::Working,
            "a marker already sitting in the baseline must never count as completion"
        );
    }

    #[tokio::test]
    async fn await_run_times_out_while_working() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        // w1:p1 is seeded Working and never transitions in this test.
        let pane = "w1:p1";
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker(); // never sent/printed -- no completion signal.
        let run = build_run("run-timeout", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        let store = NotifyStore::open_in_memory().unwrap();
        let outcome = await_run_with_poll_interval(
            &herdr,
            &engine,
            &run,
            Duration::from_millis(15),
            Duration::from_millis(5),
            Some(&store),
        )
        .await
        .unwrap();
        assert_eq!(
            outcome.status,
            RunStatus::Working,
            "a timeout on a known-Working pane must report Working, and the run stays open"
        );

        let stored = engine.get_run(&run.id).unwrap().unwrap();
        assert_eq!(stored.status, "working");
        assert!(
            store.undelivered().unwrap().is_empty(),
            "Working never notifies (D1)"
        );
        assert_eq!(
            engine.run_final_transcript(&run.id).unwrap(),
            None,
            "a run that stays open has no FINAL transcript -- the column filling in \
             is what marks the run over"
        );
    }

    /// Every status that ends a run leaves its transcript on the row, so the
    /// answer outlives the pane it was read from.
    #[tokio::test]
    async fn finish_stores_the_final_transcript_on_a_terminal_status() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4"; // seeded Idle -- a legal send target.
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        let run = build_run("run-transcript", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        send_task(&herdr, pane, &run.task, &marker).await.unwrap();
        herdr
            .send_input(pane, &marker.joined(), false)
            .await
            .unwrap();

        let outcome = await_run(&herdr, &engine, &run, Duration::from_secs(5), None)
            .await
            .unwrap();
        assert_eq!(outcome.status, RunStatus::Done);
        assert_eq!(
            engine.run_final_transcript(&run.id).unwrap().as_deref(),
            Some(outcome.delta.as_str()),
            "the delta the caller was handed is the delta the store keeps"
        );
    }

    /// P2-4: a second await on a run the ledger already records as finished
    /// answers from the store. `set_available(false)` makes EVERY herdr call
    /// -- `snapshot` and `read_pane` alike -- return an error, so an `Ok`
    /// here can only mean zero pane reads were attempted. That is what keeps
    /// a re-await honest once the run's pane has been closed.
    #[tokio::test]
    async fn await_run_answers_a_finished_run_from_the_store_without_reading_the_pane() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let mut run = build_run("run-reawait", "w2:p4", "baseline", "HERDR_DONE_x");
        run.status = "done".into();
        engine.insert_run(&run, None).unwrap();
        engine
            .set_run_final_transcript(&run.id, "all the work\nHERDR_DONE_x")
            .unwrap();

        herdr.set_available(false);
        let outcome = await_run(&herdr, &engine, &run, Duration::from_secs(5), None)
            .await
            .expect("a finished run must answer without touching herdr at all");
        assert_eq!(outcome.status, RunStatus::Done);
        assert_eq!(outcome.delta, "all the work\nHERDR_DONE_x");
    }

    /// The short-circuit reads the four terminal spellings and nothing
    /// else: `working` is open by name, and so is any status this build does
    /// not know -- an unrecognized value costs a poll, never a wrong answer.
    /// `lost` (board-run-reaper D2) is the fourth: a reaped run whose pane
    /// vanished answers from the store like any other finished run, so a
    /// later await never goes looking for the pane again.
    #[test]
    fn only_the_terminal_statuses_answer_from_the_store() {
        assert_eq!(
            RunStatus::terminal_from_stored("done"),
            Some(RunStatus::Done)
        );
        assert_eq!(
            RunStatus::terminal_from_stored("blocked"),
            Some(RunStatus::Blocked)
        );
        assert_eq!(
            RunStatus::terminal_from_stored("timeout"),
            Some(RunStatus::Timeout)
        );
        assert_eq!(
            RunStatus::terminal_from_stored("lost"),
            Some(RunStatus::Lost),
            "board-run-reaper D2: a reaped row reads terminal from the store"
        );
        assert_eq!(RunStatus::terminal_from_stored("working"), None);
        assert_eq!(RunStatus::terminal_from_stored("pending"), None);
        assert_eq!(RunStatus::terminal_from_stored("Done"), None);
        assert_eq!(RunStatus::Lost.as_str(), "lost");
        assert!(!RunStatus::Working.is_terminal());
        for status in [
            RunStatus::Done,
            RunStatus::Blocked,
            RunStatus::Timeout,
            RunStatus::Lost,
        ] {
            assert!(status.is_terminal(), "{status:?} ends the run");
        }
    }

    #[tokio::test]
    async fn await_run_returns_blocked_when_the_pane_blocks_mid_run() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let store = NotifyStore::open_in_memory().unwrap();
        let pane = "w2:p4"; // seeded Idle -- flips to Blocked mid-poll below.
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        let run = build_run("run-blocked", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        let flipper = herdr.clone();
        let flip_pane = pane.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            flipper
                .set_status(&flip_pane, AgentStatus::Blocked)
                .await
                .unwrap();
        });

        let outcome = await_run_with_poll_interval(
            &herdr,
            &engine,
            &run,
            Duration::from_secs(2),
            Duration::from_millis(5),
            Some(&store),
        )
        .await
        .unwrap();
        assert_eq!(outcome.status, RunStatus::Blocked);

        let stored = engine.get_run(&run.id).unwrap().unwrap();
        assert_eq!(stored.status, "blocked");

        let pending = store.undelivered().unwrap();
        assert_eq!(pending.len(), 1, "exactly one pending alert (D5)");
        assert_eq!(pending[0].kind, "blocked");
        assert_eq!(pending[0].run_id.as_deref(), Some(run.id.as_str()));
        assert!(pending[0].body.contains(&run.project_id));
        assert!(pending[0].body.contains(&run.pane_id));
        assert!(pending[0].body.contains(&run.id));
        assert!(
            !pending[0].body.contains(&run.task),
            "alert body must never carry the run's task text (D4): {:?}",
            pending[0].body
        );
    }

    #[tokio::test]
    async fn await_run_repeated_blocked_await_leaves_one_pending_alert() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let store = NotifyStore::open_in_memory().unwrap();
        let pane = "w1:p2"; // seeded Blocked from the start (see FakeHerdr::new's doc).
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        let run = build_run("run-blocked-twice", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        for _ in 0..2 {
            let outcome = await_run_with_poll_interval(
                &herdr,
                &engine,
                &run,
                Duration::from_millis(20),
                Duration::from_millis(5),
                Some(&store),
            )
            .await
            .unwrap();
            assert_eq!(outcome.status, RunStatus::Blocked);
        }

        let pending = store.undelivered().unwrap();
        assert_eq!(
            pending.len(),
            1,
            "D5: a run returning Blocked on two consecutive awaits must still \
             leave exactly one pending alert"
        );
    }

    #[tokio::test]
    async fn await_run_timeout_status_enqueues_exactly_one_alert() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let store = NotifyStore::open_in_memory().unwrap();
        let pane = "w2:p4";
        // Unknown is never a trustworthy signal to fall back on, so a
        // deadline reached before content stabilizes reports Timeout, not
        // Working.
        herdr.set_status(pane, AgentStatus::Unknown).await.unwrap();
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker(); // never printed -- content stays put.
        let run = build_run("run-timeout-alert", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        // poll_interval > timeout so the loop's own `remaining` cap makes
        // the very first sleep land exactly on the deadline -- the second
        // iteration's deadline check fires with stable_reads still under
        // STABILITY_READS, before content could ever settle into a Done.
        let outcome = await_run_with_poll_interval(
            &herdr,
            &engine,
            &run,
            Duration::from_millis(10),
            Duration::from_millis(50),
            Some(&store),
        )
        .await
        .unwrap();
        assert_eq!(outcome.status, RunStatus::Timeout);

        let stored = engine.get_run(&run.id).unwrap().unwrap();
        assert_eq!(stored.status, "timeout");

        let pending = store.undelivered().unwrap();
        assert_eq!(pending.len(), 1, "exactly one pending timeout alert");
        assert_eq!(pending[0].kind, "timeout");
        assert_eq!(pending[0].run_id.as_deref(), Some(run.id.as_str()));
    }

    #[tokio::test]
    async fn await_run_falls_back_to_content_stability_on_unknown_status() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4";
        herdr.set_status(pane, AgentStatus::Unknown).await.unwrap();
        let fixed_text = "a screen that never changes\n❯ ";
        herdr.seed_scroll_pane(pane, fixed_text, fixed_text, None);
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker(); // never printed -- only stability proves completion.
        let run = build_run("run-unknown-stable", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        let outcome = await_run_with_poll_interval(
            &herdr,
            &engine,
            &run,
            Duration::from_millis(200),
            Duration::from_millis(2),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            outcome.status,
            RunStatus::Done,
            "an Unknown-status pane with unchanging content must settle via stability, not time out"
        );

        let stored = engine.get_run(&run.id).unwrap().unwrap();
        assert_eq!(stored.status, "done");
    }

    /// The close guard's own fixture: a run waggledance SPAWNED, which is
    /// what `preset_label: Some(..)` means (a `DispatchTarget::Pane` run
    /// into a pane the user already owns leaves it `None`).
    fn build_spawned_run(id: &str, pane_id: &str, baseline: &str, marker: &str) -> Run {
        Run {
            preset_label: Some("claude".to_string()),
            ..build_run(id, pane_id, baseline, marker)
        }
    }

    /// Print `marker` into `pane` as the agent's own later output -- the
    /// only way the joined marker string reaches a pane in these tests, and
    /// therefore the only thing that can produce a DECLARED completion.
    async fn agent_declares_done(herdr: &FakeHerdr, pane: &str, marker: &str) {
        herdr.send_input(pane, marker, false).await.unwrap();
    }

    /// The whole point of the feature: the agent said it was done, in its
    /// own output, on a pane waggledance made -- so the pane goes away.
    #[tokio::test]
    async fn a_declared_done_on_a_spawned_pane_closes_exactly_that_pane() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4";
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        let run = build_spawned_run("run-close", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();
        agent_declares_done(&herdr, pane, &marker.joined()).await;

        let outcome = await_run(&herdr, &engine, &run, Duration::from_secs(5), None)
            .await
            .unwrap();

        assert_eq!(outcome.status, RunStatus::Done);
        assert_eq!(
            herdr.closed_panes().await,
            vec![pane.to_string()],
            "the agent declared itself done on a pane waggledance spawned -- exactly one close"
        );
        // The close does not cost the run its record: a repeat await still
        // answers from the store (dsr-3), which is why capture lands first.
        assert!(
            engine.run_final_transcript(&run.id).unwrap().is_some(),
            "the transcript must already be stored when the pane is closed"
        );
    }

    /// D2, structurally: the guard reads how completion was LEARNED, never
    /// `RunStatus`. `finish` is called here with `RunStatus::Done` on a
    /// spawned run -- everything the close needs except the agent's own
    /// declaration -- and must close nothing. A future edit that loosens the
    /// guard to "status == Done" fails exactly here.
    #[tokio::test]
    async fn a_done_that_the_agent_never_declared_closes_nothing() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let run = build_spawned_run("run-observed-done", "w2:p4", "base", "HERDR_DONE_x");
        engine.insert_run(&run, None).unwrap();

        let outcome = finish(
            &herdr,
            &engine,
            &run,
            RunStatus::Done,
            Completion::Observed,
            "some output".to_string(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(outcome.status, RunStatus::Done);
        assert!(
            herdr.closed_panes().await.is_empty(),
            "a Done nobody declared is an inference -- it may never close a pane"
        );
    }

    /// The concrete producer the test above generalizes: an `Unknown`-status
    /// pane whose screen simply stopped moving. That is equally true of an
    /// agent paused on a tool call, so its `Done` closes nothing.
    #[tokio::test]
    async fn a_stability_done_closes_nothing() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4";
        herdr.set_status(pane, AgentStatus::Unknown).await.unwrap();
        let fixed_text = "a screen that never changes\n";
        herdr.seed_scroll_pane(pane, fixed_text, fixed_text, None);
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker(); // never printed -- only stability can end this run.
        let run = build_spawned_run("run-stable-done", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();

        let outcome = await_run_with_poll_interval(
            &herdr,
            &engine,
            &run,
            Duration::from_millis(200),
            Duration::from_millis(2),
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            outcome.status,
            RunStatus::Done,
            "stability still ends the run"
        );
        assert!(
            herdr.closed_panes().await.is_empty(),
            "a screen that merely stopped moving must never kill the agent behind it"
        );
    }

    /// `DispatchTarget::Pane` dispatches into a pane the USER owns and leaves
    /// `preset_label` `None` exactly there. Even a declared done leaves it
    /// alone -- D1's rationale reaches only what waggledance made.
    #[tokio::test]
    async fn a_declared_done_on_a_pane_the_user_owns_closes_nothing() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4";
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        // build_run, not build_spawned_run: preset_label stays None.
        let run = build_run("run-user-pane", pane, &baseline, &marker.joined());
        assert!(run.preset_label.is_none(), "the case under test");
        engine.insert_run(&run, None).unwrap();
        agent_declares_done(&herdr, pane, &marker.joined()).await;

        let outcome = await_run(&herdr, &engine, &run, Duration::from_secs(5), None)
            .await
            .unwrap();

        assert_eq!(outcome.status, RunStatus::Done);
        assert!(
            herdr.closed_panes().await.is_empty(),
            "waggledance never made this pane, so it never takes it away"
        );
    }

    /// The three non-`Done` statuses that reach `finish`, each on a spawned
    /// run so `preset_label` cannot be what saves them. `Blocked` matters
    /// most: it is an agent waiting on a human, and the human needs the pane
    /// to answer in.
    #[tokio::test]
    async fn a_run_that_did_not_finish_closes_nothing() {
        for status in [RunStatus::Working, RunStatus::Timeout, RunStatus::Blocked] {
            let herdr = FakeHerdr::new();
            let engine = test_engine();
            let run = build_spawned_run("run-open", "w2:p4", "base", "HERDR_DONE_x");
            engine.insert_run(&run, None).unwrap();

            finish(
                &herdr,
                &engine,
                &run,
                status,
                Completion::Observed,
                "partial output".to_string(),
                None,
            )
            .await
            .unwrap();

            assert!(
                herdr.closed_panes().await.is_empty(),
                "{status:?} is not a completion -- it must close nothing"
            );
        }
    }

    /// A leaked pane costs machine performance; losing the run's own result
    /// costs the work. So a refused close is logged and swallowed -- the run
    /// still reports Done, and the store still carries its answer.
    #[tokio::test]
    async fn a_close_that_fails_still_reports_the_run_as_done() {
        let herdr = FakeHerdr::new();
        let engine = test_engine();
        let pane = "w2:p4";
        herdr.fail_close_pane("herdr refused the close").await;
        let baseline = capture_baseline(&herdr, pane).await.unwrap();
        let marker = mint_marker();
        let run = build_spawned_run("run-close-fails", pane, &baseline, &marker.joined());
        engine.insert_run(&run, None).unwrap();
        agent_declares_done(&herdr, pane, &marker.joined()).await;

        let outcome = await_run(&herdr, &engine, &run, Duration::from_secs(5), None)
            .await
            .unwrap();

        assert_eq!(
            outcome.status,
            RunStatus::Done,
            "the work finished; the pane is bookkeeping"
        );
        assert_eq!(
            herdr.closed_panes().await,
            vec![pane.to_string()],
            "the close was attempted -- it just did not take"
        );
        let stored = engine.get_run(&run.id).unwrap().unwrap();
        assert_eq!(stored.status, "done");
    }

    #[test]
    fn timeout_over_the_hard_cap_is_clamped() {
        assert_eq!(
            clamp_timeout(Duration::from_secs(120)),
            MAX_AWAIT_TIMEOUT,
            "a request over 60s must be clamped, never honored"
        );
        assert_eq!(
            clamp_timeout(Duration::from_secs(30)),
            Duration::from_secs(30),
            "a request under the cap passes through unchanged"
        );
    }

    fn workspace_with_anchor(
        workspace_id: &str,
        cwd: &std::path::Path,
    ) -> (
        herdr::wire::Workspace,
        herdr::wire::PaneLayout,
        herdr::wire::Pane,
    ) {
        let tab_id = format!("{workspace_id}-tab");
        let pane_id = format!("{workspace_id}-pane");
        (
            herdr::wire::Workspace {
                workspace_id: workspace_id.to_string(),
                label: workspace_id.to_string(),
                agent_status: AgentStatus::Idle,
                active_tab_id: Some(tab_id.clone()),
            },
            herdr::wire::PaneLayout {
                workspace_id: workspace_id.to_string(),
                tab_id: tab_id.clone(),
                focused_pane_id: Some(pane_id.clone()),
            },
            herdr::wire::Pane {
                pane_id,
                workspace_id: workspace_id.to_string(),
                tab_id,
                cwd: Some(cwd.to_string_lossy().into_owned()),
                foreground_cwd: None,
            },
        )
    }

    #[test]
    fn resolve_spawn_destination_picks_the_first_workspace_whose_anchor_is_contained() {
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("waggledance-orchestrate-destination-in-{pid}"));
        let elsewhere =
            std::env::temp_dir().join(format!("waggledance-orchestrate-destination-out-{pid}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        let (w_out, l_out, p_out) = workspace_with_anchor("w-outside", &elsewhere);
        let (w_in, l_in, p_in) = workspace_with_anchor("w-inside", &root);
        let snapshot = herdr::Snapshot {
            workspaces: vec![w_out, w_in],
            layouts: vec![l_out, l_in],
            panes: vec![p_out, p_in],
            ..Default::default()
        };

        let (workspace_id, cwd) = resolve_spawn_destination(&snapshot, &boundary)
            .expect("the inside workspace's anchor must resolve");
        assert_eq!(workspace_id, "w-inside");
        assert_eq!(cwd, root.canonicalize().unwrap().to_string_lossy());

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    #[test]
    fn resolve_spawn_destination_is_none_when_no_workspace_anchor_resolves() {
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("waggledance-orchestrate-destination-none-{pid}"));
        let elsewhere = std::env::temp_dir().join(format!(
            "waggledance-orchestrate-destination-none-out-{pid}"
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        let (w_out, l_out, p_out) = workspace_with_anchor("w-only-outside", &elsewhere);
        let snapshot = herdr::Snapshot {
            workspaces: vec![w_out],
            layouts: vec![l_out],
            panes: vec![p_out],
            ..Default::default()
        };

        assert!(resolve_spawn_destination(&snapshot, &boundary).is_none());

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    /// The shape that made this feature: every workspace ANCHOR sits outside
    /// the project, yet the project plainly owns a pane. Observed live on
    /// beehive, whose two agent panes resolve under its own root while the
    /// workspace labelled `beehive` holds another project's panes — the
    /// anchor is only whichever pane a human has focused, so without this
    /// fallback dispatchability moved with the cursor.
    #[test]
    fn resolve_spawn_destination_falls_through_to_a_contained_pane_when_no_anchor_resolves() {
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("waggledance-orchestrate-destination-pane-in-{pid}"));
        let elsewhere = std::env::temp_dir().join(format!(
            "waggledance-orchestrate-destination-pane-out-{pid}"
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        // The only anchored workspace points outside; the in-boundary pane is
        // a plain member of another workspace and is nobody's focus.
        let (w_out, l_out, p_out) = workspace_with_anchor("w-anchor-outside", &elsewhere);
        let stray = herdr::wire::Pane {
            pane_id: "w-other:p7".to_string(),
            workspace_id: "w-other".to_string(),
            tab_id: "w-other-tab".to_string(),
            cwd: Some(root.to_string_lossy().into_owned()),
            foreground_cwd: None,
        };
        let snapshot = herdr::Snapshot {
            workspaces: vec![w_out],
            layouts: vec![l_out],
            panes: vec![p_out, stray],
            ..Default::default()
        };

        let (workspace_id, cwd) = resolve_spawn_destination(&snapshot, &boundary)
            .expect("a pane inside the project is a destination the project owns");
        assert_eq!(workspace_id, "w-other");
        assert_eq!(cwd, root.canonicalize().unwrap().to_string_lossy());

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    /// D4: a pane carrying only `foreground_cwd` is found, because that is
    /// the second step `project_panes` already takes — a pane `ask_state`
    /// lists for a project must not be one this resolver cannot see.
    #[test]
    fn resolve_spawn_destination_reads_foreground_cwd_when_a_pane_has_no_cwd() {
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("waggledance-orchestrate-destination-fg-{pid}"));
        std::fs::create_dir_all(&root).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        let snapshot = herdr::Snapshot {
            panes: vec![herdr::wire::Pane {
                pane_id: "w9:p1".to_string(),
                workspace_id: "w9".to_string(),
                tab_id: "w9-tab".to_string(),
                cwd: None,
                foreground_cwd: Some(root.to_string_lossy().into_owned()),
            }],
            ..Default::default()
        };

        let (workspace_id, cwd) = resolve_spawn_destination(&snapshot, &boundary)
            .expect("foreground_cwd is the documented second step");
        assert_eq!(workspace_id, "w9");
        assert_eq!(cwd, root.canonicalize().unwrap().to_string_lossy());

        std::fs::remove_dir_all(&root).ok();
    }

    /// The fallback widens where a destination may be FOUND, never what
    /// counts as one: a snapshot whose panes all sit outside still resolves
    /// to nothing, so the caller's fail-closed refusal is unchanged.
    #[test]
    fn resolve_spawn_destination_still_refuses_when_no_pane_is_contained_either() {
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("waggledance-orchestrate-destination-nopane-{pid}"));
        let elsewhere = std::env::temp_dir().join(format!(
            "waggledance-orchestrate-destination-nopane-out-{pid}"
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        let snapshot = herdr::Snapshot {
            panes: vec![herdr::wire::Pane {
                pane_id: "w9:p1".to_string(),
                workspace_id: "w9".to_string(),
                tab_id: "w9-tab".to_string(),
                cwd: Some(elsewhere.to_string_lossy().into_owned()),
                foreground_cwd: Some("/definitely/not/here".to_string()),
            }],
            ..Default::default()
        };

        assert!(resolve_spawn_destination(&snapshot, &boundary).is_none());

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    #[test]
    fn verify_pane_in_boundary_accepts_a_pane_whose_cwd_is_inside_the_project() {
        let pid = std::process::id();
        let root = std::env::temp_dir().join(format!("waggledance-orchestrate-verify-in-{pid}"));
        std::fs::create_dir_all(&root).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        let (_w, _l, pane) = workspace_with_anchor("w-inside", &root);
        let pane_id = pane.pane_id.clone();
        let snapshot = herdr::Snapshot {
            panes: vec![pane],
            ..Default::default()
        };

        assert!(verify_pane_in_boundary(&snapshot, &boundary, &pane_id, "proj-1").is_ok());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verify_pane_in_boundary_refuses_a_pane_in_another_project() {
        let pid = std::process::id();
        let root = std::env::temp_dir().join(format!("waggledance-orchestrate-verify-out-{pid}"));
        let elsewhere =
            std::env::temp_dir().join(format!("waggledance-orchestrate-verify-out-other-{pid}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        // The project's boundary is `root`, but the target pane lives in a
        // sibling project's directory -- the exact cross-project dispatch this
        // check closes.
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        let (_w, _l, foreign) = workspace_with_anchor("w-other-project", &elsewhere);
        let foreign_id = foreign.pane_id.clone();
        let snapshot = herdr::Snapshot {
            panes: vec![foreign],
            ..Default::default()
        };

        let err = verify_pane_in_boundary(&snapshot, &boundary, &foreign_id, "proj-1").unwrap_err();
        assert_eq!(
            err,
            DispatchRefusal::OutsideBoundary {
                pane_id: foreign_id,
                project_id: "proj-1".to_string(),
            }
        );

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    #[test]
    fn verify_pane_in_boundary_refuses_a_pane_absent_from_the_snapshot() {
        let pid = std::process::id();
        let root =
            std::env::temp_dir().join(format!("waggledance-orchestrate-verify-absent-{pid}"));
        std::fs::create_dir_all(&root).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        // A pane id that appears nowhere in panes[] (stale, or never existed)
        // refuses the same way a cross-project one does -- never "probably
        // fine".
        let snapshot = herdr::Snapshot::default();
        let err =
            verify_pane_in_boundary(&snapshot, &boundary, "ghost:pane", "proj-1").unwrap_err();
        assert_eq!(
            err,
            DispatchRefusal::OutsideBoundary {
                pane_id: "ghost:pane".to_string(),
                project_id: "proj-1".to_string(),
            }
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verify_pane_in_boundary_falls_back_to_foreground_cwd() {
        let pid = std::process::id();
        let root = std::env::temp_dir().join(format!("waggledance-orchestrate-verify-fg-{pid}"));
        let elsewhere =
            std::env::temp_dir().join(format!("waggledance-orchestrate-verify-fg-out-{pid}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let boundary = Boundary::new(vec![root.clone()]).unwrap();

        // process cwd is outside the project, but the foreground child moved
        // into it -- `project_panes`' own second-chance rule, so containment
        // must honor it too.
        let pane = herdr::wire::Pane {
            pane_id: "w-fg-pane".to_string(),
            workspace_id: "w-fg".to_string(),
            tab_id: "w-fg-tab".to_string(),
            cwd: Some(elsewhere.to_string_lossy().into_owned()),
            foreground_cwd: Some(root.to_string_lossy().into_owned()),
        };
        let snapshot = herdr::Snapshot {
            panes: vec![pane],
            ..Default::default()
        };

        assert!(verify_pane_in_boundary(&snapshot, &boundary, "w-fg-pane", "proj-1").is_ok());

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }
}
