promote proposal for work item "board-run-reaper" (docs/history/board-run-reaper/CONTEXT.md + docs/history/board-run-reaper/plan.md) — 2 capped cell(s): brr-1, brr-2
anchor: history — docs/history/board-run-reaper/CONTEXT.md, docs/history/board-run-reaper/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/board-run-reaper/delivery.md

---
type: bee.delivery
title: board-run-reaper — delivery
description: "Delivery record proposed by bee knowledge promote for work item board-run-reaper: 2 capped cell(s), 5 recorded deviation(s)."
timestamp: 2026-08-31
bee:
  id: board-run-reaper-delivery
  lifecycle: active
  required_context: [docs/history/board-run-reaper/CONTEXT.md, docs/history/board-run-reaper/plan.md]
  sources: [docs/history/board-run-reaper/CONTEXT.md, docs/history/board-run-reaper/plan.md, .bee/cells/brr-1.json, .bee/cells/brr-2.json]
---

# board-run-reaper — Delivery

## What shipped

- **brr-1** — RunStatus::Lost, list_unattended_working_runs, and the default-on terminal.reaper_enabled switch, each with its own test (3 file(s) changed)
- **brr-2** — Reaper sweep caps vanished panes lost and finishes declared-done runs through await_run, wired behind the family + reaper switches (4 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **brr-1** — `cargo test --workspace`
- **brr-2** — `cargo test --workspace && cargo build --profile fast -p waggledance`

## Deviations

- **brr-1** — TerminalConfig's derived Default became a hand-written impl — reaper_enabled defaults true and a derive would silently ship it off, and the struct-level serde(default) routes absent TOML keys through it — hit an unforeseen obstacle
- **brr-2** — Reused await_run's real freshness rule (marker absent from the run's baseline, present in a current read) instead of the cell's described marker-count-vs-baseline-count — await_run never counted occurrences, so a count rule would have been a second, different rule — the plan was wrong about a fact
- **brr-2** — Extracted that rule into orchestrate::marker_is_fresh and made RECENT_LINES_CAP pub(crate), reserving crates/waggledance/src/orchestrate.rs — the cell asked for reuse/extraction of the helper, never a second copy — something else had to be fixed first
- **brr-2** — Added a read_pane_log seam to FakeHerdr (reserved crates/waggledance/src/herdr/fake.rs) — the must-have truth is that a gone pane gets NO pane call, and only a recorded call log tells that apart from a read that errored — hit an unforeseen obstacle
- **brr-2** — Blocked panes are skipped before the read, not just excluded from capping, so the reaper never reaches await_run's blocked branch and can never write blocked — hit an unforeseen obstacle

## Provenance

Proposed by `bee knowledge promote --work board-run-reaper` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-run-reaper/CONTEXT.md`, `docs/history/board-run-reaper/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell brr-1 — save as docs/knowledge/patterns/board-run-reaper-brr-1-pitfall.md

---
type: bee.pattern
title: board-run-reaper cell brr-1 — pitfall candidate
description: "Pitfall candidate mined from cell brr-1's capped trace: TerminalConfig's derived Default became a hand-written impl — reaper_enabled defaults true and a derive would silently ship it off, and the struct-level serde(…"
timestamp: 2026-08-31
bee:
  id: board-run-reaper-brr-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/brr-1.json]
  polarity: pitfall
---

# board-run-reaper cell brr-1 — pitfall candidate

## What the cell did

RunStatus::Lost, list_unattended_working_runs, and the default-on terminal.reaper_enabled switch, each with its own test

## Recorded evidence (verbatim from .bee/cells/brr-1.json)

- **deviation** — TerminalConfig's derived Default became a hand-written impl — reaper_enabled defaults true and a derive would silently ship it off, and the struct-level serde(default) routes absent TOML keys through it — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell brr-2 — save as docs/knowledge/patterns/board-run-reaper-brr-2-pitfall.md

---
type: bee.pattern
title: board-run-reaper cell brr-2 — pitfall candidate
description: "Pitfall candidate mined from cell brr-2's capped trace: Reused await_run's real freshness rule (marker absent from the run's baseline, present in a current read) instead of the cell's described marker-count-vs-basel…"
timestamp: 2026-08-31
bee:
  id: board-run-reaper-brr-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/brr-2.json]
  polarity: pitfall
---

# board-run-reaper cell brr-2 — pitfall candidate

## What the cell did

Reaper sweep caps vanished panes lost and finishes declared-done runs through await_run, wired behind the family + reaper switches

## Recorded evidence (verbatim from .bee/cells/brr-2.json)

- **deviation** — Reused await_run's real freshness rule (marker absent from the run's baseline, present in a current read) instead of the cell's described marker-count-vs-baseline-count — await_run never counted occurrences, so a count rule would have been a second, different rule — the plan was wrong about a fact
- **deviation** — Extracted that rule into orchestrate::marker_is_fresh and made RECENT_LINES_CAP pub(crate), reserving crates/waggledance/src/orchestrate.rs — the cell asked for reuse/extraction of the helper, never a second copy — something else had to be fixed first
- **deviation** — Added a read_pane_log seam to FakeHerdr (reserved crates/waggledance/src/herdr/fake.rs) — the must-have truth is that a gone pane gets NO pane call, and only a recorded call log tells that apart from a read that errored — hit an unforeseen obstacle
- **deviation** — Blocked panes are skipped before the read, not just excluded from capping, so the reaper never reaches await_run's blocked branch and can never write blocked — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 2 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 2 pattern candidate(s), 0 file(s) written.