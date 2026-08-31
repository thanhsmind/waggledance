# Observer-Tick Trigger — Context

**Feature slug:** observer-tick-trigger
**Date:** 2026-08-31
**Shaping session:** complete (one gray area — a locked-decision conflict discovered
during scouting — put to the owner directly; answered before this lock)
**Scope:** Standard
**Domain types:** RUN

**Origin:** foreign-origin spec drop — PBI `sup-20260831-b2e1`, provenance
`from waggledance@b3ab7b5`, owner-approved 2026-08-31.
Full request text: `docs/discovery/observer-tick-trigger/spec-drop-sup-20260831-b2e1.md`.
Originating research: `docs/history/research/demonthorn-supervisor-xia.md`.

## Feature Boundary

waggledance gains a new opt-in, event-driven daemon background task (Rust identifier:
`trigger`) that watches fleet-wide run and escalation transitions across every
registered project that has opted into orchestration, and — on a transition, never on
a timer — dispatches exactly one fixed-content cold `bee supervisor` observation tick
into the affected repo through the existing in-process dispatch path. It never judges
what it finds, never writes to any project's `.bee/supervisor/` store, and stays off
by default. The `bee supervisor` half of the same originating research (store, verbs,
frequency cap, WakeReport) was dropped separately into beehive as `sup-20260831-7f3a`
and is out of this feature's scope — this repo does not wait on it.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | **The orchestrator-dispatch D1 exception (decisions log `45a554bb-1832-4243-8a72-6327aec1e215`, touches `10ed8bf4`).** `orchestrator-dispatch` D1 (decisions log `34791df7`, cited by name only — the id no longer resolves in the active decisions store) stays the rule: waggledance never chooses WHAT to dispatch through the MCP surface; an external agent does. A narrow, logged exception is carved for exactly this one daemon task: it may autonomously fire `dispatch_run` with a FIXED, content-invariant task string on a mechanical fleet transition. It never varies task content and never chooses between actions. | `wd-supervisor-seat` D2 (`10ed8bf4`, 2026-08-30) explicitly rejected new daemon code that decides what/when to dispatch, citing D1/D2 as "not asked to reopen." This request asks for exactly that shape; the owner was shown the conflict directly (this session, 2026-08-31) and chose to proceed with a narrow, logged exception rather than a silent reinterpretation or a wholesale reopening. See `docs/history/wd-supervisor-seat/CONTEXT.md`. |
| D2 | **Naming.** New module `crates/waggledance/src/trigger.rs`, struct `Trigger`, config field `terminal.trigger_enabled`, tick counter `trigger_ticks`. The word "supervisor" never appears in any new identifier — `crates/waggledance/src/supervisor.rs` is the unrelated herdr watchdog, and the stack already has three unrelated things named "supervisor". "observer-tick" is the human/PBI-facing name (feature slug, doc titles); "trigger" is the Rust identifier. | Spec constraint #5, verbatim: "ĐỪNG đặt tên nó là supervisor." |
| D3 | **Event-driven, never a timer.** The task dispatches at most once per detected transition, gated through a cursor/dedup mechanism in the same shape as `watcher.rs`'s `StatusCursor`/`ActivityCursor`. An internal poll cadence for cheap freshness reads (herdr snapshot, `.bee/` file reads) is not the anti-pattern being forbidden — `notify`'s own watcher already polls on a fixed tick and stays event-driven in its *output* because a cursor gates what surfaces. What is forbidden is firing a supervisor tick on every poll regardless of state, which is what `bee herding control-loop --role supervisor` already does today and which this feature exists to avoid. | Spec constraint #2 and the source doc's §7.6/§8.10 (sparse, event-driven supervision vs. polling/loop debt). |
| D4 | **Exactly four transition classes fire a tick, and only these:** (a) a run capped — the reaper's own sweep verdict `Lost` or `Awaited(Done \| Timeout)`; (b) a run's pane entering `Blocked` — already detected by `watcher.rs`'s `StatusCursor` (`AgentStatus::Blocked`); (c) a run overrun — a `working` run whose age crosses a fixed threshold, computed by waggledance from its own run ledger, never LLM-judged; (d) a new escalation row appearing in a project's own `.bee/supervisor/interventions.jsonl` (`kind` = `escalation` or `urgent`) — read-only, cursor-based. | Spec constraint #2, verbatim list. "Overrun" is not an existing waggledance concept (no `overrun`/`Overrun` hit anywhere in `crates/`) — it is new and waggledance-computed, distinct from `bee supervisor`'s own `budget-overrun` *signal*, which is model-assigned only after a tick has already run. |
| D5 | **No local observation store.** The only read the trigger ever makes against `.bee/supervisor/` is the interventions cursor in D4(d), and it is read-only. The trigger never calls `bee supervisor record`, `pending`, `mark-delivered`, or any other write verb — every write after a tick fires is the dispatched agent's own responsibility inside the target repo. | Spec constraint #6, verbatim: "waggledance không giữ bản sao nào." |
| D6 | **Opt-in, default OFF**, switch `terminal.trigger_enabled`, gated under `terminal.enabled` exactly like `reaper_enabled`/`notify_enabled`/`supervisor_enabled` — same `(true, Some)/(true, None)/(false, Some)/(false, None)` reconcile shape. This task calls an LLM (spawns a dispatch), so it follows `supervisor_enabled`'s/`notify_enabled`'s off-by-default class, never `reaper_enabled`'s on-by-default class. | Spec constraint #4; `crates/waggledance-core/src/config.rs`'s own doc comment on `reaper_enabled` (read before choosing this) explains why only the reaper earns default-on: it has no external side effect beyond closing a pane the daemon itself already owns. |
| D7 | **Per-project dispatch consent is still required.** The trigger only dispatches into a project whose own `orchestration_enabled` is on. A transition in a project that has not opted in produces no tick — same per-project gate `orchestrator-dispatch` D6 / `dispatch-project-presets` D5 already require for every other dispatch path. | Keeps this feature from being a second, ungated door into a project that declined orchestration through the front one. |
| D8 | **Per-project cooldown between dispatched ticks.** The trigger enforces a minimum spacing between two ticks it dispatches into the SAME project (exact window: planning's call), so a burst of transitions (a flapping run) fires at most one tick per cooldown window, not one per transition. Still event-driven — only the *rate*, not the presence of a timer, is bounded. Cooldown suppresses the DISPATCH only, never the detection: every detector's own cursor/seen-set still advances past a transition it observed, whether or not the cooldown let a tick actually fire — a suppressed transition is dropped, never queued or retried, matching D8's own "not one per transition" wording. | The source xia doc names its own risk if this seat is stood up now: §8.11 ceremony capture / §8.14 attention dilution. A per-transition dispatch with no floor turns one flapping run into an unbounded burst of spawned agents; this decision is the structural answer, not a config knob left to the operator to discover the hard way. |
| D9 | **The trigger never re-observes its own dispatched runs.** Every tick the trigger dispatches carries a fixed, distinguishing `feature` marker on `dispatch_run` (e.g. `"observer-tick-trigger"`); every detector (D4a–D4d) filters out a run/row that already carries this marker before treating it as a transition. | Found during the plan-step hat wave (`hat-user-impact`, 2026-08-31): without this, a dispatched tick's own eventual completion (or stall) is itself a "run capped"/"run overrun" transition in the exact same project, which would wake ANOTHER tick pointed at the first tick's own run — a self-sustaining loop bounded only by D8's cooldown, never terminating. D5 (no local store) means this would be invisible even to an operator looking, since no record exists distinguishing a trigger-dispatched run from any other. |
| D10 | **`terminal.trigger_dry_run` (default `false`).** When true, every detector runs exactly as normal and logs (via `tracing::info!`, not a store — D5 is unaffected) the transition it would have acted on and the dispatch it would have made, but `dispatch_run` is never actually called. Independent of `trigger_enabled` — an operator can dry-run before ever arming real dispatch. | Found during the plan-step hat wave (`hat-alternatives`, 2026-08-31): D1's exception is self-authored and cannot yet be proven safe by anything but production fleet behavior; a dry-run mode lets an operator measure real transition volume (directly answering the risk map's "dispatch storm" row) before arming autonomous LLM dispatch, at near-zero implementation cost, and independently narrows the `hat-user-impact` finding that a fired tick is otherwise invisible outside the target project's own run ledger (D5 still holds — this is a log line, never a store). |

