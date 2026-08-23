---
type: bee.delivery
title: feature-hub — delivery
description: "Delivery record for work item feature-hub: 3 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-11
bee:
  id: feature-hub-delivery
  lifecycle: active
  areas: [bee-cockpit-board]
  required_context: []
  sources: [docs/history/feature-hub/CONTEXT.md, docs/history/feature-hub/plan.md, .bee/cells/feature-hub-1.json, .bee/cells/feature-hub-2.json, .bee/cells/feature-hub-3.json]
---

# feature-hub — Delivery

## What shipped

- **feature-hub-1** — Replaced the Kanban cell board with a Waiting on you / In Progress / Finished grouped feature list and applied the anthropic.com-inspired palette; cargo test --workspace green (752 passed) (3 file(s) changed)
- **feature-hub-2** — Feature detail page restructured into Activity/Todos/Sub-agents tabs with a chip row (lane, worktree+merge state, duration, cell count); mdview-core gained BeeCell.outcome/tests, BeeLane.route and feature_cell_span as read-only joins over already-open files; cargo test --workspace green (758 passed). (3 file(s) changed)
- **feature-hub-3** — Fixed Finished predicate (compounding-complete/archive-dir OR, not dead terminal string), corrected worktree-chip Merged/Main rule to require workspace evidence, and added regression tests for F1-F5 plus mdview-core outcome-scrub coverage; cargo test --workspace green (763 passed) (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **feature-hub-1** — `cargo test --workspace`
- **feature-hub-2** — `cargo test --workspace`
- **feature-hub-3** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work feature-hub` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/feature-hub/CONTEXT.md`, `docs/history/feature-hub/plan.md`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
