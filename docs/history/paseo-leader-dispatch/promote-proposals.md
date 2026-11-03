promote proposal for work item "paseo-leader-dispatch" (docs/history/paseo-leader-dispatch/CONTEXT.md + docs/history/paseo-leader-dispatch/plan.md) — 5 capped cell(s): pld-1, pld-2, pld-3, pld-4, pld-5
anchor: history — docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/paseo-leader-dispatch/delivery.md

---
type: bee.delivery
title: paseo-leader-dispatch — delivery
description: "Delivery record proposed by bee knowledge promote for work item paseo-leader-dispatch: 5 capped cell(s), 11 recorded deviation(s)."
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch-delivery
  lifecycle: active
  required_context: [docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md]
  sources: [docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md, .bee/cells/archive/paseo-leader-dispatch/pld-1.json, .bee/cells/archive/paseo-leader-dispatch/pld-2.json, .bee/cells/archive/paseo-leader-dispatch/pld-3.json, .bee/cells/archive/paseo-leader-dispatch/pld-4.json, .bee/cells/archive/paseo-leader-dispatch/pld-5.json]
---

# paseo-leader-dispatch — Delivery

## What shipped

- **pld-1** — PaseoCli drops paseo's timestamped stdout startup lines before every JSON, -q id and logs read; the real paseo CLI returns workspaces and an empty leaders list through it (1 file(s) changed)
- **pld-2** — waggledance_dispatch hands a task to a project's Paseo leader (leader: true) and await_run judges paseo: runs through observe_paseo_run (3 file(s) changed)
- **pld-3** — Reaper runs when reaper_enabled and (terminal or paseo) is on; herdr sweep gated live on terminal.enabled (2 file(s) changed)
- **pld-4** — Embedded server/dispatch.ts and client/composer.tsx in the Paseo setup table so waggledance setup paseo installs them (1 file(s) changed)
- **pld-5** — Supervisor template gains a leader mode; knowledge concept documents Paseo leader dispatch; README links it (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **pld-1** — `cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance-core config` — paseo 16 passed 1 ignored, config 27 passed; also ran cargo test --profile fast -p waggledance real_paseo_cli -- --ignored (1 passed against the real paseo CLI: 8 workspaces, leaders []; reopen as re…
- **pld-2** — `cargo test --profile fast -p waggledance dispatch && cargo test --profile fast -p waggledance await` — cell verify (30 dispatch + 20 await, incl. 7 new leader-dispatch and 6 new paseo-await tests); also orchestrate/reaper/mcp/paseo filters, cargo fmt --check and clippy --all-targets clean; whole suite…
- **pld-3** — `cargo test --profile fast -p waggledance reaper && cargo test --profile fast -p waggledance-core runs && cargo test --profile fast -p waggledance board` — the cell verify filters covering the reaper wiring and sweep (18+6+52 passed, incl. 2 new tests); full suite not run
- **pld-4** — `cd integrations/paseo && node --test 'test/**/*.test.ts' && npx tsc --noEmit && cd ../.. && cargo test --profile fast -p waggledance setup` — cell verify (13 node tests, tsc clean, 9 setup tests incl. embedded_tables_match_the_integration_sources); additionally cargo test --profile fast -p waggledance -p waggledance-core all green (1158+3+…
- **pld-5** — `cargo test --profile fast -p waggledance doctor` — 28 doctor tests incl. the supervisor template pins (fence count still 2); plus a deleted scratch crate parsed the concept [paseo] block with waggledance_core::Config (enabled=true, workspace_for(wagg…

## Deviations

- **pld-1** — rewrote the commit message with commit-tree and update-ref so cell: pld-1 is the last line — the concurrent-worker guard refuses commit --amend — hit an unforeseen obstacle
- **pld-1** — first commit put Co-Authored-By after the cell trailer; rewrote the message with commit-tree and update-ref so cell: pld-1 is the last line, because the concurrent-worker guard refuses commit --amend — hit an unforeseen obstacle
- **pld-2** — Done is judged on leader text after the LAST [User] <task> occurrence in the logs, not on any non-[User] line since baseline — paseo 0.11.2 ignores logs --since (34 lines with and without it on agent 68b5798) and a multi-line task continues on lines without the [User] prefix, both of which would give a false done — the plan was wrong about a fact
- **pld-2** — A reused leader returns workspace null — Leader::Existing carries no workspace and inspect does not report one — hit an unforeseen obstacle
- **pld-2** — AwaitOutcome gained warnings, surfaced in waggledance_await structuredContent, to carry the unmapped-status warning — found a better route
- **pld-2** — await_run keeps its signature and builds PaseoCli from engine.config.paseo after the stored-terminal short-circuit; await_paseo_run and observe_paseo_run are pub for the pld-3 reaper — found a better route
- **pld-2** — Removed #[allow(dead_code)] on mod paseo in main.rs after reserving it, per the pld-1 leader note — followed the plan
- **pld-2** — NOT FIXED, outside this cell: on this host paseo prints its startup log lines (HH:MM:SS.mmm › [desktop] app startup ...) on STDOUT, so pld-1 PaseoCli::exec hands them to serde_json (leaders/inspect/workspace ls fail as Unparsable) and first_id returns the noise line as the run/workspace id; the live leader path cannot work until paseo.rs drops those lines — something else had to be fixed first
- **pld-3** — herdr gate is a shared Arc<AtomicBool> set by reconcile rather than engine.config.terminal — engine config is fixed at startup, so a settings-page terminal-off would otherwise keep sweeping herdr while paseo holds the reaper up — found a better route
- **pld-5** — patterns/index.md not regenerated — it is a generated file outside the cell files; orchestrator should run bee knowledge index — something else had to be fixed first
- **pld-5** — README dispatch tool bullet also updated to name leader? as a third exclusive target — the tool list would otherwise be wrong — found a better route

## Provenance

Proposed by `bee knowledge promote --work paseo-leader-dispatch` from 5 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/paseo-leader-dispatch/CONTEXT.md`, `docs/history/paseo-leader-dispatch/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell pld-1 — save as docs/knowledge/patterns/paseo-leader-dispatch-pld-1-pitfall.md

---
type: bee.pattern
title: paseo-leader-dispatch cell pld-1 — pitfall candidate
description: "Pitfall candidate mined from cell pld-1's capped trace: rewrote the commit message with commit-tree and update-ref so cell: pld-1 is the last line — the concurrent-worker guard refuses commit --amend — hit an unfore…"
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch-pld-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/archive/paseo-leader-dispatch/pld-1.json]
  polarity: pitfall
---

# paseo-leader-dispatch cell pld-1 — pitfall candidate

## What the cell did

PaseoCli drops paseo's timestamped stdout startup lines before every JSON, -q id and logs read; the real paseo CLI returns workspaces and an empty leaders list through it

## Recorded evidence (verbatim from .bee/cells/archive/paseo-leader-dispatch/pld-1.json)

- **deviation** — rewrote the commit message with commit-tree and update-ref so cell: pld-1 is the last line — the concurrent-worker guard refuses commit --amend — hit an unforeseen obstacle
- **deviation** — first commit put Co-Authored-By after the cell trailer; rewrote the message with commit-tree and update-ref so cell: pld-1 is the last line, because the concurrent-worker guard refuses commit --amend — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell pld-2 — save as docs/knowledge/patterns/paseo-leader-dispatch-pld-2-pitfall.md

---
type: bee.pattern
title: paseo-leader-dispatch cell pld-2 — pitfall candidate
description: "Pitfall candidate mined from cell pld-2's capped trace: Done is judged on leader text after the LAST [User] <task> occurrence in the logs, not on any non-[User] line since baseline — paseo 0.11.2 ignores logs --sinc…"
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch-pld-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/archive/paseo-leader-dispatch/pld-2.json]
  polarity: pitfall
---

# paseo-leader-dispatch cell pld-2 — pitfall candidate

## What the cell did

waggledance_dispatch hands a task to a project's Paseo leader (leader: true) and await_run judges paseo: runs through observe_paseo_run

## Recorded evidence (verbatim from .bee/cells/archive/paseo-leader-dispatch/pld-2.json)

- **deviation** — Done is judged on leader text after the LAST [User] <task> occurrence in the logs, not on any non-[User] line since baseline — paseo 0.11.2 ignores logs --since (34 lines with and without it on agent 68b5798) and a multi-line task continues on lines without the [User] prefix, both of which would give a false done — the plan was wrong about a fact
- **deviation** — A reused leader returns workspace null — Leader::Existing carries no workspace and inspect does not report one — hit an unforeseen obstacle
- **deviation** — AwaitOutcome gained warnings, surfaced in waggledance_await structuredContent, to carry the unmapped-status warning — found a better route
- **deviation** — await_run keeps its signature and builds PaseoCli from engine.config.paseo after the stored-terminal short-circuit; await_paseo_run and observe_paseo_run are pub for the pld-3 reaper — found a better route
- **deviation** — Removed #[allow(dead_code)] on mod paseo in main.rs after reserving it, per the pld-1 leader note — followed the plan
- **deviation** — NOT FIXED, outside this cell: on this host paseo prints its startup log lines (HH:MM:SS.mmm › [desktop] app startup ...) on STDOUT, so pld-1 PaseoCli::exec hands them to serde_json (leaders/inspect/workspace ls fail as Unparsable) and first_id returns the noise line as the run/workspace id; the live leader path cannot work until paseo.rs drops those lines — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell pld-3 — save as docs/knowledge/patterns/paseo-leader-dispatch-pld-3-pitfall.md

---
type: bee.pattern
title: paseo-leader-dispatch cell pld-3 — pitfall candidate
description: "Pitfall candidate mined from cell pld-3's capped trace: herdr gate is a shared Arc<AtomicBool> set by reconcile rather than engine.config.terminal — engine config is fixed at startup, so a settings-page terminal-off…"
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch-pld-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/archive/paseo-leader-dispatch/pld-3.json]
  polarity: pitfall
---

# paseo-leader-dispatch cell pld-3 — pitfall candidate

## What the cell did

Reaper runs when reaper_enabled and (terminal or paseo) is on; herdr sweep gated live on terminal.enabled

## Recorded evidence (verbatim from .bee/cells/archive/paseo-leader-dispatch/pld-3.json)

- **deviation** — herdr gate is a shared Arc<AtomicBool> set by reconcile rather than engine.config.terminal — engine config is fixed at startup, so a settings-page terminal-off would otherwise keep sweeping herdr while paseo holds the reaper up — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell pld-4 — save as docs/knowledge/patterns/paseo-leader-dispatch-pld-4-pitfall.md

---
type: bee.pattern
title: paseo-leader-dispatch cell pld-4 — pitfall candidate
description: "Pitfall candidate mined from cell pld-4's capped trace: 90caf353d864"
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch-pld-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/archive/paseo-leader-dispatch/pld-4.json]
  polarity: pitfall
---

# paseo-leader-dispatch cell pld-4 — pitfall candidate

## What the cell did

Embedded server/dispatch.ts and client/composer.tsx in the Paseo setup table so waggledance setup paseo installs them

## Recorded evidence (verbatim from .bee/cells/archive/paseo-leader-dispatch/pld-4.json)

- **failure_signature** — 90caf353d864

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell pld-5 — save as docs/knowledge/patterns/paseo-leader-dispatch-pld-5-pitfall.md

---
type: bee.pattern
title: paseo-leader-dispatch cell pld-5 — pitfall candidate
description: "Pitfall candidate mined from cell pld-5's capped trace: patterns/index.md not regenerated — it is a generated file outside the cell files; orchestrator should run bee knowledge index — something else had to be fixed…"
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch-pld-5-pitfall
  lifecycle: draft
  sources: [.bee/cells/archive/paseo-leader-dispatch/pld-5.json]
  polarity: pitfall
---

# paseo-leader-dispatch cell pld-5 — pitfall candidate

## What the cell did

Supervisor template gains a leader mode; knowledge concept documents Paseo leader dispatch; README links it

## Recorded evidence (verbatim from .bee/cells/archive/paseo-leader-dispatch/pld-5.json)

- **deviation** — patterns/index.md not regenerated — it is a generated file outside the cell files; orchestrator should run bee knowledge index — something else had to be fixed first
- **deviation** — README dispatch tool bullet also updated to name leader? as a third exclusive target — the tool list would otherwise be wrong — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 5 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 5 pattern candidate(s), 0 file(s) written.