---
feature: board-new-task
started: 2026-08-22
status: locked
decisions: [82078151, cb52bbd1, 6e29ccc5]
---

# Add a task to a project's backlog from the home board

## What the user asked for

A "+ New task" button on the home board header (beside Orchestrator). It
opens a small dialog: a Task text box and — in place of the reference UI's
Agent/Model row — a Project picker. Submitting saves the task into that
project's backlog so it shows up in the board's first column.

## Locked decisions

**N1 — Entry and dialog.** (decision `82078151`)
The home topbar carries a `+ New task` button beside Orchestrator on both
home tabs. It opens an in-page overlay with a Task textarea and a Project
select listing top-level registered projects only (worktree-branch rows
excluded), preselecting the rail-filtered project when one is set, else the
first. Enter submits, Shift+Enter inserts a newline; Cancel, Esc and a scrim
click close it. On handset widths the button hides with Orchestrator.

**N2 — Storage goes through the project's own bee.** (decision `cb52bbd1`)
Submit POSTs JSON `{task}` to `/api/projects/:id/pbi`. The server runs
`<root>/.bee/bin/bee backlog pbi add --title <first non-empty line, ≤200
chars> --cos <rest, omitted when empty> --json` with cwd at the project
root. The server never writes `.bee/backlog.jsonl` itself. The result is a
`proposed` PBI, which the Todo column already renders.

**N3 — Outcome.** (decision `6e29ccc5`)
200 `{id, project_id}` → the client reloads the page. Any failure → JSON
`{error}` shown inline under the textarea, text kept. Named refusals: empty
task, unknown project, a root with no executable `.bee/bin/bee` ("this
project is not set up with bee"), a non-zero CLI exit (stderr tail).

## Out of scope

Editing or deleting tasks from the board; choosing an agent or model; a
per-project board button (the home topbar is the one entry).
