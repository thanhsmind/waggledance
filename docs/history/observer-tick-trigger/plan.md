---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Observer-Tick Trigger

Mode: `standard` — 2 risk flags: external-systems, multi-domain
Why this is the least workflow that protects the work: the feature touches a locked
cross-cutting decision (orchestrator-dispatch D1), adds a new daemon background task
that autonomously spawns LLM agents across the fleet, and spans four files across two
crates — none of that is proven safe by a smaller ceremony.

**Revision note (post hat-wave, 2026-08-31):** this plan was redrafted after the
plan-step hat wave (3 seats) found one correctness BLOCKER in the first draft — see
"Discovery" below. CONTEXT.md gained D9 and D10 in response. Nothing in D1–D8 changed.

## Requirements (from CONTEXT.md)

- D1: The orchestrator-dispatch D1 exception (decisions log `45a554bb`) — the trigger
  may autonomously fire `dispatch_run` with a FIXED, content-invariant task string on
  a mechanical fleet transition. It never varies task content, never chooses between
  actions.
- D2: Naming — module `crates/waggledance/src/trigger.rs`, struct `Trigger`, config
  field `terminal.trigger_enabled`, tick counter `trigger_ticks`. Never "supervisor".
- D3: Event-driven — dispatch is edge-triggered through a cursor/dedup, never fired on
  every poll tick regardless of state.
- D4: Exactly four transition classes, and only these: (a) run capped — **the reaper's
  own sweep verdict** `Lost` or `Awaited(Done | Timeout)`; (b) run→blocked; (c) run
  overrun; (d) new escalation row.
- D5: No local observation store — the only read against `.bee/supervisor/` is D4d's
  cursor, and it is read-only.
- D6: Opt-in, default OFF, gated under `terminal.enabled`.
- D7: Per-project dispatch consent — only dispatch into a project with
  `orchestration_enabled` on.
- D8: Per-project cooldown between dispatched ticks; suppresses the dispatch only,
  never the detection — a suppressed transition is dropped, never queued.
- D9: The trigger never re-observes its own dispatched runs — every detector filters
  out a run/row already carrying the trigger's own `feature` marker.
- D10: `terminal.trigger_dry_run` (default off) — every detector runs normally and logs
  what it would have dispatched, without ever calling `dispatch_run`.

## Load-bearing claims

