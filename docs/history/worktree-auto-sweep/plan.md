# worktree-auto-sweep — plan

## Goal

The daemon reclaims finished bee worktrees on its own. Every registered
project that carries bee gets `bee worktree prune` run on a timer, so a
merged, clean, idle worktree disappears without anyone typing the prune.

## Contract

- **Safety stays with bee.** The daemon never decides what is finished. It
  runs `<root>/.bee/bin/bee worktree prune --older-than-days <N>` with the
  project root as the working directory. bee keeps every worktree that is
  unmerged, dirty, held by a live session, mid-rebase, carrying a pending
  capture stub or a paused handoff, or locked.
- **Config.** A new `[worktree_sweep]` table in `~/.waggledance/config.toml`:
  `enabled` (default `true`), `interval_hours` (default `6`, minimum `1`),
  `older_than_days` (default `2`).
- **Which projects.** Every registered project whose root has an executable
  `.bee/bin/bee` and whose `.git` is a directory (a main checkout). A linked
  worktree registered as a project is skipped, because prune refuses to run
  from one.
- **Per run.** Each project runs under a 120 s deadline. A failure (spawn,
  timeout, non-zero exit) is logged for that project and the sweep moves on.
- **Report.** The sweep returns one report per project: the worktree ids
  removed and the ids bee kept, each with bee's own reason line. The daemon
  logs one line per removal and one summary line per sweep.
- **Timing.** The first sweep runs 5 minutes after the daemon starts, then
  every `interval_hours`. A config reload that turns the sweep off stops the
  task.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | prune already refuses unmerged, dirty, live-session and pending-capture worktrees | ran | `bee worktree prune --dry-run` on main, 2026-11-06 | `kept — ... holds 1 pending capture stub(s) main's queue never names` |
| 2 | prune takes an age override flag | ran | `bee worktree prune --help` | `--older-than-days (num) — Override the last-commit age threshold in days (default 7)` |
| 3 | prune must run from the main checkout | ran | `bee worktree prune --help` | `Must be run from the MAIN checkout` |
| 4 | the daemon already reconciles a heartbeat task slot the sweep can sit beside | read | `crates/waggledance/src/main.rs:305` | `fn reconcile_heartbeat(&self, engine: Option<Arc<waggledance_core::Engine>>) {` |
| 5 | core has a deadline-bounded process runner | read | `crates/waggledance-core/src/process.rs:54` | `pub fn run_with_deadline(` |
| 6 | the engine lists registered projects with their roots | read | `crates/waggledance/src/mcp.rs:637` | `let projects = match engine.list_projects() {` |

## Cells

| id | role | what | files |
|---|---|---|---|
| ws-1 | code | `WorktreeSweepConfig` in core config; `worktree_sweep` module in waggledance-core: project filter, prune invocation, output parse into a report; tests with a fake bee script | `crates/waggledance-core/src/config.rs`, `crates/waggledance-core/src/worktree_sweep.rs`, `crates/waggledance-core/src/lib.rs` |
| ws-2 | code | daemon task: a `sweep` slot reconciled beside the heartbeat, the interval loop, the log lines | `crates/waggledance/src/main.rs`, `crates/waggledance/src/worktree_sweep.rs` |
| ws-3 | docs | knowledge concept for the sweep and the config table | `docs/knowledge/` |

ws-2 depends on ws-1. ws-3 runs after ws-2.

## Proof

- ws-1: `cargo test -p waggledance-core worktree_sweep` and the config default
  test.
- ws-2: `cargo build --profile fast -p waggledance`, then the daemon unit test
  that the slot starts and stops with the switch.
- Whole path: run one sweep against the real registry with `older_than_days`
  set high, and confirm every project reports and nothing young is removed.
