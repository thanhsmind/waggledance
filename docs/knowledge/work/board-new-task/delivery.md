---
type: bee.delivery
title: board-new-task — delivery
description: "Delivery record for work item board-new-task: 1 capped cell adding a + New task button and dialog on the home board that files a task into a chosen project's backlog through that project's own bee CLI."
timestamp: 2026-08-23
bee:
  id: board-new-task-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [docs/history/board-new-task/CONTEXT.md, docs/history/board-new-task/plan.md]
  sources: [docs/history/board-new-task/CONTEXT.md, docs/history/board-new-task/plan.md, .bee/cells/archive/board-new-task/board-new-task-1.json]
---

# board-new-task — Delivery

## What shipped

The home board's topbar carries a **+ New task** button beside Orchestrator.
It opens a dialog with a Task box and a Project picker; submitting files the
task into that project's backlog, where it appears in the first column
(Todo / "Pending Work") as a proposed item after the page reloads.

- **board-new-task-1** — `POST /api/projects/:id/pbi` is the console's first
  mutating bee call: the server runs the chosen project's own
  `<root>/.bee/bin/bee backlog pbi add --title <first line> [--cos <rest>]
  --json` with the project root as working directory and answers
  `{id, project_id}`. The dialog is a hidden overlay rendered once per home
  page (`.task-overlay` / `.task-box`, mirroring the jump palette), wired in
  `app.js`: Enter submits, Shift+Enter breaks a line, Cancel / Esc / scrim
  close, errors render inline and keep the typed text. 4 files, commit
  `850258f`.

## Behaviour that settled

- **The project's own bee is the only writer.** The binary is resolved from
  `<root>/.bee/bin/{bee,bee.exe}` — never PATH — and `.bee/backlog.jsonl` is
  never touched by Rust. A project without a vendored bee answers 409
  "not set up with bee".
- **Task text splits at the first line.** Leading blank lines are skipped;
  the first line (clipped to 200 chars) is the title, the remainder is the
  condition of satisfaction and is omitted — not passed empty — when blank.
- **Refusals are named.** 400 empty task, 404 unknown project, 409 no bee,
  502 non-zero exit carrying the CLI's stderr tail.
- **The client reloads itself.** Live reload only watches markdown, so a
  backlog write never triggers it; the dialog calls `location.reload()` on
  200.
- **Project picker lists top-level projects only**; worktree-branch rows
  are skipped. Preselection of the rail-filtered project is done client-side
  (the server has no "selected project" input on the home page).
- On handset widths the button hides with the Orchestrator button.

## Verify

- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — green, 1226 tests (10 new: route happy/edge/refusal paths against a fake `.bee/bin/bee` script, view markup, stylesheet parity).
- Smoke run of the worktree build on a scratch port with a fake project:
  GET `/` carried the button and overlay; POST answered 400 / 404 / 200 and
  the fake bee saw `backlog pbi add --title … --cos … --json` with cwd at
  the project root.

## Deviations

- Two page-wide assertions (`!contains("error")`, `!contains("<select")`)
  were narrowed through a `without_new_task_dialog` test helper because the
  dialog legitimately carries both tokens as chrome.
- The id fallback parser is a hand-written scan, not a regex (no regex
  dependency in the crate).
- Rail-filter preselection is split server (first project) / client
  (narrow to the filtered row) rather than a new `home_page` parameter.

## Provenance

Written at feature close from the capped cell trace of `board-new-task-1`
and the three decisions logged 2026-08-22 (dialog, storage via bee CLI,
outcome/error handling).
