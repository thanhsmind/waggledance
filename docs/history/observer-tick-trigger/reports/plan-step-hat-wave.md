# Plan-step hat wave — observer-tick-trigger

3 seats, 2026-08-31, over the first draft of `docs/history/observer-tick-trigger/plan.md`.

## hat-facts-gaps (opus)

STRUCTURE
BLOCKERS:
1. D6's reconcile matrix and D2's `trigger_ticks` had no epic proof — fixed: E1's Proof
   Needed now names the on/off tick-counter test explicitly (mirrors `main.rs`'s
   existing reaper/supervisor/notify tests).
2. The first draft's D4a detector (working-set absence-diff) also fired on a run
   reaching `blocked` — outside D4's closed list and colliding with D4b — fixed by
   sourcing D4a from `Reaper`'s own verdict channel instead (which never emits
   `Awaited(Blocked)`).
3. Preset resolution was deferred to Open Questions favoring `preset_label: None`,
   which is unworkable (`DispatchTarget::Spawn` requires a resolved `entry`; an
   unlabeled run is invisible to `list_unattended_working_runs`'s own filter) — fixed:
   Approach now names `resolve_preset` made `pub(crate)` as the resolution path, and
   the Open Questions entry is removed.

WARNINGS: 3 claims-table rows were reflowed rather than byte-verbatim; one claim (bee.rs
readers "warn + skip by line number") was factually wrong (the module has zero
`tracing::` calls); `StatusCursor` claim overstated pre-filtering; claim 8's anchor was
a test fixture rather than the struct; E1 was a fat cell; the trigger's own poll
interval was unnamed. All applied directly to the redraft.

CLAIMS TABLE AUDIT: rows 1-3 corrected to single-line anchors (no reflow possible);
rows 4/6/7 confirmed byte-verbatim as originally anchored; row 8 (bee.rs) corrected
from a false claim to the verified true one (`grep -c "tracing::" ... ` → `0`); row for
`Project` fields re-anchored from the test fixture to `domain.rs`.

## hat-alternatives (opus)

SMALLER PATH: FAIL (cheaper shape exists) in the first draft — adopted in full:
1. Source D4a from `Reaper::sweep_once`'s already-computed `Vec<(String, Verdict)>`
   (currently discarded at `reaper.rs:304`) via a new optional channel param, instead
   of re-deriving a weaker, broader approximation by polling. Confirmed: `Reaper::run`
   has exactly one production caller (`main.rs:246`), so this is small and additive.
2. Merge what would have been separate capped/overrun cells is NOT adopted as literally
   proposed (capped no longer polls at all once sourced from the channel, so the two
   no longer share poll infrastructure) — but E1 stays the single skeleton cell
   carrying capped, and overrun is its own cell (E2) reusing E1's gate, which answers
   the same underlying concern (no duplicated poll/dispatch-gate code) without
   literally cramming two detectors into one cell.
3. A second acknowledgement boolean for the D1 exception — rejected in favor of D10
   (`terminal.trigger_dry_run`), a strictly more useful, equally cheap valve, adopted
   as a new locked decision.

Also adopted as WARNINGs: name the trigger's own poll interval explicitly (mirroring
`reaper::SWEEP_INTERVAL`); state the `reaper_enabled` soft-dependency explicitly (now a
required `tracing::warn!` must_have on E1, also independently raised by hat-user-impact);
flag a fleet-wide dispatch ceiling as real but out of this slice's scope (recorded in
CONTEXT.md's Deferred Ideas, not locked — no evidence yet that the fleet-wide case is
live, and D8 already bounds the per-project worst case).

## hat-user-impact (sonnet)

Found, independently of the other two seats, that a trigger-dispatched run's own
eventual completion is itself a transition the trigger's own detectors would see —
an unbounded (cooldown-rate-limited, never terminating) self-observation loop. Fixed:
CONTEXT.md D9 — every detector filters out a run/row carrying the trigger's own
`feature` marker before treating it as a transition; E1's Proof Needed now includes
this as an explicit exit criterion.

Also found: no cross-project push signal reaches the operator when a tick fires
elsewhere in the fleet; the shared `orchestration_enabled` flag conflates two different
kinds of consent (human-initiated dispatch vs. this feature's autonomous dispatch);
lifecycle of a trigger-dispatched pane depends on `reaper_enabled` being on, a separate,
independently-disableable switch. None of these are BLOCKERs against the locked
D1-D8 shape itself — recorded as CONTEXT.md's "Accepted risks" (with the
`reaper_enabled` warning promoted to a required must_have, since it is cheap and both
other seats' own WARNING lists raised it independently) and as Deferred Ideas for the
two that need real, separate feature work (a visibility surface, a finer consent lever).

## Disposition

All BLOCKERs fixed directly in the plan.md redraft (this file documents what changed
and why). No BLOCKER remains open — no second, blocker-scoped pass was needed.
CONTEXT.md gained D9 and D10; D1-D8 were not reinterpreted. plan.md's claims table was
rebuilt with single-line anchors throughout, each re-verified by hand against the real
file before this report was written.
