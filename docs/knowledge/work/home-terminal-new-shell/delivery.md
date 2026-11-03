---
type: bee.delivery
title: home-terminal-new-shell — delivery
description: "Delivery record for work item home-terminal-new-shell: the homepage Terminals tab offers the plain New shell button, making its switcher identical to the project terminal page's."
timestamp: 2026-08-31
bee:
  id: home-terminal-new-shell-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/specs/agent-terminal.md]
  sources: [.bee/lanes/home-terminal-new-shell.json, .bee/cells/htns-1.json]
---

# home-terminal-new-shell — Delivery

## What shipped

- **htns-1** — The homepage Terminals tab's pane bar offers the plain "New shell"
  button beside its configured agent presets, so its switcher is the project
  terminal page's switcher rather than a near-copy: same `pane_bar` / `pane_strip`
  / `pane_tab` markup and CSS it already shared, and now the same creation row.
  The `plain_shell` parameter that had withheld the button from this one caller
  is deleted along with the early return that omitted the `.term-create` box when
  nothing was left to offer — a box always has at least this button now. A
  selected pane outside every registered project still renders no creation
  controls at all; that remains the caller's decision, where the missing `:id`
  actually is. (2 file(s) changed)

## Why the earlier rule was reversed

`home-terminal-header` had withheld the button on the grounds that this tab was
not scoped to one project, so a shell started from it could only mean "a shell in
whichever project the watched screen belongs to". `terminals-tab-project-scope`
(above) retired that premise: the tab resolves its create target from the
effective pane's own `project_id` and its switcher lists only that project's
panes. Both pages now mean the same thing by "new shell here", and a flag with
one live value at both call sites is not a flag.

Decision record: `home-terminal-new-shell` (2026-08-31), scope repo, source user.

## Verify

`cargo test -p waggledance-core -p waggledance --no-fail-fast` — green, 1622
passed. The HTTP-level tests drive the real axum router
(`GET /?tab=terminals&pane=…`), so the whole render path is covered rather than
the view function alone. Confirmed live afterwards against the installed daemon
on 127.0.0.1:7700: the served page carries `.term-create` with its New shell
button inside `.pane-menu__panel`, beside the pane strip.

## Deviations

- The plan named `views.rs` alone; three tests pinning the old shape live in
  `server.rs` and the `views.rs` test module. `server.rs` was reserved on
  discovery.
- One of those, `homepage-terminal-full` D5's `data-project-id` pin, asserted the
  string appeared nowhere on the page — true only because its fixture configures
  no presets, so the create box never rendered. Narrowed to what D5 actually
  says (the page's only project id is `.term-create`'s own, the create route's
  target `app.js` has always read) rather than deleted.
- No route record: `bee state route --set` requires a lane-bound session and this
  harness refuses the binding command by name; writing the default record would
  have overwritten another feature's triage. Route facts live in
  `docs/history/home-terminal-new-shell/plan.md`.

## Provenance

Captured at close from the capped `htns-1` trace. Target surface chosen by the
owner from an explicit three-option question (sidebar Agents rail / Terminals tab
pane bar / agent switch drawer). Pattern candidate raised and recorded separately:
`docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md`
gained disguise 4 from the UAT install that followed this work.
