promote proposal for work item "agent-feature-resolution" (.bee/lanes/agent-feature-resolution.json) — 1 capped cell(s): afr-1
anchor: ledger — .bee/lanes/agent-feature-resolution.json
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/agent-feature-resolution/delivery.md

---
type: bee.delivery
title: agent-feature-resolution — delivery
description: "Delivery record proposed by bee knowledge promote for work item agent-feature-resolution: 1 capped cell(s), 2 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: agent-feature-resolution-delivery
  lifecycle: active
  required_context: [.bee/lanes/agent-feature-resolution.json]
  sources: [.bee/lanes/agent-feature-resolution.json, .bee/cells/afr-1.json]
---

# agent-feature-resolution — Delivery

## What shipped

- **afr-1** — Live sessions resolve their feature from claim, worktree, lane, then record, and the resolved value is written back for panes, buckets and /api/agents (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **afr-1** — `cargo test -p waggledance --quiet — related server tests green`

## Deviations

- **afr-1** — Advisory cross-worktree hold on crates/waggledance/src/server.rs from checkout waggledance--wt--board-approve-actions (cell bap-1) — hook classed it advisory, not exclusive; proceeded and flag it for merge-time coordination
- **afr-1** — Updated project_bee_activity doc comment lines that said activity.feature decides the feature — they contradicted the cell after this change

## Provenance

Proposed by `bee knowledge promote --work agent-feature-resolution` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/lanes/agent-feature-resolution.json`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell afr-1 — save as docs/knowledge/patterns/agent-feature-resolution-afr-1-pitfall.md

---
type: bee.pattern
title: agent-feature-resolution cell afr-1 — pitfall candidate
description: "Pitfall candidate mined from cell afr-1's capped trace: Advisory cross-worktree hold on crates/waggledance/src/server.rs from checkout waggledance--wt--board-approve-actions (cell bap-1) — hook classed it advisory, …"
timestamp: 2026-08-23
bee:
  id: agent-feature-resolution-afr-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/afr-1.json]
  polarity: pitfall
---

# agent-feature-resolution cell afr-1 — pitfall candidate

## What the cell did

Live sessions resolve their feature from claim, worktree, lane, then record, and the resolved value is written back for panes, buckets and /api/agents

## Recorded evidence (verbatim from .bee/cells/afr-1.json)

- **deviation** — Advisory cross-worktree hold on crates/waggledance/src/server.rs from checkout waggledance--wt--board-approve-actions (cell bap-1) — hook classed it advisory, not exclusive; proceeded and flag it for merge-time coordination
- **deviation** — Updated project_bee_activity doc comment lines that said activity.feature decides the feature — they contradicted the cell after this change

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 1 pattern candidate(s), 0 file(s) written.

---

<!-- bee:not-a-deferral: this section records a completed review with no outstanding action. -->

## Resolution — 2026-08-25, reviewed

Reviewed in a sweep of the unapplied-proposal backlog.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/agent-feature-resolution/delivery.md`
  already exists as a curated record; the generated draft would replace it with a
  list of cell ids and raw deviations.
- **(b) Area updates** — nothing proposed by the generator.
- **(c) Pattern candidates** — none promoted. The cell's two recorded deviations are
  a cross-worktree hold that the hook classed advisory and the cell flagged for
  merge-time coordination — already covered by
  `docs/knowledge/patterns/the-lock-trail-names-the-owner.md` — and a doc-comment
  correction, which is a cell doing its job rather than a trap anyone can fall into.

<!-- /bee:not-a-deferral -->
