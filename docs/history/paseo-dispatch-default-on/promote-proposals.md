promote proposal for work item "paseo-dispatch-default-on" (docs/history/paseo-dispatch-default-on/CONTEXT.md + docs/history/paseo-dispatch-default-on/plan.md) — 2 capped cell(s): pd-1, pd-2
anchor: history — docs/history/paseo-dispatch-default-on/CONTEXT.md, docs/history/paseo-dispatch-default-on/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/paseo-dispatch-default-on/delivery.md

---
type: bee.delivery
title: paseo-dispatch-default-on — delivery
description: "Delivery record proposed by bee knowledge promote for work item paseo-dispatch-default-on: 2 capped cell(s), 9 recorded deviation(s)."
timestamp: 2026-10-23
bee:
  id: paseo-dispatch-default-on-delivery
  lifecycle: active
  areas: [patterns/paseo-leader-dispatch, specs/mcp-surface]
  required_context: [docs/history/paseo-dispatch-default-on/CONTEXT.md, docs/history/paseo-dispatch-default-on/plan.md]
  sources: [docs/history/paseo-dispatch-default-on/CONTEXT.md, docs/history/paseo-dispatch-default-on/plan.md, .bee/cells/pd-1.json, .bee/cells/pd-2.json]
---

# paseo-dispatch-default-on — Delivery

## What shipped