Every row is load-bearing — the shape below assumes each of these is true. Match rule:
the evidence column is a verbatim byte substring of the anchored line (single-line
anchors used throughout so no join/reflow question can arise).

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|--------------------|
| 1 | `Reaper::sweep_once` already computes and returns `Vec<(String, Verdict)>` per sweep; `Reaper::run` discards it today — the exact hook point D4a needs. | read | `crates/waggledance/src/reaper.rs:304` | `let _ = self.sweep_once().await;` |
| 2 | `Reaper::run` has exactly one production caller, so threading an optional verdict channel through it is a small, additive, backward-compatible change. | read | `crates/waggledance/src/main.rs:246` | `sweep.run(ticks).await;` |
| 3 | `list_unattended_working_runs` returns every still-`working` waggledance-spawned run, project-blind — the source D4c's overrun scan polls directly (no new repository method needed). | read | `crates/waggledance-core/src/repository.rs:207` | `Every still-\`working\` run waggledance itself spawned, across all` |
| 4 | The same method applies no age filter — D4c's threshold check is entirely the trigger's own, never the store's. | read | `crates/waggledance-core/src/repository.rs:231` | `Age is not filtered here — the caller's grace` |
| 5 | `Engine::orchestration_allowed` is the per-project consent predicate (D7); its own doc says the caller must ALSO check `terminal.enabled` (D6) — two separate checks. | read | `crates/waggledance-core/src/engine.rs:379` | `the caller combines it with the global \`terminal.enabled\` switch` |
| 6 | `watcher::StatusCursor::diff` surfaces every status change it sees, not specifically entry-into-`Blocked` — D4b's detector must filter the result itself; the type is reusable, its output is not pre-filtered. | read | `crates/waggledance/src/watcher.rs:42` | `Feed a fresh snapshot's agent statuses; return only the changes not` |
| 7 | `dispatch_run` is a plain in-process async function taking `herdr, engine, project, target, task, feature, preset_label` — callable directly from a new background task, no MCP round-trip, and `feature: Option<&str>` is exactly D9's marker channel. | read | `crates/waggledance/src/orchestrate.rs:339` | `pub async fn dispatch_run(` |
| 8 | `DispatchTarget::Spawn` requires a resolved `entry` (the argv + env + trust conditions) to start a pane at all — a dispatch cannot skip preset resolution by passing no preset. | read | `crates/waggledance/src/orchestrate.rs:118` | `carries the command AND the conditions declared around it` |
| 9 | `list_unattended_working_runs`'s own filter requires `preset_label IS NOT NULL` — a trigger-dispatched run MUST carry a real, resolved preset label, or it becomes invisible to the trigger's own D4a/D4c sources (and to `reaper`). | read | `crates/waggledance-core/src/repository.rs:217` | `\`preset_label IS NOT NULL\`: waggledance spawned this pane.` |
| 10 | `resolve_preset` (the function that turns a caller's preset label into a real command) is a private `fn` today, and its own doc names the global-then-project order as load-bearing — the trigger needs a way to call it, not a way around it. | read | `crates/waggledance/src/mcp.rs:815` | `Reversing the order would silently` |
| 11 | No reader for `.bee/supervisor/interventions.jsonl` exists anywhere in `waggledance-core` today, and — corrected from the first draft — every existing `bee.rs` reader is SILENTLY fail-soft (no `tracing::` calls in the module at all), not "warn + skip by line number." | ran | `grep -c "tracing::" crates/waggledance-core/src/bee.rs` | `0` |
| 12 | `Project` carries `root_path: PathBuf` directly on the domain struct (not only in a test fixture) — the exact field D4d's per-project interventions-store path reads; `orchestration_enabled: bool` sits 7 lines below it in the same struct (domain.rs:18), covered by claim 5's own anchor for its semantics. | read | `crates/waggledance-core/src/domain.rs:11` | `pub root_path: PathBuf,` |
| 13 | The cancel-flag-checked-immediately-before-the-external-side-effect pattern is the established shape for every background task's one dangerous call. | read | `crates/waggledance/src/reaper.rs:270` | `if self.cancelled.load(Ordering::SeqCst) {` |
| 14 | `reaper_enabled` is the one `TerminalConfig` switch that defaults on, and the doc comment names why — `trigger_enabled` defaults off for the opposite reason (it spawns an LLM agent). | read | `crates/waggledance-core/src/config.rs:66` | `\`reaper_enabled\` is the one` |
| 15 | The existing off/on tick-counter proof pattern (`reaper_ticks`) is what every sibling background task's own test suite already uses to prove a switch flip really starts/stops the loop — `trigger_ticks` owes the same test. | read | `crates/waggledance/src/main.rs:160` | `fn reaper_ticks(&self) -> u64 {` |

## Discovery

Inspected: `reaper.rs`, `watcher.rs`, `supervisor.rs`, `main.rs`'s `TerminalBackground`,
`orchestrate.rs` (`dispatch_run`, `DispatchTarget`), `config.rs::TerminalConfig`,
`engine.rs`, `repository.rs`, `domain.rs`, `mcp.rs::resolve_preset`, `bee.rs` (no
`.bee/supervisor/` reader; confirmed zero `tracing::` calls in the module), and
`bee supervisor --help`/`--record --help` for the mailbox row shape.

**Hat-wave correction (the reason this is a second draft).** The first draft's D4a
detector diffed `list_unattended_working_runs`'s working-set across polls ("a run id
present last poll, absent this poll"). `hat-alternatives` found this fires on every
*ordinary, healthy* run completion too — not just a reaper cap — because a normal
MCP-awaited `done` and a reaper `Awaited(Done)` leave the same set the same way; the
ledger's status column cannot tell them apart from outside. `hat-facts-gaps`
independently found the same detector also fires on a run reaching `blocked`, which D4
explicitly excludes from D4a and which D4b already owns — a double-detection collision.
Both defects share one root cause and one fix: consume the reaper's own already-computed
`Verdict` (claim 1) instead of re-deriving a weaker approximation of it. `hat-user-impact`
separately found that a trigger-dispatched run, once it completes, would itself become a
"capped" transition the trigger's own detector would see and re-act on — an unbounded
(if cooldown-rate-limited) self-loop; CONTEXT.md D9 is the fix. `hat-facts-gaps` also
found the plan's preferred answer to "how does the trigger resolve a preset" (pass
`preset_label: None`) is unworkable (claims 8–10) and three of the first draft's claims
table rows were reflowed rather than verbatim (fixed above by using single-line anchors
throughout).

## Approach

**Recommended path** (cites D1–D10): one new task, `Trigger` (D2), wired into
`TerminalBackground` exactly like `reaper`/`supervisor`/`notify` (D6's reconcile shape,
claim 13, claim 15 for its test). `Trigger` owns ONE internal poll tick
(`TRIGGER_POLL_INTERVAL`, a new constant mirroring `reaper::SWEEP_INTERVAL`'s
politeness-to-herdr reasoning — D3: cheap freshness reads are fine, firing a tick on
every poll is not) and gates every detected transition through one shared
`maybe_dispatch(project, kind, evidence)`: D9 self-exclusion check → D7 consent check
(claim 5) → D8 per-project cooldown (suppress-only, cursor still advances) → D10
dry-run branch (log, return) → cancel-flag (claim 13, checked last) → `dispatch_run`
(claim 7) with a fixed task-text template naming only the transition kind and a minimal
evidence pointer (never strategy — D1), through a preset resolved by a `pub(crate)`
`resolve_preset` (claim 10) — never `preset_label: None` (claims 8–9 rule it out) — and
`feature: Some(TRIGGER_FEATURE_MARKER)` (D9).

- **D4a (capped):** `Reaper::run` (and/or `with_cancel_flag`) gains an optional
  `Option<mpsc::UnboundedSender<(String, Verdict)>>` parameter (claims 1–2); when
  `trigger_enabled`, `reconcile_reaper`'s existing construction passes `Trigger`'s
  sender. `Trigger` consumes the receiver, filters D9 (a verdict for the trigger's own
  marked run is dropped before it reaches `maybe_dispatch`), and treats `Lost` or
  `Awaited(Done | Timeout)` as "capped" — `Awaited(Blocked)` is excluded (it is D4b's,
  never D4a's).
- **D4c (overrun):** `Trigger`'s own poll of `list_unattended_working_runs` (claims
  3–4), filtered for D9, checked for age against `TRIGGER_OVERRUN_THRESHOLD` (a new
  constant), fired once per run id via a seen-set (never re-fired while it stays
  overrun).
- **D4b (blocked):** `Trigger` runs its OWN `watcher::StatusCursor` (claim 6) against
  its own herdr snapshot — independent of `notify_enabled` (D6's own reasoning: a
  switch must not silently depend on a different opt-in switch) — filtering the
  cursor's output for entry into `AgentStatus::Blocked` specifically, since the type
  itself does not pre-filter (claim 6's corrected wording).
- **D4d (new escalation row):** a new `waggledance-core::bee` reader parsing
  `<project.root_path>/.bee/supervisor/interventions.jsonl` (claim 12), filtered to
  `kind` ∈ {escalation, urgent}, cursored by the last row-count/id seen per project.
  Unlike every existing `bee.rs` reader (claim 11: confirmed silently fail-soft), this
  new reader DELIBERATELY adds `tracing::warn!` on a malformed line (naming its line
  number) before skipping it — a small, explicit improvement over the module's existing
  precedent, not a continuation of it, adopted because this reader feeds an autonomous
  dispatch decision where a silently-dropped row is a worse failure mode than it is for
  a passive UI reader. Root enumeration reuses `watcher::BeeRoots` (already re-asked
  every tick, so a project registered mid-run is picked up with no restart).

**Rejected alternatives:**
- The first draft's working-set absence-diff for D4a — rejected outright (BLOCKER from
  the hat wave): fires on ordinary healthy completions and on `blocked` transitions,
  both outside D4's closed list.
- `preset_label: None` for the trigger's own dispatches — rejected (claims 8–9): a
  spawn cannot start without a resolved `entry`, and an unlabeled run is invisible to
  the trigger's own D4a/D4c sources and to `reaper`.
- A single "god" cursor merging all four transition types into one struct — rejected:
  the four sources are genuinely different shapes (a verdict channel, a run-ledger poll,
  a herdr snapshot, a jsonl file) and a merged cursor would hide that in one large,
  harder-to-test type.
- A second acknowledgement boolean beside `trigger_enabled` for the D1 exception —
  considered at the hat wave and rejected: `trigger_enabled` is already config-file-only,
  default off, gated under `terminal.enabled`, with its own doc comment naming the D1
  exception; a second boolean set in the same edit buys nothing. D10 (dry-run) is the
  adopted, strictly more useful alternative.

**Risk map:**

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| D1 exception scope creep | MEDIUM | The fixed-task-text discipline is a convention, not compiler-enforced | a test asserting the template's wording carries no per-transition branching, cited as an explicit exit criterion on E1 (not just this risk map — the first draft's gap) |
| Self-recursion (D9) | MEDIUM, now mitigated | A trigger-dispatched run re-entering its own detectors was a real, found gap (hat-user-impact) | E1 test: a run carrying the trigger's own `feature` marker, capped, dispatches nothing |
| Dispatch storm | MEDIUM | A flapping run could fire many transitions per second | D8 cooldown test: N transitions in one window → 1 dispatch, and the cursor still advances past the other N-1 (no retry) |
| Fleet-wide burst | LOW-MEDIUM, accepted for now | D8 bounds per-project rate only; N consenting projects transitioning at once is N concurrent spawns (hat-alternatives, WARNING) | not proven in this slice — see CONTEXT.md's Deferred Ideas (fleet-wide ceiling) |
| `.bee/supervisor/interventions.jsonl` schema drift vs. the bee CLI's own writer | LOW-MEDIUM | Parsed against CLI help text, not a generated sample | trg-4 generates one real row in a scratch store and parses it before locking the reader's struct |
| Cross-project consent leak | LOW | `orchestration_allowed`/`terminal.enabled` are both re-checked every dispatch | unit test: transition in a non-consented project fires nothing |
| Stuck trigger-tick pane with `reaper_enabled: false` | LOW, accepted with a required warning | D8 bounds rate, not lifespan; only `reaper` reclaims a stuck run and it is independently disableable | E1 must_have: `tracing::warn!` at reconcile when `trigger_enabled && !reaper_enabled` |

