promote proposal for work item "term-keys-grid" (docs/history/term-keys-grid/CONTEXT.md) — 1 capped cell(s): tkg-1
anchor: history — docs/history/term-keys-grid/CONTEXT.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/term-keys-grid/delivery.md

---
type: bee.delivery
title: term-keys-grid — delivery
description: "Delivery record proposed by bee knowledge promote for work item term-keys-grid: 1 capped cell(s), 2 recorded deviation(s)."
timestamp: 2026-08-28
bee:
  id: term-keys-grid-delivery
  lifecycle: active
  required_context: [docs/history/term-keys-grid/CONTEXT.md]
  sources: [docs/history/term-keys-grid/CONTEXT.md, .bee/cells/tkg-1.json]
---

# term-keys-grid — Delivery

## What shipped

- **tkg-1** — Merged the two term-keys button groups into one 2x6 grid (Esc/Tab/Ctrl/Up/Shift/Ctrl+C, Alt/Paste/Left/Down/Right/Enter) with one-shot latching modifiers and a clipboard-backed Paste button (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **tkg-1** — `cargo test -p waggledance`

## Deviations

- **tkg-1** — extended the fix to two pinned tests the cell brief did not name by line number (views.rs terminals_tab_renders_the_switcher_pane_card_and_history_controls at ~12007, which asserted the literal class term-keys--move) — found while grepping for every remaining reference to the removed modifier class after the named tests were fixed — found a better route
- **tkg-1** — resumed after two blocked turns caused by a session-to-lane binding stuck on paseo-support instead of term-keys-grid; the coordinator rebound the session from the main checkout and the write-guard opened cleanly — hit an unforeseen obstacle, now resolved before any edit landed

## Provenance

Proposed by `bee knowledge promote --work term-keys-grid` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/term-keys-grid/CONTEXT.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell tkg-1 — save as docs/knowledge/patterns/term-keys-grid-tkg-1-pitfall.md

---
type: bee.pattern
title: term-keys-grid cell tkg-1 — pitfall candidate
description: "Pitfall candidate mined from cell tkg-1's capped trace: extended the fix to two pinned tests the cell brief did not name by line number (views.rs terminals_tab_renders_the_switcher_pane_card_and_history_controls at …"
timestamp: 2026-08-28
bee:
  id: term-keys-grid-tkg-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/tkg-1.json]
  polarity: pitfall
---

# term-keys-grid cell tkg-1 — pitfall candidate

## What the cell did

Merged the two term-keys button groups into one 2x6 grid (Esc/Tab/Ctrl/Up/Shift/Ctrl+C, Alt/Paste/Left/Down/Right/Enter) with one-shot latching modifiers and a clipboard-backed Paste button

## Recorded evidence (verbatim from .bee/cells/tkg-1.json)

- **deviation** — extended the fix to two pinned tests the cell brief did not name by line number (views.rs terminals_tab_renders_the_switcher_pane_card_and_history_controls at ~12007, which asserted the literal class term-keys--move) — found while grepping for every remaining reference to the removed modifier class after the named tests were fixed — found a better route
- **deviation** — resumed after two blocked turns caused by a session-to-lane binding stuck on paseo-support instead of term-keys-grid; the coordinator rebound the session from the main checkout and the write-guard opened cleanly — hit an unforeseen obstacle, now resolved before any edit landed

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 1 pattern candidate(s), 0 file(s) written.