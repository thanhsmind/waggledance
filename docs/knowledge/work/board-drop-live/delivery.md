---
type: bee.delivery
title: board-drop-live — delivery
description: "Delivery record for work item board-drop-live: the cross-project board no longer carries a Live section; presence across many projects was noise, and the Live strip stays on the board of the project it belongs to."
timestamp: 2026-08-13
bee:
  id: board-drop-live-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# board-drop-live — Delivery

## What shipped

The board that spans every project carried its own Live section, listing what
was running everywhere at once. It read as noise: a reader scanning many
projects wants to know which features are moving, and a list of every live
session across all of them answers a question nobody was asking at the top of
that page.

The section is gone from the cross-project board — the rendering, its callers
and its tests, not merely hidden. Two things it could have taken with it stayed:
the Features listing on that same board, and the per-project board's own Live
strip, which earns its place because there the sessions being listed all belong
to the project the reader is already looking at.

## Verify

`cargo test --workspace` green, with the tests that pinned the removed section
deleted alongside it rather than left asserting a shape that no longer exists.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from the capped trace of `drop-live-1`. The rule it
established — deliberately no cross-project Live strip — is already stated in
`docs/specs/bee-cockpit.md`.
