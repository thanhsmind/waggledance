---
type: bee.delivery
title: terminal-button-surface — delivery
description: "Delivery record for work item terminal-button-surface: the terminal's own controls sit on a defined surface instead of borrowing the page's, and the arrow keys share one row with the named reply keys."
timestamp: 2026-08-08
bee:
  id: terminal-button-surface-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/specs/agent-terminal.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# terminal-button-surface — Delivery

## What shipped

The buttons under a terminal pane had no surface of their own. They inherited
whatever the page background happened to be, which left them reading as loose
text on some themes and as a control strip on others.

They now draw on a named surface that the theme defines, so the strip reads as a
strip in every scheme, light or dark, without any button carrying a colour of
its own.

In the same pass the keys below the screen were folded into one row: the named
reply keys and the pane's own arrow keys share a single line rather than
stacking into two. The arrows stay visibly the pane's own — a reader is never
left guessing whether an arrow replies to the agent or moves within the screen.

## Verify

`cargo test --workspace` green.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from the capped trace of `terminal-button-surface-1`.
The single-row key layout is already stated in `docs/specs/agent-terminal.md`;
`term-keys-one-row` later revisited the same row.
