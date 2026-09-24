---
mode: high-risk
---

# Plan: sheets-task-sync

## Summary

Each waggledance project gets its own tab in one Google spreadsheet. The tab
lists the project's tasks (backlog items) with their subtasks (the work
cells) underneath. Pressing "Sync Sheets" on the project page, or running
`waggledance sheets sync --project <p>` / `--all`, pulls task edits made on
the sheet back into the project and then rewrites the tab from the project's
current state. On the sheet you can add tasks and change a task's title,
description and status; subtask rows are for reading only.

Mode: `high-risk` — 4 risk flags: external-systems (Google APIs), auth
(service-account key), public-contracts (new API route and CLI verb),
multi-domain (core model, HTTP client, daemon UI).
Why this is the least workflow that protects the work: the one hard-gate
flag (an external provider holding a credential) needs the full check; the
merge logic is kept pure so it is tested without Google.

## Requirements (from CONTEXT.md)

- D1: task = PBI, subtask = cells of the PBI's feature (live + archived).
- D2: sheet may add PBIs and change title/cos/status, applied only via the
  project's bee CLI; cell rows read-only.
- D3: service-account JSON key, path in `[sheets]` config, never logged or
  returned over HTTP.
- D4: manual trigger only — project-page button and `waggledance sheets sync`.
- D5: three-way per-field merge vs last-sync snapshot; bee wins conflicts,
  reported; ID-less row = new PBI; row delete never deletes; bad status
  skipped and reported; first sync adds only ID-less rows; tab rewritten
  after apply.
- D6: tab title = project name (unique); sheetId + snapshot in
  `~/.waggledance/sheets-sync/<project_id>.json`.
- D7 (a–l): safety rules from the plan review — id write-back after each
  add, empty-field skip, blank Type = task, failed ops keep sheet text, lock
  file, two-valued report lines, guarded title fallback, `--project|--all`,
  button on project overview, scrubbed text, ring signing + padded PUT +
  dropdown clear, snapshot bound to spreadsheet_id.

## Load-bearing claims

Labels are `read` (opened at the anchor) or `ran` (command executed); the
evidence column is a verbatim substring of the anchored line or output.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | The daemon crate already has an HTTPS client with JSON support | read | crates/waggledance/Cargo.toml:45 | `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }` |
| 2 | `ring` is already locked, so RS256 signing needs no new crypto crate | read | Cargo.lock:1784 | `name = "ring"` |
| 3 | `base64` is already locked, for PEM decoding and JWT segments | read | Cargo.lock:176 | `name = "base64"` |
| 4 | A project's PBIs and live cells come from one core call | read | crates/waggledance-core/src/bee.rs:1291 | `pub fn read_snapshot(root: &Path) -> BeeSnapshot {` |
| 5 | Archived cells of a feature are readable by feature slug | read | crates/waggledance-core/src/bee.rs:2596 | `pub fn read_archived_cells(root: &Path, feature: &str) -> Vec<BeeCell> {` |
| 6 | The snapshot carries the folded PBI list | read | crates/waggledance-core/src/bee.rs:617 | `pub pbis: Vec<BeePbi>,` |
| 7 | The server already resolves a project's own bee binary for write-backs | read | crates/waggledance/src/server.rs:1891 | `fn project_bee_binary(root: &std::path::Path) -> Option<PathBuf> {` |
| 8 | A POST project route pattern exists to copy | read | crates/waggledance/src/server.rs:516 | `.route("/api/projects/:id/pbi", post(create_project_pbi))` |
| 9 | CLI commands build the engine in-process, so the CLI can sync without the daemon | read | crates/waggledance/src/cli.rs:362 | `let engine = runtime::build_engine()?;` |
| 10 | bee can change a PBI title/cos without touching status | ran | .bee/bin/bee backlog pbi amend --help | `Append a PBI "amend" event updating --title and/or --cos (at least one required).` |
| 11 | The PBI status enum is exactly five values | ran | .bee/bin/bee backlog pbi status --id p-nonexist --to bogus | `expected one of proposed, in-flight, parked, done, declined` |
| 12 | Config is a serde-default struct a new section can join | read | crates/waggledance-core/src/config.rs:12 | `pub struct Config {` |
| 13 | The project overview page renders its own topbar with an empty actions slot | read | crates/waggledance/src/views.rs:2031 | `pub fn project_home_page(project: &Project, entry: Option<&str>, bee: bool) -> String {` |
| 14 | That page calls topbar_full, whose third argument is the actions slot | read | crates/waggledance/src/views.rs:2064 | `topbar = topbar_full(` |

