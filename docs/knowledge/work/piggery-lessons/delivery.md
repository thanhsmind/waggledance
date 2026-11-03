---
type: bee.delivery
title: piggery-lessons — delivery
description: "Delivery record proposed by bee knowledge promote for work item piggery-lessons: 3 capped cell(s), 3 recorded deviation(s)."
timestamp: 2026-10-16
bee:
  id: piggery-lessons-delivery
  lifecycle: active
  areas: [orchestration]
  required_context: [docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md]
  sources: [docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md, .bee/cells/pl-1.json, .bee/cells/pl-2.json, .bee/cells/pl-3.json]
---

# piggery-lessons — Delivery

## What shipped

- **pl-1** — Copied the three Paseo 0.11.2 after-permission captures into testdata/fixtures/paseo-0.11.2/ with <ID3> substitution, added their README rows, and added a fixture test asserting running/no-pending after allow, idle turn-end with PERM-DONE, and Permission on a second permission. (5 file(s) changed)
- **pl-2** — Board shows Paseo leaders blocked on a permission or lost within the hour: /api/board leader_waiting and one home rail pill per project, from the runs table (4 file(s) changed)
- **pl-3** — Migration 6 adds runs.notice/notice_state/notice_until; finish_run writes status, transcript and notice in one statement and the delivery flush reads, sends and marks notices from the store, so a caller notice survives a restart (commit a9006c6). (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **pl-1** — `cargo test --profile fast -p waggledance paseo::` — cargo test --profile fast -p waggledance paseo:: — green:unit — the cell's verify command; 34 passed 0 failed, including the new paseo::tests::the_after_permission_wait_ends_at_the_turn_not_the_allow
- **pl-2** — `cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance board` — the cell verify: 516 core + 53 board-filtered; the full waggledance run (1201 passed) also covers the rail pill test the board filter misses
- **pl-3** — `cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance delivery` — PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance delivery && cargo test --profile fast --workspace — green:unit —…

## Deviations

- **pl-2** — the leader reviewed, tested and committed the worker's diff — the dispatched worker timed out idle before committing — hit an unforeseen obstacle
- **pl-3** — the worker also reworded the pl-2 commit message by amend, code unchanged, sha ecc268c became 2687f82 — it tidied the earlier commit beside its own — something else had to be fixed first

## Provenance

Proposed by `bee knowledge promote --work piggery-lessons` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/piggery-lessons/CONTEXT.md`, `docs/history/piggery-lessons/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.
