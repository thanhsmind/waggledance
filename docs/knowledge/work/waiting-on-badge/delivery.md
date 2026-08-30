---
type: bee.delivery
title: waiting-on-badge — delivery
description: "Delivery record for work item waiting-on-badge: the kanban danger badge gated on a live waiting_on mark; run_state-only awaiting-approval reads Unreviewed."
timestamp: 2026-08-16
bee:
  id: waiting-on-badge-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/lanes/waiting-on-badge.json, .bee/cells/wob-1.json]
---

# waiting-on-badge — Delivery

## What shipped

- **wob-1** — Gate the Awaiting approval badge on a live waiting_on mark: `BeeState` parses `state.json`'s `waiting_on` into `waiting_on_live` (object with non-empty kind + subject; lenient mirror of bee's `waiting_on_is_live`), threaded to the hub card with the same active-feature gating as `run_state`; `run_state: awaiting-approval` renders the danger "Awaiting approval" chip only when the mark is live, and the neutral "Unreviewed" chip otherwise — bee derives awaiting-approval whenever any gate is pending with none later approved, and the user-invoked review gate routinely stays pending, so run_state alone must not claim a human is being waited on. Narrows kanban-live-signals D2; decision logged 2026-08-16. (2 file(s) changed)

  **Superseded 2026-08-30 by board-visibility bv-1.** "Non-empty kind + subject" was
  too lenient in one specific way: it accepted `kind: turn-end`, which AGENTS.md
  defines as the mark set at every ordinary turn end — control back with the human
  and *nothing owed*. So the idle mark read as a demand. Measured across the
  `state.json` files the board actually renders: **24 marks read live before the
  narrowing, 3 after**; every one of the 68 store-wide flips was `turn-end`, one of
  them carrying the subject "Không còn gì chờ bạn" — literally *nothing is waiting
  on you*. The live predicate is now a `kind` whitelist that excludes `turn-end` and
  keeps `gate`, `question` and any unrecognised kind (unknown stays live so a new
  kind surfaces rather than disappears). The lesson this entry already carried —
  *a derived signal must not claim a human is being waited on* — held; it was the
  definition of "live" underneath it that was wrong.

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **wob-1** — `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` (1064 passed)

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work waiting-on-badge` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/lanes/waiting-on-badge.json`. Applied 2026-08-16 from docs/history/waiting-on-badge/promote-proposals.md; proposal declared no area bullets and no pattern candidates.
