---
date: 2026-08-31
feature: observer-tick-trigger
categories: [pattern, failure]
severity: standard
tags: [background-task, self-exclusion, autonomous-dispatch]
---

# Learning: Hook a downstream consumer into an existing sweep loop via a channel, not a weaker re-derivation of its output

**Category:** pattern
**Severity:** standard
**Tags:** [background-task, reaper, event-driven]
**Applicable-when:** a new background task needs to react to a state transition an
existing sweep/reconcile loop already computes.

## What Happened

The first draft of `observer-tick-trigger`'s "run capped" detector polled the run
ledger itself and diffed the working-run set across polls to infer a cap. The
plan-step hat wave (`hat-alternatives`) found this fires on ordinary healthy run
completions too, not just a reaper cap — the ledger's status column cannot
distinguish "reaper capped it" from "an MCP awaiter finished it normally" from
outside. The same seat found `Reaper::sweep_once` (`crates/waggledance/src/reaper.rs`)
already computes and returns the exact `Vec<(String, Verdict)>` needed, and
`Reaper::run` was discarding it (`let _ = self.sweep_once().await;`,
`reaper.rs:304`) with exactly one production caller (`main.rs:246`).

## Root Cause

A new consumer that needs a sibling task's decision reached for its own poll and
its own (weaker) inference instead of asking whether the sibling already computes
the fact and merely isn't exposing it. Re-deriving state from an entity's
side-effects (a ledger row's presence/absence) is strictly less precise than the
entity's own verdict, and the gap is invisible until an edge case (a normal
completion, a different terminal status) is tested directly against the two
sources.

## Recommendation

Before writing a new poll-and-diff detector against a store another task already
sweeps, check whether that task's own sweep function already returns (or discards)
the exact decision needed. When it does, thread it out via one optional
channel/callback parameter on the existing `run()`/`with_cancel_flag` entry point
— additive, backward-compatible, and every existing caller and test keeps its
current signature with `None` behavior byte-for-byte (verified here: `reaper.rs`'s
own tests were unchanged by the addition).

# Learning: An entity-scoped self-exclusion marker cannot close a loop through a side-channel with no entity pointer

**Category:** failure
**Severity:** standard
**Tags:** [self-exclusion, autonomous-dispatch, loop-prevention]
**Applicable-when:** an autonomous task marks the entities it creates (a run, a
job, a record) so it never re-observes its own output, AND that task's action can
also produce output in a store that isn't keyed by that entity.

## What Happened

`observer-tick-trigger`'s D9 decision requires every dispatched tick to carry a
`feature` marker on its `Run` row, and every one of the trigger's four detectors
filters out a run/row already carrying that marker before treating it as a new
transition (CONTEXT.md D9; verified in code by the per-slice review judge across
all four detectors, not just the tested ones). Three of the four detectors resolve
back to a `Run` row and the filter works. The fourth (D4d, new escalation-mailbox
rows in `.bee/supervisor/interventions.jsonl`) reads a row that carries no run
pointer at all — a mailbox entry the woken agent itself writes has no field the
trigger's marker could ever reach. The detector's own D9 filter is a documented
no-op on this path: a dispatched tick that goes on to record `kind: escalation`
creates a transition the trigger will see and re-act on next poll. Bounded by the
per-project cooldown (D8), not unbounded, but not closed either.

## Root Cause

Self-exclusion via an entity marker only closes the loop through channels that
carry that entity's identity. A design with N output channels needs the check
verified per channel, not assumed from having implemented it on the primary one
— "D9 is done" was true of the run-based detectors and false of the mailbox-based
one, and nothing failed loudly to say so; it took an independent reviewer tracing
each detector's code path, not just its tests, to find the gap.

## Recommendation

When an autonomous task's action can write to more than one kind of store or
signal, enumerate every store its own detectors watch and verify the
self-exclusion marker (or an equivalent) reaches each one specifically — a marker
on the "obvious" entity (here, the dispatched run) does not transitively protect a
side-channel the action also touches (here, a mailbox row with no run pointer).
When it can't reach a channel by construction (no shared key exists), that gap is
either closed by a different mechanism (a time/target correlation) or explicitly
accepted and bounded (here, by the existing per-project cooldown) — never silently
assumed closed because the primary channel is.
