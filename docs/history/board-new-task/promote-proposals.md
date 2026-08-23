promote proposal for work item "board-new-task" (docs/history/board-new-task/CONTEXT.md + docs/history/board-new-task/plan.md) — 1 capped cell(s): board-new-task-1
anchor: history — docs/history/board-new-task/CONTEXT.md, docs/history/board-new-task/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/board-new-task/delivery.md

---
type: bee.delivery
title: board-new-task — delivery
description: "Delivery record proposed by bee knowledge promote for work item board-new-task: 1 capped cell(s), 5 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: board-new-task-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [docs/history/board-new-task/CONTEXT.md, docs/history/board-new-task/plan.md]
  sources: [docs/history/board-new-task/CONTEXT.md, docs/history/board-new-task/plan.md, .bee/cells/archive/board-new-task/board-new-task-1.json]
---

# board-new-task — Delivery

## What shipped

- **board-new-task-1** — POST /api/projects/:id/pbi files the home board's + New task dialog into a project's backlog through that project's own bee (4 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **board-new-task-1** — `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Deviations

- **board-new-task-1** — home_page has no server-side selected-project input, so the select preselects the first top-level project and app.js narrows it to the rail-filtered one when the filter has picked out a single row
- **board-new-task-1** — no regex crate is a dependency, so parse_pbi_id fallback is a hand-written id scan rather than a regex
- **board-new-task-1** — narrowed two pre-existing page-wide assertions (no raw error text with herdr down; no select on the terminals tab) via a without_new_task_dialog test helper — the dialog carries both tokens as chrome
- **board-new-task-1** — registered my own worker record with bee state worker add: dispatch had left both it and the file reservations unset, and cells finish refused without it
- **board-new-task-1** — probed bee backlog pbi add against the worktree own .bee while scouting, creating PBI p-ee25b0ff; retracted through the CLI with pbi status --to declined (the file is gitignored and not in the commit)

## Provenance

Proposed by `bee knowledge promote --work board-new-task` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-new-task/CONTEXT.md`, `docs/history/board-new-task/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "board-new-task" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-23T01:03:08.099Z), the work item declares no bee.areas.

area bee-cockpit:
  - [board-new-task-1] POST /api/projects/:id/pbi files the home board's + New task dialog into a project's backlog through that project's own bee — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/archive/board-new-task/board-new-task-1.json)

area web-interface:
  - [board-new-task-1] POST /api/projects/:id/pbi files the home board's + New task dialog into a project's backlog through that project's own bee — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/archive/board-new-task/board-new-task-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell board-new-task-1 — save as docs/knowledge/patterns/board-new-task-board-new-task-1-pitfall.md

---
type: bee.pattern
title: board-new-task cell board-new-task-1 — pitfall candidate
description: "Pitfall candidate mined from cell board-new-task-1's capped trace: home_page has no server-side selected-project input, so the select preselects the first top-level project and app.js narrows it to the rail-filtered one when t…"
timestamp: 2026-08-22
bee:
  id: board-new-task-board-new-task-1-pitfall
  lifecycle: draft
  areas: [bee-cockpit, web-interface]
  sources: [.bee/cells/archive/board-new-task/board-new-task-1.json]
  polarity: pitfall
---

# board-new-task cell board-new-task-1 — pitfall candidate

## What the cell did

POST /api/projects/:id/pbi files the home board's + New task dialog into a project's backlog through that project's own bee

## Recorded evidence (verbatim from .bee/cells/archive/board-new-task/board-new-task-1.json)

- **deviation** — home_page has no server-side selected-project input, so the select preselects the first top-level project and app.js narrows it to the rail-filtered one when the filter has picked out a single row
- **deviation** — no regex crate is a dependency, so parse_pbi_id fallback is a hand-written id scan rather than a regex
- **deviation** — narrowed two pre-existing page-wide assertions (no raw error text with herdr down; no select on the terminals tab) via a without_new_task_dialog test helper — the dialog carries both tokens as chrome
- **deviation** — registered my own worker record with bee state worker add: dispatch had left both it and the file reservations unset, and cells finish refused without it
- **deviation** — probed bee backlog pbi add against the worktree own .bee while scouting, creating PBI p-ee25b0ff; retracted through the CLI with pbi status --to declined (the file is gitignored and not in the commit)

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 2 area bullet(s), 1 pattern candidate(s), 0 file(s) written.