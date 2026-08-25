# The Board Reads The Same Declaration — Context

**Feature slug:** board-entry-conditions
**Date:** 2026-08-25
**Shaping session:** complete
**Scope:** Quick
**Domain types:** RUN

## What was asked

Finish `herding-entry-conditions`: the dispatch path honours a project's whole
`herding.agents` entry, but the board's **Start / Run review / Run compound** still
resolve argv-only and so apply neither `env` nor `workspace_trust`. Two spawners, one
declaration, two behaviours.

## What was found

- The board resolves through `bee::herding_agent_argv(root)`, which walks
  `herding.agent_command` — an inline argv array, or a label looked up in
  `herding.agents` — and returns tokens. Everything the entry declares around the command
  is dropped on the floor.
- `herding_entry_for_label` already exists (`herding-entry-conditions` D1) and returns the
  whole entry. What is missing is the *default* form of it: resolve `agent_command` into an
  entry rather than an argv.
- `server.rs:2288` and `:2318` build `DispatchTarget::Spawn` and currently synthesise an
  entry with no conditions — a placeholder left deliberately, with a comment saying
  widening the board is its own change. This is that change.

## What will be done

Add `herding_default_entry(root)`, the entry-shaped sibling of `herding_agent_argv`:
an array `agent_command` becomes an entry with that argv and no conditions; a string one
resolves through `herding_entry_for_label` and keeps whatever it declares. The board's two
call sites pass it straight into `Spawn { entry, .. }`, which already honours conditions.

## Locked decisions

| ID | Decision |
|----|----------|
| D1 | The board resolves its default through the full entry shape. Conditions are applied by the same code the dispatch path uses — there is no second implementation of seeding or exporting. |
| D2 | The inline-argv form of `agent_command` declares no conditions and behaves exactly as it does today. |
| D3 | Nothing about the write changes: still only the directory being spawned into, still never removing, still idempotent, still fail-open (`herding-entry-conditions` D3–D5, unchanged and not re-decided here). |
| D4 | **The directory the board trusts is the feature's own granted worktree** — a sibling of the project root, deliberately outside the boundary the MCP path validates against. That is stated rather than assumed: it is a directory bee created for this project, and it is the directory the agent is actually about to run in, which is the thing D3 has always been about. |

## What this does not change

- The MCP dispatch path, already shipped and unchanged.
- The trust write's constraints, its fail-open behaviour, or the env rules — inherited
  whole from `herding-entry-conditions`, not re-decided.
- Whether a failed seeding is reported. The board surfaces run outcomes on the card; the
  warning's route to a board reader is **an open gap**, named here rather than smoothed:
  `herding-entry-conditions` D9 gave the dispatch answer a home for it, and the board has
  no equivalent field yet.

## Out of scope

- Surfacing the seeding warning on a board card (the gap above).
- bee's built-in entries, still not mirrored.
