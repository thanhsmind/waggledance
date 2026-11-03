promote proposal for work item "changes-diff-screen" (docs/history/changes-diff-screen/CONTEXT.md + docs/history/changes-diff-screen/plan.md) — 8 capped cell(s): cds-1, cds-2, cds-3, cds-4, cds-5, cds-6, cds-7, cds-8
anchor: history — docs/history/changes-diff-screen/CONTEXT.md, docs/history/changes-diff-screen/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/changes-diff-screen/delivery.md

---
type: bee.delivery
title: changes-diff-screen — delivery
description: "Delivery record proposed by bee knowledge promote for work item changes-diff-screen: 8 capped cell(s), 15 recorded deviation(s)."
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-delivery
  lifecycle: active
  areas: [web-interface]
  required_context: [docs/history/changes-diff-screen/CONTEXT.md, docs/history/changes-diff-screen/plan.md]
  sources: [docs/history/changes-diff-screen/CONTEXT.md, docs/history/changes-diff-screen/plan.md, .bee/cells/cds-1.json, .bee/cells/cds-2.json, .bee/cells/cds-3.json, .bee/cells/cds-4.json, .bee/cells/cds-5.json, .bee/cells/cds-6.json, .bee/cells/cds-7.json, .bee/cells/cds-8.json]
---

# changes-diff-screen — Delivery

## What shipped

