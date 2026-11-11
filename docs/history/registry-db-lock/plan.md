# registry-db-lock — plan

Lane: high-risk (concurrent writers across processes, a destructive schema
change on the user's live registry, a release). Decisions: `CONTEXT.md`.
Hat-wave synthesis: `hat-wave.md`.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | Every upsert deletes the old FTS row by unindexed columns | read | crates/waggledance-core/src/repository.rs:974 | `"DELETE FROM files_fts WHERE project_id=?1 AND rel_path=?2",` |
| 2 | Those columns are UNINDEXED, so the DELETE is a full scan | ran | crates/waggledance-core/src/repository.rs:1511 | `project_id UNINDEXED,` and `EXPLAIN QUERY PLAN` on the copy: `SCAN files_fts VIRTUAL TABLE INDEX 0:` |
| 3 | The daemon's startup sweep re-upserts every file through refresh | read | crates/waggledance/src/server.rs:418 | `match engine.refresh(&project.id) {` |
| 4 | refresh_stale looks rows up linearly | read | crates/waggledance-core/src/engine.rs:239 | `let stale = match existing.iter().find(\|f\| f.rel_path == rel) {` |
| 5 | Dispatch takes an explicit project id | read | crates/waggledance/src/mcp.rs:318 | `"required": ["project", "task"]` |
| 6 | Writers wait at most 5 s for the lock | read | crates/waggledance-core/src/repository.rs:40 | `conn.pragma_update(None, "busy_timeout", 5000).ok();` |
| 7 | One per-file indexer makes dispatch inserts fail on the real data | ran | .bee/tmp/dblock/contend.py | two indexers: `dispatcher 3: ok=24 locked=2 worst_wait=5.01s` |
| 8 | The batched purge keeps every lock hold short and shrinks the file | ran | .bee/tmp/dblock/batched.py | `total=3.1s worst_batch=0.028s drop=0.004s vacuum=0.33s`, 25722880 bytes |

## Cells

```json
[
 {
  "id": "rdl-1",
  "feature": "registry-db-lock",
  "title": "Drop the full-text store and make an unchanged reconcile write nothing",
  "lane": "high-risk",
  "role": "code",
  "files": [
   "crates/waggledance-core/src/repository.rs",
   "crates/waggledance-core/src/engine.rs",
   "crates/waggledance-core/src/indexer.rs",
   "crates/waggledance-core/src/domain.rs",
   "crates/waggledance-core/Cargo.toml",
   "Cargo.toml",
   "Cargo.lock"
  ],
  "action": "Implement plan.md section 'Cell rdl-1' exactly: no reads or writes of files_fts outside reclaim_legacy_index; migration 13 content_hash without backfill; batched reclaim_legacy_index with VACUUM and wal_checkpoint(TRUNCATE); hash-based change detection; refresh delegates to refresh_stale with a HashMap lookup and a link pass that writes only differing links; diacritic- and case-folded title/path search. Include the reproduction test that fails on the base commit with 'database is locked'.",
  "verify": "cargo test --profile fast -p waggledance-core",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [],
  "decisions": [
   "acf96e76-e9f8-48bd-b241-24df98c59520",
   "1f69553c-bc43-4f5b-b1c2-f1352fb686a9",
   "e54f5a88-d0e0-4ec0-b9be-592c5ff373bc",
   "f58c312c-3b55-4d46-881a-63e10cb43f15"
  ],
  "read_first": [
   "docs/history/registry-db-lock/CONTEXT.md",
   "docs/history/registry-db-lock/plan.md"
  ],
  "must_haves": {
   "truths": [
    "Engine::refresh of an unchanged project returns Ok while another connection holds BEGIN IMMEDIATE on the same file",
    "No statement outside reclaim_legacy_index reads or writes files_fts",
    "A touched byte-identical file reports no change; an edited one does",
    "reclaim_legacy_index empties files_fts, keeps files and runs rows, and a second call returns false",
    "Search for 'dac ta' finds a document titled 'Đặc tả'"
   ]
  }
 },
 {
  "id": "rdl-2",
  "feature": "registry-db-lock",
  "title": "Reclaim the old index at daemon start and serve title/path search",
  "lane": "high-risk",
  "role": "code",
  "files": [
   "crates/waggledance/src/server.rs",
   "crates/waggledance/src/mcp.rs",
   "crates/waggledance/src/cli.rs",
   "crates/waggledance/src/views.rs"
  ],
  "action": "Implement plan.md section 'Cell rdl-2' exactly: serve calls reclaim_legacy_index once before the reconcile sweep, logs size before/after, a reclaim error warns and startup continues; MCP search description and zero-hit wording say titles and paths; CLI prints title and URL; web search page shows title and path, no excerpt line.",
  "verify": "cargo test --profile fast -p waggledance",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [
   "rdl-1"
  ],
  "decisions": [
   "f58c312c-3b55-4d46-881a-63e10cb43f15",
   "1f69553c-bc43-4f5b-b1c2-f1352fb686a9"
  ],
  "read_first": [
   "docs/history/registry-db-lock/CONTEXT.md",
   "docs/history/registry-db-lock/plan.md"
  ],
  "must_haves": {
   "truths": [
    "serve runs reclaim_legacy_index once before the reconcile sweep and keeps starting when it errors",
    "waggledance_search zero-hit answer says only titles and paths were searched",
    "The web search page shows each hit's title and path"
   ]
  }
 },
 {
  "id": "rdl-3",
  "feature": "registry-db-lock",
  "title": "Keep ignored and excluded paths out of the watcher's index",
  "lane": "high-risk",
  "role": "code",
  "files": [
   "crates/waggledance/src/watch.rs",
   "crates/waggledance-core/src/indexer.rs"
  ],
  "action": "Implement plan.md section 'Cell rdl-3' exactly: indexer::walk_would_index honoring exclude_patterns components and every .gitignore between root and file; the watcher indexes a markdown path only when walk_would_index is true or get_file already has a row.",
  "verify": "cargo test --profile fast -p waggledance watch",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [
   "rdl-1"
  ],
  "decisions": [
   "e54f5a88-d0e0-4ec0-b9be-592c5ff373bc"
  ],
  "read_first": [
   "docs/history/registry-db-lock/CONTEXT.md",
   "docs/history/registry-db-lock/plan.md"
  ],
  "must_haves": {
   "truths": [
    "The watcher does not index a markdown file under an excluded directory or ignored by a nested .gitignore",
    "The watcher still reports a change for an ignored file that already has a row"
   ]
  }
 }
]
```

### Cell rdl-1 — store and engine (crates/waggledance-core)

Files: `src/repository.rs`, `src/engine.rs`, `src/indexer.rs`, `src/domain.rs`
if `SearchResult` changes, `Cargo.toml` (workspace) and
`crates/waggledance-core/Cargo.toml` for `unicode-normalization`.

Contract:

- Nothing in the new code reads or writes `files_fts`. `SCHEMA` still creates
  the empty legacy table (old running binaries need it), but no statement in
  the new code touches it apart from `reclaim_legacy_index`.
- Migration 13 adds `files.content_hash TEXT NOT NULL DEFAULT ''`.
  `SCHEMA_VERSION` = 13. The migration does not backfill hashes: it reads no
  file. The first change event for a pre-upgrade file therefore reports
  `changed = true` once.
- `SqliteStore::reclaim_legacy_index(&self) -> Result<bool>`: when
  `files_fts` holds at least one row, delete the rows of its shadow tables
  `files_fts_content`, `files_fts_docsize` and `files_fts_data` in batches of
  200 rowids, each batch its own `BEGIN IMMEDIATE` transaction (never one long
  transaction, never a bare `DROP TABLE` on the full table: that held the
  write lock 43.5 s on the real data), then drop `files_fts`, recreate it
  empty with the old shape (`fts5(project_id UNINDEXED, rel_path UNINDEXED,
  title, content)`), run `VACUUM`, then `PRAGMA wal_checkpoint(TRUNCATE)`,
  and return `true`. When it holds no row, return `false` and write nothing.
- `upsert_file(&IndexedFile, content_hash: &str)` writes only the `files`
  row. `delete_file` and `delete_project` no longer touch `files_fts`.
  `file_content` is replaced by `file_hash(project_id, rel_path) ->
  Option<String>`. The hash helper (FNV-1a 64 over the bytes, plus the byte
  length, as hex) lives in one place.
- `set_file_links` writes nothing when the stored target set already equals
  the new one.
- `search(query, project_id, limit)`: match in Rust over `list_files` rows
  (or all projects' rows). Fold both sides: Unicode NFD, drop combining
  marks, map `đ`/`Đ` to `d`, lowercase. Every whitespace term must be a
  substring of the folded title or folded `rel_path`. Title hits rank before
  path-only hits, then `rel_path` ascending. `excerpt` is empty, `score` 0.
- `Engine::index_file_incremental` returns `changed = true` only when the hash
  differs from the stored one or there was no row. An equal hash with moved
  metadata updates the row and returns `false`.
- `Engine::refresh` delegates to `refresh_stale` and then runs the link pass;
  there is one walk path (engine.rs comment on `refresh_stale`). The lookup of
  stored rows in `refresh_stale` uses a `HashMap` keyed by `rel_path`, not a
  linear `find`. A stale file whose hash is unchanged updates metadata only.
  The link pass still reads every file at startup but writes only links that
  differ.

Tests (behavior, in the crate):

- Reproduction, fails on the base commit with "database is locked": a second
  `Connection` on the same temp file holds `BEGIN IMMEDIATE` while
  `Engine::refresh` reconciles an already-indexed, unchanged project; the
  refresh must return `Ok`.
- A touched, byte-identical file: `index_file_incremental` returns `false`; an
  edited one returns `true`.
- `reclaim_legacy_index` on a database holding old-shape `files_fts` rows:
  returns `true`, `files_fts` exists and is empty, `files` and `runs` rows
  are kept; a second call returns `false`.
- Search: title hit, path hit, multi-term AND, a Vietnamese title found by its
  unaccented spelling ("dac ta" finds "Đặc tả"), project filter.
- Existing tests that assert FTS content or `<mark>` excerpts are rewritten to
  the new contract.

### Cell rdl-2 — daemon reclaim and search surfaces (crates/waggledance)

Depends on rdl-1.

Files: `src/server.rs`, `src/mcp.rs`, `src/cli.rs`, `src/views.rs`.

Contract:

- `serve` calls `reclaim_legacy_index` once at startup, before the reconcile
  sweep, and logs the outcome with the registry file size before and after. A
  reclaim error is logged as a warning and `serve` keeps starting. Old MCP
  servers still running refill `files_fts` with the documents they view, so
  each later `serve` start reclaims again; at that size the purge and `VACUUM`
  take well under a second.
- MCP `waggledance_search`: the description says it matches titles and paths;
  a zero-hit answer says only titles and paths were searched. CLI `search`
  prints title and URL. The web search page shows title and path, no excerpt
  line.

Tests: MCP search returns a path hit and the zero-hit wording; the web search
page renders a hit with its path; startup reclaim is called and a reclaim
error does not stop startup.

### Cell rdl-3 — watcher filter (crates/waggledance, crates/waggledance-core)

Independent of rdl-2; depends on rdl-1 only for `get_file`.

Files: `crates/waggledance/src/watch.rs`, `crates/waggledance-core/src/indexer.rs`.

Contract: `indexer::walk_would_index(root, path, exclude) -> bool` answers
what `scan_markdown_files` would answer for one path: false when a component
between `root` and the path is named in `exclude`, or when a `.gitignore` in
`root` or any directory between `root` and the file ignores it (nested files
included, the same files the walk honors). The watcher indexes a markdown path
only when `walk_would_index` is true or `get_file` already has a row for it.

Tests: the watcher skips a file in an excluded directory and a file ignored by
a nested `.gitignore`, and still reports a change for an ignored file that
already has a row.

## Proof at feature end

- `cargo test --workspace` in the worktree, plus the reproduction test red on
  the base commit (recorded output).
- Size and reclaim window: the new binary's reclaim on an un-reclaimed copy of
  the real registry while four processes insert runs with `busy_timeout`
  5000 ms; record the duration, the inserts that failed (target zero), and the
  size of the main file plus `-wal` before and after.
- Live path: a second daemon under a scratch HOME holding the copied
  registry; four concurrent run inserts during its startup reclaim and
  reconcile, none locked.
- Release: next patch version through the repo's release path.
