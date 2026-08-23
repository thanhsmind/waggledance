---
type: bee.delivery
title: board-declutter — delivery
description: "Delivery record for work item board-declutter: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-11
bee:
  id: board-declutter-delivery
  lifecycle: active
  areas: [bee-cockpit-board]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/board-declutter.json, .bee/cells/board-declutter-1.json]
---

# board-declutter — Delivery

## What shipped

- **board-declutter-1** — Removed stepper/KPI/velocity/attention/working-now from the bee board page; Feature Hub is now the first main section; retired/rewrote board-page tests that asserted the removed markup and added a hub-first regression test (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **board-declutter-1** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work board-declutter` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/board-declutter.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
