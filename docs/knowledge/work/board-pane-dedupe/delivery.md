---
type: bee.delivery
title: board-pane-dedupe — delivery
description: "Delivery record for work item board-pane-dedupe: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: board-pane-dedupe-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/cells/archive/board-pane-dedupe/board-pane-dedupe-1.json]
---

# board-pane-dedupe — Delivery

## What shipped

A board card never shows the same terminal twice. A terminal whose shell sits
in the project's main checkout while its foreground process runs inside a
feature's branch checkout counts as present in both places; after
board-pane-lane-pin the main-checkout share was keyed by the session's own
feature, which is the very feature the branch-checkout share already gave it,
so one terminal wore two identical markers. The branch-checkout marker is the
one kept. Decision logged 2026-08-23 (touches 3daa1ea7). *Reconciled
2026-08-23 (board-approve-actions):* the same feature join also selects the
one pane a board Approve/Reject reaches.

## Verify

`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D
warnings && cargo test -p waggledance` green at 811, up from 809; one new
test on the merge rule. Deployed to artifact.gogl.be the same day.

## Deviations

None recorded in the capped cell trace.
