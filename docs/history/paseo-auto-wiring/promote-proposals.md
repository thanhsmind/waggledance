promote proposal for work item "paseo-auto-wiring" (docs/history/paseo-auto-wiring/CONTEXT.md + docs/history/paseo-auto-wiring/plan.md) — 4 capped cell(s): paw-1, paw-2, paw-3, paw-4
anchor: history — docs/history/paseo-auto-wiring/CONTEXT.md, docs/history/paseo-auto-wiring/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/paseo-auto-wiring/delivery.md

---
type: bee.delivery
title: paseo-auto-wiring — delivery
description: "Delivery record proposed by bee knowledge promote for work item paseo-auto-wiring: 4 capped cell(s), 8 recorded deviation(s)."
timestamp: 2026-10-15
bee:
  id: paseo-auto-wiring-delivery
  lifecycle: active
  required_context: [docs/history/paseo-auto-wiring/CONTEXT.md, docs/history/paseo-auto-wiring/plan.md]
  sources: [docs/history/paseo-auto-wiring/CONTEXT.md, docs/history/paseo-auto-wiring/plan.md, .bee/cells/paw-1.json, .bee/cells/paw-2.json, .bee/cells/paw-3.json, .bee/cells/paw-4.json]
---

# paseo-auto-wiring — Delivery

## What shipped

