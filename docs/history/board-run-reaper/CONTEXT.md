# Board Run Reaper — Context

**Feature slug:** board-run-reaper
**Date:** 2026-08-31
**Shaping session:** complete
**Scope:** Standard
**Domain types:** RUN

## Feature Boundary

A background reaper inside the waggledance daemon awaits every
waggledance-spawned run that nobody else is awaiting, so a run that prints its
`HERDR_DONE_<nonce>` marker gets its transcript stored, its status capped, and
its pane closed without a human or an MCP caller in the loop — and a run whose
pane has vanished stops lying `working` forever. It ends at the ledger and the
pane: no new UI surface, no notification changes, no change to the close-guard
semantics `orchestrate::finish` already enforces.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never
reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 (`eecfefeb`) | The reaper covers EVERY waggledance-spawned `working` run — `preset_label` present — board-dispatched and abandoned MCP dispatches alike. | The leak is who-calls-await, not which button dispatched; three MCP runs from 2026-08-25 sit `working` for the same reason the board compound pane stayed open. |
| D2 (`4047ca75`) | A `working` run whose pane is absent from the herdr snapshot is marked status `lost` — a row-only transition; nothing is closed or killed by inference. | A gone pane has no process to protect, but a row lying `working` pollutes the ledger. dispatch-submit-and-reclaim's declared-completion-only pane close is untouched: `lost` never closes anything. |
| D3 (`c8847fb7`) | A cleanly finished run raises no notification — the notify channel stays blocked-only. The board reflecting done/lost is the whole surface. | — |

### Agent's Discretion

Where the loop lives (a tokio task beside the existing watchers/supervisor),
its poll cadence, and how it avoids double-awaiting a run an MCP caller is
actively awaiting are planning's implementation choices — constrained only by
D1–D3 and by reusing `orchestrate::finish`'s three close guards verbatim
(declared marker + waggledance-spawned + transcript stored), never a second
copy of them.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| lost | Terminal run status for a `working` run whose pane no longer exists in the herdr snapshot. Row-only: no pane action ever accompanies it. |
| reaper | The in-daemon background loop that awaits unattended runs. NOT `supervisor.rs` — that is the herdr watchdog and stays untouched. |

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `crates/waggledance/src/orchestrate.rs` — `await_run` (the poll loop, marker
  detection, `AWAIT_POLL_INTERVAL`) and `finish` (transcript-then-status write,
  the three pane-close guards, `Completion::Declared`). The reaper drives these
  functions; it does not reimplement them.
- `crates/waggledance/src/mcp.rs:1084` — the only current `await_run` caller
  (`waggledance_await`), clamped to 60 s per call.
- `crates/waggledance/src/server.rs` `dispatch_board_run` — the board's
  fire-and-forget dispatch this feature is cleaning up after.
- `crates/waggledance/src/main.rs` `TerminalBackground` — the reconcile pattern
  (opt-in switch, cancel flag, tick counters) the supervisor and notify tasks
  already follow; the natural home for wiring a third background task.

### Established Patterns

- Declared-completion whitelist (`Completion` enum) — only the agent's own
  marker may close a pane; any new "the run is over" conclusion defaults to
  `Observed` and must not acquire close rights.
- Board run lock reads "a `working` run row JOINED with a live pane"
  (`server.rs` bee_action run half) — `lost` naturally releases it; verify the
  JOIN treats `lost` as terminal.

## Open Questions

- None product-side. Implementation choices above are delegated.

## Provenance

Diagnosed live 2026-08-31: board compound run `run-27796bcbe3ae80ff` finished
(`HERDR_DONE_ecc4188080ea4dd5` on pane w2:pR) yet sat `working` with its pane
open until a manual `waggledance_await`. Backlog PBI `board-run-reaper`.
Owner answered the three shaping questions 2026-08-31 (scope / lost / no
notification), all per recommendation.
