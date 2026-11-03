promote proposal for work item "console-rail-orchestrator" (docs/history/console-rail-orchestrator/CONTEXT.md + docs/history/console-rail-orchestrator/plan.md) — 4 capped cell(s): cro-1, cro-2, cro-3, cro-4
anchor: history — docs/history/console-rail-orchestrator/CONTEXT.md, docs/history/console-rail-orchestrator/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/console-rail-orchestrator/delivery.md

---
type: bee.delivery
title: console-rail-orchestrator — delivery
description: "Delivery record proposed by bee knowledge promote for work item console-rail-orchestrator: 4 capped cell(s), 14 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: console-rail-orchestrator-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/history/console-rail-orchestrator/CONTEXT.md, docs/history/console-rail-orchestrator/plan.md]
  sources: [docs/history/console-rail-orchestrator/CONTEXT.md, docs/history/console-rail-orchestrator/plan.md, .bee/cells/archive/console-rail-orchestrator/cro-1.json, .bee/cells/archive/console-rail-orchestrator/cro-2.json, .bee/cells/archive/console-rail-orchestrator/cro-3.json, .bee/cells/archive/console-rail-orchestrator/cro-4.json]
---

# console-rail-orchestrator — Delivery

## What shipped

- **cro-1** — Home tab strip retired; a real Orchestrator anchor in the topbar's right slot is the board's entry point (3 file(s) changed)
- **cro-2** — Rail gained a Pinned group of live terminals and now renders beside the terminals view; Board row dropped (3 file(s) changed)
- **cro-3** — Rail project groups are collapsible <details> with a remembered collapsed set and filter-forced opening (4 file(s) changed)
- **cro-4** — Re-synced the cockpit spec's front-page passage to the Orchestrator link, the Pinned rail and collapsible project groups (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **cro-1** — `cargo test -p waggledance -- home_page topbar`
- **cro-2** — `cargo test -p waggledance -- home_page project_rail terminals_tab`
- **cro-3** — `cargo test -p waggledance -- home_page project_rail proj_group`
- **cro-4** — `rg -n "tab strip|Orchestrator|Pinned" docs/specs/bee-cockpit.md`

## Deviations

- **cro-1** — Rewrote terminals_tab_distinguishes_herdr_off_from_no_agents_running too: not on the cell name list but it sliced on the same strip literal, which the cell rule covers
- **cro-1** — Added an absence assertion (no <nav class="fg-tabs">) to the two views.rs tests as well as the server ones, so a quiet return of the strip is a red anywhere
- **cro-2** — pinned dot uses role=img + aria-label instead of aria-hidden: a pinned row has no badge pills to carry its status as words, and no visually-hidden utility exists in this stylesheet to add one without a design-system decision
- **cro-2** — empty-state class is fg-empty, not the cell's fg-muted: fg-muted has no rule anywhere in app.css or atelier/, while fg-empty is the rail's own existing muted-line class
- **cro-2** — two unauthenticated-leak tests narrowed, not deleted: D2 puts an unassigned pane's id in the rail's pinned href on the board, so both tests now pin the id to exactly that one address (name, title and cwd stay absolutely absent) — the same id was already readable at /?tab=terminals unauthenticated on this build, so the surface moved rather than widened; flagging for the human as a privacy-surface note
- **cro-2** — home_page_renders_cross_project_features_beside_the_project_rail_on_kanban rewritten: D3 makes the terminals view render the project list, so the assertion now pins the rail's position (sibling before <main>) instead of its absence
- **cro-2** — dead .home-sidebar__nav/__row/--on CSS removed — the retired Board row was their only user
- **cro-2** — effective_pane extracted from terminals_tab so the rail and the screen resolve ?pane identically
- **cro-3** — Cell said the summary holds the parent name line plus the delete form and the body holds proj-row__meta; the shipped row had the meta INSIDE the proj-row__link anchor, so the meta was lifted out of the anchor into the group body — the anchor now wraps dot + name only.
- **cro-3** — Cell described wrapping the group directly; a <ul> may only contain <li>, so the <details> sits inside <li class="proj-row proj-group__row"> and branch rows moved into a nested <ul class="proj-group__branches"> in the body.
- **cro-3** — No JS click-guard added for the link/remove control inside <summary>: both are activatable elements, so a click is their own activation and never also toggles the group.
- **cro-3** — home_page_lists_projects_as_rows_with_each_worktree_under_its_parent (server.rs) had its orphan-row literal retargeted from <li class="proj-row"> to the new group wrapper — literal moved in lockstep, test not deleted.
- **cro-4** — Also replaced the stale phrase the home Kanban tab at line 47 with the home board — same retired tab vocabulary, same file
- **cro-4** — Added console-theme-kanban and console-rail-orchestrator to the spec frontmatter sources and bumped updated to 2026-08-22