### Accepted risks (found during the hat wave, not blocking)

- **No cross-project push signal.** D5 forbids a local observation store, and this
  feature adds no board/UI surface (out of scope). An operator watching project A gets
  no push when a tick fires in project B — only D10's dry-run log (when on) or the
  target project's own run ledger surface it. Accepted for this feature; a real fix is
  a board/notify surface, deferred (see Deferred Ideas).
- **Consent conflation.** D7 reuses the same `orchestration_enabled` bit that already
  gates human-initiated dispatch (`orchestrator-dispatch`, `wd-supervisor-seat`). A
  project that opted in for that reason gets this feature's autonomous dispatch too,
  the moment the *operator* (not that project's own team) turns `trigger_enabled` on
  fleet-wide — no separate, per-project re-consent exists. Accepted for this feature
  (a finer-grained consent lever is a real but separate feature); the fixed task-text
  template (D1) is the one mitigation in scope — the target repo can tell a trigger
  tick apart from a human-initiated one by its wording.
- **Lifecycle depends on `reaper_enabled` being on.** D8 bounds dispatch *rate*, not a
  dispatched run's lifespan — only `reaper`'s own sweep reclaims a stuck run, and it is
  a separate, independently-disableable switch. `trigger_enabled: true` with
  `reaper_enabled: false` is a legal config combination that leaves a stuck trigger tick
  unreclaimed forever. Mitigation (required, not optional): the trigger's `reconcile`
  logs a `tracing::warn!` when `trigger_enabled && !reaper_enabled` — this is a
  `must_have` on the skeleton cell, not left to Agent's Discretion.

### Agent's Discretion

Everything the decisions above leave open is planning's/execution's choice, constrained
by D1–D10:

- The overrun threshold's exact value and whether it is a hard constant (reaper's own
  style) or a new config field.
