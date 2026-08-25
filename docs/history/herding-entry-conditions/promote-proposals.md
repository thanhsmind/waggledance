promote proposal for work item "herding-entry-conditions" (docs/history/herding-entry-conditions/CONTEXT.md + docs/history/herding-entry-conditions/plan.md) — 3 capped cell(s): hec-1, hec-2, hec-3
anchor: history — docs/history/herding-entry-conditions/CONTEXT.md, docs/history/herding-entry-conditions/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/herding-entry-conditions/delivery.md

---
type: bee.delivery
title: herding-entry-conditions — delivery
description: "Delivery record proposed by bee knowledge promote for work item herding-entry-conditions: 3 capped cell(s), 10 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: herding-entry-conditions-delivery
  lifecycle: active
  required_context: [docs/history/herding-entry-conditions/CONTEXT.md, docs/history/herding-entry-conditions/plan.md]
  sources: [docs/history/herding-entry-conditions/CONTEXT.md, docs/history/herding-entry-conditions/plan.md, .bee/cells/hec-1.json, .bee/cells/hec-2.json, .bee/cells/hec-3.json]
---

# herding-entry-conditions — Delivery

## What shipped

- **hec-1** — waggledance reads both entry shapes bee declares, so agy-flash resolves — and the conditions stop at the spawn path (2 file(s) changed)
- **hec-2** — waggledance can seed a foreign trust store, adding one boundary-validated directory and nothing else (1 file(s) changed)
- **hec-3** — agy-flash starts for real: run-48e951cf2a67257a, and the trust store gained beehive and nothing else (4 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **hec-1** — `cargo test -p waggledance-core --lib bee::`
- **hec-2** — `cargo test -p waggledance --bin waggledance herdr::`
- **hec-3** — `cargo test -p waggledance`

## Deviations

- **hec-1** — The cell required the existing herding suite to pass UNEDITED. That was wrong as written: four tests pinned 'the object form does not resolve', which is exactly what D1 reverses, so they could not pass unchanged. They were updated with the reason recorded where they stood, and the rules underneath them re-asserted rather than dropped; the declared-but-unstartable case needed a new example since agy-flash is no longer one.
- **hec-1** — Ran inline rather than through a dispatched execution worker.
- **hec-1** — Four tests pinning the old refusal were updated rather than passing unedited: their premise was this reader being incomplete, not a rule. Reason recorded at each site; the rules they guarded are still asserted.
- **hec-2** — The seeder lives in herdr/mod.rs beside the spawn path rather than in a module of its own — CONTEXT.md left its home to the agent's discretion, and it has exactly one caller.
- **hec-2** — Ran inline rather than through a dispatched execution worker.
- **hec-2** — The seeder sits in herdr/mod.rs beside its single caller rather than in its own module.
- **hec-3** — server.rs was not in the cell's declared files but had to change: DispatchTarget::Spawn now carries the whole entry rather than a bare argv, so the board's two call sites had to follow. The board keeps its argv-only behaviour — widening it to the full entry shape is its own change, noted at the call site.
- **hec-3** — Ran inline rather than through a dispatched execution worker.
- **hec-3** — server.rs changed outside the declared file list because the Spawn variant now carries an entry; the board keeps argv-only behaviour.
- **hec-3** — Turned beehive orchestration.enabled on for one dispatch and off again immediately.

## Provenance

Proposed by `bee knowledge promote --work herding-entry-conditions` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/herding-entry-conditions/CONTEXT.md`, `docs/history/herding-entry-conditions/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell hec-1 — save as docs/knowledge/patterns/herding-entry-conditions-hec-1-pitfall.md

---
type: bee.pattern
title: herding-entry-conditions cell hec-1 — pitfall candidate
description: "Pitfall candidate mined from cell hec-1's capped trace: The cell required the existing herding suite to pass UNEDITED. That was wrong as written: four tests pinned 'the object form does not resolve', which is exactl…"
timestamp: 2026-08-25
bee:
  id: herding-entry-conditions-hec-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/hec-1.json]
  polarity: pitfall
---

# herding-entry-conditions cell hec-1 — pitfall candidate

## What the cell did

waggledance reads both entry shapes bee declares, so agy-flash resolves — and the conditions stop at the spawn path

## Recorded evidence (verbatim from .bee/cells/hec-1.json)

- **deviation** — The cell required the existing herding suite to pass UNEDITED. That was wrong as written: four tests pinned 'the object form does not resolve', which is exactly what D1 reverses, so they could not pass unchanged. They were updated with the reason recorded where they stood, and the rules underneath them re-asserted rather than dropped; the declared-but-unstartable case needed a new example since agy-flash is no longer one.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — Four tests pinning the old refusal were updated rather than passing unedited: their premise was this reader being incomplete, not a rule. Reason recorded at each site; the rules they guarded are still asserted.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell hec-2 — save as docs/knowledge/patterns/herding-entry-conditions-hec-2-pitfall.md

---
type: bee.pattern
title: herding-entry-conditions cell hec-2 — pitfall candidate
description: "Pitfall candidate mined from cell hec-2's capped trace: The seeder lives in herdr/mod.rs beside the spawn path rather than in a module of its own — CONTEXT.md left its home to the agent's discretion, and it has exac…"
timestamp: 2026-08-25
bee:
  id: herding-entry-conditions-hec-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/hec-2.json]
  polarity: pitfall
---

# herding-entry-conditions cell hec-2 — pitfall candidate

## What the cell did

waggledance can seed a foreign trust store, adding one boundary-validated directory and nothing else

## Recorded evidence (verbatim from .bee/cells/hec-2.json)

- **deviation** — The seeder lives in herdr/mod.rs beside the spawn path rather than in a module of its own — CONTEXT.md left its home to the agent's discretion, and it has exactly one caller.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — The seeder sits in herdr/mod.rs beside its single caller rather than in its own module.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell hec-3 — save as docs/knowledge/patterns/herding-entry-conditions-hec-3-pitfall.md

---
type: bee.pattern
title: herding-entry-conditions cell hec-3 — pitfall candidate
description: "Pitfall candidate mined from cell hec-3's capped trace: server.rs was not in the cell's declared files but had to change: DispatchTarget::Spawn now carries the whole entry rather than a bare argv, so the board's two…"
timestamp: 2026-08-25
bee:
  id: herding-entry-conditions-hec-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/hec-3.json]
  polarity: pitfall
---

# herding-entry-conditions cell hec-3 — pitfall candidate

## What the cell did

agy-flash starts for real: run-48e951cf2a67257a, and the trust store gained beehive and nothing else

## Recorded evidence (verbatim from .bee/cells/hec-3.json)

- **deviation** — server.rs was not in the cell's declared files but had to change: DispatchTarget::Spawn now carries the whole entry rather than a bare argv, so the board's two call sites had to follow. The board keeps its argv-only behaviour — widening it to the full entry shape is its own change, noted at the call site.
- **deviation** — Ran inline rather than through a dispatched execution worker.
- **deviation** — server.rs changed outside the declared file list because the Spawn variant now carries an entry; the board keeps argv-only behaviour.
- **deviation** — Turned beehive orchestration.enabled on for one dispatch and off again immediately.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 3 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 3 pattern candidate(s), 0 file(s) written.