- **cds-1** — Working-tree diff end to end: git_diff.rs + Engine::changes + GET /p/:id/_changes rendering side-by-side sections, with the D3 empty state and the D5 aggregate filter (5 file(s) changed)
- **cds-2** — Changes screen matches the screenshot shape: dir-grouped changed-file sidebar with M/A/D/R badges and per-file counts, sticky per-file headers, both panes syntax-highlighted with a theme-derived red/green palette, and Docs|Code|Changes in the topbar everywhere (4 file(s) changed)
- **cds-3** — Reviewed marks land: header checkbox per file, mirrored sidebar tick, live N/M counter with a complete state, and localStorage marks keyed by a server-emitted content hash so an edit drops the stale mark (4 file(s) changed)
- **cds-4** — Sec-Fetch guard now allows cross-site top-level navigations (GET/HEAD + navigate + document), unblocking the post-Access-login redirect; cross-site POSTs and non-navigation GETs still 421 (1 file(s) changed)
- **cds-5** — ?commit=<sha> shows that commit against its parent end to end; invalid values fall back to the working tree without echoing (4 file(s) changed)
- **cds-6** — The Changes header picks its base: Working tree plus recent commits, reviewed marks keyed per base (3 file(s) changed)
- **cds-7** — embed=1 renders the Changes and Code pages without the topbar; sidebars, picker and reviewed marks stay, and in-page links carry the flag (4 file(s) changed)
- **cds-8** — Terminal page gains FILES | DIFF toggles that split an embed=1 iframe over the terminal, default closed (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **cds-1** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-2** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-3** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-4** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-5** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-6** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-7** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`
- **cds-8** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`

## Deviations

- **cds-1** — Added a fourth git call (rev-parse --show-toplevel) ahead of the plan three, purely to classify D3 — outside a repository git diff falls into --no-index mode and fails about HEAD rather than about the repository, so the 3-call design could not tell not-a-repository from any other failure — the plan was wrong about a fact
- **cds-1** — Kept the per-line pairing (DiffLine with each side own line number) beside the full old/new texts the plan names, because the plan phase-1 demo is a side-by-side table with line numbers and rebuilding that pairing from two texts would need the LCS diff the plan rejected — found a better route
- **cds-1** — Forced --src-prefix=a/ --dst-prefix=b/ on the patch call: the machine own gitconfig sets diff.mnemonicPrefix, so section headers came back as c/ and w/ and no path would have matched — hit an unforeseen obstacle
- **cds-1** — Section::Changes was added to the topbar enum but section_switch still renders only Docs|Code, so no Docs or Code page changes yet — the third link is phase 2 declared scope — found a better route
- **cds-2** — Highlighting reads a section-owned RenderService behind a OnceLock in server.rs rather than the engine renderer — Engine keeps its `render` field private and engine.rs is outside this cell's files, so adding an accessor there would have been out-of-scope; the diff needs two texts per section that exist nowhere on disk anyway — found a better route
- **cds-2** — Moved code_tree's doc comment back onto code_tree — cds-1 inserted changes_page between that comment and its function, so the Code sidebar's docs were attached to the Changes page — something else had to be fixed first
- **cds-2** — The Changes breadcrumb is position:static (Docs' sticky breadcrumb would have collided at the same 53px offset with the sticky per-file headers the plan asked for) — hit an unforeseen obstacle
- **cds-3** — Loosened two existing assertions that pinned the changeset section tag literally (views.rs sidebar_rows_anchor_to_the_sections_they_name, server.rs changes_route_renders_the_real_working_tree_diff) and reserved server.rs to do it — the section had to gain data-path/data-key, and the literals ended at the closing angle bracket — hit an unforeseen obstacle
- **cds-4** — followed the plan
- **cds-5** — ChangesView went from an enum to a struct {content, commits, base} instead of gaining a variant — the cell asks it to carry the commit list AND the active base for cds-6, and an enum cannot hold either beside its payload; the two views.rs test constructors were updated with it — found a better route
- **cds-5** — Reverted rustfmt collateral in guide.rs, herdr/socket.rs and orchestrate.rs — `cargo fmt -p waggledance` reflowed three files my cell does not name and does not reserve; my four files keep the formatting — something else had to be fixed first
- **cds-6** — Loosened one pinned literal in the_page_carries_everything_a_reviewed_mark_hangs_on — the screen root now carries data-base beside data-project-id, which is exactly what the reviewed key needed — found a better route
- **cds-6** — Fixed only my own two rustfmt hits by hand rather than running cargo fmt over the crate — guide.rs/herdr/orchestrate.rs carry pre-existing drift from a different rustfmt version and are not this cell's files — hit an unforeseen obstacle
- **cds-7** — app.js was edited though the cell files list omitted it — the cell action requires the base picker to carry embed=1 through its scripted navigation and the scrollspy assumes the 53px topbar, both of which live only in app.js; reserved under w-cds-7 before writing — the plan was wrong about a fact
- **cds-8** — followed the plan

## Provenance

Proposed by `bee knowledge promote --work changes-diff-screen` from 8 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/changes-diff-screen/CONTEXT.md`, `docs/history/changes-diff-screen/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "changes-diff-screen" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-30T12:36:30.552Z), the work item declares no bee.areas.

area web-interface:
  - [cds-1] Working-tree diff end to end: git_diff.rs + Engine::changes + GET /p/:id/_changes rendering side-by-side sections, with the D3 empty state and the D5 aggregate filter — feature-wide sync per the scribing stamp, 5 file(s) changed (trace .bee/cells/cds-1.json)
  - [cds-2] Changes screen matches the screenshot shape: dir-grouped changed-file sidebar with M/A/D/R badges and per-file counts, sticky per-file headers, both panes syntax-highlighted with a theme-derived red/green palette, and Docs|Code|Changes in the topbar everywhere — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/cds-2.json)
  - [cds-3] Reviewed marks land: header checkbox per file, mirrored sidebar tick, live N/M counter with a complete state, and localStorage marks keyed by a server-emitted content hash so an edit drops the stale mark — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/cds-3.json)
  - [cds-4] Sec-Fetch guard now allows cross-site top-level navigations (GET/HEAD + navigate + document), unblocking the post-Access-login redirect; cross-site POSTs and non-navigation GETs still 421 — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/cds-4.json)
  - [cds-5] ?commit=<sha> shows that commit against its parent end to end; invalid values fall back to the working tree without echoing — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/cds-5.json)
  - [cds-6] The Changes header picks its base: Working tree plus recent commits, reviewed marks keyed per base — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/cds-6.json)
  - [cds-7] embed=1 renders the Changes and Code pages without the topbar; sidebars, picker and reviewed marks stay, and in-page links carry the flag — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/cds-7.json)
  - [cds-8] Terminal page gains FILES | DIFF toggles that split an embed=1 iframe over the terminal, default closed — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/cds-8.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell cds-1 — save as docs/knowledge/patterns/changes-diff-screen-cds-1-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-1 — pitfall candidate
