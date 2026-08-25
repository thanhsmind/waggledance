---
type: bee.pattern
title: Deferring a commit on a contended file does not protect the boundary
description: "Pitfall: a cell that leaves its change uncommitted because the file already holds someone else's in-flight work does not keep the two apart — it only hands the sweeping to whoever commits that file next."
timestamp: 2026-08-25
bee:
  id: deferring-a-commit-on-a-contended-file
  lifecycle: active
  areas: [workflow-state, orchestration]
  sources: [docs/knowledge/work/todo-column-collapse/delivery.md]
  polarity: pitfall
  signature: commit_pending
---

# Deferring a commit on a contended file does not protect the boundary

## The trap

One commit per cell meets a file that already carries unrelated uncommitted
work. Committing it would sweep a stranger's changes into this cell's commit, so
the cell caps with the change left in the tree and a recorded reason — the
change is real, the tests are green, and the commit is left "for the owner to
split".

Nobody splits it. The next session to touch that file commits it, and the
deferred change rides along inside a commit whose subject describes something
else entirely. The boundary the deferral was protecting is crossed anyway; what
actually changed is who did the sweeping and how the result is described.

## What it looked like here

A cell folding one board column capped with the reason recorded and no commit.
The change went on to land inside a commit named for the unrelated agent-mark
work it shared the file with. The trace still read "uncommitted" at scribing
time, so the delivery record had to be written against the repository's own
commit log rather than against the cell's own record. A second feature hit the
same shape from the other side, its worker committing one file while a sibling
worktree held the other.

## The practice

- Prefer a path-scoped commit of your own paths over deferring: it lands the
  change under an accurate subject and leaves the neighbour's work untouched.
- If the commit really must wait, say who is expected to make it and when —
  "left for the owner to split" names nobody and expires immediately.
- At scribing time, check where a `commit_pending` change actually ended up
  before describing it. The cell's own trace is frozen at cap and will still
  claim the work is uncommitted long after someone else has committed it.