## Provenance

Proposed by `bee knowledge promote --work console-rail-orchestrator` from 4 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/console-rail-orchestrator/CONTEXT.md`, `docs/history/console-rail-orchestrator/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "console-rail-orchestrator" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-22T10:02:46.388Z), the work item declares no bee.areas.

area bee-cockpit:
  - [cro-1] Home tab strip retired; a real Orchestrator anchor in the topbar's right slot is the board's entry point — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/archive/console-rail-orchestrator/cro-1.json)
  - [cro-2] Rail gained a Pinned group of live terminals and now renders beside the terminals view; Board row dropped — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/archive/console-rail-orchestrator/cro-2.json)
  - [cro-3] Rail project groups are collapsible <details> with a remembered collapsed set and filter-forced opening — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/archive/console-rail-orchestrator/cro-3.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell cro-1 — save as docs/knowledge/patterns/console-rail-orchestrator-cro-1-pitfall.md

---
type: bee.pattern
title: console-rail-orchestrator cell cro-1 — pitfall candidate
description: "Pitfall candidate mined from cell cro-1's capped trace: Rewrote terminals_tab_distinguishes_herdr_off_from_no_agents_running too: not on the cell name list but it sliced on the same strip literal, which the cell rul…"
timestamp: 2026-08-22
bee:
  id: console-rail-orchestrator-cro-1-pitfall
  lifecycle: draft
  areas: [bee-cockpit]
  sources: [.bee/cells/archive/console-rail-orchestrator/cro-1.json]
  polarity: pitfall
---

# console-rail-orchestrator cell cro-1 — pitfall candidate

## What the cell did

Home tab strip retired; a real Orchestrator anchor in the topbar's right slot is the board's entry point

## Recorded evidence (verbatim from .bee/cells/archive/console-rail-orchestrator/cro-1.json)

- **deviation** — Rewrote terminals_tab_distinguishes_herdr_off_from_no_agents_running too: not on the cell name list but it sliced on the same strip literal, which the cell rule covers
- **deviation** — Added an absence assertion (no <nav class="fg-tabs">) to the two views.rs tests as well as the server ones, so a quiet return of the strip is a red anywhere

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cro-2 — save as docs/knowledge/patterns/console-rail-orchestrator-cro-2-pitfall.md

---
type: bee.pattern
title: console-rail-orchestrator cell cro-2 — pitfall candidate
description: "Pitfall candidate mined from cell cro-2's capped trace: pinned dot uses role=img + aria-label instead of aria-hidden: a pinned row has no badge pills to carry its status as words, and no visually-hidden utility exis…"
timestamp: 2026-08-22
bee:
  id: console-rail-orchestrator-cro-2-pitfall
  lifecycle: draft
  areas: [bee-cockpit]
  sources: [.bee/cells/archive/console-rail-orchestrator/cro-2.json]
  polarity: pitfall
---

# console-rail-orchestrator cell cro-2 — pitfall candidate

## What the cell did

Rail gained a Pinned group of live terminals and now renders beside the terminals view; Board row dropped

## Recorded evidence (verbatim from .bee/cells/archive/console-rail-orchestrator/cro-2.json)

- **deviation** — pinned dot uses role=img + aria-label instead of aria-hidden: a pinned row has no badge pills to carry its status as words, and no visually-hidden utility exists in this stylesheet to add one without a design-system decision
- **deviation** — empty-state class is fg-empty, not the cell's fg-muted: fg-muted has no rule anywhere in app.css or atelier/, while fg-empty is the rail's own existing muted-line class
- **deviation** — two unauthenticated-leak tests narrowed, not deleted: D2 puts an unassigned pane's id in the rail's pinned href on the board, so both tests now pin the id to exactly that one address (name, title and cwd stay absolutely absent) — the same id was already readable at /?tab=terminals unauthenticated on this build, so the surface moved rather than widened; flagging for the human as a privacy-surface note
- **deviation** — home_page_renders_cross_project_features_beside_the_project_rail_on_kanban rewritten: D3 makes the terminals view render the project list, so the assertion now pins the rail's position (sibling before <main>) instead of its absence
- **deviation** — dead .home-sidebar__nav/__row/--on CSS removed — the retired Board row was their only user
- **deviation** — effective_pane extracted from terminals_tab so the rail and the screen resolve ?pane identically

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cro-3 — save as docs/knowledge/patterns/console-rail-orchestrator-cro-3-pitfall.md

