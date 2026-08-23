---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: board-new-task

Mode: `standard` — 2 risk flags: multi-domain, external-systems (the server
spawns a per-project bee process, the first mutating bee call the console
makes). Four product files, one walking skeleton.

## Requirements (from CONTEXT.md)
- N1 topbar button + overlay (textarea, project select, keys, handset hide).
- N2 `POST /api/projects/:id/pbi` → `<root>/.bee/bin/bee backlog pbi add`.
- N3 reload on 200; inline `{error}` with the four named refusals.

## Discovery
`home_page` (`views.rs:209`) builds the Orchestrator anchor at 350–363 and
hands it to `topbar_full`'s actions slot (368). Projects arrive as
`&[(Project, usize, Vec<TerminalPaneView>)]`; worktree rows are recognised by
`worktree_branch(&id)` (582–605). The Todo column renders
`snapshot.backlog.pbis` filtered to `status == "proposed"` (4298, 4666), so
storage is the whole job. `server.rs` has no `Command::new` today; the
nearest shapes are `terminal_create_pane` (JSON in/out, 3118) and
`register_project` (`spawn_blocking`, 1575), with root resolution via
`st.engine.get_project(&id).root_path`. The overlay precedent is the jump
palette (`app.js:247-291`, `app.css:1793-1819`: `.jump-overlay` +
`var(--z-modal)`); the submit precedent is `postJson` + `afterCreate`
(`app.js:2253-2279`). Live reload ignores `.jsonl` (`watch.rs:62`), so the
client reloads itself (N3).

## Approach
One new POST route and one overlay; no new module, no new state. The bee
binary is the project's own `.bee/bin/bee` (N2) — found or refused, never
searched on PATH. The CLI runs through `tokio::process::Command` with
`current_dir(root)`, stdout parsed for `id`. Rejected: writing
`backlog.jsonl` from Rust (CLI-only boundary); a `<dialog>` element (no
precedent, the palette overlay already handles scrim + Esc).

Risk map: spawn path / MEDIUM / route tests with a fake `.bee/bin/bee` shell
script in `fresh_root` · overlay markup / LOW / literal view tests · CSS /
LOW / stylesheet-parity tests · JS / LOW / manual check at staging.

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 Skeleton | server route + view overlay + JS + CSS in one cell | one user-visible surface, end-to-end | click + New task, pick a project, type, Enter → row in Todo | close |

Single cell: the four files are one seam each and the feature is not
demoable until all four exist. Splitting server from view would ship a
button that does nothing or a route nothing calls.

## Test matrix
- Happy: POST `{task:"Fix header\nAlign to 8px"}` with a fake bee script
  printing `{"id":"p-abc"}` → 200 `{id:"p-abc", project_id}`; script saw
  `backlog pbi add --title Fix header --cos Align to 8px --json` and cwd
  = root.
- Edge: single-line task → no `--cos`; 201-char first line clipped to 200;
  leading blank lines skipped.
- Error: empty/whitespace task → 400; unknown id → 404; no `.bee/bin/bee`
  → 409 with the "not set up with bee" text; script exits 1 → 502 with the
  stderr tail.
- View: `home_page` renders the `topbar__new-task` button on both tabs and
  an overlay whose select lists top-level projects and omits worktree rows;
  stylesheet carries the overlay rule and the handset hide rule.

## Smaller path
Checked: a plain form POST + redirect would drop the overlay JS, but loses
inline errors (N3) and the Enter-to-submit key (N1). Rejected on the locked
decisions; PASS on the chosen shape.
