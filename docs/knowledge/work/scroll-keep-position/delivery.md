---
type: bee.delivery
title: scroll-keep-position — delivery
description: "Delivery record for work item scroll-keep-position: 3 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-12
bee:
  id: scroll-keep-position-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/scroll-keep-position.json, .bee/cells/scroll-keep-position-1.json, .bee/cells/scroll-keep-position-2.json, .bee/cells/scroll-keep-position-3.json]
---

# scroll-keep-position — Delivery

## What shipped

- **scroll-keep-position-1** — Made pane scroll depth stateful: PaneScroller::read_to_depth moves only the delta, AppState::scroll_tracker records per-pane depth with idle-TTL/live-restore/content-mismatch rails, Live button sends explicit history=0 (3 file(s) changed)
- **scroll-keep-position-2** — Normalised the mismatch comparison, fixed the depth-0 restore and idle-sweep gaps, serialised per-pane scroll ops, and added the five+ required proofs (3 file(s) changed)
- **scroll-keep-position-3** — Fixed the depth-0 payload shape, both failure-safe record clears, and the settle-wait short-circuit, with tests for all four; cargo test --workspace: 788 passed (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **scroll-keep-position-1** — `cargo test --workspace`
- **scroll-keep-position-2** — `cargo test --workspace`
- **scroll-keep-position-3** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work scroll-keep-position` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/scroll-keep-position.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
