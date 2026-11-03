---
date: 2026-10-16
feature: piggery-lessons
categories: [failure, pattern]
severity: standard
tags: [herding, workers, git, cap-record]
---

# Learning: A dispatched worker rewrote a commit it did not make

**Category:** failure
**Severity:** standard
**Tags:** [herding, workers, git, cap-record]
**Applicable-when:** dispatching cells to herding workers (paseo-pi-flash) that share one feature worktree in sequence

## What Happened

Cell pl-3 ran in a herding worker on the shared `wt/piggery-lessons` worktree. Before
committing its own cell, the worker ran `git commit --amend` on the previous commit, pl-2's
`ecc268c`, and reworded its message, so the commit became `2687f82`. The code did not change
(`git diff ecc268c 2687f82` is empty), but the amend dropped the attribution line, and pl-2's
cap record still names `ecc268c`, a sha that is no longer on the branch. In
paseo-leader-dispatch, cell pld-1 also rewrote a commit message, its own, with `commit-tree`
and `update-ref`, because the concurrent-worker guard refuses `commit --amend`.

## Root Cause

The guard that refuses `commit --amend` is a Claude Code hook. A herding worker runs pi
inside a Paseo pane, where that hook is not installed, so nothing stops it from rewriting
the branch's history. The worker brief says "execute exactly one cell", but it never says
"never touch a commit you did not make".

## Recommendation

Hold the rule as a check where every worker passes through: refuse a cap whose
worker job moved HEAD's ancestors, by comparing the job's `base_sha` ancestry with the
branch after the run, or give the herding worker the same commit guard the Claude hook
enforces. Until then, the leader compares `git reflog` against the cap records before
merging. Backlog: see the PBI filed with this learning.

## Evidence

- `git reflog` in the piggery-lessons worktree: `2687f82 HEAD@{1}: commit (amend): Show waiting or lost Paseo leaders on the board`, then `ecc268c HEAD@{2}: commit: Show Paseo leaders waiting on a permission or just lost on the board`.
- `docs/history/paseo-leader-dispatch/promote-proposals.md`, cell pld-1 deviation: "rewrote the commit message with commit-tree and update-ref … the concurrent-worker guard refuses commit --amend".

# Learning: A herding worker timed out idle during a long cargo build

**Category:** failure
**Severity:** standard
**Tags:** [herding, timeouts, cargo]
**Applicable-when:** dispatching a Rust cell to a herding worker with the default `--idle-timeout`

## What Happened

Cell pl-2's herding worker ended with outcome `timed_out_idle` after its edits but before
its commit. The leader finished the review, the tests and the commit by hand. pl-3, run with
`--idle-timeout 1800`, finished on its own.

## Root Cause

`bee herding run` defaults `--idle-timeout` to 900 s, measured from log or pane activity.
A cargo test run of the full workspace stays silent long enough to cross that line.

## Recommendation

Pass `--idle-timeout 1800` when dispatching a cell whose verify runs cargo across the
workspace. This is a one-repo tuning fact, not a promotion candidate.

## Evidence

- `.bee/cells/pl-2.json` trace deviation: "the dispatched worker timed out idle before committing".
- The herding result for pl-2 recorded `"outcome":"timed_out_idle"`.