## Discovery

A read-only gather mapped the project store, bee readers, bee CLI shell-out,
config, axum routes, CLI enum, dependencies and topbar (report:
`.bee/mailbox/job-1790215063903-2071929-1/report-1.md`). No Google or JWT
code exists in the repo. The target spreadsheet is empty. The plan review
(five seats) found the button anchored on the cross-project home page, a
tab-hijack path, a duplicate-PBI path, and an empty-amend error loop; D7
closes each.

## Approach

Recommended path: a pure merge core in `waggledance-core` (D1, D2, D5, D6,
D7 data rules), a small Google client in the daemon crate (D3, D7k), then one
runner that both surfaces call (D4, D7). The runner locks, reads the tab,
plans ops with the pure core, applies ops through the project's bee CLI
writing each new id back immediately, re-reads bee, rewrites the tab,
saves the snapshot.

Rejected: `yup-oauth2`/`gcp_auth`/`jsonwebtoken` (new crate trees; ring
already signs RS256); writing `.bee/backlog.jsonl` directly (violates D2);
per-cell revision history for last-writer-wins (not exposed by the values
API); column-wide status dropdown (subtask statuses would show invalid).

Risk map:

| Component | Risk | Lands in | Proof needed |
|---|---|---|---|
| Merge rules (D5, D7) | HIGH | sts-1 | unit tests per rule |
| Duplicate PBI after partial failure | HIGH | sts-3 | runner test: add ok, rewrite fails, 2nd sync adds 0 |
| JWT + token exchange | MEDIUM | sts-2 | mock token endpoint verifies signature |
| Key leakage | HIGH | sts-2, sts-3 | tests: no private_key bytes in Debug, errors, /api/config |
| Tab hijack by same name | MEDIUM | sts-3 | two same-name projects get two tabs |
| Real Google call | MEDIUM | uat | user runs a sync with their service account |

Waves: sts-1 ∥ sts-2 (disjoint files) → sts-3 (needs both) → sts-4 (docs of
the finished surface).

## Shape

Epic map. Outcome: a manual button syncs one project's tasks with its tab.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| Merge core | D5/D7 correctness | Wrong merge silently loses edits | 1 | unit tests |
| Google client | D3 auth, API I/O | External provider | 1 | mock-server tests |
| Runner + surfaces | D4, D6, D7 | User-visible path | 1 | runner, route and view tests |
| Setup docs | D3 onboarding | Service account steps are non-obvious | 1 | pointer check |

One slice; all four cells are current.

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| sts-1 | Add the pure Sheets sync model and config section | crates/waggledance-core/src/sheets_sync.rs (new), crates/waggledance-core/src/lib.rs, crates/waggledance-core/src/config.rs | — | nothing yet; unblocks the runner with tested merge rules | cargo test -p waggledance-core sheets_sync |
| sts-2 | Add a service-account Google Sheets client | crates/waggledance/src/gsheets.rs (new), crates/waggledance/src/main.rs, crates/waggledance/Cargo.toml, Cargo.lock | — | nothing yet; unblocks the runner with a tested client | cargo test -p waggledance gsheets |
| sts-3 | Wire the sync runner to an API route, CLI verb and project button | crates/waggledance/src/sheets_sync_run.rs (new), crates/waggledance/src/main.rs, crates/waggledance/src/server.rs, crates/waggledance/src/cli.rs, crates/waggledance/src/views.rs, crates/waggledance/assets/app.js | sts-1, sts-2 | "Sync Sheets" on the project page and `waggledance sheets sync` report added/updated/conflicts | cargo test -p waggledance sheets_sync |
| sts-4 | Document Google Sheets sync setup | docs/knowledge/concepts/google-sheets-task-sync.md (new), README.md | sts-3 | README explains how to create the key, share the sheet and sync | rg pointer check |

