---
type: bee.delivery
title: board-topbar-polish — delivery
description: "Delivery record for work item board-topbar-polish: 1 capped cell quieting the active Orchestrator pill on the home topbar and turning the New task box into a real textarea."
timestamp: 2026-08-23
bee:
  id: board-topbar-polish-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [docs/knowledge/work/board-new-task/delivery.md]
  sources: [.bee/cells/archive/board-topbar-polish/board-topbar-polish-1.json]
---

# board-topbar-polish — Delivery

## What shipped

Two follow-ups the user spotted on the shipped New task dialog:

- **The current-page Orchestrator pill is quiet.** It used the chalk action
  fill, which glared against the dark console beside the ghost buttons next
  to it. It is now an ink wash on the bar's own surface
  (`color-mix(in srgb, var(--color-text) 10%, transparent)`, strong border,
  body text colour; 14 % on hover). The filled shape still marks "you are
  here"; the colour no longer shouts it.
- **The Task box is a writing area.** The textarea carried only `fg-input`,
  whose fixed `--input-height` squashed it to one line. It now carries the
  atelier textarea modifier `fg-input--area` (auto height, padding) plus a
  `7.5rem` min-height on `.task-box__input`, `rows="5"`.

- **board-topbar-polish-1** — `app.css`, `views.rs`; commit `054e6b7`.

## Behaviour that settled

- A textarea styled with the atelier kit needs `fg-input fg-input--area`;
  `fg-input` alone is a one-line field by design.
- Topbar "current" state on the console theme is an ink-wash pill, not the
  action fill.

## Verify

`cargo fmt --all --check && cargo clippy -p waggledance --all-targets -- -D
warnings && cargo test -p waggledance home_page` (33) + `new_task` (5) +
`stylesheet` — green. Live check after reinstall + `systemctl --user restart
waggledance`: `/` renders the new textarea class, `/static/app.css` carries
the new pill rule.

## Deviations

None.
