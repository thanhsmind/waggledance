---
type: bee.pattern
title: An FTS5 DELETE by UNINDEXED columns scans the whole table and holds the write lock — "database is locked"
description: "Pitfall: files_fts kept project_id and rel_path UNINDEXED, so every per-file DELETE/SELECT was a full scan of a ~1 GB table under the write lock; a reconcile that re-upserted unchanged files made dispatch inserts fail with database is locked."
timestamp: 2026-11-11
bee:
  id: an-fts5-delete-by-unindexed-column-scans-the-whole-table
  lifecycle: active
  areas: [registry]
  anchors: [crates/waggledance-core/src/repository.rs, crates/waggledance-core/src/engine.rs, crates/waggledance-core/src/indexer.rs, crates/waggledance/src/watch.rs, crates/waggledance/src/server.rs, crates/waggledance/src/mcp.rs, crates/waggledance/src/views.rs, crates/waggledance/src/cli.rs]
  sources: [docs/history/registry-db-lock/CONTEXT.md, docs/history/registry-db-lock/plan.md]
  polarity: pitfall
  critical: true
  signature: database is locked
---

# An FTS5 DELETE by UNINDEXED columns scans the whole table and holds the write lock

Read when: runs fail with "database is locked", registry.db is far larger than its `files` and `runs` rows explain, or someone proposes storing content in a virtual table keyed by unindexed columns.

## The trap

`files_fts` declared `project_id` and `rel_path` UNINDEXED. FTS5 cannot use an index for them, so the per-file `DELETE ... WHERE project_id=? AND rel_path=?` was `SCAN files_fts VIRTUAL TABLE`, a full scan of a ~1 GB table while holding the write lock. The startup reconcile re-upserted every indexed file whether or not it changed, so one process held the lock for minutes and other processes' inserts waited out the 5 s `busy_timeout`.

## What held

- Change detection is a content hash on the `files` row (migration 13, not backfilled), so an unchanged file is never rewritten and an unchanged reconcile writes nothing.
- The reproduction is a second connection holding `BEGIN IMMEDIATE` while `Engine::refresh` reconciles an indexed, unchanged project: red on the base commit with "database is locked", green after.
- `reclaim_legacy_index` empties the legacy table's shadow tables in 200-rowid `BEGIN IMMEDIATE` batches, then drops, recreates empty, `VACUUM`s and truncates the WAL. A bare `DROP TABLE` on the full table held the lock 43.5 s.
- The watcher indexed any markdown path it saw, including ignored and excluded ones the walk never lists. It now indexes a path only when `indexer::walk_would_index` agrees with the walk or the path already has a `files` row, so a known ignored file still reports its change.
- `serve` calls `reclaim_legacy_index` once before the startup reconcile and keeps starting when it errors. Old MCP servers still running refill `files_fts`, so every later `serve` start reclaims again. Search surfaces (MCP, CLI, web page) say titles and paths and show no excerpt; a zero-hit MCP answer says contents were not searched.

## Rejected

Keeping FTS and indexing a lookup column: the owner chose to drop stored content (decision acf96e76); title and path search is done in Rust over `files` rows with diacritic and case folding.
