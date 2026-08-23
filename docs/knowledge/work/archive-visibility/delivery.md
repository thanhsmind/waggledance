---
type: bee.delivery
title: archive-visibility — delivery
description: "Delivery record for work item archive-visibility: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-11
bee:
  id: archive-visibility-delivery
  lifecycle: active
  areas: [bee-cockpit-board]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/archive-visibility.json, .bee/cells/archive-visibility-1.json]
---

# archive-visibility — Delivery

## What shipped

- **archive-visibility-1** — Feature and cell detail pages now merge archived cells (Closed header with done count); main board KPIs/buckets stay archive-free (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **archive-visibility-1** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work archive-visibility` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/archive-visibility.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
