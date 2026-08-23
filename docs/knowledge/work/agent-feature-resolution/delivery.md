---
type: bee.delivery
title: agent-feature-resolution — delivery
description: "Delivery record for work item agent-feature-resolution: 1 capped cell(s), 2 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: agent-feature-resolution-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/cells/archive/agent-feature-resolution/afr-1.json]
---

# agent-feature-resolution — Delivery

## What shipped

A live session's feature is resolved once, by the strongest evidence first,
and that one value feeds everything that names a session's feature — the
terminal markers on board cards, the per-feature agent buckets, and the
agents listing:

1. the feature of the cell the session currently holds;
2. else the feature of the branch checkout the session's working directory
   sits in;
3. else the lane the session is bound to;
4. else whatever the session's own activity record says (which, for an
   unbound session, is the project's active feature).

Before this the record's own word (step 4) was the only source, so a session
working a feature's checkout without a binding, or holding a cell of another
feature, was filed under the project's active feature. Together with
board-pane-lane-pin (which pins a main-checkout marker to the session's
feature) this is what makes three sessions in one checkout land on three
different cards.

## Verify

`cargo test -p waggledance --quiet` green at 815; the touched crate's own
server tests cover the resolution order.

## Deviations

- The work proceeded under an advisory cross-checkout hold on the same source
  file from board-approve-actions; the hold is advisory by design (different
  checkouts), and the merge reconciled cleanly. Not promoted as a pattern —
  it is the hold mechanism working as specified, not a pitfall.
- Doc comments that said the activity record alone decides the feature were
  corrected in the same change.