## Shape

**Epic map.** Feature outcome: waggledance dispatches exactly one fixed-content
`bee supervisor` observation tick per detected fleet transition, into the affected
consenting repo, and stays silent otherwise. Repo-reality basis: claims 1–15 above.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| E1 | Skeleton: config (`trigger_enabled`, `trigger_dry_run`) + `Trigger` task (`trigger_ticks`, cancel-flag, reconcile) + `maybe_dispatch` gate (D7/D8/D9/D10) + `resolve_preset` made callable + D4a (capped, via `Reaper`'s new verdict channel) | The walking skeleton — proves the whole path end to end on the cheapest, now-corrected detector, and carries every cross-cutting piece (D9, D10, preset resolution) the other three epics reuse | trg-1 | `reconcile_trigger` on/off flips `trigger_ticks` (mirrors `main.rs`'s existing reaper/supervisor/notify tests, claim 15); a `Reaper`-sourced `Lost`/`Awaited(Done\|Timeout)` verdict in a consented project dispatches exactly once, with a real resolved preset label; the SAME verdict for a run carrying the trigger's own `feature` marker (D9) dispatches nothing; a non-consented project dispatches nothing; two capped verdicts inside the cooldown window (D8) dispatch once and the second is not retried; `trigger_dry_run` (D10) logs and calls `dispatch_run` zero times; `trigger_enabled && !reaper_enabled` logs a warning; the fixed task-text template carries no per-transition branching |
| E2 | D4c overrun detection | Reuses E1's poll target and dispatch gate; only the trigger condition and the fire-once dedup are new | trg-2 | a `working` row (D9-excluded runs skipped) past `TRIGGER_OVERRUN_THRESHOLD` dispatches once; the same still-overrun row on the next poll does not re-dispatch |
| E3 | D4b blocked detection | Independent herdr-status source, reusing `watcher::StatusCursor` with its own entry-into-`Blocked` filter | trg-3 | entry into `Blocked` dispatches once; staying `Blocked` or leaving it dispatches nothing (mirrors `watcher.rs`'s own test table); runs independent of `notify_enabled` |
| E4 | D4d escalation-row detection | The one genuinely new I/O surface (claim 11); needs its own reader in `waggledance-core`, deliberately more vocal on parse failure than existing precedent | trg-4 | a new `escalation`/`urgent` row (generated once for real, per the risk map) dispatches once; an `intervention` row does not; a missing store reads empty; a malformed line warns (naming its line number) and is skipped |

Slice queue: trg-1 → trg-2 → trg-3 → trg-4, in that order (each later cell reuses E1's
dispatch gate, so E1 must land first); all four are the current slice — the spec's
accept-when is exactly these four transition classes together, and none is deferrable
to an unscheduled future slice.

## Test matrix

Triad per epic, at its smallest demonstrating size — see "Proof Needed" column above
for the concrete cases per epic. No `edge-dimensions.md` (standard, not hard-gate).

## Open Questions

- The exact overrun threshold (`TRIGGER_OVERRUN_THRESHOLD`, E2) and per-project
  cooldown window (E1/D8) values, and the trigger's own poll interval
  (`TRIGGER_POLL_INTERVAL`, E1) — planning sets sane constants mirroring `reaper.rs`'s
  own `GRACE_WINDOW`/`SWEEP_INTERVAL` style; execution may tune within the cell.
- The exact fixed task-text template's wording — drafted in trg-1, checked against D1's
  content-invariance constraint via the E1 exit criterion above before the cell caps.
- `.bee/supervisor/interventions.jsonl`'s real row shape — trg-4 generates one real row
  in a scratch store and parses it before locking the reader's struct, per the risk map.

## Out of scope

- Any change to `bee supervisor`'s own store, verbs, or CLI (beehive `sup-20260831-7f3a`,
  explicitly not waited on).
- A UI surface for the trigger's config switch or its dispatch history — not asked for;
  the switch is config-file-only like `supervisor_enabled`/`notify_enabled` today. This
  is also the "no cross-project push signal" accepted risk (CONTEXT.md).
- Refining `orchestrator-dispatch` D1 beyond the narrow, logged exception (D1 above) —
  the general rule is untouched.
- A fleet-wide dispatch ceiling beyond D8's per-project cooldown (CONTEXT.md Deferred
  Ideas) and a finer-grained, per-purpose consent lever (CONTEXT.md Deferred Ideas,
  "consent conflation") — both real, both separate feature-sized asks.
