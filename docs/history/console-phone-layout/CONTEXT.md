---
feature: console-phone-layout
started: 2026-08-22
status: locked
decisions: [d87ef556, 41015896, 6d3f820f, 8c86d602]
---

# Phone layout for the homepage shell

## What the user asked for

Continue the mobile work: the homepage's two-column shell (project rail +
board / terminals view) has no narrow-screen behaviour — the phone phase
owed since console-theme-kanban (its D3: the phone screen is responsive
CSS over the same markup, no second route).

## Locked decisions

**P1 — One column at the existing 700px breakpoint; the rail becomes an
off-canvas drawer.** (decision `d87ef556`)
CSS-only checkbox toggle (the topbar-menu pattern), closed by backdrop or
toggle, no script, survives the homepage's full reload.

**P2 — A bottom tab bar of four items: Board · Agents · Projects ·
Settings.** (decision `41015896`)
*Reconciled 2026-08-23:* the bar is collapsible — hidden by default behind a
remembered bottom-edge handle, visible with scripting off (decision
`75a5b463`, tabbar-collapse).
Real anchors (`/?tab=kanban`, `/?tab=terminals`, `/settings`) except
Projects, which is the drawer's toggle label; the current item carries
`aria-current`. The topbar's Orchestrator button hides at that width. No
FAB, no bell.

**P3 — Three stat tiles above the board: working / need you /
mergeable.** (decision `6d3f820f`)
working = in-progress group count; need you = features with a waiting-on
mark; mergeable = ready-to-merge count. Big mono number over a small
label, faint at zero, each an in-page anchor to its section. In the
markup at every width, shown only at the handset breakpoint.

**P4 — Columns stack as sections, need-you-first, empty ones hidden.**
(decision `8c86d602`)
Order on a phone: groups with waiting-on marks, ready to merge, in
progress, the rest. A zero-card group is hidden. ARCHIVE stays a
collapsed details at the bottom. Everything inside the board's one
existing `@media (max-width: 700px)` block.

## Boundaries this feature inherits

- Cockpit read-only (`docs/specs/bee-cockpit.md`); nothing without a data
  source is drawn (console-theme-kanban D2).
- `bee_hub_style_puts_in_progress_order_rule_only_inside_the_narrow_media_query`
  pins exactly one narrow media query in the board style.
- The 53px topbar height is a sticky-offset contract.
- `.home-shell` wraps both the board and the terminals view; the collapse
  applies to both.
- Desktop layout (>700px) is unchanged pixel-for-pixel in intent: tiles
  and the tab bar are hidden there, the rail stays in flow.
