---
type: bee.delivery
title: feature-titles — delivery
description: "Delivery record for work item feature-titles: 2 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-11
bee:
  id: feature-titles-delivery
  lifecycle: active
  areas: [bee-cockpit-board]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/feature-titles.json, .bee/cells/feature-titles-1.json, .bee/cells/feature-titles-2.json]
---

# feature-titles — Delivery

## What shipped

- **feature-titles-1** — Human titles + descriptions from CONTEXT.md; docs links on feature detail (3 file(s) changed)
- **feature-titles-2** — Replaced the feature detail Sub-agents tab with a Terminal tab listing/linking the project's live agent-terminal panes (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **feature-titles-1** — `cargo test --workspace`
- **feature-titles-2** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work feature-titles` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/feature-titles.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
