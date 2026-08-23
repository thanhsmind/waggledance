---
type: bee.delivery
title: scroll-fab-clears-tabbar — delivery
description: "Delivery record for work item scroll-fab-clears-tabbar: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: scroll-fab-clears-tabbar-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/cells/sf-1.json]
---

# scroll-fab-clears-tabbar — Delivery

## What shipped

- **sf-1** — Scroll column clears the handset tab bar (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **sf-1** — `cargo test -p waggledance the_home_shell_collapses`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work scroll-fab-clears-tabbar` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run after checking each line against the shipped source and the area specs.
