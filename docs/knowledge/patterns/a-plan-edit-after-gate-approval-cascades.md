---
type: bee.pattern
title: A plan edit after gate approval cascades into re-verdicting every conflict candidate
description: "Pitfall: a close door that asks for a citation in plan.md meets the plan-freeze write guard, and the only unblock — plan-rev bump — unapproves the merged gate and forces a verdict on every derived candidate before it can be re-approved."
timestamp: 2026-08-31
bee:
  id: a-plan-edit-after-gate-approval-cascades
  lifecycle: active
  areas: [workflow-state]
  sources: [docs/history/observer-tick-trigger/plan.md]
  polarity: pitfall
  signature: plan.md edit refused mid-close
---

# A plan edit after gate approval cascades into re-verdicting every conflict candidate

## The trap

A close door asks for a small text change in `plan.md` — a doc-deferral note, a
routing citation. The shape+execution gate is already approved, so `plan.md` is
frozen and the write guard refuses the edit.

The only unblock is `bee state plan-rev bump`. It does what it says, and one
thing it does not say: bumping `plan_rev` **unapproves shape and execution as a
side effect**, because the plan-time conflict precondition ties `conflict_review`
to `plan_rev`. Re-approving the merged gate then refuses until every conflict
candidate carries a verdict — so the sequence is `plan-conflicts derive`, one
`plan-conflicts verdict` per candidate, and only then `bee gate --merge`.

For `observer-tick-trigger` that derive produced **29 candidates**, spanning
many unrelated features' decisions rather than only ones touching the plan.

## Why it is worth knowing in advance

Each verdict is cheap — mostly `compatible`, and a bulk loop handles them — so
the cost is not the work. The cost is discovering mid-close that a one-line doc
edit has silently un-approved the gate you already hold, and then meeting a
refusal that names a precondition you had no reason to be thinking about.

## The cheaper route

Ask whether the plan edit is needed at all. A frozen plan may take a stamp but
not a content edit, and the citation the door wants can often live where the
door actually reads it — the cell, the decision log, or `CONTEXT.md` — none of
which are frozen. Bump `plan_rev` when the plan is genuinely wrong, not to
satisfy a citation that has another home.

## Related

- [[the-first-commit-of-a-frozen-plan]] — the same freeze, met from the commit
  side.
