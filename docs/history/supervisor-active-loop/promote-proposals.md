promote proposal for work item "supervisor-active-loop" (docs/history/supervisor-active-loop/plan.md) — 2 capped cell(s): sal-1, sal-2
anchor: history — docs/history/supervisor-active-loop/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/supervisor-active-loop/delivery.md

---
type: bee.delivery
title: supervisor-active-loop — delivery
description: "Delivery record proposed by bee knowledge promote for work item supervisor-active-loop: 2 capped cell(s), 4 recorded deviation(s)."
timestamp: 2026-10-18
bee:
  id: supervisor-active-loop-delivery
  lifecycle: active
  required_context: [docs/history/supervisor-active-loop/plan.md]
  sources: [docs/history/supervisor-active-loop/plan.md, .bee/cells/sal-1.json, .bee/cells/sal-2.json]
---

# supervisor-active-loop — Delivery

## What shipped

- **sal-1** — supervisor report runs openclaw agent into wd's main session (deliver telegram, json, never --local), CLI found from the openclaw-gateway APPDIR or [openclaw] cli; kind board; supervisor default sonnet (3 file(s) changed)
- **sal-2** — Supervisor doc, pattern doc and wd skill teach the active loop: report to wd, done report, advisor-first, reconcile and board (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **sal-1** — `cargo test --profile fast -p waggledance seat:: && cargo test --profile fast -p waggledance-core config::` — 24 and 28 passed, rerun by the leader; the worker also drove one live report (t-probe) that returned status ok via openclaw-cli; full suite reported green by the worker, not rerun by the leader
- **sal-2** — `cargo test --profile fast -p waggledance seat::` — embedded supervisor doc test the_embedded_doc_matches_the_repo_file plus seat tests, 22 passed, run against sal-1 uncommitted seat.rs/config.rs in the shared tree

## Deviations

- **sal-1** — CLI help in cli.rs still said webhook; the leader fixed it in e06cc0b — something else had to be fixed first
- **sal-2** — Routed the supervisor own questions (which project, unknown corr) through the advisor too, via one shared Asking an advisor section — the cell says every question goes to an advisor first — found a better route
- **sal-2** — Board report uses --project all --corr board — report takes free strings and the plan names no values — something else had to be fixed first
- **sal-2** — Pattern doc describes the CLI report path per plan.md since sal-1 owns code but not this spec — the plan was wrong about a fact

## Provenance

Proposed by `bee knowledge promote --work supervisor-active-loop` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/supervisor-active-loop/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell sal-1 — save as docs/knowledge/patterns/supervisor-active-loop-sal-1-pitfall.md

---
type: bee.pattern
title: supervisor-active-loop cell sal-1 — pitfall candidate
description: "Pitfall candidate mined from cell sal-1's capped trace: CLI help in cli.rs still said webhook; the leader fixed it in e06cc0b — something else had to be fixed first"
timestamp: 2026-10-18
bee:
  id: supervisor-active-loop-sal-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/sal-1.json]
  polarity: pitfall
---

# supervisor-active-loop cell sal-1 — pitfall candidate

## What the cell did

supervisor report runs openclaw agent into wd's main session (deliver telegram, json, never --local), CLI found from the openclaw-gateway APPDIR or [openclaw] cli; kind board; supervisor default sonnet

## Recorded evidence (verbatim from .bee/cells/sal-1.json)

- **deviation** — CLI help in cli.rs still said webhook; the leader fixed it in e06cc0b — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell sal-2 — save as docs/knowledge/patterns/supervisor-active-loop-sal-2-pitfall.md

---
type: bee.pattern
title: supervisor-active-loop cell sal-2 — pitfall candidate
description: "Pitfall candidate mined from cell sal-2's capped trace: Routed the supervisor own questions (which project, unknown corr) through the advisor too, via one shared Asking an advisor section — the cell says every quest…"
timestamp: 2026-10-18
bee:
  id: supervisor-active-loop-sal-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/sal-2.json]
  polarity: pitfall
---

# supervisor-active-loop cell sal-2 — pitfall candidate

## What the cell did

Supervisor doc, pattern doc and wd skill teach the active loop: report to wd, done report, advisor-first, reconcile and board

## Recorded evidence (verbatim from .bee/cells/sal-2.json)

- **deviation** — Routed the supervisor own questions (which project, unknown corr) through the advisor too, via one shared Asking an advisor section — the cell says every question goes to an advisor first — found a better route
- **deviation** — Board report uses --project all --corr board — report takes free strings and the plan names no values — something else had to be fixed first
- **deviation** — Pattern doc describes the CLI report path per plan.md since sal-1 owns code but not this spec — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 2 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 2 pattern candidate(s), 0 file(s) written.