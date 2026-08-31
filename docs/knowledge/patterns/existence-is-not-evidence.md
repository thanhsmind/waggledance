---
type: bee.pattern
title: Existence is not evidence — a plan that checks a thing exists has not checked what it holds
description: "Pitfall: a planner confirms a field, file, status or method EXISTS and then asserts what it CONTAINS or whether its path ever RUNS, so the plan is built on inferences that read like verified facts and only fail at the live proof."
timestamp: 2026-08-30
bee:
  id: existence-is-not-evidence
  lifecycle: active
  areas: [orchestration, workflow-state]
  sources: [docs/history/board-visibility/plan.md, docs/history/board-visibility/plan-slice-2.md, docs/history/dispatch-submit-and-reclaim/plan.md]
  polarity: pitfall
  signature: existence is not evidence
---

# Existence is not evidence — a plan that checks a thing exists has not checked what it holds

## The trap

Planning is cheap to get wrong because producing a plan has no failure mode. A
grep confirms a field exists; the plan then asserts what that field contains. A
help page describes a flag; the plan then asserts what the daemon does with it. A
search finds zero render sites; the plan then calls the data *forgotten* rather
than asking whether it was **removed**.

Each of those is an existence check standing in for an evidence check. They are
fast, they feel like verification, and nothing contradicts them until something
downstream actually runs.

Five wrong premises reached two gates in one session, all the same shape:

- A close guard was written around a run status that **does not exist** — the enum
  was never opened, its shape was inferred from what such code usually looks like.
- A status was treated as having one producer when it had **two**, the second being
  a screen-stability guess; the guard would have killed live agents.
- A field's *values* were never read, only its *schema*. The recorded values were
  a tool name — unusable — while the code already derived a better sentence.
- Two fields were called "forgotten" because nothing rendered them. They were
  **deliberate deletions**, with the reason in a comment three lines from where the
  search ran.
- A daemon's readiness semantics were taken from CLI help text. The socket method
  did something else entirely, and only a raw wire probe settled it.

Every one of those had green tests. Tests do not catch assertion errors, because
the assertion never became code.

## The tell

- A plan sentence about behaviour whose evidence is a name, a schema, a type, or a
  help page — anything that says a thing *is*, offered as proof of what it *does*.
- "Nothing renders it" concluded as "nobody wanted it", with no check for who
  removed it and why.
- Documentation about a program used as documentation about its daemon, library,
  or socket API. Help text describes the CLI's composite behaviour, not the
  method's contract.
- The first contact with real data or real behaviour sitting in the **last** cell
  of the slice.

## What to do instead

- **Tag every load-bearing claim with how it was established** — read, ran, or
  inferred — and refuse the gate while an inferred claim is still load-bearing.
  Promote it or delete the claim. This is checkable by a reader in seconds and it
  is what a plan review actually verifies.
- **Read values, not schemas.** Opening the data costs one command. A field's
  existence says nothing about whether what it holds is worth rendering.
- **Ask a daemon, don't read about it.** For any external process, replay the exact
  payload and record the answer. A probe with a deliberately invalid argument
  distinguishes "method missing" from "argument wrong" in one call.
- **Move one cheap reality check ahead of the plan.** Render the page once, read
  one real record, run the path once — before the shape is drafted, not after it
  is approved. Both plans that failed this way would have died in seconds.
- **Zero call sites is a question, not an answer.** Check the history and the
  comments before treating unused data as an oversight.

## Recurrence

- `dispatch-submit-and-reclaim` (2026-08-30) — three premises wrong; the plan
  review caught two, the live proof caught the third after a wrong fix had already
  landed.
- `board-visibility` (2026-08-30) — rev 1's centrepiece would have made the board
  worse; rev 2 shipped only after the review read the actual recorded values. Its
  slice-2 plan then carried per-claim evidence tags and reached the gate with none
  inferred.
- **Porting from an upstream reference**, same shape at a different scale: a repo
  was treated as a source to port *from* because it had once been one. Measured,
  the local implementation had long overtaken it — 21602 and 33759 lines against
  the upstream's 933 and 1168 — and the feature named for porting had already
  shipped here weeks earlier. The name existed; the relationship it implied did
  not. Before any "port X from Y", verify the direction still runs that way.

## Related

- `prove-the-whole-path.md` is the same discipline one layer down: that one is
  *before the cap*, this one is *before the plan*. A slice can satisfy either and
  still fail the other.
