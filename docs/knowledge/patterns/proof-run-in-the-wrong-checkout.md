---
type: bee.pattern
title: A green proof run in a checkout that lacks the change proves nothing
description: "Pitfall: a cap's verify command runs in whatever checkout the session happens to sit in, so a feature living on a branch worktree can be certified green by a run that never saw it."
timestamp: 2026-08-25
bee:
  id: proof-run-in-the-wrong-checkout
  lifecycle: active
  areas: [orchestration, workflow-state]
  sources: [docs/history/waggledance-rename/promote-proposals.md]
  polarity: pitfall
  signature: run in the MAIN checkout
---

# A green proof run in a checkout that lacks the change proves nothing

## The trap

Work lives in its own branch checkout; the session driving it sits somewhere
else. The verify command is a plain test invocation with no path in it, so it
runs against whatever tree the shell is standing in. When that tree is the main
checkout, the run is green — honestly green, for code that does not contain the
change. The cap records a passing command and the change is certified by
evidence that never touched it.

It is a quiet failure because every visible artifact is correct: the command is
the declared one, the output is real, the count is plausible. Only the working
directory is wrong, and nothing in the recorded proof line names it.

## What it looked like here

During the rename of the crate family, the first cell's own trace had to record
that its test run had executed in the main checkout, which did not carry the
rename at all — the change lived on the branch. Later cells in the same feature
corrected the habit explicitly, each recording evidence as "run in the worktree"
and naming the passing count against a stated baseline. The correction was
carried in prose by whoever remembered, cell by cell, rather than by anything
that would refuse a run from the wrong place.

## The practice

- A proof line names where it ran when the work is not in the main checkout —
  "run in the worktree" is part of the evidence, not a courtesy.
- A passing count is worth little on its own; a count against a stated baseline
  ("905 passed, baseline 900") is what shows the run saw new code.
- When a check cannot run at all in the available environment, the honest record
  is that it did not run, naming what was missing — never a neighbouring
  command's green standing in for it.
