---
type: bee.delivery
title: term-keys-one-row — delivery
description: "Delivery record for work item term-keys-one-row: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: term-keys-one-row-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/cells/tko-1.json]
---

# term-keys-one-row — Delivery

## What shipped

- **tko-1** — Terminal keys share one row on a handset (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **tko-1** — `cargo test -p waggledance -- term_key term_controls`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work term-keys-one-row` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run after checking each line against the shipped source and the area specs.
