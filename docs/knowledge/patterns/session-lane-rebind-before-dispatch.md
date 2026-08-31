---
type: bee.pattern
title: A carried-over session must rebind its lane before dispatching workers
description: "Pitfall: bee's write guard resolves gates through the CALLING SESSION's bound lane, not the worktree's own state record — a session picked up from a prior feature stays bound to that feature's lane and every dispatched worker's write is refused until the session is rebound from the main checkout."
timestamp: 2026-08-31
bee:
  id: session-lane-rebind-before-dispatch
  lifecycle: active
  sources: [.bee/cells/tkg-1.json, docs/history/term-keys-grid/promote-proposals.md]
  polarity: pitfall
---

# A carried-over session must rebind its lane before dispatching workers

## The trap

A session that already did work on one feature (e.g. `board-visibility`) is
reused to pick up a different feature (e.g. `term-keys-grid`). The session's
own lane binding does not follow the new work automatically — it stays
pointed at the old feature. The write guard resolves every gate check through
that *bound lane*, not through the worktree's own state record, so every
dispatched worker's write is refused (the guard reads the wrong feature's
gates) even though the worktree itself is correctly on the new feature.

## The tell

- A dispatched worker blocks on a write-guard refusal that names a gate or
  feature the current task has nothing to do with.
- The session was previously bound to a different feature earlier in the same
  wall-clock session.

## What to do instead

Rebind the session from the main checkout before dispatching execution
workers into the new feature's worktree — the harness's worktree-isolation
hook refuses any command containing the word "bind" while a session is
isolated inside a worktree, so this must happen before entering it.

## Recurrence

- `term-keys-grid` (2026-08-28) — two blocked turns before the coordinator
  rebound the session from the main checkout.
- `home-terminal-panel` era terminal-soft-keys work (2026-08-28/29) — the same
  stuck-lane symptom, resolved the same way, recorded independently before
  this pattern was named.
