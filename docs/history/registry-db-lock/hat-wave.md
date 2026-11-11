# registry-db-lock — hat-wave synthesis

Seats run 2026-11-11: facts-gaps, alternatives, user-impact, value. The risks
seat did not start: `bee herding run` refused it because the main checkout's
`.bee/bin/bee` lacks the worker-guard hook (remedy `bee onboard --apply` in
main, where another session is live). Its CRUD-delete question (can the purge
be undone) is answered below from the measurements.

| Finding | Seats | Taken into plan.md |
|---|---|---|
| The reclaim's own write-lock window is unbounded | facts-gaps, alternatives, user-impact | Measured: bare `DROP TABLE` 43.5 s, batched purge 3.1 s with no batch over 28 ms, `VACUUM` 0.33 s. Purge is batched; the proof runs concurrent inserts across the reclaim. |
| `VACUUM` in WAL mode leaves the size in `-wal` | facts-gaps | `wal_checkpoint(TRUNCATE)` after `VACUUM`; the proof measures main file plus `-wal`. |
| Migration 13 must not backfill hashes | facts-gaps | Stated: no backfill, one extra reload per pre-upgrade file. |
| `refresh_stale` looks rows up with a linear `find` (O(n²) at 43k files) | facts-gaps, alternatives, value | `HashMap` lookup. |
| `refresh` should delegate to `refresh_stale`, one walk path | alternatives, value | Taken. |
| Watcher check must match the walk, nested `.gitignore` included | facts-gaps, alternatives, user-impact | `walk_would_index` honors every `.gitignore` between root and file. |
| Split the watcher filter from the lock fix | alternatives | rdl-3. |
| SQL `LIKE` breaks accented (Vietnamese) search | user-impact | Search folds diacritics and case in Rust; a Vietnamese test. |
| Zero hits must say only titles and paths were searched | user-impact | rdl-2 MCP wording. |
| Old sessions refill `files_fts`; each `serve` restart reclaims again | facts-gaps, alternatives, user-impact | Accepted and stated: the refill is only viewed docs, the purge is sub-second at that size. |
| Reclaim failure must not stop `serve` | user-impact | Logged warning, startup continues. |
| The configured proof command targets `packages/bee-rs`, absent here | value | Cell proofs use `cargo test`; the feature-end proof is `cargo test --workspace`. |

Reversibility (risks seat): the purge deletes only derived data. Every row
of `files_fts` is rebuilt from the markdown on disk by any binary that still
indexes content, and the `files`, `links`, `runs`, `inbox`, `tasks` and
`leader_holds` tables are untouched (copy check: `integrity_check` ok,
43,018 `files` and 226 `runs` rows kept).
