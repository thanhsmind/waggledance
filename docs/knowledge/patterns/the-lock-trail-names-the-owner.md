---
type: bee.pattern
title: The lock trail names the owner; session start times only correlate with it
description: "Practice: to find which session wrote an untracked file, match the contention log's lock trail against file mtimes instead of bracketing by session start times — the log survives the case that defeats inference, an owner whose transcript lives under a different project than the repo it writes into."
timestamp: 2026-08-25
bee:
  id: lock-trail-names-the-owner
  lifecycle: active
  areas: [orchestration, workflow-state]
  sources: [.bee/logs/contention.jsonl]
  polarity: practice
  signature: contention.jsonl
---

# The lock trail names the owner; session start times only correlate with it

## The situation

A checkout is dirty and nobody will admit to it. The usual move is to list the live
sessions, bracket their start times around the file's mtime, and name the one that
fits. That is correlation, and it fails in the exact case that matters most: a session
rooted in one project writing into a sibling repo never appears in that repo's session
list at all, so the true owner is not among the candidates being bracketed.

## The practice

Read the contention log and match its lock trail — caller session plus timestamp —
against the file mtimes. The log records who actually held which lock at which second,
which is evidence rather than inference, and it names cross-project owners the session
list cannot see.

Found this way once already: a peer session blocked on a dirty checkout, whose owner
turned out to be a session rooted in a different project entirely.

## The second half, learned the same turn

A worktree's *creation* is not evidence that the dirt in the main checkout is about to
move into it. Check what the worktree actually contains — tracked files and what is on
disk — before telling anyone to wait for a move that may never come. Inferring the
move from the creation is the same correlation-shaped reasoning the lock trail beats.
