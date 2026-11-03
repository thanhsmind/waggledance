---
type: bee.delivery
title: board-run-reaper — delivery
description: "Delivery record proposed by bee knowledge promote for work item board-run-reaper: 2 capped cell(s), 5 recorded deviation(s)."
timestamp: 2026-08-31
bee:
  id: board-run-reaper-delivery
  lifecycle: active
  required_context: [docs/history/board-run-reaper/CONTEXT.md, docs/history/board-run-reaper/plan.md]
  sources: [docs/history/board-run-reaper/CONTEXT.md, docs/history/board-run-reaper/plan.md, .bee/cells/brr-1.json, .bee/cells/brr-2.json]
---

# board-run-reaper — Delivery

## What shipped

- **brr-1** — RunStatus::Lost, list_unattended_working_runs, and the default-on terminal.reaper_enabled switch, each with its own test (3 file(s) changed)
- **brr-2** — Reaper sweep caps vanished panes lost and finishes declared-done runs through await_run, wired behind the family + reaper switches (4 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **brr-1** — `cargo test --workspace`
- **brr-2** — `cargo test --workspace && cargo build --profile fast -p waggledance`

## Deviations

- **brr-1** — TerminalConfig's derived Default became a hand-written impl — reaper_enabled defaults true and a derive would silently ship it off, and the struct-level serde(default) routes absent TOML keys through it — hit an unforeseen obstacle
- **brr-2** — Reused await_run's real freshness rule (marker absent from the run's baseline, present in a current read) instead of the cell's described marker-count-vs-baseline-count — await_run never counted occurrences, so a count rule would have been a second, different rule — the plan was wrong about a fact
- **brr-2** — Extracted that rule into orchestrate::marker_is_fresh and made RECENT_LINES_CAP pub(crate), reserving crates/waggledance/src/orchestrate.rs — the cell asked for reuse/extraction of the helper, never a second copy — something else had to be fixed first
- **brr-2** — Added a read_pane_log seam to FakeHerdr (reserved crates/waggledance/src/herdr/fake.rs) — the must-have truth is that a gone pane gets NO pane call, and only a recorded call log tells that apart from a read that errored — hit an unforeseen obstacle
- **brr-2** — Blocked panes are skipped before the read, not just excluded from capping, so the reaper never reaches await_run's blocked branch and can never write blocked — hit an unforeseen obstacle

## Provenance

Proposed by `bee knowledge promote --work board-run-reaper` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-run-reaper/CONTEXT.md`, `docs/history/board-run-reaper/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.
