---
type: bee.concept
title: Google Sheets task sync
description: "How to set up and run two-way task synchronization between Waggledance projects and Google Sheets."
timestamp: 2026-09-24
bee:
  id: google-sheets-task-sync
  lifecycle: active
  areas: [orchestration, integrations]
  required_context: [docs/history/sheets-task-sync/CONTEXT.md]
  sources: [docs/history/sheets-task-sync/CONTEXT.md, crates/waggledance-core/src/sheets_sync.rs, crates/waggledance/src/sheets_sync_run.rs]
---

# Google Sheets task sync

Waggledance supports two-way task synchronization between project backlog tasks and a Google spreadsheet. Each registered project receives a dedicated tab in the spreadsheet.

## Set up Google Cloud and credentials

To configure Google Sheets synchronization, complete the following setup steps:

1. Open the [Google Cloud Console](https://console.cloud.google.com/) and create a project or select an existing project.
2. Enable the Google Sheets API for your Google Cloud project.
3. Open **IAM & Admin** > **Service Accounts**, and click **Create Service Account**. Enter a name and click **Done**.
4. Click on the created service account, open the **Keys** tab, click **Add Key** > **Create new key**, select **JSON**, and click **Create**.
5. Save the downloaded JSON key file to a secure directory outside any Git repository, for example `~/.waggledance/credentials/google-sheets-key.json`.
6. Restrict file permissions on the private key file:
   ```sh
   chmod 600 ~/.waggledance/credentials/google-sheets-key.json
   ```
7. Open your target Google spreadsheet in a web browser. Find the `client_email` value inside your downloaded JSON key file. Click **Share** in the spreadsheet, paste the service account email, assign the **Editor** role, and click **Send**.

## Configure Waggledance

Add a `[sheets]` section to `~/.waggledance/config.toml`. Run `waggledance config edit` to open the file in your default editor:

```toml
[sheets]
spreadsheet_id = "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms"
credentials_path = "/home/username/.waggledance/credentials/google-sheets-key.json"
```

The code reads two configuration keys under `[sheets]`:
- `spreadsheet_id`: The spreadsheet identifier from the Google Sheets URL between `/d/` and `/edit`.
- `credentials_path`: The absolute filesystem path to your downloaded service account JSON key file.

Restart the daemon after updating the configuration file:
```sh
waggledance restart
```

## Synchronize tasks

To run a synchronization, use the web interface or the CLI:

- **Web interface**: Open a project overview page in Waggledance and click **Sync Sheets** in the top bar. The button disables during the sync, displays results, and refreshes the page on success.
- **CLI for one project**: Run `waggledance sheets sync --project <name-or-id>`.
- **CLI for all projects**: Run `waggledance sheets sync --all`.

Add `--json` to output machine-readable synchronization reports.

## Sheet layout

Each project tab uses row 1 as a frozen header with columns A through F:

| Column | Header | Description |
|---|---|---|
| A | ID | Backlog item ID (PBI ID), or empty for new tasks |
| B | Type | Row type: `task` or `subtask` |
| C | Title | Task title, or subtask title prefixed with `↳ ` |
| D | Status | Task status or subtask status |
| E | Description | Task conditions of satisfaction, or subtask outcome |
| F | Feature | Associated feature slug |

- Task rows represent backlog items (PBIs).
- Subtask rows represent feature work cells (both active and archived) and sit directly under their parent task.
- Column D on task rows provides a dropdown list with five allowed statuses: `proposed`, `in-flight`, `parked`, `done`, and `declined`.

## Editable fields

The spreadsheet allows edits on task rows only:

- You can add a new task by creating a row with an empty column A and entering a title in column C.
- You can edit Title (column C), Status (column D), and Description (column E) on existing task rows.
- Edits apply through the project's own `bee` CLI commands (`backlog pbi add`, `backlog pbi amend`, and `backlog pbi status`). Waggledance never writes `.bee` files directly.
- Subtask rows are read-only. Manual edits to subtask rows on the sheet are overwritten on the next sync.

## Merge rules

Waggledance performs a three-way merge per field against the snapshot saved during the last sync:

- **Sheet change only**: If a field changed on the sheet but remained unchanged in bee, the sheet value applies to bee.
- **Both sides changed**: If a field changed on both sides, bee takes precedence. The conflict is recorded in the report with both values.
- **New task rows**: A row with no ID in column A and a non-empty title in column C creates a new PBI in bee.
- **Immediate ID write-back**: After each successful task creation, the new PBI ID is written back to column A before subsequent operations run.
- **Emptied fields**: If a title or description is cleared on the sheet, the change is skipped and reported. The `bee` CLI cannot clear a field.
- **Blank type**: An ID-less row with a title and blank Type defaults to a task.
- **Invalid status**: A status value outside the five allowed values is skipped and reported.
- **Deleted rows**: Deleting a row from the sheet never deletes a task in bee.
- **First sync**: When no snapshot exists, existing IDs take values from bee, and only rows without IDs are added.
- **Failed operations**: If a bee command fails, the sheet text remains on the tab and is excluded from the snapshot.
- **Exclusive lock**: Only one sync runs per project at a time using a lock file. A concurrent second sync returns an already-running error.
- **Tab identity**: The tab numeric identifier (`sheetId`) and snapshot are stored in `~/.waggledance/sheets-sync/<project_id>.json`. Renaming a tab in Google Sheets does not break synchronization.

## Known limits

- **Columns A through F are rewritten each sync**: After applying changes, columns A:F on the tab are replaced from current project state.
- **Extra columns G and beyond are not row-stable**: When tasks are reordered, inserted, or removed during a rewrite, data in extra columns does not move with its corresponding row.
- **Edits made during sync can be lost**: If you edit the spreadsheet while a synchronization is running, those edits can be overwritten when the tab is replaced.
- **Fields cannot be cleared from the sheet**: Clearing a title or description on the sheet does not clear the field in bee.
- **Subtasks cannot be created or edited on the sheet**: Subtask rows reflect internal work cells and are managed by bee.
