---
type: bee.delivery
title: console-rail-orchestrator — delivery
description: "Delivery record for work item console-rail-orchestrator: 4 capped cell(s), 14 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: console-rail-orchestrator-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: []
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

Proposed by `bee knowledge promote --work console-rail-orchestrator` from 4 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/console-rail-orchestrator/CONTEXT.md`, `docs/history/console-rail-orchestrator/plan.md`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run after checking each line against the shipped source and the area specs.
