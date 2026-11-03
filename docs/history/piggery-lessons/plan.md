---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Piggery lessons, the three fixes

## Summary

Three fixes from `docs/history/research/piggery-lessons-xia.md`:

1. Two new Paseo captures pin the after-permission path, and fixture tests
   assert it. The code's logic does not change (D5).
2. The board shows a Paseo leader that waits on a permission, or was just
   lost, in `/api/board` and on the project's home rail row (D6).
3. A caller notice lives on its run row, so it survives a daemon restart, and
   an expired notice is recorded as dropped instead of vanishing (D7).

Mode: `standard`. Risk flags: data model (runs migration 6), public contract
(`/api/board` gains a key, additive). Cells 2 and 3 both touch
`repository.rs` and `engine.rs`, so they run in order. Cell 1 is disjoint
and runs beside them.

## Requirements (from CONTEXT.md)

- D5: captures `inspect-after-allow.out`, `wait-after-allow.out` and
  `wait-second-permission.out` under `testdata/fixtures/paseo-0.11.2/`, with
  README rows and tests through `parse_inspect` and `parse_turn_end`.
- D6: `leader_waiting` in `/api/board`, a pill on the home rail row, and the
  `runs` table as the only source. Blocked always counts; lost counts while
  `updated_at` is within the last hour.
- D7: migration 6 adds `notice`, `notice_state` and `notice_until`. `end()`
  is one UPDATE. The flush reads from and writes to the store, a newer notice
  replaces a pending one, and spacing stays in memory.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | After `permit allow` the agent is still running with no pending permission | ran | `.bee/tmp/pg/probe2/inspect-after-allow-1.out` | `"Status": "running",` |
| 2 | `wait` after the allow returns at the turn end, reporting idle | ran | `.bee/tmp/pg/probe2/wait-after-allow-1.out` | `PERM-DONE"` |
| 3 | `wait` reports a second permission instead of idle | ran | `.bee/tmp/pg/probe3/wait-after-allow-1.out` | `"status": "permission",` |
| 4 | The board reads only working runs | read | `crates/waggledance/src/server.rs:3496` | `filter(\|r\| r.status == "working")` |
| 5 | Runs schema is version 5, so new columns are migration 6 | read | `crates/waggledance-core/src/repository.rs:677` | `pub const SCHEMA_VERSION: i64 = 5;` |
| 6 | Notices are an in-memory list dropped after an hour | read | `crates/waggledance/src/delivery.rs:463` | `delivery dropped a notice to {} after 1h` |
| 7 | Transcript and status are two statements | read | `crates/waggledance/src/delivery.rs:437` | `set_run_final_transcript(&run.id, &text)` |

## Cells

| Id | Title | Files | Deps | Verify |
|----|-------|-------|------|--------|
| pl-1 | Pin the after-permission wait with Paseo captures | `crates/waggledance/testdata/fixtures/paseo-0.11.2/*`, `crates/waggledance/src/paseo.rs` | — | `cargo test --profile fast -p waggledance paseo::` |
| pl-2 | Show waiting or lost Paseo leaders on the board | `crates/waggledance-core/src/repository.rs`, `crates/waggledance-core/src/engine.rs`, `crates/waggledance/src/server.rs`, `crates/waggledance/src/views.rs` | — | `cargo test --profile fast -p waggledance-core runs && cargo test --profile fast -p waggledance board` |
| pl-3 | Keep caller notices on the run row | `crates/waggledance-core/src/repository.rs`, `crates/waggledance-core/src/engine.rs`, `crates/waggledance/src/delivery.rs` | pl-2 | `cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance delivery` |

The feature closes with `cargo test --profile fast --workspace`, run once
after pl-3. No new code comments: the repo's `no_code_comments` rule holds.

## Out of scope

- The Paseo plugin drawing `leader_waiting`. It reads `/api/board` and keeps
  working; drawing the new key is its own PBI if wanted.
- Restart handling for in-flight runs (`leader-delivery` D6 stays).
