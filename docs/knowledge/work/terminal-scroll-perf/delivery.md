---
type: bee.delivery
title: terminal-scroll-perf — delivery
description: "Delivery record for work item terminal-scroll-perf: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-12
bee:
  id: terminal-scroll-perf-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/terminal-scroll-perf.json, .bee/cells/terminal-scroll-perf-1.json]
---

# terminal-scroll-perf — Delivery

## What shipped

- **terminal-scroll-perf-1** — rAF-throttled resize refit with width-gating, two-read fitScreenFont, and .term-screen scroll hints pinned by a server.rs test (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **terminal-scroll-perf-1** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work terminal-scroll-perf` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/terminal-scroll-perf.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