- The per-project cooldown window's exact value (D8) and whether it is a constant or
  configurable.
- The exact fixed task-text template sent on dispatch — it MUST stay content-invariant
  per D1/D3 (never adds strategy, opinion, or a suggested answer) but MAY name the
  transition kind and a minimal evidence pointer (run id, pane id, or escalation row
  id) so the woken agent knows why it was woken, the same way `reaper`'s own tracing
  line names what it capped and why.
- Whether `mcp.rs::resolve_preset` needs its visibility changed to be callable from a
  new `trigger` module, or whether preset resolution is better extracted into a shared
  helper both call.
- The exact field shape to parse from `.bee/supervisor/interventions.jsonl` — confirm
  against a real record (e.g. `bee supervisor record --kind escalation ...` run once
  against a disposable scratch store) rather than assuming from CLI help text alone.
- Cell breakdown and slice order.

## Terms

| Term | Meaning in this feature |
|------|--------------------------|
| Trigger | This feature's own daemon background task. Never "supervisor" (see D2). |
| Tick | One dispatched run of a cold `bee supervisor` observation, in the target repo, following exactly one detected transition. |
| Transition | One of the four D4 events. The unit the trigger actually watches for — never a fixed interval. |
| Overrun | A `working` run whose age has crossed a waggledance-computed threshold (D4c). Distinct from `bee supervisor`'s own `budget-overrun` signal, which a tick assigns only after it runs. |

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/reaper.rs` — sweep loop + `Verdict` enum (`Lost`,
  `Awaited(RunStatus)`, `LeftAlone`, `TooYoung`); the source for the "run capped"
  transition (D4a), and the closest structural twin for the new task's own
  slot/cancel-flag/tick-counter shape.
- `crates/waggledance/src/watcher.rs` — `StatusCursor`/`ActivityCursor` +
  `PollWatcher::with_bee_roots`; `StatusCursor` already fires exactly on entry into
  `AgentStatus::Blocked` (D4b), and `BeeRoots`/`activity_states_from` is the direct
  model for a third cursor reading the escalation mailbox (D4d) instead of activity
  state.
- `crates/waggledance/src/orchestrate.rs:339` — `dispatch_run(herdr, engine, project,
  target, task, feature, preset_label)`: the in-process function to call directly: no
  MCP JSON-RPC round-trip needed since the caller is in-daemon.
- `crates/waggledance/src/mcp.rs:822` — `resolve_preset`: preset/role resolution
  against a target project's own `.bee/config.json` `herding.agents`; already proven
  for a `"supervisor"`-labeled entry by `p-9212cae8`/`p-42cbde3a`.
- `crates/waggledance-core/src/bee.rs` — `read_snapshot`/`BeeSnapshot` is the only
  prior art for reading a project's `.bee/` store from `waggledance-core`; there is no
  existing reader for `.bee/supervisor/interventions.jsonl` today — D4d needs a new
  function following the same fail-soft posture every other bee.rs reader uses
  (missing store reads empty, one unparseable line warns-and-skips by line number,
  never an error).
- `crates/waggledance/src/main.rs` — `TerminalBackground`, `reconcile_reaper`/
  `reconcile_supervisor`/`reconcile_notify`: the reconcile/slot/cancel-flag/
  tick-counter shape every background task already follows; `reconcile_reaper` is the
  closest twin (also `Engine`-gated, since the trigger also needs the run ledger and
  the project registry).
- `crates/waggledance-core/src/config.rs` — `TerminalConfig` + its hand-written
  `Default` impl (lines ~63–163); the new `trigger_enabled: bool` field and its own
  doc comment land here, next to `reaper_enabled`'s doc comment explaining exactly why
  only reaper defaults on (read it before writing the new field's own comment).

### Established Patterns

- Cancel-flag checked immediately before the one call with an external side effect
  (`reaper.rs:270-275`, `supervisor.rs:161-164`) — the new task's one `dispatch_run`
  call must take the same guard, checked last, right before the call.
- Fail-closed on unverifiable state (`reaper.rs`'s `older_than`, its snapshot-failure
  branch) — a transition source that cannot be read cleanly yields no transition,
  never a guessed one.

### Integration Points

- `crates/waggledance/src/main.rs` `TerminalBackground::reconcile` — wire
  `reconcile_trigger` alongside the other three.
- `crates/waggledance-core/src/engine.rs` — `Engine::list_projects()` /
  `orchestration_allowed` — the registered-project roots and the per-project consent
  check D7 requires.

## Canonical References

- `docs/discovery/observer-tick-trigger/spec-drop-sup-20260831-b2e1.md` — the contract
- `docs/history/research/demonthorn-supervisor-xia.md` — originating research
- `docs/history/orchestrator-dispatch/CONTEXT.md` (D1, decisions log `34791df7`) —
  the rule this feature carves a narrow, cited exception into (D1 above)
- `docs/history/wd-supervisor-seat/CONTEXT.md` (D2 `10ed8bf4`, D3 `4d173a07`) — the
  precedent this feature's D1 exception is measured against
- decisions log `45a554bb-1832-4243-8a72-6327aec1e215` — this feature's own D1
  exception, as actually logged
- `crates/waggledance-core/src/config.rs` — `reaper_enabled`'s own doc comment,
  required reading per the spec before choosing the new switch's default

## Outstanding Questions

### Resolve Before Planning

None — the one real gray area found during scouting (the D1 conflict) was put to the
owner directly and resolved as D1 above.

### Deferred To Planning

- [ ] Overrun threshold (D4c): exact value, and constant vs. config field.
- [ ] Per-project cooldown window (D8): exact value, and constant vs. config field.
- [ ] Exact fixed task-text template (Agent's Discretion) — draft it and check it
      against D1/D3's content-invariance constraint.
- [ ] `resolve_preset` visibility / extraction (Agent's Discretion).
- [ ] `.bee/supervisor/interventions.jsonl` row shape — confirm against one real
      generated record rather than CLI help text alone.
- [ ] Cell breakdown and slice order.

## Deferred Ideas

- Any refinement to the bee-side `bee supervisor` handshake itself — tracked
  separately under beehive PBI `sup-20260831-7f3a`; this feature does not wait on it
  (spec, explicit).
- A board/notify surface showing trigger-dispatched runs across the whole fleet (closes
  the "no cross-project push signal" accepted risk above) — not asked for, no UI scope
  in this feature.
- A finer-grained, per-purpose consent lever distinguishing "this project accepts
  human-initiated dispatch" from "this project accepts autonomous trigger dispatch"
  (closes the "consent conflation" accepted risk above) — a real but separate feature;
  `orchestration_enabled` stays the single flag for both, for now.
- A fleet-wide dispatch ceiling (bounding total concurrent trigger-originated runs
  across all projects, on top of D8's per-project cooldown) — raised at the hat wave
  (`hat-alternatives`) as a WARNING, not a BLOCKER; left to Agent's Discretion in
  plan.md rather than locked here, since D8 already bounds the worst case per project
  and no evidence yet shows the fleet-wide case is live.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs (D1–D10) are stable — cited, never
renumbered. Planning reads the locked decisions, the code context, the canonical
references, and the deferred-to-planning questions above before drafting cells. D9 and
D10 were added post-Gate-1 by planning's hat wave (2026-08-31) — new decisions are the
sanctioned way to evolve a locked record; D1–D8 were not reinterpreted.
