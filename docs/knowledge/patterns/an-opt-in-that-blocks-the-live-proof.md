---
type: bee.pattern
title: An opt-in that blocks the live proof is not a coverage gap
description: "Practice: when a deliberate per-project opt-in refuses the end-to-end run that would prove a feature, the refusal is the safety feature working — prove what can be proved, name what stayed unproven, and treat flipping the opt-in as the owner's call rather than the agent's."
timestamp: 2026-08-25
bee:
  id: an-opt-in-that-blocks-the-live-proof
  lifecycle: active
  areas: [orchestration, workflow-state]
  sources: [.bee/cells/archive/dispatch-project-presets/dpp-2.json, .bee/cells/archive/herdr-protocol-20/hp20-4.json, docs/knowledge/work/dispatch-project-presets/delivery.md]
  polarity: practice
  signature: live proof stops at the opt-in
---

# An opt-in that blocks the live proof is not a coverage gap

## The situation

A feature is meant to be proved end to end against something real, and the real
thing is guarded by a deliberate opt-in that is switched off — a per-project
enable flag, a consent setting, a safety default. The proof stops at the guard.

The tempting readings are both wrong. Reading the refusal as a hole in the feature
sends someone hunting a bug that is not there. Flipping the switch to get a green
run quietly changes a safety posture the owner chose, on the agent's own authority,
for the convenience of a proof.

## What it looked like here

Two features hit the same wall within days and answered it differently, both
defensibly. A dispatch feature's live run was refused at every call because the
target project had orchestration switched off; the cell proved the resolver by unit
cases, recorded that the live path stopped at the opt-in, and said plainly that
flipping it was the owner's call. A terminal-protocol port needed one real spawn to
prove anything at all, so it turned the same opt-in on for a single dispatch, gave
the spawned agent a task that touched no file, turned it straight back off, and
re-verified that the refusal had returned.

The difference is not that one was careful and the other was not. It is how much
the proof was worth: the port had no other way to know the protocol worked, and the
dispatch feature did.

## The practice

- Record the refusal as the guard working, with the switch named, rather than as an
  open gap or a failed proof.
- Prove every piece that does not need the guard, and state exactly which claim is
  left resting on unit evidence alone.
- If the live run is the only thing that can prove the feature, ask the owner, keep
  the window to one run, give it work that changes nothing, restore the switch
  immediately, and verify that the refusal came back — the restore is part of the
  proof, not cleanup after it.
- Never leave the switch flipped at cap. A safety default that a cell turned off is
  the cell's to turn back on.
