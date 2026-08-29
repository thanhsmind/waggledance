---
type: bee.delivery
title: term-keys-grid — delivery
description: "Delivery record for work item term-keys-grid: the terminal soft keys are one 2x6 grid with one-shot latching Ctrl/Shift/Alt modifiers, a clipboard-backed Paste button that fills the reply box, and a dedicated Ctrl+C interrupt."
timestamp: 2026-08-29
bee:
  id: term-keys-grid-delivery
  lifecycle: active
  areas: [agent-terminal, web-interface]
  required_context: [docs/history/term-keys-grid/CONTEXT.md]
  sources: [docs/history/term-keys-grid/CONTEXT.md]
---

# term-keys-grid — Delivery

## What shipped

The terminal page used to offer two separate soft-key clusters — an arrow pad
and a named-key row. They are now one 2×6 grid modeled on paseo's mobile key
bar (term-keys-grid D1): Esc, Tab, Ctrl, ↑, Shift, Ctrl+C across the top;
Alt, Paste, ←, ↓, →, Enter along the bottom. There is no keyboard-toggle
button; the freed slot keeps Ctrl+C as a dedicated interrupt, and the grid
renders everywhere the terminal control widget set does — the project terminal
tab and the unassigned panes view alike.

Ctrl, Shift and Alt are one-shot latches (term-keys-grid D2): tapping one arms
it (aria-pressed carries the visual state), tapping any plain key while armed
sends the single combined wire name — Ctrl then ↑ sends `ctrl+up` — and clears
the latch; tapping the armed modifier again disarms it. A second modifier
replaces the first, typed characters are never captured into a combo, and the
armed-Ctrl tap on the Ctrl+C button still sends plain `ctrl+c`, never a doubled
prefix. Bare keys keep their existing herdr wire names, so the server's `/keys`
route and herdr's key vocabulary are untouched.

Paste reads the clipboard and inserts into the reply textarea for review before
anything is sent — never directly into the pane — and renders disabled when the
clipboard read API is unavailable (term-keys-grid D3).

## Verify

`cargo test -p waggledance -- terminal` green at cap (158 terminal-scoped
tests, including every pinned markup/CSS/wiring assertion rewritten from the
old two-group layout to the grid), plus a clean `cargo build --profile fast
-p waggledance`. Merged to main at 2eb4708.

## Deviations

One pinned test outside the cell's named line numbers
(`terminals_tab_renders_the_switcher_pane_card_and_history_controls`) still
asserted the removed `term-keys--move` class and was updated in the same
commit — found by sweeping for every remaining reference to the removed class.
