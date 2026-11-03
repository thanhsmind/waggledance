promote proposal for work item "pi-paseo-plugin" (docs/history/pi-paseo-plugin/CONTEXT.md + docs/history/pi-paseo-plugin/plan.md) — 4 capped cell(s): ppp-1, ppp-2, ppp-3, ppp-4
anchor: history — docs/history/pi-paseo-plugin/CONTEXT.md, docs/history/pi-paseo-plugin/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/pi-paseo-plugin/delivery.md

---
type: bee.delivery
title: pi-paseo-plugin — delivery
description: "Delivery record proposed by bee knowledge promote for work item pi-paseo-plugin: 4 capped cell(s), 6 recorded deviation(s)."
timestamp: 2026-10-06
bee:
  id: pi-paseo-plugin-delivery
  lifecycle: active
  required_context: [docs/history/pi-paseo-plugin/CONTEXT.md, docs/history/pi-paseo-plugin/plan.md]
  sources: [docs/history/pi-paseo-plugin/CONTEXT.md, docs/history/pi-paseo-plugin/plan.md, .bee/cells/ppp-1.json, .bee/cells/ppp-2.json, .bee/cells/ppp-3.json, .bee/cells/ppp-4.json]
---

# pi-paseo-plugin — Delivery

## What shipped

- **ppp-1** — GET /api/board serves in_progress and ready_to_merge JSON built from bee_classify_features, no paths (2 file(s) changed)
- **ppp-2** — pi extension in integrations/pi registers the waggledance MCP server (direct exposure), inert under WAGGLEDANCE_DISABLED=1 (4 file(s) changed)
- **ppp-3** — Paseo plugin waggledance: sidebar surface, workspace panel and settings over /api/board + /api/agents, read-only (14 file(s) changed)
- **ppp-4** — waggledance setup pi|paseo [--remove] [--json] installs the embedded pi extension and Paseo plugin idempotently, refusing directories without VERSION (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **ppp-1** — `cargo test --profile fast -p waggledance api_board` — the cell verify filter over the new route test (1 passed); feature_hub (21) also green as neighbor check, clippy --all-targets clean; full suite not run
- **ppp-2** — `node --test integrations/pi/index.test.ts` — the cell verify; 2/2 pass (registration with fake pi, disabled case); live pi load not run (worker-outward guard refuses starting pi), left to ppp-4
- **ppp-3** — `node --test integrations/paseo/test/server.test.ts && (cd integrations/paseo && npm install --no-audit --no-fund && npx tsc --noEmit)` — the cell verify: 9 server tests over real HTTP fixtures and a fake binary, plus tsc against @getpaseo/plugin 0.10.3; client rendering and a live Paseo install are not exercised (ppp-4 owns the live i…
- **ppp-4** — `cargo test --profile fast -p waggledance setup::` — 9 setup tests passed (full crate 1127 passed); the built binary /home/thanhsmind/.cache/cargo-target/fast/waggledance (content-confirmed via strings) was driven with PI_CODING_AGENT_DIR on a scratch …

## Deviations

- **ppp-1** — dropped the projects array from /api/board per the leader note — /api/projects already serves it — the plan was wrong about a fact
- **ppp-2** — skipped the optional live pi check — the worker-outward guard refuses running pi from a worker — hit an unforeseen obstacle
- **ppp-3** — added a never-answers timeout test and a missing-binary test beyond the listed truths — cheap coverage of the 5 s timeout and the bin fallback — found a better route
- **ppp-3** — AbortController+setTimeout instead of AbortSignal.timeout — react-native globals shadow AbortSignal without timeout under tsc — hit an unforeseen obstacle
- **ppp-4** — the embed drift test also excludes dotfiles besides test/, node_modules/, the lockfile and *.test.ts — integrations/paseo carries a .gitignore that is not a shipped file — the plan was wrong about a fact
- **ppp-4** — setup paseo runs paseo plugin ls before writing any file — a missing paseo binary then fails before touching ~/.waggledance/paseo — found a better route

## Provenance

Proposed by `bee knowledge promote --work pi-paseo-plugin` from 4 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/pi-paseo-plugin/CONTEXT.md`, `docs/history/pi-paseo-plugin/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell ppp-1 — save as docs/knowledge/patterns/pi-paseo-plugin-ppp-1-pitfall.md

---
type: bee.pattern
title: pi-paseo-plugin cell ppp-1 — pitfall candidate
description: "Pitfall candidate mined from cell ppp-1's capped trace: dropped the projects array from /api/board per the leader note — /api/projects already serves it — the plan was wrong about a fact"
timestamp: 2026-10-06
bee:
  id: pi-paseo-plugin-ppp-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/ppp-1.json]
  polarity: pitfall
---

# pi-paseo-plugin cell ppp-1 — pitfall candidate

## What the cell did

GET /api/board serves in_progress and ready_to_merge JSON built from bee_classify_features, no paths

## Recorded evidence (verbatim from .bee/cells/ppp-1.json)

- **deviation** — dropped the projects array from /api/board per the leader note — /api/projects already serves it — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ppp-2 — save as docs/knowledge/patterns/pi-paseo-plugin-ppp-2-pitfall.md

---
type: bee.pattern
title: pi-paseo-plugin cell ppp-2 — pitfall candidate
description: "Pitfall candidate mined from cell ppp-2's capped trace: skipped the optional live pi check — the worker-outward guard refuses running pi from a worker — hit an unforeseen obstacle"
timestamp: 2026-10-06
bee:
  id: pi-paseo-plugin-ppp-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/ppp-2.json]
  polarity: pitfall
