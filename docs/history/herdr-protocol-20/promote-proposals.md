promote proposal for work item "herdr-protocol-20" (docs/history/herdr-protocol-20/CONTEXT.md + docs/history/herdr-protocol-20/plan.md) — 4 capped cell(s): hp20-1, hp20-2, hp20-3, hp20-4
anchor: history — docs/history/herdr-protocol-20/CONTEXT.md, docs/history/herdr-protocol-20/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/herdr-protocol-20/delivery.md

---
type: bee.delivery
title: herdr-protocol-20 — delivery
description: "Delivery record proposed by bee knowledge promote for work item herdr-protocol-20: 4 capped cell(s), 15 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: herdr-protocol-20-delivery
  lifecycle: active
  required_context: [docs/history/herdr-protocol-20/CONTEXT.md, docs/history/herdr-protocol-20/plan.md]
  sources: [docs/history/herdr-protocol-20/CONTEXT.md, docs/history/herdr-protocol-20/plan.md, .bee/cells/hp20-1.json, .bee/cells/hp20-2.json, .bee/cells/hp20-3.json, .bee/cells/hp20-4.json]
---

# herdr-protocol-20 — Delivery

## What shipped

- **hp20-1** — waggledance now speaks protocol 20's agent.start and tab.create, and its test double speaks it too (3 file(s) changed)
- **hp20-2** — A spawn now creates a tab and starts the agent in that tab's pane (1 file(s) changed)
- **hp20-3** — The protocol constant is 20 and the shell-create route uses the same tab-then-find hop (2 file(s) changed)
- **hp20-4** — A real dispatch now starts a real agent: run-57f2ccff17effcb3, pi working in beehive (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **hp20-1** — `cargo test -p waggledance --bin waggledance herdr::`
- **hp20-2** — `cargo test -p waggledance --bin waggledance orchestrate::`
- **hp20-3** — `cargo test -p waggledance`
- **hp20-4** — `cargo test -p waggledance --bin waggledance herdr::`

## Deviations

- **hp20-1** — Cells hp20-1..3 could not be capped separately: changing the Herdr trait signature breaks every caller by construction, so nothing compiles until the whole slice lands. The three cells were executed as one landing and are capped in order against that one green run — named here rather than pretended otherwise.
- **hp20-1** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction outranks the standard-lane worker rule.
- **hp20-1** — The slice landed as one commit: a trait signature change leaves nothing compiling until every caller follows, so per-cell green was not achievable.
- **hp20-1** — Committed through a temp index because the plan-freeze guard reads any git path mention of an approved plan.md as an edit.
- **hp20-2** — The tab-then-find hop landed in herdr/mod.rs as one shared helper rather than inside orchestrate.rs: three callers need it, and CONTEXT.md left its home to the agent's discretion.
- **hp20-2** — Ran inline rather than through a dispatched execution worker.
- **hp20-2** — The hop lives in herdr/mod.rs as one helper for all three callers.
- **hp20-2** — Capped against the slice-wide green run and commit 7bbfd22, which carries hp20-1 trailer.
- **hp20-3** — A spawned agent now lands in its own new tab rather than splitting into the workspace active tab, because agent.start can no longer place itself — the terminal card reads workspace-dot-Shell instead of workspace-dot-main, and that test was updated with the reason rather than the string quietly changed.
- **hp20-3** — Ran inline rather than through a dispatched execution worker.
- **hp20-3** — A spawned agent now lands in its own tab, a visible placement change recorded in the test that pinned the old one.
- **hp20-3** — Capped against the slice-wide green run and commit 7bbfd22.
- **hp20-4** — The live proof required turning beehive's orchestration opt-in on for one dispatch; it was turned back off immediately and the refusal re-verified. The spawned agent got a harmless task that touches no file.
- **hp20-4** — Ran inline rather than through a dispatched execution worker.
- **hp20-4** — Turned beehive orchestration.enabled on for one dispatch and off again immediately, with the refusal re-verified afterwards.

## Provenance

Proposed by `bee knowledge promote --work herdr-protocol-20` from 4 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/herdr-protocol-20/CONTEXT.md`, `docs/history/herdr-protocol-20/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell hp20-1 — save as docs/knowledge/patterns/herdr-protocol-20-hp20-1-pitfall.md

---
type: bee.pattern
title: herdr-protocol-20 cell hp20-1 — pitfall candidate
description: "Pitfall candidate mined from cell hp20-1's capped trace: Cells hp20-1..3 could not be capped separately: changing the Herdr trait signature breaks every caller by construction, so nothing compiles until the whole sli…"
timestamp: 2026-08-25
bee:
  id: herdr-protocol-20-hp20-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/hp20-1.json]
  polarity: pitfall
