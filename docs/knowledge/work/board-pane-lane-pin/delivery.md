---
type: bee.delivery
title: board-pane-lane-pin — delivery
description: "Delivery record for work item board-pane-lane-pin: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: board-pane-lane-pin-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/cells/archive/board-pane-lane-pin/board-pane-lane-pin-1.json]
---

# board-pane-lane-pin — Delivery

## What shipped

A board card's terminal markers are now decided first by the session, then by
the checkout. A session's own activity record names the feature it is working
(its bound lane, else the project's active feature), so a session that speaks
is marked on that one feature's card and on no other — even when it runs in the
project's main checkout beside sessions working other features. Only a terminal
no session claims (a shell, an agent bee never saw) still falls back to the
checkout rule from card-terminals: shared across every main-checkout feature of
the project.

Refines card-terminals, where every main-checkout session fanned out to every
Main feature: three Main features in one checkout each showed the same three
agents and the same "Showing 1 of 3 sessions" line. Decision 3d631a7a (a session
joins a board card by `activity.feature`) already named the rule; this work
applies it to the card markers. Decision logged 2026-08-23, touches 3d631a7a.

The markers keep their "terminals of this checkout" label: the unclaimed share
is still the checkout's, which is what keeps the label honest.

## Verify

`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
&& cargo test -p waggledance` green at 809, up from 806. Three new tests on the
keying rule: a bound pane lands on its own feature only while an unbound one
fans out; a pane bound to a feature outside the board's Main set still keys its
own feature; all-bound panes never fan out. Deployed to artifact.gogl.be the
same day.

## Deviations

None recorded in the capped cell trace.
