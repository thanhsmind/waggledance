---
type: bee.delivery
title: bee-board-pm — delivery
description: "Delivery record for work item bee-board-pm: the bee board was rebuilt to answer a manager's questions in order — where the work stands, what needs a person, what is moving — and to never print a path from the machine it reads."
timestamp: 2026-08-06
bee:
  id: bee-board-pm-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# bee-board-pm — Delivery

## What shipped

The board listed cells in four state columns. That answers a worker's question —
what is open — and not the one a person arriving at the page actually has: how
is this going, and does anything need me.

**Reading order.** The page now opens with a header, a lifecycle stepper, the
headline numbers, a card naming what is being worked on right now, and a panel
of what needs a person. Everything that was already there follows underneath,
untouched.

**Needs-attention is derived, not decorated.** Blocked cells and unreadable store
files each raise their own item; a paused or kindless handoff raises one; a
recorded gate-bypass level raises one, reported as recorded rather than as
effective, since the board cannot know what the session actually did. Items sort
heaviest first with a stable tie-break, so the same store always produces the
same order.

**Work is placed by phase, not by cell state.** Every feature the store knows
about is placed on its own phase, taking the union of the lane records and the
active feature so a feature named in only one of them still appears. Finished
work is listed from what actually shipped, deduplicated against the phase board.
The supporting panels — backlog by status, the review queue by state with open
P1s called out, delivery speed, sessions, worktrees, workspaces and process
health — close the page, and a store with none of these renders the panel empty
rather than absent.

**A gate that is currently approved reads as approved.** An earlier revocation
only labels a gate that has since been undone; it no longer overrides a gate that
was subsequently granted.

**No path from the machine reaches the page.** Every free-text field the reader
produces — cell titles, a lane's next action, backlog and finding text, decision
text, a handoff's next action — is scrubbed of absolute paths, including a path
wrapped in brackets, quotes, backticks or trailing punctuation, and including
Windows-shaped paths. A feature name coming from the store is validated before it
is ever joined onto a path, and the reader only ever checks for the presence of a
feature's proposal file rather than reading it.

**Narrow and dark are first-class.** The board collapses to one column on a
narrow screen, its wide phase columns scroll within themselves rather than
stretching the page, the finished-work control shows a visible focus ring, and
the dark-scheme rules are proven rather than assumed.

## Verify

`cargo test --workspace` green at each of the eighteen caps, with the scrubbing,
the phase union, the attention rules, the gate-revocation rule and the
directory-aware tree comparison each carrying their own tests. The final cell
rewrote `docs/specs/bee-cockpit.md` to describe the board as built, which is why
that spec reads as it does today.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from eighteen capped cell traces.
