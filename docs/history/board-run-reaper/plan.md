# Board Run Reaper — Plan

**Feature:** board-run-reaper · **Lane:** standard · **Class:** feature
**Flags:** data-model, external-systems · **Product files:** ~6
**Decisions:** D1 (`eecfefeb` — reap every waggledance-spawned working run),
D2 (`4047ca75` — gone pane ⇒ status `lost`, row-only), D3 (`c8847fb7` — clean
finish raises no notification)
**Context:** `docs/history/board-run-reaper/CONTEXT.md`
**Worktree:** `waggledance--wt--board-run-reaper` (branch `wt/board-run-reaper`)
**Hat wave:** skipped — clear-ask fast path, reason logged in decisions.

## Problem

`orchestrate::finish` is the only place a run gets capped and its pane closed,
and `await_run` is its only road — called from exactly one place, the
`waggledance_await` MCP tool. A run dispatched from the board (or an MCP
dispatch whose caller never awaits) stays `working` forever and its pane stays
open after the agent prints `HERDR_DONE_<nonce>`. Live evidence 2026-08-31:
board compound run `run-27796bcbe3ae80ff` done-but-open until a manual await;
three runs from 2026-08-25 still `working` with panes long gone.

## Shape

A third background task beside the supervisor and notify tasks — the
**reaper** — swept on a fixed interval from `TerminalBackground::reconcile`,
same slot/cancel-flag/tick-counter pattern. Each sweep, deliberately
conservative (it may only do what D1/D2 name; it never invents a verdict):

1. List every `working` run with `preset_label` present (all projects) whose
   `updated_at` is older than a 60 s grace window (lets an interactive
   dispatch+await settle first; a race that slips through is benign — see
   claims C1, C8).
2. Take one herdr snapshot. Pane absent → `update_run_status(run, "lost")`
   (D2). Row-only: no pane call, no transcript write (there is no pane left to
   read).
3. Pane present → one `read_pane`; only when the run's marker appears fresh
   against its baseline (same count-vs-baseline rule `await_run` uses) does the
   reaper call `orchestrate::await_run` with a short timeout — which finds the
   fresh marker, stores the transcript, caps the run `done`, and closes the
   pane through `finish`'s three guards, unchanged (D1).
4. Anything else (working, blocked, unknown, unsettled) → skipped this sweep.
   The reaper never writes `blocked` or `timeout`; blocked panes already
   belong to the notify watcher. A clean `done` raises no notification —
   `finish`'s existing `is_run_notifiable` gate already says no (D3).

`lost` joins the status vocabulary as a terminal value: `RunStatus::Lost`,
`as_str "lost"`, added to `terminal_from_stored`'s whitelist. The board's
per-feature run lock reads only `status='working'` rows, so a `lost` cap
releases a stuck card with no board-side change.

Switch: `reaper_enabled` in `[terminal]`, default **true**, mastered by the
existing terminal-family switch like the other two background tasks. Rationale
for default-on where supervisor/notify default off: the reaper only tidies
runs waggledance itself dispatched, and the family master switch still governs
everything; a family switched off runs no reaper at all.

### SMALLER PATH check

Cheaper shapes considered: folding the sweep into the notify watcher loop
(couples two unrelated cadences and puts run-capping in a status-transition
watcher — rejected); skipping `lost` (violates locked D2); skipping the config
switch (breaks the one background-task pattern all siblings follow, saves ~10
lines — rejected). The two-cell shape below is the smallest that honors every
locked decision. PASS.

## Load-bearing claims

| # | Claim | Anchor | Label | Evidence |
|---|-------|--------|-------|----------|
| C1 | Pane close happens only in `finish`, guarded by Declared marker + `preset_label.is_some()` + transcript stored | `crates/waggledance/src/orchestrate.rs:802-873` | read | Guard doc + `if completion == Completion::Declared && run.preset_label.is_some() && transcript_stored` |
| C2 | `await_run` has exactly one production caller | `crates/waggledance/src/mcp.rs:1084` | ran | `rg -ln await_run crates/` → mcp.rs, orchestrate.rs only |
| C3 | Stored-status terminal whitelist is done/blocked/timeout; `working` and unknown strings read as open | `crates/waggledance/src/orchestrate.rs:585-598` | read | `terminal_from_stored` array; `Lost` slots in beside them |
| C4 | Board's per-feature run lock selects only `status='working'` rows | `crates/waggledance-core/src/repository.rs:196-205` | read | `WHERE … AND status='working'` in `list_live_runs_for_feature` |
| C5 | `TerminalBackground::reconcile` already runs the supervisor + notify slot/cancel/tick pattern to copy | `crates/waggledance/src/main.rs:144-157` | read | `reconcile_supervisor` / `reconcile_notify` calls |
| C6 | `[terminal]` config carries `supervisor_enabled` / `notify_enabled` booleans to extend | `crates/waggledance-core/src/config.rs:75-78` | read | struct fields |
| C7 | Marker freshness is judged against the run's pre-send baseline, never bare string presence | `crates/waggledance/src/orchestrate.rs:205-213,245-250` | read | baseline doc + `capture_baseline` |
| C8 | Unattended `working` rows exist right now (the defect is live, not theoretical) | runs ledger via `waggledance_runs` 2026-08-31 | ran | `run-48e951cf`, `run-57f2ccff` (2026-08-25), board run `run-27796bc` before manual await |

## Cells (current slice — one slice, walking skeleton)

1. **brr-1 — status + store + config vocabulary.** `RunStatus::Lost`
   (`as_str`, `terminal_from_stored`), repository
   `list_unattended_working_runs()` (all projects, `preset_label IS NOT NULL`,
   `status='working'`), config `reaper_enabled` default true. Tests: enum
   round-trip incl. `lost`; query returns only preset-labeled working rows;
   config default/parse.
2. **brr-2 — the reaper task, wired.** New `crates/waggledance/src/reaper.rs`
   sweep loop per the Shape above (grace window, snapshot, lost-cap,
   marker-precheck → `await_run`); wired as `reconcile_reaper` in
   `TerminalBackground` behind family switch + `reaper_enabled`. Tests with
   `FakeHerdr`: gone pane → `lost`, no pane call; fresh-marker pane → done +
   pane closed; quiet working pane → untouched; blocked pane → untouched;
   young run inside grace window → untouched.

Verify: `cargo test --workspace` (the recorded `commands.test`), plus
`cargo build --profile fast -p waggledance`.

## Rollback

Both cells sit behind `reaper_enabled`; setting it false (or the family
switch off) restores today's behavior exactly. No schema migration — `lost`
is a new value in an existing TEXT column; old builds read it as an open row
(C3's whitelist posture), which costs one extra poll, never a wrong close.

## Open Questions

None — implementation choices delegated in CONTEXT.md are exercised above.
