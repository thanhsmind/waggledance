---
date: 2026-08-23
feature: console-theme-kanban, console-rail-orchestrator, console-phone-layout, bee-agent-activity, board-new-task
categories: [pattern, failure, decision]
severity: standard
tags: [tests, proof, dispatch, board]
---

# Learning: A test that moved with a literal is narrowed, never deleted

**Category:** pattern
**Severity:** standard
**Tags:** [tests, board]
**Applicable-when:** a UI cell moves or renames a literal that an exact-match
test pins (a class name, a heading word, a column count).

## What Happened

Across seven cells in five features (baa-2, baa-7, cro-1, cro-3, cpl-1, cpl-2,
board-new-task-1) the worker found an exact-literal test red because the
literal it pinned had moved, and in every case the cell trace records the test
**re-pointed or narrowed to the new truth** rather than deleted. The one time a
test was weakened instead of re-pointed (ctk-5, the colour-alone chip test) the
judge failed the round and the worker repaired it stronger ("tone variety, not
a bare count").

## Root Cause

Exact-literal tests on markup are the board's only guard against a silently
broken page (CONTEXT.md of console-rail-orchestrator: "a quietly broken one is
a red base"). Deleting one trades a loud red for a quiet gap; weakening one
keeps the name of the guard without the guard.

## Recommendation

When a literal moves, re-target the test at the new literal in the same cell
and keep its assertion at least as strong; record the retarget as a deviation
line. Never delete or loosen the test to pass.

# Learning: A cell's verify is narrower than CI

**Category:** failure
**Severity:** standard
**Tags:** [proof]
**Applicable-when:** calling a slice green from a cell's recorded verify.

## What Happened

During console-theme-kanban the orchestrator reported "green" three times on
`cargo test -p waggledance` while the declared `commands.test` (fmt + clippy
`-D warnings` + workspace test) was red: ctk-5 left 6 rustfmt diffs and 2
clippy errors that only ctk-6's worker found and fixed as a red base (capture
stub e4d5769c; ctk-6 trace "fixed a red base (6 rustfmt + 2 clippy)"). baa-3
likewise fixed a pre-existing rustfmt violation in a reserved file.

## Root Cause

The cell `verify` field is a scoped subset; a proof line quoting it reads as
full coverage when it is not. fmt and clippy are cheap and both gate CI.

## Recommendation

Run `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D
warnings` once per slice before the slice is called green, whatever the
cell's own verify says. Filed as backlog friction for a durable owner (a
slice-close check).

# Learning: Herding availability is an environment fact, not a subcommand

**Category:** failure
**Severity:** standard
**Tags:** [dispatch]
**Applicable-when:** deciding whether to route a cell to the herding transport
or fall back to the model runtime.

## What Happened

`bee herding status` is declared in the command registry but not built into
the shipped binary; its not-built message read as "herding unavailable", and
two generation-tier cells in console-theme-kanban took the model fallback
while herdr was alive and listed the session itself (stub 888123f3). Separately,
`bee dispatch prepare` returned a generation payload for ctk-6 although
`bee cells tier` had recorded ceiling, and offers no `--tier` flag
(stub ca2ca7b7).

## Root Cause

Transport availability lives in `HERDR_PANE_ID` / `HERDR_ENV`; the recorded
cell tier does not reach the prepared payload.

## Recommendation

Check the HERDR env vars before concluding a pane transport is unreachable;
for a ceiling cell, run it on the session model and ignore a herding payload
that names a cheaper slot. Both filed as backlog friction against bee.
