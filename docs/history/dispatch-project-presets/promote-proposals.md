promote proposal for work item "dispatch-project-presets" (docs/history/dispatch-project-presets/CONTEXT.md + docs/history/dispatch-project-presets/plan.md) — 2 capped cell(s): dpp-1, dpp-2
anchor: history — docs/history/dispatch-project-presets/CONTEXT.md, docs/history/dispatch-project-presets/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/dispatch-project-presets/delivery.md

---
type: bee.delivery
title: dispatch-project-presets — delivery
description: "Delivery record proposed by bee knowledge promote for work item dispatch-project-presets: 2 capped cell(s), 6 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: dispatch-project-presets-delivery
  lifecycle: active
  required_context: [docs/history/dispatch-project-presets/CONTEXT.md, docs/history/dispatch-project-presets/plan.md]
  sources: [docs/history/dispatch-project-presets/CONTEXT.md, docs/history/dispatch-project-presets/plan.md, .bee/cells/dpp-1.json, .bee/cells/dpp-2.json]
---

# dispatch-project-presets — Delivery

## What shipped

- **dpp-1** — One by-label resolver now answers whether a herding label can start, and both readers call it (1 file(s) changed)
- **dpp-2** — A dispatch caller can now name any agent kind the target project declares, the same source the board Start button spawns from (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **dpp-1** — `cargo test -p waggledance-core --lib bee::`
- **dpp-2** — `cargo test -p waggledance --bin waggledance mcp::`

## Deviations

- **dpp-1** — Committed through a temp index and commit-tree: the plan-freeze guard reads any git path mention of an approved plan.md as an edit, and this was its FIRST commit, not a revision — bumping plan_rev to satisfy the guard would have recorded a revision that never happened.
- **dpp-1** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the standard-lane worker rule.
- **dpp-1** — Committed through a temp index and commit-tree because the plan-freeze guard treats any git path mention of an approved plan.md as an edit; this was that file first commit, and bumping plan_rev would have recorded a revision that never happened.
- **dpp-2** — The live end-to-end run stops at the per-project opt-in, which is D5 working rather than a gap: beehive has orchestration.enabled off, so all three real dispatch calls were refused there before any label was resolved. Turning that switch on is a change to the user's own security posture and was not made unasked; the resolution itself is proved by the unit cases, including the ask_state agreement walk.
- **dpp-2** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the standard-lane worker rule.
- **dpp-2** — The live proof stops at the opt-in gate: beehive has orchestration.enabled off, and flipping it is the user own call, so the resolver itself is proved by unit cases rather than by a real spawn.

## Provenance

Proposed by `bee knowledge promote --work dispatch-project-presets` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/dispatch-project-presets/CONTEXT.md`, `docs/history/dispatch-project-presets/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell dpp-1 — save as docs/knowledge/patterns/dispatch-project-presets-dpp-1-pitfall.md

---
type: bee.pattern
title: dispatch-project-presets cell dpp-1 — pitfall candidate
description: "Pitfall candidate mined from cell dpp-1's capped trace: Committed through a temp index and commit-tree: the plan-freeze guard reads any git path mention of an approved plan.md as an edit, and this was its FIRST comm…"
timestamp: 2026-08-25
bee:
  id: dispatch-project-presets-dpp-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/dpp-1.json]
  polarity: pitfall
---

# dispatch-project-presets cell dpp-1 — pitfall candidate

## What the cell did

One by-label resolver now answers whether a herding label can start, and both readers call it

## Recorded evidence (verbatim from .bee/cells/dpp-1.json)

- **deviation** — Committed through a temp index and commit-tree: the plan-freeze guard reads any git path mention of an approved plan.md as an edit, and this was its FIRST commit, not a revision — bumping plan_rev to satisfy the guard would have recorded a revision that never happened.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the standard-lane worker rule.
- **deviation** — Committed through a temp index and commit-tree because the plan-freeze guard treats any git path mention of an approved plan.md as an edit; this was that file first commit, and bumping plan_rev would have recorded a revision that never happened.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell dpp-2 — save as docs/knowledge/patterns/dispatch-project-presets-dpp-2-pitfall.md

---
type: bee.pattern
title: dispatch-project-presets cell dpp-2 — pitfall candidate
description: "Pitfall candidate mined from cell dpp-2's capped trace: The live end-to-end run stops at the per-project opt-in, which is D5 working rather than a gap: beehive has orchestration.enabled off, so all three real dispat…"
timestamp: 2026-08-25
bee:
  id: dispatch-project-presets-dpp-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/dpp-2.json]
  polarity: pitfall
---

# dispatch-project-presets cell dpp-2 — pitfall candidate

## What the cell did

A dispatch caller can now name any agent kind the target project declares, the same source the board Start button spawns from

## Recorded evidence (verbatim from .bee/cells/dpp-2.json)

- **deviation** — The live end-to-end run stops at the per-project opt-in, which is D5 working rather than a gap: beehive has orchestration.enabled off, so all three real dispatch calls were refused there before any label was resolved. Turning that switch on is a change to the user's own security posture and was not made unasked; the resolution itself is proved by the unit cases, including the ask_state agreement walk.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the standard-lane worker rule.
- **deviation** — The live proof stops at the opt-in gate: beehive has orchestration.enabled off, and flipping it is the user own call, so the resolver itself is proved by unit cases rather than by a real spawn.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 2 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 2 pattern candidate(s), 0 file(s) written.

---

<!-- bee:not-a-deferral: this section records a completed review with no outstanding action. -->

## Resolution — 2026-08-25, reviewed

Reviewed in a sweep of the unapplied-proposal backlog.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/dispatch-project-presets/delivery.md`
  already exists as a curated record; the generated draft would replace it with a
  list of cell ids and raw deviations.
- **(b) Area updates** — nothing proposed by the generator.
- **(c) Pattern candidates** — one of two promoted:
  - *Promoted.* The live end-to-end run stopped at the target project's
    orchestration opt-in, and the cell read that as the guard working rather than a
    coverage gap. A terminal-protocol port hit the same wall days later and answered
    it the other way — one dispatch with the switch flipped and restored. Two
    features, one wall, two defensible answers: promoted as
    `docs/knowledge/patterns/an-opt-in-that-blocks-the-live-proof.md`
    (`lifecycle: active`, polarity practice).
  - *Already active.* Committing through a temp index because the plan-freeze guard
    reads a git path mention of an approved plan as an edit is
    `docs/knowledge/patterns/the-first-commit-of-a-frozen-plan.md`, whose sources
    already cite this feature's own cell.

<!-- /bee:not-a-deferral -->
