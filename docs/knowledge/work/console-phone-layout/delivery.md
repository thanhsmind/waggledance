---
type: bee.delivery
title: console-phone-layout — delivery
description: "Delivery record for work item console-phone-layout: 3 capped cell(s), 6 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: console-phone-layout-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: []
  sources: [docs/history/console-phone-layout/CONTEXT.md, docs/history/console-phone-layout/plan.md, .bee/cells/archive/console-phone-layout/cpl-1.json, .bee/cells/archive/console-phone-layout/cpl-2.json, .bee/cells/archive/console-phone-layout/cpl-3.json]
---

# console-phone-layout — Delivery

## What shipped

- **cpl-1** — Handset shell: rail becomes a CSS-only drawer and a four-item bottom tab bar appears under 700px; desktop untouched (3 file(s) changed)
- **cpl-2** — Handset board leads with working/need-you/mergeable tiles and stacks groups need-you-first inside the one narrow media block (1 file(s) changed)
- **cpl-3** — Cockpit spec now describes the handset shell, bottom bar, stat figures and stacking order (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **cpl-1** — `cargo test -p waggledance -- home_page tabbar rail`
- **cpl-2** — `cargo test -p waggledance -- bee_hub hub_stats`
- **cpl-3** — `rg -n "handset|tab bar|need you" docs/specs/bee-cockpit.md`

## Deviations

- **cpl-1** — Scoped the pre-existing terminals_view_with_a_vanished_pane assertion to the rail slice: it counted aria-current document-wide, and the tab bar is a second landmark with its own correct marker
- **cpl-1** — Test bounds the CSS slice at the next @media rather than end-of-file — app.css already carries a second max-width:700px block for the file page
- **cpl-2** — need-you tile anchors at #hub-in-progress: In Progress is the only group this board ever hands a non-zero waiting_count, so the first-waiting-group target and the nothing-waiting fallback are the same section (documented on bee_hub_stat_tiles)
- **cpl-2** — bee_hub_group_renders_console_header_anatomy_... kept but its wrapper literal updated to the new opening tag, since id and data-hub-waiting are required on that wrapper
- **cpl-2** — server.rs listed on the cell went untouched: its route-level board tests already cover the rendered style block and stayed green
- **cpl-3** — First write command ran without the BEE_AGENT_NAME prefix; the file was already reserved to phone-3 and later commands carried it

## Provenance

Proposed by `bee knowledge promote --work console-phone-layout` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/console-phone-layout/CONTEXT.md`, `docs/history/console-phone-layout/plan.md`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run after checking each line against the shipped source and the area specs.