description: "Pitfall candidate mined from cell cds-1's capped trace: Added a fourth git call (rev-parse --show-toplevel) ahead of the plan three, purely to classify D3 — outside a repository git diff falls into --no-index mode a…"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-1-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-1.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-1 — pitfall candidate

## What the cell did

Working-tree diff end to end: git_diff.rs + Engine::changes + GET /p/:id/_changes rendering side-by-side sections, with the D3 empty state and the D5 aggregate filter

## Recorded evidence (verbatim from .bee/cells/cds-1.json)

- **deviation** — Added a fourth git call (rev-parse --show-toplevel) ahead of the plan three, purely to classify D3 — outside a repository git diff falls into --no-index mode and fails about HEAD rather than about the repository, so the 3-call design could not tell not-a-repository from any other failure — the plan was wrong about a fact
- **deviation** — Kept the per-line pairing (DiffLine with each side own line number) beside the full old/new texts the plan names, because the plan phase-1 demo is a side-by-side table with line numbers and rebuilding that pairing from two texts would need the LCS diff the plan rejected — found a better route
- **deviation** — Forced --src-prefix=a/ --dst-prefix=b/ on the patch call: the machine own gitconfig sets diff.mnemonicPrefix, so section headers came back as c/ and w/ and no path would have matched — hit an unforeseen obstacle
- **deviation** — Section::Changes was added to the topbar enum but section_switch still renders only Docs|Code, so no Docs or Code page changes yet — the third link is phase 2 declared scope — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-2 — save as docs/knowledge/patterns/changes-diff-screen-cds-2-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-2 — pitfall candidate
description: "Pitfall candidate mined from cell cds-2's capped trace: Highlighting reads a section-owned RenderService behind a OnceLock in server.rs rather than the engine renderer — Engine keeps its `render` field private and e…"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-2-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-2.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-2 — pitfall candidate

## What the cell did

Changes screen matches the screenshot shape: dir-grouped changed-file sidebar with M/A/D/R badges and per-file counts, sticky per-file headers, both panes syntax-highlighted with a theme-derived red/green palette, and Docs|Code|Changes in the topbar everywhere

## Recorded evidence (verbatim from .bee/cells/cds-2.json)

