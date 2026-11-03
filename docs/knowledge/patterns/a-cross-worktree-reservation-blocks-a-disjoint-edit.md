---
type: bee.pattern
title: A cross-worktree reservation blocks the claim even when the two edits are disjoint
description: "Pitfall: reservations are per file, so an unrelated feature's live hold on views.rs refuses bee cells claim for a change that touches entirely different lines — and the write guard treats the same hold as advisory, so the two doors disagree."
timestamp: 2026-08-31
bee:
  id: a-cross-worktree-reservation-blocks-a-disjoint-edit
  lifecycle: active
  areas: [workflow-state]
  sources: [.bee/cells/tst-1.json, .bee/cells/htns-1.json]
  polarity: pitfall
  signature: RESERVATION_CONFLICT on a shared file
---

# A cross-worktree reservation blocks the claim even when the two edits are disjoint

## The trap

`trigger-settings-toggle` needed two checkboxes in `views.rs`. `bee cells claim`
refused with `RESERVATION_CONFLICT`: an unrelated live worktree —
`home-terminal-new-shell`, cell `htns-1` — held a cross-worktree reservation on
that file. The two edits were disjoint by line range and could not have
collided.

Reservations are per **file**, and `views.rs` in this repo is a ten-thousand-line
file that nearly every feature touches. Any two concurrent features are
therefore likely to contend on it regardless of what they are actually changing.

## The two doors disagree

The claim door refuses on that hold. The write-guard hook flags the same hold as
**advisory only** and lets the edit through. So the state a worker lands in is:
the file is writable, the cell is unclaimable, and neither door is wrong about
its own rule.

`tst-1` resolved it by making the `views.rs` edit under the advisory path, then
formally claiming and finishing once the hold cleared naturally a few minutes
later. That works, and it is worth naming what it costs: for those minutes the
work existed with no claim behind it, which is exactly the state the claim is
supposed to prevent.

## What to do

Triage, do not wait in silence and do not write through a hold you have not
reasoned about. The multi-session rule already says it: take disjoint items
first, split scope to the disjoint files when the split is natural, and defer
the overlapped remainder with a recorded reason. On a shared file this large,
"disjoint by line range" is a real argument — record it, and re-claim once the
hold clears rather than skipping the claim entirely.

## Related

- [[deferring-a-commit-on-a-contended-file]] — the same contention at commit
  time, and why deferring does not protect the boundary.
- [[a-private-index-commit-leaves-the-shared-one-lying]] — the commit-side
  mechanism for landing your own paths while a sibling holds the tree.
