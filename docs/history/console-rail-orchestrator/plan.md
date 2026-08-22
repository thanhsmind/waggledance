---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: console-rail-orchestrator

Mode: `standard` — 2 risk flags: covered-contract-change, multi-domain
Why this is the least workflow that protects the work: four product files and
~30 literal-asserting tests move together; one cell per surface keeps each
test migration beside the markup it pins.

## Requirements (from CONTEXT.md)
- D1: tab strip retired; topbar-right `Orchestrator` anchor to `/?tab=kanban`.
- D2: rail `Pinned` group of live terminals above `Projects`; heading anchors
  `/?tab=terminals`; `Board` row dropped.
- D3: rail renders on the terminals view; ≤1 `aria-current` in the rail.
- D4: `<details>` project groups, open by default, collapsed set in localStorage.

## Discovery
Gather digest over `views.rs`/`server.rs`/`app.js`/`app.css`: the strip is
`home_tab_strip` (`views.rs:176`) placed `topbar → tabs → section`
(`views.rs:310`); the rail is `project_sidebar` (`views.rs:363`), Kanban-only;
the terminals inventory is `terminals_menu_panes` (`server.rs:3407`), already
threaded into `home_page`; the topbar right slot is `topbar_full`'s actions
slot (`views.rs:6593`). No collapse persistence exists for the rail.

## Approach
Recommended: keep `HomeTab`, `RegisterFlag`, and the terminals route intact;
change composition only — `home_page` wraps both tab bodies in `home-shell`
with the rail (D3), `topbar_full`'s action slot receives the Orchestrator
anchor (D1), `project_sidebar` grows a pinned group fed by the existing
`TerminalsMenuPane` list (D2), and the per-project `<li>` becomes a
`<details>` group (D4). Rejected: a JS-driven tab switch (violates
homepage-terminals D1 real-anchor rule); a second drawer instance (duplicate
`#agent-drawer-toggle`).

Risk map: `views.rs` composition / MEDIUM / view + server tests migrated in
the same cell · `app.js` filter × details / LOW / selector-parity test ·
`app.css` rail on terminals view / LOW / look at staging.

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 Orchestrator button | `home_tab_strip` removed; `topbar_full` action slot carries the Orchestrator anchor (`fg-btn topbar__orchestrator`, `aria-current="page"` on the board); strip tests retargeted | D1 is the smallest visible change and frees the strip literals every other test slices on | `/` shows the button top-right, no strip | 2 |
| 2 Pinned terminals + rail on both views | `project_sidebar` gains `Pinned` group from `terminals_menu_panes`, drops `Board`; `home_page` renders `home-shell` for Terminals too; `terminals_tab` keeps its body inside the shell's `<main>` | D2/D3 are one composition change | rail shows live panes; clicking one opens it beside the rail | 3 |
| 3 Collapsible projects | `<details class="proj-group" open data-project-id>` around each group; JS persists `waggledance-rail-collapsed`, filter opens matches; CSS for summary/chevron | D4 independent of 2 but shares `project_sidebar` | collapse a project, reload, still collapsed | 4 |
| 4 Spec sync | `docs/specs/bee-cockpit.md` homepage section re-synced (strip gone, pinned group, collapse) | spec already stale since ctk-12 | spec matches shipped markup | close |

## Test matrix
- Happy: button current on board; pinned rows link `/?tab=terminals&pane=`;
  selected pane row current on terminals view; details open by default.
- Edge: no panes → "No agents running" line, heading still anchors;
  `register_error` + `?tab=terminals` → board + banner; `?tab=projects` → board.
- Error: vanished `?pane` → "This terminal is gone." still beside the rail;
  rail holds ≤1 `aria-current`.
- Existing literal tests (`home_page_tab_strip_marks_exactly_one_tab_selected`,
  `home_page_with_empty_board_still_renders_tab_strip_and_terminals_tab`,
  `home_page_with_no_qualifying_project_still_renders_the_tab_strip`,
  `home_page_register_error_forces_the_tab…`, `project_rail_is_a_named_landmark…`,
  `kanban_tab_rail_carries_everything…`, `home_page_script_selectors_match…`)
  are rewritten to the new literals, never deleted.

## Out of scope
- `+ New task`, bell, per-project icon buttons (no data source).
- Phone layout of the rail (still OWED).
- Any change to the Terminals route or `HomeTab` parsing.