```json
[
  {
    "id": "sts-1",
    "feature": "sheets-task-sync",
    "title": "Add the pure Sheets sync model and config section",
    "lane": "high-risk",
    "role": "code",
    "deps": [],
    "decisions": ["D1", "D2", "D5", "D6", "D7"],
    "files": ["crates/waggledance-core/src/sheets_sync.rs", "crates/waggledance-core/src/lib.rs", "crates/waggledance-core/src/config.rs"],
    "read_first": ["docs/history/sheets-task-sync/CONTEXT.md", "crates/waggledance-core/src/config.rs", "crates/waggledance-core/src/bee.rs"],
    "affects_skills": [],
    "affects_specs": [],
    "behavior_change": true,
    "action": "Create module sheets_sync in waggledance-core (declare `pub mod sheets_sync;` in lib.rs). Add to Config a serde-default field `sheets: SheetsConfig` with `spreadsheet_id: String` and `credentials_path: Option<PathBuf>`, empty/None by default, plus `SheetsConfig::is_configured()` (per D3). In sheets_sync define: SheetRow {id, kind (Task|Subtask), title, status, description, feature}; `HEADER` = [ID, Type, Title, Status, Description, Feature]; `PBI_STATUSES` = proposed, in-flight, parked, done, declined; `render_rows(pbis: &[BeePbi], cells_for: impl Fn(&str) -> Vec<BeeCell>) -> Vec<Vec<String>>` producing header, then each PBI row followed by its feature's cells as subtask rows (title prefixed '↳ ', description = cell outcome or empty) per D1 and CONTEXT.md 'Sheet layout'; PBIs with empty feature get no subtasks; also return (or expose a helper for) the 0-based indices of task rows for the dropdown. `parse_rows(values: &[Vec<String>]) -> Vec<SheetRow>` skips the header and fully blank rows, tolerates short rows, and treats a row with blank Type as Task (D7c); keep each row's sheet index. SyncSnapshot {spreadsheet_id: String, sheet_id: Option<i64>, tab_title: Option<String>, pbis: BTreeMap<String, SnapPbi{title, cos, status}>} serde-able; `plan_sync(bee: &[BeePbi], sheet: &[SheetRow], snapshot: Option<&SyncSnapshot>, spreadsheet_id: &str) -> SyncPlan` returning ops (Add{row_index, title, cos, status}, Amend{id, title: Option, cos: Option}, SetStatus{id, to}) plus `conflicts` and `skipped` as Vec<String> whose lines carry both the sheet and the bee value (D7f), implementing D5 exactly: only Task rows are considered (subtask rows never yield ops, D2); a snapshot whose spreadsheet_id differs is treated as absent (D7l); per field, three-way compare trimmed sheet vs snapshot vs bee; sheet-only change -> op; both changed and differ -> conflict, no op; no snapshot entry -> no op for existing IDs; a title or cos emptied on the sheet -> skipped with reason 'bee cannot clear a field' (D7b); ID-less row with non-empty title -> Add (status defaults proposed; invalid status -> Add with proposed plus a skipped note); invalid status on existing ID -> skipped; ID not in bee -> skipped; rows missing from the sheet -> nothing. Also: `snapshot_from(spreadsheet_id, sheet_id, tab_title, bee: &[BeePbi], keep_sheet: &[(id, field)]) -> SyncSnapshot` where fields listed in keep_sheet (failed ops, D7d) take the previous snapshot value instead of bee's; `overlay_failed(rows: &mut Vec<Vec<String>>, failed_adds: &[SheetRow], failed_fields: &[(id, field, sheet_value)])` that re-appends failed Add rows without an ID and writes the sheet value into failed fields (D7d); `unique_tab_title(name, taken: &[String]) -> String` appending ' (2)', ' (3)' (D6). Unit tests in the module for every D5/D7 rule named here, render/parse round-trip, snapshot spreadsheet mismatch, overlay_failed, unique_tab_title, and a Config TOML round-trip keeping [sheets].",
    "verify": "cargo test --profile fast -p waggledance-core sheets_sync",
    "must_haves": {
      "truths": ["A title edited only on the sheet yields one Amend op", "A field edited on both sides yields a conflict line naming both values and no op", "An ID-less titled row with blank Type yields an Add op", "First sync without snapshot yields no ops for existing IDs", "A snapshot for another spreadsheet_id is ignored", "An emptied description on an existing ID is skipped, never an Amend", "Subtask rows never yield ops", "Status 'bogus' on an existing ID is skipped with a reason"],
      "artifacts": [{"path": "crates/waggledance-core/src/sheets_sync.rs", "substantive": "render_rows, parse_rows, plan_sync, snapshot_from, overlay_failed, unique_tab_title with unit tests; no TODO stubs"}],
      "key_links": ["lib.rs declares pub mod sheets_sync", "Config has a sheets field of type SheetsConfig"],
      "prohibitions": ["No network or tokio code in waggledance-core", "No existing Config field renamed or removed"]
    }
  },
  {
    "id": "sts-2",
    "feature": "sheets-task-sync",
    "title": "Add a service-account Google Sheets client",
    "lane": "high-risk",
    "role": "code",
    "deps": [],
    "decisions": ["D3", "D6", "D7"],
    "files": ["crates/waggledance/src/gsheets.rs", "crates/waggledance/src/main.rs", "crates/waggledance/Cargo.toml", "Cargo.lock"],
    "read_first": ["docs/history/sheets-task-sync/CONTEXT.md", "crates/waggledance/Cargo.toml"],
    "affects_skills": [],
    "affects_specs": [],
    "behavior_change": true,
    "action": "Add direct dependencies `ring = \"0.17\"` and `base64 = \"0.22\"` to crates/waggledance/Cargo.toml (both already in Cargo.lock; no new crate may enter the lock) and declare `mod gsheets;` in main.rs. In gsheets.rs: load a service-account key file (JSON: client_email, private_key, token_uri) into a struct whose Debug impl redacts private_key (D3); parse the PKCS#8 PEM by stripping the BEGIN/END lines and base64-decoding, then `ring::signature::RsaKeyPair::from_pkcs8`; build the JWT as b64url(header {alg RS256, typ JWT}).b64url(claims {iss=client_email, scope=https://www.googleapis.com/auth/spreadsheets, aud=token url, iat, exp=iat+3600}) signed with RSA_PKCS1_SHA256 (D7k). `SheetsClient::new(key, http: reqwest::Client)` with overridable `token_url` and `api_base` (defaults https://oauth2.googleapis.com/token and https://sheets.googleapis.com/v4) so tests point at a local server; POST grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer and cache the access token until 60s before expiry. Methods: `list_tabs(spreadsheet_id) -> Vec<(i64, String)>` (GET spreadsheets/{id}?fields=sheets.properties), `add_tab(spreadsheet_id, title) -> i64` (batchUpdate addSheet with frozen row 1), `read_values(spreadsheet_id, title) -> Vec<Vec<String>>` (GET values/'title'!A:F; missing cells -> empty strings), `write_cell(spreadsheet_id, title, a1, value)` (PUT one cell, RAW — used for the D7a id write-back), `replace_values(spreadsheet_id, title, rows, previous_row_count)` that pads with empty rows up to previous_row_count and issues ONE PUT values/'title'!A1 RAW (no clear call, D7k), `set_status_dropdown(spreadsheet_id, sheet_id, task_row_indices)` as ONE batchUpdate: first a setDataValidation with no rule over D2:D, then ONE_OF_LIST proposed,in-flight,parked,done,declined on each task row's D cell (D7k). Quote tab titles in A1 ranges (single quotes, doubling embedded quotes) and URL-encode the range. Map HTTP errors to an error type carrying status and Google's error.message, never the key or token. Tests (tokio, local axum mock server): the token request carries a JWT that verifies via ring UnparsedPublicKey from the key pair's public key, with correct claims; the token is cached across two calls; read_values pads missing cells; replace_values sends one PUT covering the previous row count; set_status_dropdown sends the clearing request before the rules; an error body and Debug output never contain private_key bytes; a tab title with a quote is escaped. Use a fixed test-only PKCS#8 PEM constant generated for tests — never a real credential.",
    "verify": "cargo test --profile fast -p waggledance gsheets",
    "must_haves": {
      "truths": ["The token request body carries a JWT signed RS256 with iss=client_email and the spreadsheets scope", "A second API call within the token lifetime reuses the cached token", "Formatting the key struct or any error never prints private_key", "Rewriting a tab is a single PUT with no clear request"],
      "artifacts": [{"path": "crates/waggledance/src/gsheets.rs", "substantive": "service-account token flow plus list_tabs, add_tab, read_values, write_cell, replace_values, set_status_dropdown with mock-server tests"}],
      "key_links": ["main.rs declares mod gsheets", "Cargo.toml lists ring and base64 as direct deps"],
      "prohibitions": ["No real credential or network call in tests", "No new crate added to Cargo.lock", "No jsonwebtoken, yup-oauth2 or gcp_auth"]
    }
  },
  {
    "id": "sts-3",
    "feature": "sheets-task-sync",
    "title": "Wire the sync runner to an API route, CLI verb and project button",
    "lane": "high-risk",
    "role": "code",
    "deps": ["sts-1", "sts-2"],
    "decisions": ["D2", "D3", "D4", "D5", "D6", "D7"],
    "files": ["crates/waggledance/src/sheets_sync_run.rs", "crates/waggledance/src/main.rs", "crates/waggledance/src/server.rs", "crates/waggledance/src/cli.rs", "crates/waggledance/src/views.rs", "crates/waggledance/assets/app.js"],
    "read_first": ["docs/history/sheets-task-sync/CONTEXT.md", "crates/waggledance-core/src/sheets_sync.rs", "crates/waggledance/src/gsheets.rs", "crates/waggledance/src/server.rs", "crates/waggledance/src/cli.rs"],
    "affects_skills": [],
    "affects_specs": [],
    "behavior_change": true,
    "action": "Create sheets_sync_run.rs (declare mod in main.rs) with `async fn sync_project(project: &Project, cfg: &SheetsConfig, client: &SheetsClient, state_dir: &Path) -> Result<SyncReport>`; runner tests point SheetsClient at a local axum mock via its api_base/token_url (no trait). Steps: refuse with a setup hint when cfg is not configured or the project has no bee binary (reuse server.rs `project_bee_binary`; if you move it to a shared place keep both its cfg(unix) and non-unix helper forms); take an exclusive per-project lock file state_dir/sheets-sync/<project_id>.lock (create_new; stale after 10 minutes) and return a distinct 'already running' error if held (D7e); load snapshot from state_dir/sheets-sync/<project_id>.json; find the tab by stored sheet_id, else by title only if no other <id>.json in that dir claims that sheet_id, else add_tab(unique_tab_title) (D6, D7g); read_values; `bee::read_snapshot` for PBIs; plan_sync; apply ops by running the project's bee CLI with current_dir = project root and stdin null exactly like server.rs create_project_pbi (`backlog pbi add --title --cos(if non-empty) --status --json`, parse the id as create_project_pbi does, then immediately `write_cell` the id into that row's column A (D7a); `backlog pbi amend --id --title/--cos`; `backlog pbi status --id --to`) — never write .bee files (D2); collect per-op failures into the report without aborting; re-read bee and read_archived_cells per feature only now (D7l note); render_rows; overlay_failed for failed ops (D7d); replace_values with the previous row count; set_status_dropdown on task rows; save the snapshot via snapshot_from (keeping failed fields) atomically (write temp + rename); release the lock in all paths. SyncReport {project, tab, added, amended, status_changed, conflicts, skipped, errors} serializes to JSON. Surfaces (D4): POST /api/projects/:id/sheets/sync returns the report, 404 unknown project, 409 when locked, 400 with setup hint when unconfigured, error JSON shaped like pbi_error and never containing key material; CLI `waggledance sheets sync (--project <id-or-name> | --all) [--json]` builds the engine in-process, refuses without one of the two flags (D7h), refuses an ambiguous name by listing matching ids, prints each project's one-line summary as it completes, exits non-zero if any project errored; on the project overview page (`project_home_page` in views.rs, whose `topbar_full(` call passes \"\" as the actions slot) add a 'Sync Sheets' ghost button carrying data-project-id, rendered only when [sheets] is configured — add a parameter for that and update every caller in server.rs (D7i); in app.js the button disables and shows 'Syncing…' while the POST is in flight, then shows the summary inline and reloads the page on success, or shows the error. Name every new test with 'sheets_sync' in its path or name so the verify filter runs them. Tests: runner with mock Sheets server and a temp project whose .bee/bin/bee is a stub shell script recording argv and printing a JSON id — first sync writes header+rows and saves a snapshot with spreadsheet_id; a sheet title edit produces an amend call; a conflict is reported with both values; an ID-less row produces one add and its id is written back before the rewrite; add succeeds but the rewrite fails -> a second sync issues zero adds; a failing bee op keeps the sheet text on the tab; two same-name projects get two tabs; a held lock returns the already-running error; route tests for 404, 400 unconfigured and that GET /api/config after configuring contains no private_key bytes; views test that the button appears only when configured.",
    "verify": "cargo test --profile fast -p waggledance sheets_sync",
    "must_haves": {
      "truths": ["POST /api/projects/:id/sheets/sync returns a JSON report with added/amended/status_changed/conflicts/skipped/errors", "`waggledance sheets sync` without --project or --all refuses", "The project overview page shows a Sync Sheets button only when [sheets] is configured", "Every bee write goes through the project's bee CLI", "A second sync after a failed rewrite creates no duplicate PBI", "A concurrent second sync gets an already-running error"],
      "artifacts": [{"path": "crates/waggledance/src/sheets_sync_run.rs", "substantive": "sync_project runner with lock, id write-back, failed-op overlay, and tests using a mock Sheets server and stub bee"}],
      "key_links": ["server.rs routes /api/projects/:id/sheets/sync to the runner", "cli.rs has a Sheets subcommand calling the runner", "views.rs project_home_page renders the button", "app.js posts to the route"],
      "prohibitions": ["No timer or watcher-triggered sync (D4)", "No direct write to .bee/ files (D2)", "No key or token in any response or log line (D3)", "Existing New task button markup on the home page unchanged"]
    }
  },
  {
    "id": "sts-4",
    "feature": "sheets-task-sync",
    "title": "Document Google Sheets sync setup",
    "lane": "high-risk",
    "role": "docs",
    "deps": ["sts-3"],
    "decisions": ["D3", "D4", "D5", "D7"],
    "files": ["docs/knowledge/concepts/google-sheets-task-sync.md", "README.md"],
    "read_first": ["docs/history/sheets-task-sync/CONTEXT.md", "README.md", "crates/waggledance/src/sheets_sync_run.rs"],
    "affects_skills": [],
    "affects_specs": [],
    "behavior_change": false,
    "action": "Write a how-to concept doc: create a Google Cloud service account, enable the Sheets API, download the JSON key to a path outside any repo (chmod 600), share the spreadsheet with the client_email as Editor, add [sheets] spreadsheet_id and credentials_path to ~/.waggledance/config.toml, then press Sync Sheets on a project's overview page or run `waggledance sheets sync --project <name>` / `--all`. Include the sheet layout table, what can be edited (D2), the merge rules (D5, D7) as a short reference list, and known limits: columns A:F are rewritten each sync, extra columns G+ are not row-stable, an edit made on the sheet while a sync is running can be lost, a field cannot be cleared from the sheet. Add a short README section linking to it. Match the frontmatter of sibling files in docs/knowledge/concepts/.",
    "verify": "rg -n 'google-sheets-task-sync' README.md && rg -n 'credentials_path' docs/knowledge/concepts/google-sheets-task-sync.md",
    "must_haves": {
      "truths": ["README links to the setup doc", "The doc names every config key the code reads", "The doc states the known limits"],
      "artifacts": [{"path": "docs/knowledge/concepts/google-sheets-task-sync.md", "substantive": "setup steps, layout, editable fields, merge rules, known limits"}],
      "key_links": ["README.md links docs/knowledge/concepts/google-sheets-task-sync.md"],
      "prohibitions": ["No real key, email or token in the doc"]
    }
  }
]
```

## Test matrix

High-risk: the 12 edge dimensions, applied where they bite.

| Dimension | Probe | Pass when |
|---|---|---|
| Happy path | first sync of a project with 2 PBIs, 3 cells | tab holds header + 2 task + 3 subtask rows; snapshot file exists with spreadsheet_id |
| Empty input | project with no PBIs; empty sheet | tab holds header only; zero ops |
| Boundary | short rows, trailing blank rows, cell with no outcome | parse_rows yields rows without panic |
| Invalid input | status `bogus` on an existing ID; description emptied | both skipped with reasons; no op |
| Concurrency | title edited on sheet and in bee between syncs; two syncs at once | conflict line names both values; second sync gets already-running |
| Idempotence | two syncs with no edits | second run reports zero ops |
| Partial failure | bee op exits non-zero; rewrite fails after an add | report.errors has the entry and sheet text survives; next sync adds 0 |
| Auth failure | token endpoint returns 401 | error surfaces Google's message; no key bytes |
| Secrets | Debug of key, errors, GET /api/config | no `private_key` value in output |
| Unknown data | sheet row ID not in bee; blank Type on new row | ID skipped with reason; blank-Type row added as task |
| Rename / identity | tab renamed; two projects share a name; spreadsheet_id changed | found by sheet_id; two tabs; old snapshot ignored |
| Config missing | no [sheets] section | route 400 and CLI refuse with setup hint; button hidden |

## Open Questions

(none)

## Out of scope

- Timed or watcher-driven sync (D4).
- Editing cells from the sheet (D2).
- Deleting PBIs or clearing fields from the sheet (D5, D7b).
- OAuth user login or Apps Script.
