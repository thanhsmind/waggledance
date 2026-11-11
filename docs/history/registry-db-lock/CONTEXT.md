# registry-db-lock — CONTEXT

Owner request (2026-11-11): stop "database is locked" on `waggledance_dispatch`
and shrink `~/.waggledance/registry.db` (1 GB). Updated the same day by owner
decision acf96e76: drop the full-text store entirely.

## Root cause of the lock (verified 2026-11-11)

- `files_fts` is an FTS5 table whose `project_id` and `rel_path` columns are
  `UNINDEXED`. `upsert_file` and `delete_file` run
  `DELETE FROM files_fts WHERE project_id=?1 AND rel_path=?2`, and
  `file_content` runs the matching `SELECT`. FTS5 cannot use an index for
  those columns, so each statement is a full scan of the ~1 GB table
  (`EXPLAIN QUERY PLAN`: `SCAN files_fts VIRTUAL TABLE INDEX 0:`). The DELETE
  holds the write lock for the whole scan.
- The daemon's startup reconcile (`server.rs` `reconcile_registered_projects`
  → `Engine::refresh` → `IndexService::index_project`) re-upserts every one of
  the 43,018 indexed files, changed or not. The live daemon (pid 375483,
  started 20:02) was at 100 % CPU for its whole 77-minute life, had read
  12.8 TB from the page cache, and was still reconciling at 21:09, the
  minute the five dispatches failed.
- The journal for 2026-11-09 shows the reverse: the reconcile itself failing
  with "database is locked" while another process wrote. The watcher
  (`index_file_incremental`) and the MCP search refresh (`refresh_stale`)
  run the same per-file statements.
- Reproduced on a copy of the real registry (`.bee/tmp/dblock/contend.py`):
  one process running the per-file upsert pattern makes run inserts from four
  other processes wait up to 5.01 s with one failure; two such processes make
  every dispatcher fail with "database is locked".

## Locked decisions

- D1. The `files_fts` table and its content are dropped (owner decision
  acf96e76). Nothing in the new code reads or writes it.
- D2. Old binaries that are still running (MCP servers of sessions opened
  before the upgrade) still reference `files_fts`. The upgrade therefore
  replaces it with an empty table of the same shape instead of leaving it
  missing. Only `waggledance serve` reclaims the space: at startup, when
  `files_fts` holds any row, it empties the FTS shadow tables in short
  batches (200 rows per `BEGIN IMMEDIATE` transaction), drops and recreates
  the empty table, and runs `VACUUM` once. MCP servers and CLI commands never
  run the purge or `VACUUM`. Measured on a copy of the real registry: a plain
  `DROP TABLE files_fts` held the write lock for 43.5 s; the batched purge
  took 3.1 s in total with no batch over 28 ms, `VACUUM` 0.33 s, and the file
  went from 1,040,687,104 to 25,722,880 bytes with `integrity_check` ok and
  all 43,018 `files` and 226 `runs` rows kept.
- D3. Change detection uses a content hash stored in `files.content_hash`
  (migration 13: FNV-1a 64-bit over the bytes, plus the byte length, as hex).
  A file whose hash matches the stored one is not rewritten; when only its
  mtime moved, the metadata row is updated and no reload is reported.
- D4. Reconciling an unchanged project performs no write. `Engine::refresh`
  stays the one door for the startup sweep and the explicit refresh
  (stale-index-refresh-1), but skips files whose size and mtime match, then
  files whose hash matches, and rewrites a file's links only when they differ.
- D5. Search surfaces stay (MCP `waggledance_search`, CLI `search`, the web
  search box) and match on title and path only: each whitespace-separated
  term must appear, case-insensitively, in the title or the repo-relative
  path. Results carry no content excerpt. Reason: finding a document by name
  is the viewer's job and costs no stored content; removing the surfaces
  would break 50 recorded agent calls for no size gain.
- D6. The watcher indexes a markdown path only when the full walk would index
  it (no path component named in `indexing.exclude_patterns`, not ignored by
  the project's `.gitignore`), or when the file already has a row because
  someone opened it in the viewer. Live reload for opened documents keeps
  working; churn in `.bee/tmp` and `node_modules` stops reaching the database.
- D7. Project recognition needs nothing new now: `waggledance_dispatch`
  requires an explicit `project` id (`mcp.rs` schema, `required: ["project",
  "task"]`).
- D8. `busy_timeout` stays at 5000 ms, and the registry stays one file. With
  no long-running writer left, a split database or routing MCP writes through
  the daemon adds moving parts without removing a failure.
- D9. Long-lived MCP servers are expected. A stdio MCP server exits on EOF of
  its stdin, which happens when its parent session exits. On 2026-11-11 every
  long-lived one had a live parent: two `agy` sessions up 2 d 13 h and the
  Claude desktop session up 1 d 10 h. No change.

## Read set

### Knowledge

| Concept | Grade at read | Why it matters |
|---------|---------------|----------------|
| docs/knowledge/patterns/a-second-daemon-proves-it-without-taking-the-users.md | pattern | The live-path proof runs a second daemon under a scratch HOME on the copied registry. |
| docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md | pattern | The proof must run the freshly built artifact, confirmed by content. |

### Code

| Path | What it decides |
|------|-----------------|
| `crates/waggledance-core/src/repository.rs` | Schema, the FTS statements that scan the table, `busy_timeout`, migrations. |
| `crates/waggledance-core/src/engine.rs` | `refresh`, `refresh_stale`, `index_file_incremental` change detection. |
| `crates/waggledance-core/src/indexer.rs` | The walk filter (`.gitignore`, `exclude_patterns`). |
| `crates/waggledance/src/server.rs` | Startup reconcile sweep and the web search page. |
| `crates/waggledance/src/watch.rs` | Watcher reindex and live-reload broadcast. |
| `crates/waggledance/src/mcp.rs` | `waggledance_search` and `waggledance_dispatch` run persistence. |

## Out of scope

- Indexing opt-out per project (the llm-wiki question is moot under D1).
- Purging existing `files` rows for scratch paths: they cost a few MB, and D6
  stops new ones.
