---
type: bee.delivery
title: term-reply-composer — delivery
description: "Delivery record for work item term-reply-composer: the terminal reply widget is one chat-style composer card — borderless textarea on top, a small round + attach and a round action-colored ↑ Send (39px each) inside the card; Approve and Stage moved up into the soft-key grid."
timestamp: 2026-08-29
bee:
  id: term-reply-composer-delivery
  lifecycle: active
  areas: [agent-terminal, web-interface]
  required_context: [docs/history/term-reply-composer/CONTEXT.md]
  sources: [docs/history/term-reply-composer/CONTEXT.md]
---

# term-reply-composer — Delivery

## What shipped

The reply widget under a terminal pane used to be two separate blocks — a
bordered textarea box and a loose row of Approve/Stage/Send buttons below it.
It is now one bordered, rounded composer card modeled on a chat composer
(term-reply-composer D1): the borderless textarea sits on top, and the controls
row lives inside the card at the bottom. Every class name survived the
restructure, so the posting wiring in `assets/app.js` kept working against the
same selectors.

User testing then reshaped the controls twice. Approve and Stage left the
composer entirely and took the soft-key grid slots that Ctrl and Shift held —
row 1 of the grid is now Esc, Tab, Approve, ↑, Stage, Ctrl+C, and Alt is the
only latch modifier left (superseding the term-keys-grid decisions that placed
Ctrl/Shift there). Approve keeps its disabled/title gating; `app.js` finds both
buttons via `form.closest(".term-pane")` since they sit outside the form now.
The composer's bottom row keeps only two controls: the + attach button as a
small circle on the left and Send as a round, action-colored circle carrying an
↑ glyph (accessible name stays "Send" via aria-label). After two size rounds
(56px, 45px) both circles settled at 39px.

## Verify

`cargo test --profile fast -p waggledance` green at every cap — 912 passed, 0
failed — covering every rewritten reply/attach/grid markup and CSS assertion
plus the new nesting pins. Merged to main across four merges, final at 0977974.

## Deviations

Size settled iteratively through live UAT on the phone (2× → 45px → 39px)
rather than in one pass — each round its own capped cell and merge.
