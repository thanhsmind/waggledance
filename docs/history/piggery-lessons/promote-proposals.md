promote proposal for work item "piggery-lessons" (docs/history/piggery-lessons/CONTEXT.md + docs/history/piggery-lessons/plan.md) — 3 capped cell(s): pl-1, pl-2, pl-3
anchor: history — docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/piggery-lessons/delivery.md

---
type: bee.delivery
title: piggery-lessons — delivery
description: "Delivery record proposed by bee knowledge promote for work item piggery-lessons: 3 capped cell(s), 3 recorded deviation(s)."
timestamp: 2026-10-16
bee:
  id: piggery-lessons-delivery
  lifecycle: active
  areas: [orchestration]
  required_context: [docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md]
  sources: [docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md, .bee/cells/pl-1.json, .bee/cells/pl-2.json, .bee/cells/pl-3.json]
---

# piggery-lessons — Delivery

## What shipped

- **pl-1** — Copied the three Paseo 0.11.2 after-permission captures into testdata/fixtures/paseo-0.11.2/ with <ID3> substitution, added their README rows, and added a fixture test asserting running/no-pending after allow, idle turn-end with PERM-DONE, and Permission on a second permission. (5 file(s) changed)
- **pl-2** — Board shows Paseo leaders blocked on a permission or lost within the hour: /api/board leader_waiting and one home rail pill per project, from the runs table (4 file(s) changed)
- **pl-3** — Migration 6 adds runs.notice/notice_state/notice_until; finish_run writes status, transcript and notice in one statement and the delivery flush reads, sends and marks notices from the store, so a caller notice survives a restart (commit a9006c6). (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **pl-1** — `cargo test --profile fast -p waggledance paseo::` — cargo test --profile fast -p waggledance paseo:: — green:unit — the cell's verify command; 34 passed 0 failed, including the new paseo::tests::the_after_permission_wait_ends_at_the_turn_not_the_allow
- **pl-2** — `cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance board` — the cell verify: 516 core + 53 board-filtered; the full waggledance run (1201 passed) also covers the rail pill test the board filter misses
- **pl-3** — `cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance delivery` — PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance delivery && cargo test --profile fast --workspace — green:unit —…

## Deviations

- **pl-2** — the leader reviewed, tested and committed the worker's diff — the dispatched worker timed out idle before committing — hit an unforeseen obstacle
- **pl-2** — worker timed out idle before commit; leader finished review, test and commit
- **pl-3** — the worker also reworded the pl-2 commit message by amend, code unchanged, sha ecc268c became 2687f82 — it tidied the earlier commit beside its own — something else had to be fixed first

## Provenance

Proposed by `bee knowledge promote --work piggery-lessons` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/piggery-lessons/CONTEXT.md`, `docs/history/piggery-lessons/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "piggery-lessons" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-10-16T11:07:46.955Z), the work item declares no bee.areas.

area orchestration:
  (no capped behavior_change cell exists for this feature)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell pl-2 — save as docs/knowledge/patterns/piggery-lessons-pl-2-pitfall.md

---
type: bee.pattern
title: piggery-lessons cell pl-2 — pitfall candidate
description: "Pitfall candidate mined from cell pl-2's capped trace: the leader reviewed, tested and committed the worker's diff — the dispatched worker timed out idle before committing — hit an unforeseen obstacle"
timestamp: 2026-10-16
bee:
  id: piggery-lessons-pl-2-pitfall
  lifecycle: draft
  areas: [orchestration]
  sources: [.bee/cells/pl-2.json]
  polarity: pitfall
---

# piggery-lessons cell pl-2 — pitfall candidate

## What the cell did

Board shows Paseo leaders blocked on a permission or lost within the hour: /api/board leader_waiting and one home rail pill per project, from the runs table

## Recorded evidence (verbatim from .bee/cells/pl-2.json)

- **deviation** — the leader reviewed, tested and committed the worker's diff — the dispatched worker timed out idle before committing — hit an unforeseen obstacle
- **deviation** — worker timed out idle before commit; leader finished review, test and commit

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell pl-3 — save as docs/knowledge/patterns/piggery-lessons-pl-3-pitfall.md

---
type: bee.pattern
title: piggery-lessons cell pl-3 — pitfall candidate
description: "Pitfall candidate mined from cell pl-3's capped trace: the worker also reworded the pl-2 commit message by amend, code unchanged, sha ecc268c became 2687f82 — it tidied the earlier commit beside its own — something…"
timestamp: 2026-10-16
bee:
  id: piggery-lessons-pl-3-pitfall
  lifecycle: draft
  areas: [orchestration]
  sources: [.bee/cells/pl-3.json]
  polarity: pitfall
---

# piggery-lessons cell pl-3 — pitfall candidate

## What the cell did

Migration 6 adds runs.notice/notice_state/notice_until; finish_run writes status, transcript and notice in one statement and the delivery flush reads, sends and marks notices from the store, so a caller notice survives a restart (commit a9006c6).

## Recorded evidence (verbatim from .bee/cells/pl-3.json)

- **deviation** — the worker also reworded the pl-2 commit message by amend, code unchanged, sha ecc268c became 2687f82 — it tidied the earlier commit beside its own — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 3 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 2 pattern candidate(s), 0 file(s) written.