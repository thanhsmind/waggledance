promote proposal for work item "home-terminal-panel" (docs/history/home-terminal-panel/CONTEXT.md + docs/history/home-terminal-panel/plan.md) — 4 capped cell(s): htp-1, htp-2, htp-3, htp-4
anchor: history — docs/history/home-terminal-panel/CONTEXT.md, docs/history/home-terminal-panel/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/home-terminal-panel/delivery.md

---
type: bee.delivery
title: home-terminal-panel — delivery
description: "Delivery record proposed by bee knowledge promote for work item home-terminal-panel: 4 capped cell(s), 4 recorded deviation(s)."
timestamp: 2026-08-30
bee:
  id: home-terminal-panel-delivery
  lifecycle: active
  areas: [web-interface]
  required_context: [docs/history/home-terminal-panel/CONTEXT.md, docs/history/home-terminal-panel/plan.md]
  sources: [docs/history/home-terminal-panel/CONTEXT.md, docs/history/home-terminal-panel/plan.md, .bee/cells/htp-1.json, .bee/cells/htp-2.json, .bee/cells/htp-3.json, .bee/cells/htp-4.json]
---

# home-terminal-panel — Delivery

## What shipped

- **htp-1** — nav=1 renders the Code tree and the changed-file list as base-targeted nav-only frames (3 file(s) changed)
- **htp-2** — Terminals tab gains a Files|Diff sidebar and a named panel frame above the terminal, every URL server-emitted and every frame lazy (3 file(s) changed)
- **htp-3** — panel=1: the embedded pages render without their in-page sidebar inside the homepage panel; header, picker, reviewed counter and sections stay, and every emitted URL threads the flag (4 file(s) changed)
- **htp-4** — Open-in-Code-view link carries the page's own chrome query (full/embed/panel) (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **htp-1** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **htp-2** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **htp-3** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **htp-4** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`

## Deviations

- **htp-1** — nav-mode base picker drops data-base-url so app.js never binds its scripted handler — that handler rebuilds the URL from data-embed alone and would strip nav=1, and app.js is outside this cell's files; the plain GET form with hidden embed+nav fields navigates instead — hit an unforeseen obstacle
- **htp-1** — split changes_unavailable into a text half so the nav list can carry the reason inline — the full screen's wording ("see the note beside it") names a pane nav mode does not have — a missing piece the outcome depends on
- **htp-2** — The sidebar's project id rides data-panel-project, not data-project-id — homepage-terminal-full D5 forbids a page-root project id on this tab and a shipped test pins its absence; a widget-scoped name keeps that invariant true — hit an unforeseen obstacle
- **htp-2** — Skipped the optional nav-frame base-picker polish htp-1 suggested — it would have grown the diff into a fourth behaviour with no test in this cell's matrix — something else had to be fixed first

## Provenance

Proposed by `bee knowledge promote --work home-terminal-panel` from 4 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/home-terminal-panel/CONTEXT.md`, `docs/history/home-terminal-panel/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "home-terminal-panel" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-30T13:46:44.355Z), the work item declares no bee.areas.

area web-interface:
  - [htp-1] nav=1 renders the Code tree and the changed-file list as base-targeted nav-only frames — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/htp-1.json)
  - [htp-2] Terminals tab gains a Files|Diff sidebar and a named panel frame above the terminal, every URL server-emitted and every frame lazy — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/htp-2.json)
  - [htp-3] panel=1: the embedded pages render without their in-page sidebar inside the homepage panel; header, picker, reviewed counter and sections stay, and every emitted URL threads the flag — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/htp-3.json)
  - [htp-4] Open-in-Code-view link carries the page's own chrome query (full/embed/panel) — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/htp-4.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell htp-1 — save as docs/knowledge/patterns/home-terminal-panel-htp-1-pitfall.md

---
type: bee.pattern
title: home-terminal-panel cell htp-1 — pitfall candidate
description: "Pitfall candidate mined from cell htp-1's capped trace: nav-mode base picker drops data-base-url so app.js never binds its scripted handler — that handler rebuilds the URL from data-embed alone and would strip nav=1…"
timestamp: 2026-08-30
bee:
  id: home-terminal-panel-htp-1-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/htp-1.json]
  polarity: pitfall
---

# home-terminal-panel cell htp-1 — pitfall candidate

## What the cell did

nav=1 renders the Code tree and the changed-file list as base-targeted nav-only frames

## Recorded evidence (verbatim from .bee/cells/htp-1.json)

- **deviation** — nav-mode base picker drops data-base-url so app.js never binds its scripted handler — that handler rebuilds the URL from data-embed alone and would strip nav=1, and app.js is outside this cell's files; the plain GET form with hidden embed+nav fields navigates instead — hit an unforeseen obstacle
- **deviation** — split changes_unavailable into a text half so the nav list can carry the reason inline — the full screen's wording ("see the note beside it") names a pane nav mode does not have — a missing piece the outcome depends on

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell htp-2 — save as docs/knowledge/patterns/home-terminal-panel-htp-2-pitfall.md

---
type: bee.pattern
title: home-terminal-panel cell htp-2 — pitfall candidate
description: "Pitfall candidate mined from cell htp-2's capped trace: The sidebar's project id rides data-panel-project, not data-project-id — homepage-terminal-full D5 forbids a page-root project id on this tab and a shipped tes…"
timestamp: 2026-08-30
bee:
  id: home-terminal-panel-htp-2-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/htp-2.json]
  polarity: pitfall
---

# home-terminal-panel cell htp-2 — pitfall candidate

## What the cell did

Terminals tab gains a Files|Diff sidebar and a named panel frame above the terminal, every URL server-emitted and every frame lazy

## Recorded evidence (verbatim from .bee/cells/htp-2.json)

- **deviation** — The sidebar's project id rides data-panel-project, not data-project-id — homepage-terminal-full D5 forbids a page-root project id on this tab and a shipped test pins its absence; a widget-scoped name keeps that invariant true — hit an unforeseen obstacle
- **deviation** — Skipped the optional nav-frame base-picker polish htp-1 suggested — it would have grown the diff into a fourth behaviour with no test in this cell's matrix — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 4 capped cell(s) mined, 1 delivery draft, 4 area bullet(s), 2 pattern candidate(s), 0 file(s) written.