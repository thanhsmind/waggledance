---
type: bee.area
title: worktree-auto-sweep — the daemon reclaims finished bee worktrees
description: "On a timer, the daemon runs each registered bee project's own bee worktree prune with a configured age floor, so merged, clean, idle worktrees disappear without anyone running prune by hand. bee alone decides what is finished; the daemon only schedules it and logs the verdicts."
timestamp: 2026-11-06
bee:
  id: worktree-auto-sweep
  lifecycle: active
  areas: [daemon]
  sources: [docs/history/worktree-auto-sweep/plan.md]
---

# worktree-auto-sweep — the daemon reclaims finished bee worktrees

## What it does

Every `interval_hours`, the daemon lists the registered projects and, for
each one that is a bee main checkout, runs
`<root>/.bee/bin/bee worktree prune --older-than-days <older_than_days>`
with the project root as its working directory. The first sweep runs five
minutes after the daemon starts.

A project counts as a bee main checkout when `<root>/.bee/bin/bee` is an
executable file and `<root>/.git` is a directory. A linked worktree that was
registered as a project is skipped, because prune refuses to run from one.

## Who decides what is finished

bee does. The daemon passes only the age floor. Prune keeps a worktree when
its branch is not merged into main, the tree has modified or untracked
files, a live session holds it, a rebase or merge is in progress, it holds
a pending capture stub or a paused handoff, its last commit is younger than
the floor, or it is locked with `git worktree lock`. A removal also drops
the branch (`git branch -d`), the grant, the workspace record and the
matching Paseo workspace.

## Configuration

`~/.waggledance/config.toml`:

```toml
[worktree_sweep]
enabled = true
interval_hours = 6
older_than_days = 2
```

These are the defaults, so a config without the table sweeps. An
`interval_hours` below 1 runs hourly. The two-day floor keeps a feature's
worktree through the uat window after `bee worktree merge`, where follow-up
fixes still land in it. Setting `enabled = false` stops the task on the next
config reconcile.

## What it reports

Each project yields removed and kept verdicts, each with bee's own reason,
or a failure line. A project runs under a 120-second deadline, and a failure
in one project never stops the sweep of the rest. The daemon logs one info
line per removed worktree and one summary line per sweep
(`worktree sweep: N projects, R removed, K kept, F failed`).

## Pointers

- `crates/waggledance-core/src/worktree_sweep.rs`: project filter, prune
  call, output parsing.
- `crates/waggledance/src/worktree_sweep.rs`: the timer loop and log lines.
- `crates/waggledance/src/main.rs`: `reconcile_sweep`, the task slot.
- `crates/waggledance-core/src/config.rs`: `WorktreeSweepConfig`.
