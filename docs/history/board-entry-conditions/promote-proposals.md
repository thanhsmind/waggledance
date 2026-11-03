promote proposal for work item "board-entry-conditions" (docs/history/board-entry-conditions/CONTEXT.md) — 1 capped cell(s): bec-1
anchor: history — docs/history/board-entry-conditions/CONTEXT.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/board-entry-conditions/delivery.md

---
type: bee.delivery
title: board-entry-conditions — delivery
description: "Delivery record proposed by bee knowledge promote for work item board-entry-conditions: 1 capped cell(s), 3 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: board-entry-conditions-delivery
  lifecycle: active
  required_context: [docs/history/board-entry-conditions/CONTEXT.md]
  sources: [docs/history/board-entry-conditions/CONTEXT.md, .bee/cells/bec-1.json]
---

# board-entry-conditions — Delivery

## What shipped

- **bec-1** — The board honours a project's whole declaration, so one entry no longer behaves two ways (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **bec-1** — `cargo test -p waggledance-core --lib bee:: && cargo test -p waggledance`

## Deviations

- **bec-1** — No live proof for this one: exercising it would mean a board Start, which opens a fresh worktree and a real agent run on the user's machine for a card they did not ask to start. The shared spawn path it now reaches was proved live in herding-entry-conditions (run-48e951cf2a67257a), and what this cell adds on top is the resolver, which the unit cases cover.
- **bec-1** — Ran inline rather than through a dispatched execution worker.
- **bec-1** — No live run: a board Start would open a worktree and start a real agent for a card the user did not ask to start; the spawn path underneath was proved live in the previous feature.

## Provenance

Proposed by `bee knowledge promote --work board-entry-conditions` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-entry-conditions/CONTEXT.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell bec-1 — save as docs/knowledge/patterns/board-entry-conditions-bec-1-pitfall.md

---
type: bee.pattern
title: board-entry-conditions cell bec-1 — pitfall candidate
description: "Pitfall candidate mined from cell bec-1's capped trace: No live proof for this one: exercising it would mean a board Start, which opens a fresh worktree and a real agent run on the user's machine for a card they did…"
timestamp: 2026-08-25
bee:
  id: board-entry-conditions-bec-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/bec-1.json]
  polarity: pitfall
---

# board-entry-conditions cell bec-1 — pitfall candidate

## What the cell did

The board honours a project's whole declaration, so one entry no longer behaves two ways

## Recorded evidence (verbatim from .bee/cells/bec-1.json)

- **deviation** — No live proof for this one: exercising it would mean a board Start, which opens a fresh worktree and a real agent run on the user's machine for a card they did not ask to start. The shared spawn path it now reaches was proved live in herding-entry-conditions (run-48e951cf2a67257a), and what this cell adds on top is the resolver, which the unit cases cover.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — No live run: a board Start would open a worktree and start a real agent for a card the user did not ask to start; the spawn path underneath was proved live in the previous feature.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 1 pattern candidate(s), 0 file(s) written.