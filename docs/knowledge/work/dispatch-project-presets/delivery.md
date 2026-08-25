---
type: bee.delivery
title: dispatch-project-presets — delivery
description: "Delivery record for work item dispatch-project-presets: waggledance_dispatch resolves a preset label against the target project's own herding.agents after the global list, so an orchestrator can spawn the agent kinds a project actually declares — the same source the board's Start button spawns from."
timestamp: 2026-08-25
bee:
  id: dispatch-project-presets-delivery
  lifecycle: active
  areas: [agent-terminal, orchestration]
  required_context: [docs/history/dispatch-project-presets/CONTEXT.md, docs/history/dispatch-project-presets/plan.md]
  sources: [docs/history/dispatch-project-presets/CONTEXT.md, docs/history/dispatch-project-presets/plan.md, docs/knowledge/work/ask-state-fleet-read/delivery.md, docs/knowledge/work/board-run-actions/delivery.md]
---

# dispatch-project-presets — Delivery

## What shipped

The board and the MCP tool disagreed on the same machine. `board-run-actions` D4 has
Start resolving its spawn argv from the target project's own `herding.agents`, while
`waggledance_dispatch` searched only the global `terminal.agent_presets` list — empty on
this machine — and refused everything else. A human could spawn `claude-sonnet` in
beehive; an agent holding that machine's MCP tools could not.

A `preset` label now falls through to the target project's own registry when the global
list misses. `ask-state-fleet-read` made those labels visible; this makes them usable.

Nothing about who may dispatch changed: `terminal.enabled` plus the per-project
`orchestration.enabled` are still the only gate, still off by default, and still checked
before any label is resolved.

## Locked decisions

| ID | Decision |
|----|----------|
| dispatch-project-presets D1 | Global `terminal.agent_presets` first, then the target project's `herding.agents`; a label in both resolves to the global one. Global-first is what makes this purely additive — an installation that configured a label keeps it aimed where it was, and only labels that refuse today can begin resolving. |
| dispatch-project-presets D2 | The project-side argv comes from that project's own `.bee/config.json`, read by the same resolver the board spawns through. The caller supplies a label only, never argv/env/cwd, from either source (`orchestrator-dispatch` D3 unweakened). |
| dispatch-project-presets D3 | A label in neither source is refused before any herdr call, naming the label **and** the project whose registry was searched. |
| dispatch-project-presets D4 | A label the project declares but that cannot be started refuses in those terms, never as "unknown". |
| dispatch-project-presets D5 | No new permission, switch or config key. |
| dispatch-project-presets D6 | `ask_state`'s `herding.resolvable` and this resolver share one implementation of "can this label start?". |
| dispatch-project-presets D7 | The run records the label it was given, not which source resolved it. |

## Contract

`waggledance_dispatch(project, preset, task)` — same inputs, wider resolution:

1. `config.terminal.agent_presets` (global, operator-authored).
2. The target project's `.bee/config.json` → `herding.agents[<label>]`.

Refusals, all before any herdr call:

- **Nobody declares it** — `unknown agent preset: <label> (searched the configured
  presets and project <id>'s herding.agents)`. With two sources searched, "unknown"
  alone no longer says where to look.
- **The project declares it but it cannot start** — names the label, says its
  `herding.agents` entry is not an argv this reader understands, and deliberately does
  **not** contain the word "unknown", which would send a caller hunting a typo that does
  not exist. bee's own `{argv, workspace_trust}` object form is the live case.

## Learned

**Two readers of one rule will drift, and the drift has a shape.** `herding.agents` was
read in two places with the rule spelled twice — the argv resolver's lookup, and an
inline `matches!(entry, Value::Array(tokens) if argv_tokens(tokens).is_some())` in the
registry reader that `ask_state` publishes as `resolvable`. They agreed the day they
were written. Left alone, the failure they produce is worse than either answer on its
own: a tool that advertises a label as usable and then refuses it. There is now one
`herding_argv_for_label`, both readers call it, and one test walks every declared label
asserting `ask_state` and `dispatch` give the same answer.

**An unedited green suite is the proof a refactor changed nothing.** Cell 1's whole
verification was that the existing herding cases pass with no case touched — so the rule
was "do not edit a test to fit", stated in the cell rather than hoped for.

## Verification

`cargo test -p waggledance` (884) and `-p waggledance-core` (438) on main after the
merge, green, `cargo fmt --check` clean, clippy warning count unchanged.

Live check: three real dispatch calls into beehive (`pi-agy-flash-3.7`, `agy-flash`,
`nope`) were all refused at the per-project opt-in before any label was resolved — D5
working, not a gap. The resolution itself is proved by unit cases, including the
`ask_state` agreement walk; a real spawn would require turning `orchestration.enabled`
on for a live project, which is the operator's decision, not the agent's.

## Open gaps

- bee's object form still does not resolve, so `agy-flash` is declared-but-unstartable
  everywhere. PBI `p-9212cae8`. This feature made that refusal honest; making it work is
  a separate change.
- `waggledance`, `jarvis`, `jarvis-mcp` and `memorypad` declare no `herding` block, so
  neither the board nor dispatch can start anything in them.
- No project has `orchestration.enabled` on, so nothing dispatches unattended today —
  by design, and worth stating so the capability is not mistaken for an active posture.
