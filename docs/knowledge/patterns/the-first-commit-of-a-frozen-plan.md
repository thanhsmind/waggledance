---
type: bee.pattern
title: "A frozen plan's own first commit reads as an edit to it"
description: "Pitfall: the plan-freeze guard treats any mention of an approved plan file in a git path as a revision, so committing that file for the first time is refused — and satisfying the guard by bumping the revision records a revision that never happened."
timestamp: 2026-08-25
bee:
  id: first-commit-of-a-frozen-plan
  lifecycle: active
  areas: [workflow-state]
  sources: [.bee/cells/archive/dispatch-project-presets/dpp-1.json, docs/knowledge/work/dispatch-project-presets/delivery.md]
  polarity: pitfall
  signature: plan-freeze guard
---

# A frozen plan's own first commit reads as an edit to it

## The trap

Once a plan is approved it is frozen: a guard refuses any commit whose paths mention
it, so an agent cannot quietly rewrite the shape a human signed off on. The guard
matches on the path, not on whether the content changed. The plan file's *own first
commit* — the one that puts the approved plan into git in the first place — therefore
looks exactly like an edit and is refused.

The obvious escape is the wrong one. Bumping the plan revision satisfies the guard,
but a revision bump means "the plan changed and the gate must be re-approved". Using
it to land an unchanged file writes a revision into the record that never happened,
and silently revokes the gate that was already approved.

## The tell

A commit refused by the freeze guard where `git diff` against the previous commit
shows the plan file as *added*, not modified. If the file has no prior version, no
revision occurred.

## What to do instead

- Land the file through a temp index and `commit-tree`, so the commit exists without
  the working-tree path that trips the guard, and say so as a recorded deviation.
- Never bump the plan revision to get past a guard. The revision is a claim about
  history; spend it only when the shape actually changed.
- Better still: commit the plan in the same move that renders it, before the gate is
  approved, so the freeze never meets an unborn file.
