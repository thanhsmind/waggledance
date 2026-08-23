---
type: bee.delivery
title: scroll-fab — delivery
description: "Delivery record for work item scroll-fab: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-12
bee:
  id: scroll-fab-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/scroll-fab.json, .bee/cells/scroll-fab-1.json]
---

# scroll-fab — Delivery

## What shipped

- **scroll-fab-1** — Reworked pane scroll controls into a round Older/Newer/Live column with Newer wired to the shared depth path (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **scroll-fab-1** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work scroll-fab` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/scroll-fab.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
