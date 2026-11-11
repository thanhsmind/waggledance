---
type: bee.pattern
title: An FTS5 DELETE by UNINDEXED columns scans the whole table and holds the write lock — "database is locked"
description: "Retired pitfall: files_fts kept project_id and rel_path UNINDEXED, so every per-file DELETE was a full scan of a ~1 GB table under the write lock and dispatch inserts failed with database is locked. headless-supervisor dropped the table."
timestamp: 2026-11-11
bee:
  id: an-fts5-delete-by-unindexed-column-scans-the-whole-table
  lifecycle: retired
  areas: [registry]
  anchors: [crates/waggledance-core/src/repository.rs]
  sources: [docs/history/registry-db-lock/CONTEXT.md, docs/history/registry-db-lock/plan.md, docs/history/headless-supervisor/CONTEXT.md]
  polarity: pitfall
  signature: database is locked
---

# An FTS5 DELETE by UNINDEXED columns scans the whole table and holds the write lock

Read when: runs fail with "database is locked", registry.db is far larger than its `runs` rows explain, or someone proposes storing content in a virtual table keyed by unindexed columns.

Retired by headless-supervisor D4: the `files`, `links` and `files_fts` tables were the lock source and are dropped by the last entry in `MIGRATIONS` in `crates/waggledance-core/src/repository.rs`. Nothing in waggledance indexes project content any more, so the trap cannot recur here. The lesson stays for any future virtual table.

## The trap

FTS5 cannot use an index for UNINDEXED columns, so a `DELETE ... WHERE project_id=? AND rel_path=?` is a full scan of the virtual table while the write lock is held. A startup reconcile that rewrote every file whether or not it changed held that lock for minutes, and other processes' inserts waited out the 5 s `busy_timeout`.

## Rejected

Keeping FTS and indexing a lookup column: the owner chose to drop stored content (decision acf96e76), and then to drop the whole file store (headless-supervisor D4). A bare `DROP TABLE` on the full legacy table held the lock 43.5 s, which is why old databases were emptied in batches before the drop.
