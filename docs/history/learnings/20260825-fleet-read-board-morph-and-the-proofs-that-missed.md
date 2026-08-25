---
date: 2026-08-25
feature: ask-state-fleet-read, dispatch-project-presets, board-live-morph, session-work-line, spawn-destination-fallback
categories: [failure, pattern, decision]
severity: critical
tags: [testing, proof-scope, orchestration, renderer, knowledge]
---

# Learning: Two green suites, two inert features, and the proofs that would have caught both

**Category:** failure
**Severity:** critical
**Tags:** [testing, proof-scope, orchestration]
**Applicable-when:** capping any cell that publishes a new field, or that adds
markup or style to a renderer whose tests assert on strings.

## What Happened

`ask-state-fleet-read` shipped a pane inventory on the state rollup and capped
green: 875 unit tests passed. Against a running daemon the field was null on every
single call. The reader took the orchestration handle out of a slot; its tests filled
that slot themselves, while production builds the handle lazily and had not built it
yet at read time. The D5 test passed *for the wrong reason* — it hands the function
the slot it wants. The defect surfaced only when the built binary was run against the
real registry, and cost a fix-first cell (`asfr-4`) opened against already-capped work.

`board-live-morph` produced three deviations in two cells, none about behaviour: a
key attribute could not sit beside its sibling because two dozen assertions pin those
two attributes literally adjacent; three whole-fragment assertions outside the cell's
named anchors broke and only the full package run surfaced them; and new style rules
had to be placed after the real rules of the same name because two tests select by
first- and second-literal-substring match.

## Root Cause

Both are the same defect at different altitudes: the proof was scoped to the unit and
the seam belonged to nobody. In the first case the seam is between a lazy constructor
and its reader — a unit test cannot see it, because the test *is* the constructor. In
the second the seam is between the renderer's output ordering and a body of
assertions that made that ordering a contract nobody wrote down; the cell's own
named-anchor run is scoped exactly small enough to miss it.

Root cause of the miss, not the bug: "related tests green" was read as "the tests this
cell names", when for a published field the related test is one real call, and for a
renderer it is the package suite.

## Recommendation

- When a cell publishes a new field, read that field once from the **built binary**
  before capping. One real call is cheaper than the fix-first cell it replaces.
- When a cell touches a renderer whose tests assert on strings, run the **package**
  suite, not the cell's named anchors. Grep the test file for the class or attribute
  first: `assert_eq!` on a whole fragment, a substring spanning two attributes, or any
  *n*-th-occurrence selector means the file's ordering is load-bearing before you edit.
- Prefer passing a built dependency in over reaching into a slot for it. A parameter
  cannot be empty at the wrong moment.

Promoted as [the-test-builds-the-collaborator-production-does-not](../../knowledge/patterns/the-test-builds-the-collaborator-production-does-not.md)
and [assertions-that-pin-literal-adjacency](../../knowledge/patterns/assertions-that-pin-literal-adjacency.md).
`prove-the-whole-path` — already critical — recurred here and now carries a
`bee.signature`, so `bee knowledge report` counts it instead of leaving the count to
memory.

---

# Learning: One resolver, or the human and the agent will disagree

**Category:** decision
**Severity:** standard
**Tags:** [orchestration, mcp]
**Applicable-when:** two callers on the same machine resolve the same
operator-facing label.

## What Happened

The board resolved a spawn command from the *target project's* own configuration.
The dispatch tool resolved the same kind of label against a global list that was
empty in practice. Same machine, same labels, two resolvers — so a human clicking
Start could spawn an agent kind into another project and an agent calling dispatch
could not. `dispatch-project-presets` collapsed both onto one by-label resolver.

Found while distilling an external project, not while using the feature: the two
paths each look correct in isolation, and nothing fails loudly when they diverge —
one of them simply never finds anything.

## Root Cause

A label is an operator-facing name, so its meaning must have exactly one home. When
a second caller grows its own lookup, the divergence presents as an empty result
rather than an error, which is why it survived.

## Recommendation

When adding a caller for an existing operator-facing label, find the resolver the
existing caller uses and call it. If it is not callable from there, make it callable
— never write the second lookup. A second resolver that returns empty is
indistinguishable from a correctly-configured absence.

---

# Learning: The lock trail is evidence; session start times are correlation

**Category:** pattern
**Severity:** standard
**Tags:** [orchestration, multi-session]
**Applicable-when:** attributing an untracked or modified file to the session that
wrote it.

## What Happened

A peer session was blocked on a dirty checkout. Bracketing the file mtimes against
the live sessions' start times named no owner, because the real owner was a session
rooted in a *different* project writing into this one — it never appeared in this
repo's session list at all. Matching `.bee/logs/contention.jsonl`'s lock trail
(caller session + timestamp) against the mtimes named it directly.

The same turn also produced its inverse: a worktree's *creation* was read as evidence
that the dirt in main was about to move into it. It was not. The worktree's contents
said so.

## Root Cause

Start-time bracketing infers the owner from co-occurrence; the lock log records who
actually held the lock. Only the second survives a cross-project owner.

## Recommendation

Read the contention log before naming an owner, and check a worktree's actual
contents before telling anyone to wait for a move. Promoted as
[the-lock-trail-names-the-owner](../../knowledge/patterns/the-lock-trail-names-the-owner.md).

---

# Learning: A watcher that only knows the projects it started with

**Category:** failure
**Severity:** standard
**Tags:** [daemon, testing]
**Applicable-when:** testing any live-reload behaviour.

## What Happened

Registering a project into an already-running daemon indexes it and serves it
correctly, but broadcasts nothing on the live channel for it. A page for that project
sits live-reload-dead. Cost an hour chasing a phantom bug in new client code that was
in fact working.

## Root Cause

The watcher watches the set of projects known at startup. Registration adds to the
index, not to the watch set.

## Recommendation

Restart the daemon after registering a project, before testing anything live. The
tell is zero change frames on the live channel for a project whose pages otherwise
render fine. Recorded in the Daemon spec's settled edge cases.
