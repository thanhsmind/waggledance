---
feature: console-rail-orchestrator
started: 2026-08-22
status: locked
decisions: [393b93bb, 94cb9efb, 2bbc26bc, deca78d2]
---

# Orchestrator button, pinned terminals, collapsible projects

## What the user asked for

A screenshot of the homepage board annotated three ways, against the
agent-orchestrator shell:

1. Terminals appear in the left rail, as a group like the reference's
   "Pinned" list, above the projects.
2. The Kanban tab moves to the top-right of the topbar and is renamed
   "Orchestrator".
3. Each project's info block in the rail can be collapsed.

## Locked decisions

**D1 — The tab strip is retired; an `Orchestrator` button in the topbar's
right slot is the board's entry point.** (decision `393b93bb`)
A real anchor to `/?tab=kanban`, marked current on the board. The `tab=`
query values (`kanban`, `projects`, `terminals`) keep resolving exactly as
before; only the strip under the topbar disappears.
*Reconciled 2026-08-23:* this holds on wide screens only — at the 700px
handset breakpoint decision `41015896` hides the Orchestrator button and
the bottom tab bar's Board item is the board's entry point.

**D2 — The rail gains a `Pinned` group of live agent terminals above
`Projects`.** (decision `94cb9efb`)
Same inventory as the Terminals switcher (non-shell panes, sorted
blocked → working → rest). Each row: status dot, project label,
workspace·tab, linking to `/?tab=terminals&pane=<id>`. The group heading is
itself an anchor to `/?tab=terminals` and renders regardless of herdr's
state (keeps homepage-terminals D8); with no panes the group shows one
muted "No agents running" line. The rail's `Board` row is dropped.

**D3 — The rail renders on the terminals view too.** (decision `2bbc26bc`)
The terminals view sits inside the same `home-shell` beside the rail. The
rail carries at most one `aria-current="page"`: the pinned row of the
selected pane on the terminals view; none on the board (the Orchestrator
button is current there).

**D4 — Project groups collapse with a native `<details>`.** (decision
`deca78d2`)
Summary = the project's name line; open by default; the collapsed set is
remembered per browser in `localStorage["waggledance-rail-collapsed"]`
keyed by project id; the rail filter forces a matching group open.

## Boundaries this feature inherits

- Cockpit is read-only (`docs/specs/bee-cockpit.md`); the reference's
  `+ New task`, bell, and per-project icon buttons have no source and are
  not rendered (console-theme-kanban D2).
- `/?tab=projects` still maps to the board (ctk-12); a `register_error`
  still forces the board so the banner lands on the rendered section.
- Exact-literal tests on the strip and the rail move in lockstep with the
  markup — a quietly broken one is a red base.
- The phone phase of the rail stays OWED; this feature adds no
  narrow-screen behaviour. *Reconciled 2026-08-23:* that phase landed
  under console-phone-layout (decisions `d87ef556`, `41015896`).