---

# pi-paseo-plugin cell ppp-2 — pitfall candidate

## What the cell did

pi extension in integrations/pi registers the waggledance MCP server (direct exposure), inert under WAGGLEDANCE_DISABLED=1

## Recorded evidence (verbatim from .bee/cells/ppp-2.json)

- **deviation** — skipped the optional live pi check — the worker-outward guard refuses running pi from a worker — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ppp-3 — save as docs/knowledge/patterns/pi-paseo-plugin-ppp-3-pitfall.md

---
type: bee.pattern
title: pi-paseo-plugin cell ppp-3 — pitfall candidate
description: "Pitfall candidate mined from cell ppp-3's capped trace: added a never-answers timeout test and a missing-binary test beyond the listed truths — cheap coverage of the 5 s timeout and the bin fallback — found a better…"
timestamp: 2026-10-06
bee:
  id: pi-paseo-plugin-ppp-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/ppp-3.json]
  polarity: pitfall
---

# pi-paseo-plugin cell ppp-3 — pitfall candidate

## What the cell did

Paseo plugin waggledance: sidebar surface, workspace panel and settings over /api/board + /api/agents, read-only

## Recorded evidence (verbatim from .bee/cells/ppp-3.json)

- **deviation** — added a never-answers timeout test and a missing-binary test beyond the listed truths — cheap coverage of the 5 s timeout and the bin fallback — found a better route
- **deviation** — AbortController+setTimeout instead of AbortSignal.timeout — react-native globals shadow AbortSignal without timeout under tsc — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ppp-4 — save as docs/knowledge/patterns/pi-paseo-plugin-ppp-4-pitfall.md

---
type: bee.pattern
title: pi-paseo-plugin cell ppp-4 — pitfall candidate
description: "Pitfall candidate mined from cell ppp-4's capped trace: the embed drift test also excludes dotfiles besides test/, node_modules/, the lockfile and *.test.ts — integrations/paseo carries a .gitignore that is not a sh…"
timestamp: 2026-10-06
bee:
  id: pi-paseo-plugin-ppp-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/ppp-4.json]
  polarity: pitfall
---

# pi-paseo-plugin cell ppp-4 — pitfall candidate

## What the cell did

waggledance setup pi|paseo [--remove] [--json] installs the embedded pi extension and Paseo plugin idempotently, refusing directories without VERSION

## Recorded evidence (verbatim from .bee/cells/ppp-4.json)

- **deviation** — the embed drift test also excludes dotfiles besides test/, node_modules/, the lockfile and *.test.ts — integrations/paseo carries a .gitignore that is not a shipped file — the plan was wrong about a fact
- **deviation** — setup paseo runs paseo plugin ls before writing any file — a missing paseo binary then fails before touching ~/.waggledance/paseo — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 4 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 4 pattern candidate(s), 0 file(s) written.