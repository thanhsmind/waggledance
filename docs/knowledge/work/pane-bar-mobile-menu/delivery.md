---
type: bee.delivery
title: pane-bar-mobile-menu — delivery
description: "Delivery record for work item pane-bar-mobile-menu: on a narrow screen the pane row stays one line — the pane being viewed, with every other pane and the creation controls behind one menu."
timestamp: 2026-08-08
bee:
  id: pane-bar-mobile-menu-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/specs/agent-terminal.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# pane-bar-mobile-menu — Delivery

## What shipped

The row that switches between terminal panes listed every pane plus the controls
for making a new one. On a phone that row wrapped into several lines and pushed
the terminal itself down the page — the one thing the operator opened the page
to read.

Below the shared narrow-screen threshold the row is now always a single line: it
shows the pane being viewed, and one menu control holds everything else — every
other pane, and the controls that create one. Above the threshold the row is
unchanged, because there the full list fits and is faster to use than a menu.

## Verify

`cargo test --workspace` green.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from the capped trace of `pane-bar-mobile-menu-1`. The
behavior is already stated in `docs/specs/agent-terminal.md` under the shared
narrow-screen threshold, which this feature's row obeys.