- **pd-1** — Dispatch, board actions, trigger and Settings read one computed consent: on unless .waggledance.toml says orchestration = false or cannot be parsed (10 file(s) changed)
- **pd-2** — Living docs describe dispatch as on by default with orchestration = false as the only per-project off switch (7 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **pd-1** — `cargo test --profile fast -p waggledance-core -p waggledance && cargo fmt --check` — the cell's own verify over both crates it touched: core 566 passed, daemon 1283 passed 2 ignored plus 3+5 integration, fmt clean; no live dispatch to a-blog run here (that is the plan's separate live…
- **pd-2** — `rg -n 'opted into orchestration|opt.*in.*orchestration' README.md docs/specs docs/knowledge/patterns docs/waggledance-supervisor-skill-template.md` — cell verify finds zero hits (exit 1); a wider rg for 'orchestration = true|opted into' outside docs/history, docs/knowledge/work and crates also finds none; bee knowledge check reports 0 errors (323 …

## Deviations

- **pd-1** — Edited crates/waggledance-core/src/project_settings.rs (reserved under pd-1-worker) to add ProjectSettings.unreadable, set when an existing file cannot be read or parsed, and shortened its parse warning to '<path> failed to parse: <err>' since the file is no longer ignored — the action named extending project_settings::load but the file was not in the cell's list — something else had to be fixed first
- **pd-1** — Edited crates/waggledance-core/src/config.rs (reserved) instead of crates/waggledance/src/config.rs, deleting the stale trigger_enabled doc sentence about orchestration_enabled — the cell listed a daemon config.rs that does not exist; the trigger doc lives in core — the plan was wrong about a fact
- **pd-1** — list_projects_with_queued_paseo_runs was left alone — it returns project ids (Vec<String>), not Project, so there is no flag to fill — the plan was wrong about a fact
- **pd-1** — Board 403 text names the file and both remedies (remove orchestration = false, or fix a file that does not parse) in one message rather than splitting by cause — the board gate reads only the filled bool, and one message covers both off causes without threading the warning through — found a better route
- **pd-1** — Dropped the set_orchestration_enabled setup line from server.rs approving_uat_from_the_board_re_renders_the_card_as_uat_approved so the existing whole-router board test proves buttons appear on a default project — cheaper than a new board fixture — found a better route
- **pd-1** — Settings view signature became &[(Project, Option<String>)] carrying the unreadable warning per project; server settings_page_handler fills it from Engine::project_settings — the off reason needs the warning and views has no engine — found a better route
- **pd-2** — Renamed the title (not the id or filename) of an-opt-in-that-blocks-the-live-proof.md to 'A safety switch that blocks the live proof' and added a paragraph pointing at decision fb1b49ab, instead of marking it superseded — the practice still holds for the opt-out switch, so re-scoping keeps it truthful — found a better route
- **pd-2** — Reverted docs/knowledge/work/homepage-tabs/index.md, which bee knowledge index rewrote — outside the cell's files and unrelated to this change — hit an unforeseen obstacle
- **pd-2** — Quoted the refusals from dispatch_off rather than not_opted_in, and documented that a broken .waggledance.toml turns dispatch off and that Paseo leader dispatch is not gated by terminal.enabled — per the leader note after pd-1 landed — the plan was wrong about a fact

## Provenance

Proposed by `bee knowledge promote --work paseo-dispatch-default-on` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/paseo-dispatch-default-on/CONTEXT.md`, `docs/history/paseo-dispatch-default-on/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "paseo-dispatch-default-on" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-10-23T06:50:40.899Z), the work item declares no bee.areas.

area patterns/paseo-leader-dispatch:
  - [pd-1] Dispatch, board actions, trigger and Settings read one computed consent: on unless .waggledance.toml says orchestration = false or cannot be parsed — feature-wide sync per the scribing stamp, 10 file(s) changed (trace .bee/cells/pd-1.json)

area specs/mcp-surface:
  - [pd-1] Dispatch, board actions, trigger and Settings read one computed consent: on unless .waggledance.toml says orchestration = false or cannot be parsed — feature-wide sync per the scribing stamp, 10 file(s) changed (trace .bee/cells/pd-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell pd-1 — save as docs/knowledge/patterns/paseo-dispatch-default-on-pd-1-pitfall.md

---
type: bee.pattern
title: paseo-dispatch-default-on cell pd-1 — pitfall candidate
description: "Pitfall candidate mined from cell pd-1's capped trace: Edited crates/waggledance-core/src/project_settings.rs (reserved under pd-1-worker) to add ProjectSettings.unreadable, set when an existing file cannot be read…"
timestamp: 2026-10-23
bee:
  id: paseo-dispatch-default-on-pd-1-pitfall
  lifecycle: draft
  areas: [patterns/paseo-leader-dispatch, specs/mcp-surface]
  sources: [.bee/cells/pd-1.json]
  polarity: pitfall
---

# paseo-dispatch-default-on cell pd-1 — pitfall candidate

## What the cell did

Dispatch, board actions, trigger and Settings read one computed consent: on unless .waggledance.toml says orchestration = false or cannot be parsed

## Recorded evidence (verbatim from .bee/cells/pd-1.json)

- **deviation** — Edited crates/waggledance-core/src/project_settings.rs (reserved under pd-1-worker) to add ProjectSettings.unreadable, set when an existing file cannot be read or parsed, and shortened its parse warning to '<path> failed to parse: <err>' since the file is no longer ignored — the action named extending project_settings::load but the file was not in the cell's list — something else had to be fixed first
- **deviation** — Edited crates/waggledance-core/src/config.rs (reserved) instead of crates/waggledance/src/config.rs, deleting the stale trigger_enabled doc sentence about orchestration_enabled — the cell listed a daemon config.rs that does not exist; the trigger doc lives in core — the plan was wrong about a fact
- **deviation** — list_projects_with_queued_paseo_runs was left alone — it returns project ids (Vec<String>), not Project, so there is no flag to fill — the plan was wrong about a fact
- **deviation** — Board 403 text names the file and both remedies (remove orchestration = false, or fix a file that does not parse) in one message rather than splitting by cause — the board gate reads only the filled bool, and one message covers both off causes without threading the warning through — found a better route
- **deviation** — Dropped the set_orchestration_enabled setup line from server.rs approving_uat_from_the_board_re_renders_the_card_as_uat_approved so the existing whole-router board test proves buttons appear on a default project — cheaper than a new board fixture — found a better route
- **deviation** — Settings view signature became &[(Project, Option<String>)] carrying the unreadable warning per project; server settings_page_handler fills it from Engine::project_settings — the off reason needs the warning and views has no engine — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell pd-2 — save as docs/knowledge/patterns/paseo-dispatch-default-on-pd-2-pitfall.md

---
type: bee.pattern
title: paseo-dispatch-default-on cell pd-2 — pitfall candidate
description: "Pitfall candidate mined from cell pd-2's capped trace: Renamed the title (not the id or filename) of an-opt-in-that-blocks-the-live-proof.md to 'A safety switch that blocks the live proof' and added a paragraph poi…"
timestamp: 2026-10-23
bee:
  id: paseo-dispatch-default-on-pd-2-pitfall
  lifecycle: draft
  areas: [patterns/paseo-leader-dispatch, specs/mcp-surface]
  sources: [.bee/cells/pd-2.json]
  polarity: pitfall
---

# paseo-dispatch-default-on cell pd-2 — pitfall candidate

## What the cell did

Living docs describe dispatch as on by default with orchestration = false as the only per-project off switch

## Recorded evidence (verbatim from .bee/cells/pd-2.json)

- **deviation** — Renamed the title (not the id or filename) of an-opt-in-that-blocks-the-live-proof.md to 'A safety switch that blocks the live proof' and added a paragraph pointing at decision fb1b49ab, instead of marking it superseded — the practice still holds for the opt-out switch, so re-scoping keeps it truthful — found a better route
- **deviation** — Reverted docs/knowledge/work/homepage-tabs/index.md, which bee knowledge index rewrote — outside the cell's files and unrelated to this change — hit an unforeseen obstacle
- **deviation** — Quoted the refusals from dispatch_off rather than not_opted_in, and documented that a broken .waggledance.toml turns dispatch off and that Paseo leader dispatch is not gated by terminal.enabled — per the leader note after pd-1 landed — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 2 capped cell(s) mined, 1 delivery draft, 2 area bullet(s), 2 pattern candidate(s), 0 file(s) written.