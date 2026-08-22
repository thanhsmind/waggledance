---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: console-phone-layout

Mode: `standard` — 2 risk flags: covered-contract-change, multi-domain
Why this is the least workflow that protects the work: three product files,
one literal-pinned test on the media block, and a user-visible surface that
must keep desktop untouched — one cell per layer keeps the proof beside it.

## Requirements (from CONTEXT.md)
- P1: one column ≤700px; rail as CSS-only off-canvas drawer.
- P2: bottom tab bar Board · Agents · Projects · Settings; Orchestrator
  button hidden at that width.
- P3: stat tiles working / need you / mergeable above the board, anchors to
  sections, visible only on phone.
- P4: sections stacked need-you-first, empty hidden, inside the one existing
  narrow media block.

## Discovery
Gather over `app.css`, `views.rs`, `app.js`, and the reference
`packages/mobile`: no media query touches `.home-shell`/`.home-sidebar`
(app.css:729 "OWED"); the board's only narrow block is inside
`bee_hub_style` (views.rs ~2785) and a test pins its count at one; the topbar
menu already ships a CSS-only `:checked ~ panel` drawer pattern
(app.css:244-389); the reference phone shell is a four-tab bottom bar, a
vertical section list (empty zones dropped, needs-you first) and three stat
tiles that jump to sections.

## Approach
Markup additions in `home_page` (tab bar, drawer toggle + backdrop) and in the
board render (tiles); CSS additions in `app.css`'s shell block under a new
`@media (max-width: 700px)` for the shell (app.css may add one — only the
board's inline style is count-pinned), and the stacking/order/empty rules
inside the board's existing narrow block. Rejected: generalising the
`.layout` JS drawer (script-bound, loses state on reload); a separate mobile
route (console-theme-kanban D3 forbids it).

Risk map: shell collapse / MEDIUM / server test renders `/` and asserts the
drawer + tab bar literals · board stacking / LOW / existing media-count test
plus a new order/hide literal test · tiles / LOW / counts asserted against a
fixture board.

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 Phone shell | `home_page` adds `<input id="rail-toggle">`, backdrop label, and `<nav class="home-tabbar">` with four items; app.css narrow block hides the rail off-canvas, shows the tab bar, hides `.topbar__orchestrator` | P1/P2 are the frame everything else sits in | at 390px the rail is gone, the tab bar is at the bottom, Projects opens the rail | 2 |
| 2 Phone board | tiles markup + counts; stacking order, empty-hide, tile visibility inside the existing board media block | P3/P4 need the frame | at 390px the board leads with three tiles and stacked sections | 3 |
| 3 Spec sync | `docs/specs/bee-cockpit.md` gains the handset passage | spec is the state layer | spec matches | close |

## Test matrix
- Happy: `/` renders the tab bar with Board current; `/?tab=terminals` with
  Agents current; tiles carry the fixture's counts and `href="#..."` anchors.
- Edge: zero counts → tile faint class; a group with zero cards carries a
  hook the CSS hides; the drawer toggle id is unique on the page.
- Error/contract: exactly one `@media (max-width: 700px)` in
  `bee_hub_style`; `order: -1` semantics preserved; desktop markup literals
  from console-rail-orchestrator tests unchanged.

## Out of scope
- FAB, bell, project switcher sheet (no source / read-only).
- Redesigning card anatomy on phone beyond stacking.
- File-page drawer (`.layout`) — untouched.