- **deviation** — Highlighting reads a section-owned RenderService behind a OnceLock in server.rs rather than the engine renderer — Engine keeps its `render` field private and engine.rs is outside this cell's files, so adding an accessor there would have been out-of-scope; the diff needs two texts per section that exist nowhere on disk anyway — found a better route
- **deviation** — Moved code_tree's doc comment back onto code_tree — cds-1 inserted changes_page between that comment and its function, so the Code sidebar's docs were attached to the Changes page — something else had to be fixed first
- **deviation** — The Changes breadcrumb is position:static (Docs' sticky breadcrumb would have collided at the same 53px offset with the sticky per-file headers the plan asked for) — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-3 — save as docs/knowledge/patterns/changes-diff-screen-cds-3-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-3 — pitfall candidate
description: "Pitfall candidate mined from cell cds-3's capped trace: Loosened two existing assertions that pinned the changeset section tag literally (views.rs sidebar_rows_anchor_to_the_sections_they_name, server.rs changes_rou…"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-3-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-3.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-3 — pitfall candidate

## What the cell did

Reviewed marks land: header checkbox per file, mirrored sidebar tick, live N/M counter with a complete state, and localStorage marks keyed by a server-emitted content hash so an edit drops the stale mark

## Recorded evidence (verbatim from .bee/cells/cds-3.json)

- **deviation** — Loosened two existing assertions that pinned the changeset section tag literally (views.rs sidebar_rows_anchor_to_the_sections_they_name, server.rs changes_route_renders_the_real_working_tree_diff) and reserved server.rs to do it — the section had to gain data-path/data-key, and the literals ended at the closing angle bracket — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-4 — save as docs/knowledge/patterns/changes-diff-screen-cds-4-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-4 — pitfall candidate
description: "Pitfall candidate mined from cell cds-4's capped trace: followed the plan"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-4-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-4.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-4 — pitfall candidate

## What the cell did

Sec-Fetch guard now allows cross-site top-level navigations (GET/HEAD + navigate + document), unblocking the post-Access-login redirect; cross-site POSTs and non-navigation GETs still 421

## Recorded evidence (verbatim from .bee/cells/cds-4.json)

- **deviation** — followed the plan

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-5 — save as docs/knowledge/patterns/changes-diff-screen-cds-5-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-5 — pitfall candidate
description: "Pitfall candidate mined from cell cds-5's capped trace: ChangesView went from an enum to a struct {content, commits, base} instead of gaining a variant — the cell asks it to carry the commit list AND the active base…"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-5-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-5.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-5 — pitfall candidate

## What the cell did

?commit=<sha> shows that commit against its parent end to end; invalid values fall back to the working tree without echoing

## Recorded evidence (verbatim from .bee/cells/cds-5.json)

- **deviation** — ChangesView went from an enum to a struct {content, commits, base} instead of gaining a variant — the cell asks it to carry the commit list AND the active base for cds-6, and an enum cannot hold either beside its payload; the two views.rs test constructors were updated with it — found a better route
- **deviation** — Reverted rustfmt collateral in guide.rs, herdr/socket.rs and orchestrate.rs — `cargo fmt -p waggledance` reflowed three files my cell does not name and does not reserve; my four files keep the formatting — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-6 — save as docs/knowledge/patterns/changes-diff-screen-cds-6-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-6 — pitfall candidate
description: "Pitfall candidate mined from cell cds-6's capped trace: Loosened one pinned literal in the_page_carries_everything_a_reviewed_mark_hangs_on — the screen root now carries data-base beside data-project-id, which is ex…"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-6-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-6.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-6 — pitfall candidate

## What the cell did

The Changes header picks its base: Working tree plus recent commits, reviewed marks keyed per base

## Recorded evidence (verbatim from .bee/cells/cds-6.json)

- **deviation** — Loosened one pinned literal in the_page_carries_everything_a_reviewed_mark_hangs_on — the screen root now carries data-base beside data-project-id, which is exactly what the reviewed key needed — found a better route
- **deviation** — Fixed only my own two rustfmt hits by hand rather than running cargo fmt over the crate — guide.rs/herdr/orchestrate.rs carry pre-existing drift from a different rustfmt version and are not this cell's files — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-7 — save as docs/knowledge/patterns/changes-diff-screen-cds-7-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-7 — pitfall candidate
description: "Pitfall candidate mined from cell cds-7's capped trace: app.js was edited though the cell files list omitted it — the cell action requires the base picker to carry embed=1 through its scripted navigation and the scr…"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-7-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-7.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-7 — pitfall candidate

## What the cell did

embed=1 renders the Changes and Code pages without the topbar; sidebars, picker and reviewed marks stay, and in-page links carry the flag

## Recorded evidence (verbatim from .bee/cells/cds-7.json)

- **deviation** — app.js was edited though the cell files list omitted it — the cell action requires the base picker to carry embed=1 through its scripted navigation and the scrollspy assumes the 53px topbar, both of which live only in app.js; reserved under w-cds-7 before writing — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cds-8 — save as docs/knowledge/patterns/changes-diff-screen-cds-8-pitfall.md

---
type: bee.pattern
title: changes-diff-screen cell cds-8 — pitfall candidate
description: "Pitfall candidate mined from cell cds-8's capped trace: followed the plan"
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-cds-8-pitfall
  lifecycle: draft
  areas: [web-interface]
  sources: [.bee/cells/cds-8.json]
  polarity: pitfall
---

# changes-diff-screen cell cds-8 — pitfall candidate

## What the cell did

Terminal page gains FILES | DIFF toggles that split an embed=1 iframe over the terminal, default closed

## Recorded evidence (verbatim from .bee/cells/cds-8.json)

- **deviation** — followed the plan

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 8 capped cell(s) mined, 1 delivery draft, 8 area bullet(s), 8 pattern candidate(s), 0 file(s) written.