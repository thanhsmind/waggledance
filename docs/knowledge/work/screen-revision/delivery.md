---
type: bee.delivery
title: screen-revision — delivery
description: "Delivery record for work item screen-revision: a terminal pane's revision is derived from the text actually rendered, so a pane whose output moves is seen to move instead of freezing on its first frame."
timestamp: 2026-08-07
bee:
  id: screen-revision-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/specs/agent-terminal.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# screen-revision — Delivery

## What shipped

A terminal pane froze on its first frame. The page redraws only when the screen
it is showing reports a new revision, and that revision was being echoed
straight from the terminal multiplexer's own field rather than derived from what
the reader actually rendered. When that field did not move, the page concluded
nothing had changed and stopped repainting a pane whose output was in fact
scrolling.

The revision is now computed from the rendered text itself: same text, same
revision; different text, different revision. A pane whose output is moving is
therefore always seen to move, and a pane that is genuinely idle still reports a
stable revision, so the page does not repaint on every tick.

## Verify

`cargo test --workspace` green. The defect test was written first and watched
fail: a pane whose output changed between two reads must report different
revisions. Beside it — an unchanged pane reports the same revision (the redraw
suppression still works), an empty screen reports a stable value rather than
panicking or using zero as a sentinel, two different panes holding identical
text do not collide in a way that suppresses either one's updates, and the
unassigned group's screen endpoint behaves the same. Confirmed by eye against
the running daemon on a pane whose output was moving.

## Deviations

None recorded.

## Pointers

`revision_of` in `crates/waggledance-core/src/ansi.rs`. The cell trace names this
function under the crate's former name (`mdview_core`), retired by
`waggledance-rename`.

## Provenance

Written at bundle cleanup from the capped trace of `sr-1`; the crate path was
re-checked against the shipped source rather than copied from the trace.
