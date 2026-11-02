promote proposal for work item "usage-paseo-projects" (.bee/lanes/usage-paseo-projects.json) — 1 capped cell(s): upp-1
anchor: ledger — .bee/lanes/usage-paseo-projects.json
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/usage-paseo-projects/delivery.md

---
type: bee.delivery
title: usage-paseo-projects — delivery
description: "Delivery record proposed by bee knowledge promote for work item usage-paseo-projects: 1 capped cell(s), 1 recorded deviation(s)."
timestamp: 2026-10-23
bee:
  id: usage-paseo-projects-delivery
  lifecycle: active
  required_context: [.bee/lanes/usage-paseo-projects.json]
  sources: [.bee/lanes/usage-paseo-projects.json, .bee/cells/upp-1.json]
---

# usage-paseo-projects — Delivery

## What shipped

- **upp-1** — List only Paseo-managed projects on the Usage tab (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **upp-1** — `cargo test -p waggledance-core agent_usage && cargo test -p waggledance usage` — 16 + 8 passed over the store reader and usage_summary; full suites not run, fmt clean, clippy adds no new warning; live check follows merge

## Deviations

- **upp-1** — New tests were written after the code, never watched red; the unmanaged-project assertion fails by construction without the filter

## Provenance

Proposed by `bee knowledge promote --work usage-paseo-projects` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/lanes/usage-paseo-projects.json`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell upp-1 — save as docs/knowledge/patterns/usage-paseo-projects-upp-1-pitfall.md

---
type: bee.pattern
title: usage-paseo-projects cell upp-1 — pitfall candidate
description: "Pitfall candidate mined from cell upp-1's capped trace: New tests were written after the code, never watched red; the unmanaged-project assertion fails by construction without the filter"
timestamp: 2026-10-23
bee:
  id: usage-paseo-projects-upp-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/upp-1.json]
  polarity: pitfall
---

# usage-paseo-projects cell upp-1 — pitfall candidate

## What the cell did

List only Paseo-managed projects on the Usage tab

## Recorded evidence (verbatim from .bee/cells/upp-1.json)

- **deviation** — New tests were written after the code, never watched red; the unmanaged-project assertion fails by construction without the filter

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 1 pattern candidate(s), 0 file(s) written.