---
type: bee.pattern
title: console-rail-orchestrator cell cro-3 — pitfall candidate
description: "Pitfall candidate mined from cell cro-3's capped trace: Cell said the summary holds the parent name line plus the delete form and the body holds proj-row__meta; the shipped row had the meta INSIDE the proj-row__link…"
timestamp: 2026-08-22
bee:
  id: console-rail-orchestrator-cro-3-pitfall
  lifecycle: draft
  areas: [bee-cockpit]
  sources: [.bee/cells/archive/console-rail-orchestrator/cro-3.json]
  polarity: pitfall
---

# console-rail-orchestrator cell cro-3 — pitfall candidate

## What the cell did

Rail project groups are collapsible <details> with a remembered collapsed set and filter-forced opening

## Recorded evidence (verbatim from .bee/cells/archive/console-rail-orchestrator/cro-3.json)

- **deviation** — Cell said the summary holds the parent name line plus the delete form and the body holds proj-row__meta; the shipped row had the meta INSIDE the proj-row__link anchor, so the meta was lifted out of the anchor into the group body — the anchor now wraps dot + name only.
- **deviation** — Cell described wrapping the group directly; a <ul> may only contain <li>, so the <details> sits inside <li class="proj-row proj-group__row"> and branch rows moved into a nested <ul class="proj-group__branches"> in the body.
- **deviation** — No JS click-guard added for the link/remove control inside <summary>: both are activatable elements, so a click is their own activation and never also toggles the group.
- **deviation** — home_page_lists_projects_as_rows_with_each_worktree_under_its_parent (server.rs) had its orphan-row literal retargeted from <li class="proj-row"> to the new group wrapper — literal moved in lockstep, test not deleted.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell cro-4 — save as docs/knowledge/patterns/console-rail-orchestrator-cro-4-pitfall.md

---
type: bee.pattern
title: console-rail-orchestrator cell cro-4 — pitfall candidate
description: "Pitfall candidate mined from cell cro-4's capped trace: Also replaced the stale phrase the home Kanban tab at line 47 with the home board — same retired tab vocabulary, same file"
timestamp: 2026-08-22
bee:
  id: console-rail-orchestrator-cro-4-pitfall
  lifecycle: draft
  areas: [bee-cockpit]
  sources: [.bee/cells/archive/console-rail-orchestrator/cro-4.json]
  polarity: pitfall
---

# console-rail-orchestrator cell cro-4 — pitfall candidate

## What the cell did

Re-synced the cockpit spec's front-page passage to the Orchestrator link, the Pinned rail and collapsible project groups

## Recorded evidence (verbatim from .bee/cells/archive/console-rail-orchestrator/cro-4.json)

- **deviation** — Also replaced the stale phrase the home Kanban tab at line 47 with the home board — same retired tab vocabulary, same file
- **deviation** — Added console-theme-kanban and console-rail-orchestrator to the spec frontmatter sources and bumped updated to 2026-08-22

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 4 capped cell(s) mined, 1 delivery draft, 3 area bullet(s), 4 pattern candidate(s), 0 file(s) written.

---

<!-- bee:not-a-deferral: this section records a completed review with no outstanding action. -->

## Resolution — 2026-08-25, the spec already records this feature

Reviewed in the sweep of the unapplied-proposal backlog.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/console-rail-orchestrator/delivery.md`
  already exists as a curated record.
- **(b) Area updates** — not applied. `docs/specs/bee-cockpit.md` and `docs/specs/web-interface.md` names `console-rail-orchestrator` in its own
  `sources`, which is the receipt that this feature's behaviour was merged into
  the spec when it closed. The generated bullets restate the same work in
  implementation vocabulary — function and type names — which a spec carries only
  in its Pointers, so applying them would add nothing the spec does not say and
  would break its vocabulary rule. This resolution rests on that receipt and on
  the spec owning the area, not on a line-by-line re-reading of every bullet.
- **(c) Pattern candidates** — none promoted from here.

<!-- /bee:not-a-deferral -->