- **paw-1** — New leaders always start with a provider (file, global, else first available+enabled from provider ls); none available requeues before the send; Paseo JSON error objects on stdout or stderr surface code and message (5 file(s) changed)
- **paw-2** — Projects opt into dispatch from orchestration = true in .waggledance.toml; refusal names settings page and the absolute file path plus parse warnings (4 file(s) changed)
- **paw-3** — Paseo workspace roots auto-register from the delivery task; leader workspace pick with reason; refusal removed (7 file(s) changed)
- **paw-4** — Concept, README and supervisor template document zero-config Paseo wiring; repo opts in via .waggledance.toml (5 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **paw-1** — `cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance setup && cargo test --profile fast -p waggledance-core config` — cell verify (41+11+27 passed); also full cargo test --profile fast -p waggledance 1198 passed 2 ignored, clippy -D warnings on both crates and fmt --check clean
- **paw-2** — `cargo test --profile fast -p waggledance-core project_settings && cargo test --profile fast -p waggledance-core orchestration` — loader and engine opt-in gate are what changed; also ran cargo test -p waggledance dispatch (38), trigger (32), leader_dispatch (5), clippy -D warnings and fmt --check, all green; full suite not run
- **paw-3** — `cargo test --profile fast -p waggledance discovery && cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance delivery` — the cell verify over discovery, leader pick and delivery (4, 37 with 2 ignored real-host tests, 17 passed); also full cargo test --profile fast -p waggledance 1193 passed 2 ignored, clippy -D warning…
- **paw-4** — `cargo test --profile fast -p waggledance doctor` — the supervisor template is include_str! into doctor.rs and its assertions (incl. exactly two fences) live in the doctor tests; concept and README TOML examples plus the repo file were parsed with pro…

## Deviations

- **paw-1** — MISSING_PROVIDER fixture saved as npm-run-missing-provider.err and exec now reads stderr for a JSON error object — paseo run writes that error to stderr with empty stdout — the plan was wrong about a fact
- **paw-1** — paseo run without --provider writes its MISSING_PROVIDER JSON to stderr with empty stdout, so the fixture is npm-run-missing-provider.err (not .out) and exec now pipes stderr and checks it for a JSON error object starting at a { line; documents still parse from stdout only
- **paw-1** — advisor consult skipped: no Agent tool in this worker
- **paw-3** — the Paseo store is reached through a new Paseo trait method stored_workspaces (PaseoCli reads $PASEO_HOME or ~/.paseo projects/workspaces.json, the fake holds a Vec) — tests inject the fixture with no parameter threaded through delivery — found a better route
- **paw-3** — FakePaseo gained workspaces_fail; two delivery tests used the removed two-workspace refusal as their pre-send failure and now fail through workspace ls instead — the plan was wrong about a fact
- **paw-3** — the transcript header is kept per run in delivery State and prefixed when the final transcript is written — no transcript header existed to extend — found a better route
- **paw-3** — discovery skip bounds (home, tmp) are a Bounds value on Delivery so tests can register roots under the system temp dir — tests run under /tmp, which D6 excludes — hit an unforeseen obstacle
- **paw-4** — regenerated docs/knowledge/patterns/index.md (reserved first) — the concept description changed — something else had to be fixed first

## Provenance

Proposed by `bee knowledge promote --work paseo-auto-wiring` from 4 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/paseo-auto-wiring/CONTEXT.md`, `docs/history/paseo-auto-wiring/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell paw-1 — save as docs/knowledge/patterns/paseo-auto-wiring-paw-1-pitfall.md

---
type: bee.pattern
title: paseo-auto-wiring cell paw-1 — pitfall candidate
description: "Pitfall candidate mined from cell paw-1's capped trace: MISSING_PROVIDER fixture saved as npm-run-missing-provider.err and exec now reads stderr for a JSON error object — paseo run writes that error to stderr with e…"
timestamp: 2026-10-15
bee:
  id: paseo-auto-wiring-paw-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/paw-1.json]
  polarity: pitfall
---

# paseo-auto-wiring cell paw-1 — pitfall candidate

## What the cell did

New leaders always start with a provider (file, global, else first available+enabled from provider ls); none available requeues before the send; Paseo JSON error objects on stdout or stderr surface code and message

## Recorded evidence (verbatim from .bee/cells/paw-1.json)

- **deviation** — MISSING_PROVIDER fixture saved as npm-run-missing-provider.err and exec now reads stderr for a JSON error object — paseo run writes that error to stderr with empty stdout — the plan was wrong about a fact
- **deviation** — paseo run without --provider writes its MISSING_PROVIDER JSON to stderr with empty stdout, so the fixture is npm-run-missing-provider.err (not .out) and exec now pipes stderr and checks it for a JSON error object starting at a { line; documents still parse from stdout only
- **deviation** — advisor consult skipped: no Agent tool in this worker

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell paw-3 — save as docs/knowledge/patterns/paseo-auto-wiring-paw-3-pitfall.md

---
type: bee.pattern
title: paseo-auto-wiring cell paw-3 — pitfall candidate
description: "Pitfall candidate mined from cell paw-3's capped trace: the Paseo store is reached through a new Paseo trait method stored_workspaces (PaseoCli reads $PASEO_HOME or ~/.paseo projects/workspaces.json, the fake holds …"
timestamp: 2026-10-15
bee:
  id: paseo-auto-wiring-paw-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/paw-3.json]
  polarity: pitfall
---

# paseo-auto-wiring cell paw-3 — pitfall candidate

## What the cell did

Paseo workspace roots auto-register from the delivery task; leader workspace pick with reason; refusal removed

## Recorded evidence (verbatim from .bee/cells/paw-3.json)

- **deviation** — the Paseo store is reached through a new Paseo trait method stored_workspaces (PaseoCli reads $PASEO_HOME or ~/.paseo projects/workspaces.json, the fake holds a Vec) — tests inject the fixture with no parameter threaded through delivery — found a better route
- **deviation** — FakePaseo gained workspaces_fail; two delivery tests used the removed two-workspace refusal as their pre-send failure and now fail through workspace ls instead — the plan was wrong about a fact
- **deviation** — the transcript header is kept per run in delivery State and prefixed when the final transcript is written — no transcript header existed to extend — found a better route
- **deviation** — discovery skip bounds (home, tmp) are a Bounds value on Delivery so tests can register roots under the system temp dir — tests run under /tmp, which D6 excludes — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell paw-4 — save as docs/knowledge/patterns/paseo-auto-wiring-paw-4-pitfall.md

---
type: bee.pattern
title: paseo-auto-wiring cell paw-4 — pitfall candidate
description: "Pitfall candidate mined from cell paw-4's capped trace: regenerated docs/knowledge/patterns/index.md (reserved first) — the concept description changed — something else had to be fixed first"
timestamp: 2026-10-15
bee:
  id: paseo-auto-wiring-paw-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/paw-4.json]
  polarity: pitfall
---

# paseo-auto-wiring cell paw-4 — pitfall candidate

## What the cell did

Concept, README and supervisor template document zero-config Paseo wiring; repo opts in via .waggledance.toml

## Recorded evidence (verbatim from .bee/cells/paw-4.json)

- **deviation** — regenerated docs/knowledge/patterns/index.md (reserved first) — the concept description changed — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 4 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 3 pattern candidate(s), 0 file(s) written.