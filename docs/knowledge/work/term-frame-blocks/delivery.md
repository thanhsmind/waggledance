---
type: bee.delivery
title: term-frame-blocks — delivery
description: "Delivery record for work item term-frame-blocks: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-12
bee:
  id: term-frame-blocks-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/term-frame-blocks.json, .bee/cells/term-frame-blocks-1.json]
---

# term-frame-blocks — Delivery

## What shipped

- **term-frame-blocks-1** — Wrapped box-drawing frame runs in .term-frame divs, closing/reopening SGR spans across the boundary, so tables and TUI frames keep their grid on phones while prose still wraps (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **term-frame-blocks-1** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work term-frame-blocks` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/term-frame-blocks.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
