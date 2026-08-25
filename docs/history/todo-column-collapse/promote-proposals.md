promote proposal for work item "todo-column-collapse" (.bee/logs/scribing-runs.jsonl + .bee/lanes/todo-column-collapse.json) — 1 capped cell(s): tcc-1
anchor: ledger — .bee/logs/scribing-runs.jsonl, .bee/lanes/todo-column-collapse.json
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/todo-column-collapse/delivery.md

---
type: bee.delivery
title: todo-column-collapse — delivery
description: "Delivery record proposed by bee knowledge promote for work item todo-column-collapse: 1 capped cell(s), 3 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: todo-column-collapse-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [.bee/logs/scribing-runs.jsonl, .bee/lanes/todo-column-collapse.json]
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/todo-column-collapse.json, .bee/cells/tcc-1.json]
---

# todo-column-collapse — Delivery

## What shipped

- **tcc-1** — The board's Todo column now ships folded behind a native disclosure on both boards (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **tcc-1** — `cargo test -p waggledance bee_hub`

## Deviations

- **tcc-1** — Left the change uncommitted: views.rs already carried unrelated in-flight edits, and one commit per cell cannot be honoured without sweeping them in.
- **tcc-1** — No commit: views.rs held substantial unrelated uncommitted work before this cell started, so committing the file would have swept it in; left staged-free for the user to split.
- **tcc-1** — cargo fmt normalised one pre-existing unformatted assertion at views.rs:16566 that belongs to that in-flight work.

## Provenance

Proposed by `bee knowledge promote --work todo-column-collapse` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/todo-column-collapse.json`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "todo-column-collapse" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-25T11:22:37.115Z), the work item declares no bee.areas.

area bee-cockpit:
  (no capped behavior_change cell exists for this feature)

area web-interface:
  (no capped behavior_change cell exists for this feature)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell tcc-1 — save as docs/knowledge/patterns/todo-column-collapse-tcc-1-pitfall.md

---
type: bee.pattern
title: todo-column-collapse cell tcc-1 — pitfall candidate
description: "Pitfall candidate mined from cell tcc-1's capped trace: Left the change uncommitted: views.rs already carried unrelated in-flight edits, and one commit per cell cannot be honoured without sweeping them in."
timestamp: 2026-08-25
bee:
  id: todo-column-collapse-tcc-1-pitfall
  lifecycle: draft
  areas: [bee-cockpit, web-interface]
  sources: [.bee/cells/tcc-1.json]
  polarity: pitfall
---

# todo-column-collapse cell tcc-1 — pitfall candidate

## What the cell did

The board's Todo column now ships folded behind a native disclosure on both boards

## Recorded evidence (verbatim from .bee/cells/tcc-1.json)

- **deviation** — Left the change uncommitted: views.rs already carried unrelated in-flight edits, and one commit per cell cannot be honoured without sweeping them in.
- **deviation** — No commit: views.rs held substantial unrelated uncommitted work before this cell started, so committing the file would have swept it in; left staged-free for the user to split.
- **deviation** — cargo fmt normalised one pre-existing unformatted assertion at views.rs:16566 that belongs to that in-flight work.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 1 pattern candidate(s), 0 file(s) written.
---

## Resolution — 2026-08-25, nothing left to apply

Reviewed at feature close. All three sections are already covered; none is
applied from here.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/todo-column-collapse/delivery.md`
  already exists as a curated record and says more than this draft: why the
  column was the one worth folding, what the summary keeps, that nothing
  persists between visits, both boards changed together, the locked decision
  `99d0b579`, and the live-board confirmation against a running daemon. The
  draft would be a downgrade.
- **(b) Area updates** — nothing proposed. The generator found no capped
  `behavior_change` cell for either `bee-cockpit` or `web-interface`, so there
  is no candidate bullet to sync.
<!-- bee:not-a-deferral: this bullet names the already-active pattern "Deferring a commit on a contended file does not protect the boundary" and quotes its practice. The word "deferring" is that pattern's subject, not a promise to act later — the work it describes is done. -->
- **(c) Pattern candidate** — already promoted, not from here. The three
  recorded deviations are one phenomenon, and it landed as
  `docs/knowledge/patterns/deferring-a-commit-on-a-contended-file.md`
  (`lifecycle: active`), which cites this feature's delivery record as its
  source and generalizes past this cell into a practice: prefer a path-scoped
  commit of your own paths over deferring. Saving the draft candidate beside it
  would duplicate it at a lower quality.
<!-- /bee:not-a-deferral -->
