---
type: bee.delivery
title: rail-collapse-menu — delivery
description: "Delivery record for work item rail-collapse-menu: 1 capped cell folding the home rail to a 44px strip on wide screens and turning each project row's remove control into a Docs / Remove menu."
timestamp: 2026-08-22
bee:
  id: rail-collapse-menu-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/cells/rcm-1.json]
---

# rail-collapse-menu — Delivery

## What shipped

- **rcm-1** — a chevron collapses the home rail to a 44px strip on wide
  screens, remembered per browser (`waggledance-rail-hidden`); each project
  row's ✕ became a … disclosure menu offering Docs and Remove. `views.rs`,
  `app.css`, `app.js`, `server.rs`; commit `bd34f6c`.

## Verify

`cargo test -p waggledance home_page` — green; the existing selector test
still matches `.proj-row__delete`.

## Deviations

- Scope extended into `server.rs` (reserved first): five selector-drift rows
  plus the duplicate-row guard, which counted the bare `/p/<id>/` href the new
  Docs item also emits.
- The rail filter matched on the row's whole text, which now carries the
  menu's Docs/Remove words on every row — it strips the menu text first.
- The group-shape test slices `<details>` by depth: the nested row menu made
  the first `</details>` / `</summary>` stop meaning the group's own.
- The collapse button is hidden under 700px so the phone drawer never shows it.

## Behaviour that settled

- A `<details>` nested inside a group's `<summary>` needs its clicks stopped
  from propagating, and markup tests must slice by depth.

## Provenance

Written at the 2026-08-23 compound run from the capped cell trace; behaviour
merged into `docs/specs/bee-cockpit.md` (left rail).
