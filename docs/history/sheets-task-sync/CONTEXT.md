# Context: sheets-task-sync

Two-way sync between each waggledance project's tasks and one Google
spreadsheet, one tab per project.

Target spreadsheet (user-provided, empty at 2026-09-24):
`1wVu6PostKTHNnYgNRrNIyS1yl1KI6De2SyZD3xPkzJw`.

## Locked decisions

- **D1 — Task model.** A task row is a PBI from the project's
  `.bee/backlog.jsonl`. Its subtask rows are the cells of the PBI's
  `feature` (live and archived). A PBI with no feature has no subtasks.
  (user, 2026-09-24)
- **D2 — Write-back scope.** From the sheet, a user may add a PBI and change
  a PBI's title, description (cos) and status. Every change is applied
  through the project's own bee CLI (`backlog pbi add|amend|status`), never
  by writing `.bee/` files. Subtask (cell) rows are read-only: sheet edits
  to them are overwritten on the next sync. (user, 2026-09-24)
- **D3 — Auth.** A Google service-account JSON key, stored outside every
  repo; its path lives in `~/.waggledance/config.toml` under `[sheets]`.
  The user shares the spreadsheet with the service-account email. The key
  is never logged and never sent in an HTTP response. (user, 2026-09-24)
- **D4 — Trigger.** Sync runs only when asked: a "Sync Sheets" button on
  the project page, and `waggledance sheets sync`. No timer, no watcher.
  (user, 2026-09-24)
- **D5 — Merge rules.** Three-way merge per PBI field against the snapshot
  saved at the last sync. Changed only on the sheet → applied to bee.
  Changed on both sides → bee wins, reported as a conflict. A row with no ID
  and a non-empty title → new PBI. A deleted row never deletes a PBI. A
  status outside `proposed, in-flight, parked, done, declined` is skipped and
  reported. With no snapshot (first sync), existing IDs take the bee value;
  only ID-less rows are added. After changes apply, the whole tab is
  rewritten from bee. (agent default, logged)
- **D6 — Tab identity.** Tab title = project name (made unique if two
  projects share a name). The tab's numeric `sheetId` and the snapshot are
  kept in `~/.waggledance/sheets-sync/<project_id>.json`, so a tab the user
  renames keeps syncing. (agent default, logged)

- **D7 — Safety rules (from the plan review).** (a) After each successful
  add, the new PBI id is written into that row's ID cell before the next
  op. (b) An emptied title or description is skipped and reported; bee
  cannot clear a field. (c) An ID-less titled row with blank Type is a task.
  (d) A failed op keeps the user's sheet text on the rewritten tab and out of
  the snapshot. (e) One sync per project at a time (lock file); a second
  caller gets 409. (f) Conflict/skipped report lines carry both values.
  (g) Tab lookup falls back to title only when no other project's state
  claims that sheetId. (h) The CLI needs `--project` or `--all`. (i) The
  button sits on the project overview page, disables while running,
  reloads on success. (j) The sheet shows the same path-scrubbed text the UI
  shows. (k) JWT signed with `ring` directly; the tab is rewritten by one
  padded PUT; the status dropdown is cleared on `D2:D` before being set on
  task rows. (l) The snapshot records its `spreadsheet_id`; a mismatch
  discards it (first-sync rules). (agent, logged)

## Sheet layout

Row 1 is a frozen header. Columns:

| A | B | C | D | E | F |
|---|---|---|---|---|---|
| ID | Type | Title | Status | Description | Feature |

- Task row: `Type = task`, Description = PBI `cos`.
- Subtask row: `Type = subtask`, Title prefixed `↳ `, Status = cell status,
  Description = cell outcome; placed directly under its task.
- Status column on task rows carries a dropdown of the five PBI statuses.
