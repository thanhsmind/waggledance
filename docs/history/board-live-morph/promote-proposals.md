promote proposal for work item "board-live-morph" (docs/history/board-live-morph/CONTEXT.md) — 2 capped cell(s): blm-1, blm-2
anchor: history — docs/history/board-live-morph/CONTEXT.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/board-live-morph/delivery.md

---
type: bee.delivery
title: board-live-morph — delivery
description: "Delivery record proposed by bee knowledge promote for work item board-live-morph: 2 capped cell(s), 3 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: board-live-morph-delivery
  lifecycle: active
  required_context: [docs/history/board-live-morph/CONTEXT.md]
  sources: [docs/history/board-live-morph/CONTEXT.md, .bee/cells/blm-1.json, .bee/cells/blm-2.json]
---

# board-live-morph — Delivery

## What shipped

- **blm-1** — Added data-hub-key (the thing's own detail href) to the board card shell div and finished row anchor, keyed off one hoisted let so the key and link can never disagree (1 file(s) changed)
- **blm-2** — In-place board patch replaces the reload: keyed reconcile + FLIP in app.js, board CSS transitions in views.rs, two new server.rs pins (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **blm-1** — `cargo test -p waggledance`
- **blm-2** — `cargo test -p waggledance`

## Deviations

- **blm-1** — bee_hub_finished_row places data-hub-key AFTER href rather than after data-hub-group: two dozen pre-existing substring assertions pin the literal adjacency data-hub-group=X href= back-to-back on a row, and inserting between them would break all of them for no behavioural gain since the row href IS already the stable key value
- **blm-1** — fixed two whole-shell literal assertions and one shell-prefix substring assertion not named in the cells known-anchors list (bee_hub_card_emits_terminal_badges_matching_project_badges_markup_shape, bee_hub_card_with_no_panes_renders_no_badge_container, and the fg-card bee-hub__shell assert_eq at the with_no_project_label test) — the full test run surfaced them
- **blm-2** — Placed the new .bee-hub__shell/.bee-hub__row transition CSS after the two real rules of the same name rather than immediately beside .bee-hub__cards, because two existing tests locate those rules by first/second literal-substring match and an earlier occurrence of the same substring in a combined selector broke both; the CSS still lives inside bee_hub_style() as required.

## Provenance

Proposed by `bee knowledge promote --work board-live-morph` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-live-morph/CONTEXT.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell blm-1 — save as docs/knowledge/patterns/board-live-morph-blm-1-pitfall.md

---
type: bee.pattern
title: board-live-morph cell blm-1 — pitfall candidate
description: "Pitfall candidate mined from cell blm-1's capped trace: bee_hub_finished_row places data-hub-key AFTER href rather than after data-hub-group: two dozen pre-existing substring assertions pin the literal adjacency dat…"
timestamp: 2026-08-25
bee:
  id: board-live-morph-blm-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/blm-1.json]
  polarity: pitfall
---

# board-live-morph cell blm-1 — pitfall candidate

## What the cell did

Added data-hub-key (the thing's own detail href) to the board card shell div and finished row anchor, keyed off one hoisted let so the key and link can never disagree

## Recorded evidence (verbatim from .bee/cells/blm-1.json)

- **deviation** — bee_hub_finished_row places data-hub-key AFTER href rather than after data-hub-group: two dozen pre-existing substring assertions pin the literal adjacency data-hub-group=X href= back-to-back on a row, and inserting between them would break all of them for no behavioural gain since the row href IS already the stable key value
- **deviation** — fixed two whole-shell literal assertions and one shell-prefix substring assertion not named in the cells known-anchors list (bee_hub_card_emits_terminal_badges_matching_project_badges_markup_shape, bee_hub_card_with_no_panes_renders_no_badge_container, and the fg-card bee-hub__shell assert_eq at the with_no_project_label test) — the full test run surfaced them

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell blm-2 — save as docs/knowledge/patterns/board-live-morph-blm-2-pitfall.md

---
type: bee.pattern
title: board-live-morph cell blm-2 — pitfall candidate
description: "Pitfall candidate mined from cell blm-2's capped trace: Placed the new .bee-hub__shell/.bee-hub__row transition CSS after the two real rules of the same name rather than immediately beside .bee-hub__cards, because t…"
timestamp: 2026-08-25
bee:
  id: board-live-morph-blm-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/blm-2.json]
  polarity: pitfall
---

# board-live-morph cell blm-2 — pitfall candidate

## What the cell did

In-place board patch replaces the reload: keyed reconcile + FLIP in app.js, board CSS transitions in views.rs, two new server.rs pins

## Recorded evidence (verbatim from .bee/cells/blm-2.json)

- **deviation** — Placed the new .bee-hub__shell/.bee-hub__row transition CSS after the two real rules of the same name rather than immediately beside .bee-hub__cards, because two existing tests locate those rules by first/second literal-substring match and an earlier occurrence of the same substring in a combined selector broke both; the CSS still lives inside bee_hub_style() as required.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 2 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 2 pattern candidate(s), 0 file(s) written.