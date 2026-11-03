# Slash Builtin Commands — Context

**Feature slug:** slash-builtin-commands
**Date:** 2026-08-31
**Shaping session:** clear-ask fast path (gate bypass full)
**Scope:** Standard
**Domain types:** UI, HTTP

## Feature Boundary

The `/` suggest menu already lists a project's file-based commands and skills
([[composer-slash-suggest]]). It does not list what the agent itself answers
to — `/model`, `/usage`, `/compact` and the rest of Claude Code's own built-in
commands. This feature adds those, resolved from the agent actually running in
that pane, so a shell pane suggests none and a Claude pane suggests Claude's.

It ends at the suggestion list: waggledance never interprets or executes a
slash command, and the built-in table is a static, refreshable snapshot of a
vendor's registrations — not a live query of the running process.

Original ask (verbatim): «Command / còn thiếu các command có sẵn của agent ví
dụ như claude code sẽ có các command riên như /model /usage ...»

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Built-in commands ship as a static per-vendor table in the binary, generated from the installed agent's own bundle by a committed extraction tool. Only the `claude` vendor has a table today. | The registrations are real data, not recall — a memory-written list had `/cost`, `/review`, `/doctor`, `/todos`, `/rewind`, `/vim` in it, none of which claude 2.1.251 registers. No other agent CLI is installed here to extract from, and inventing one vendor's list from memory is the exact error the tool exists to prevent. |
| D2 | The endpoint learns the pane's agent kind: `/p/:id/_slash?pane=<pane_id>` joins the herdr snapshot's `agents[]` (`Agent.pane_id` → `Agent.kind`) and maps that kind to a vendor with the same substring classification `views.rs::bee_hub_agent_logo` already uses. No pane, an unresolvable pane, or a plain shell pane (never present in `agents[]`) yields file-based entries only. | A shell pane suggesting `/model` would be a lie; the join is one snapshot call the page already makes elsewhere. |
| D3 | Built-in entries carry `kind: "builtin"`, a third value beside `command` and `skill`, and lose a name collision to file-based entries — the existing first-seen-wins shadow order becomes project → user → builtin. | The menu already badges by `kind`, so a third value needs no JS change; a project command the user wrote is the one they meant. |

### Agent's Discretion

Where the generated table lives, its exact Rust shape, whether the argument
hint is rendered, and how the fetch URL carries the pane id are implementation
choices — constrained only by D1–D3 and by the existing `{name, kind,
description}` JSON contract (`ae531e75`).

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| built-in | A slash command the agent CLI itself answers, registered in its own bundle — never a file under `.claude/`. |
| vendor | The agent family a pane's `kind` string classifies to (`claude`, `codex`, …), the key the built-in table is looked up by. |

## Existing Code Context

- `crates/waggledance/src/slash.rs` — `SlashEntry`, `slash_entries`, the
  first-seen-wins shadow rule, and the module's tempfile fixture tests.
- `crates/waggledance/src/server.rs` — `/p/:id/_slash` and `/_slash` routes
  (registered beside `_jump`), plus the two route-level tests.
- `crates/waggledance/src/herdr/wire.rs:46-52` — `Agent { pane_id, kind }`;
  `wire.rs:151` `Pane` carries no kind, which is why the join goes through
  `agents[]`.
- `crates/waggledance/src/views.rs:792` — `bee_hub_agent_logo`, the existing
  kind→vendor substring classification to mirror.
- `crates/waggledance/assets/app.js:148` — `wireSlashSuggest(input, fetchUrl)`
  and its three call sites (project panes ~3735, unassigned ~3957, paseo ~4444).