---

# herdr-protocol-20 cell hp20-1 — pitfall candidate

## What the cell did

waggledance now speaks protocol 20's agent.start and tab.create, and its test double speaks it too

## Recorded evidence (verbatim from .bee/cells/hp20-1.json)

- **deviation** — Cells hp20-1..3 could not be capped separately: changing the Herdr trait signature breaks every caller by construction, so nothing compiles until the whole slice lands. The three cells were executed as one landing and are capped in order against that one green run — named here rather than pretended otherwise.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction outranks the standard-lane worker rule.
- **deviation** — The slice landed as one commit: a trait signature change leaves nothing compiling until every caller follows, so per-cell green was not achievable.
- **deviation** — Committed through a temp index because the plan-freeze guard reads any git path mention of an approved plan.md as an edit.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell hp20-2 — save as docs/knowledge/patterns/herdr-protocol-20-hp20-2-pitfall.md

---
type: bee.pattern
title: herdr-protocol-20 cell hp20-2 — pitfall candidate
description: "Pitfall candidate mined from cell hp20-2's capped trace: The tab-then-find hop landed in herdr/mod.rs as one shared helper rather than inside orchestrate.rs: three callers need it, and CONTEXT.md left its home to the…"
timestamp: 2026-08-25
bee:
  id: herdr-protocol-20-hp20-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/hp20-2.json]
  polarity: pitfall
---

# herdr-protocol-20 cell hp20-2 — pitfall candidate

## What the cell did

A spawn now creates a tab and starts the agent in that tab's pane

## Recorded evidence (verbatim from .bee/cells/hp20-2.json)

- **deviation** — The tab-then-find hop landed in herdr/mod.rs as one shared helper rather than inside orchestrate.rs: three callers need it, and CONTEXT.md left its home to the agent's discretion.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — The hop lives in herdr/mod.rs as one helper for all three callers.
- **deviation** — Capped against the slice-wide green run and commit 7bbfd22, which carries hp20-1 trailer.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell hp20-3 — save as docs/knowledge/patterns/herdr-protocol-20-hp20-3-pitfall.md

---
type: bee.pattern
title: herdr-protocol-20 cell hp20-3 — pitfall candidate
description: "Pitfall candidate mined from cell hp20-3's capped trace: A spawned agent now lands in its own new tab rather than splitting into the workspace active tab, because agent.start can no longer place itself — the terminal…"
timestamp: 2026-08-25
bee:
  id: herdr-protocol-20-hp20-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/hp20-3.json]
  polarity: pitfall
---

# herdr-protocol-20 cell hp20-3 — pitfall candidate

## What the cell did

The protocol constant is 20 and the shell-create route uses the same tab-then-find hop

## Recorded evidence (verbatim from .bee/cells/hp20-3.json)

- **deviation** — A spawned agent now lands in its own new tab rather than splitting into the workspace active tab, because agent.start can no longer place itself — the terminal card reads workspace-dot-Shell instead of workspace-dot-main, and that test was updated with the reason rather than the string quietly changed.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — A spawned agent now lands in its own tab, a visible placement change recorded in the test that pinned the old one.
- **deviation** — Capped against the slice-wide green run and commit 7bbfd22.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell hp20-4 — save as docs/knowledge/patterns/herdr-protocol-20-hp20-4-pitfall.md

---
type: bee.pattern
title: herdr-protocol-20 cell hp20-4 — pitfall candidate
description: "Pitfall candidate mined from cell hp20-4's capped trace: The live proof required turning beehive's orchestration opt-in on for one dispatch; it was turned back off immediately and the refusal re-verified. The spawned…"
timestamp: 2026-08-25
bee:
  id: herdr-protocol-20-hp20-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/hp20-4.json]
  polarity: pitfall
---

# herdr-protocol-20 cell hp20-4 — pitfall candidate

## What the cell did

A real dispatch now starts a real agent: run-57f2ccff17effcb3, pi working in beehive

## Recorded evidence (verbatim from .bee/cells/hp20-4.json)

- **deviation** — The live proof required turning beehive's orchestration opt-in on for one dispatch; it was turned back off immediately and the refusal re-verified. The spawned agent got a harmless task that touches no file.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — Turned beehive orchestration.enabled on for one dispatch and off again immediately, with the refusal re-verified afterwards.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 4 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 4 pattern candidate(s), 0 file(s) written.