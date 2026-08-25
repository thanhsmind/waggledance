---
type: bee.delivery
title: ask-state-fleet-read — delivery
description: "Delivery record for work item ask-state-fleet-read: waggledance_ask_state gains two read-only fleet fields — the project's herding agent labels (never their argv) and its own contained pane inventory with bee's state joined in — so an orchestrator can reuse an idle pane instead of only ever spawning."
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-delivery
  lifecycle: active
  areas: [agent-terminal, bee-cockpit, orchestration]
  required_context: [docs/history/ask-state-fleet-read/CONTEXT.md]
  sources: [docs/history/ask-state-fleet-read/CONTEXT.md, docs/history/research/swarmforge-platoon-layer.md, docs/history/orchestrator-dispatch/CONTEXT.md, docs/knowledge/work/board-run-actions/delivery.md]
---

# ask-state-fleet-read — Delivery

## What shipped

The board could see the fleet; agents could not. `board-run-actions` D4 already reads
each project's own `herding.agent_command` to spawn, and every board surface lists that
project's panes — but `waggledance_ask_state` answered with bee state alone, so an
orchestrator holding the MCP tools had no way to learn which agent kinds a project
offers or which panes were already running. Its only route was to spawn, which is the
opposite of the operating rule this work came from: **one herdr per machine, and any
action that can use an existing pane should use it.**

`ask_state` now carries two additive read-only fields on both its answer shapes:

- **`herding`** — `{default, agents, resolvable}`, the labels from that project's own
  `.bee/config.json`. Labels only; the argv behind them never leaves `bee.rs`.
- **`panes`** — that project's own contained pane inventory (`pane_id`, `kind`,
  `status`, `bee_state`, `bee_feature`, `workspace`, `tab`), read as ONE herdr snapshot
  per call however many projects the answer covers.

Nothing gained authority. `orchestration.enabled` plus `terminal.enabled` remain the
only gate on dispatch, and no reuse rule went into the daemon — waggledance reports the
inventory, the orchestrator decides what to do with it.

## Locked decisions

| ID | Decision |
|----|----------|
| ask-state-fleet-read D1 | Two additive fields on the existing `ask_state_digest`, carried by both the single-project and rollup forms. No eighth MCP tool — `orchestrator-dispatch` D2 locks the dispatch-family count, and an additive field leaves it untouched. |
| ask-state-fleet-read D2 | `herding` publishes labels only — the `agent_command` default, every key of `herding.agents`, and the `resolvable` subset. The argv is never published in any form. `orchestrator-dispatch` D3 refuses argv *from* a caller; publishing it *to* one would defeat that by convention instead of by code. |
| ask-state-fleet-read D3 | `panes` is the existing `project_panes` projection, contained by the project's own boundary, carrying exactly seven fields. The agent's `name` and `title` are not carried. |
| ask-state-fleet-read D4 | One herdr `snapshot()` per tool call; the rollup filters that single snapshot per project. |
| ask-state-fleet-read D5 | Unreachable herdr → `panes: null` plus a named `panes_error`, bee state returned in full. Never a tool error — the same posture `waggledance_search` takes with its `refresh` flag. |
| ask-state-fleet-read D6 | `terminal.enabled` off → the `panes` key is absent entirely, so the answer is byte-identical for callers who never set that switch. |
| ask-state-fleet-read D7 | Seeing a pane grants nothing; `orchestration.enabled` stays the only dispatch gate. |
| ask-state-fleet-read D8 | Reuse-before-spawn is the calling agent's policy, never waggledance's — `orchestrator-dispatch` D1 says waggledance never decides what to dispatch. |

## Contract

`waggledance_ask_state(project?)` — unchanged inputs, two new payload keys per project:

```json
"herding": { "default": "claude-sonnet",
             "agents": ["claude-sonnet", "pi-opencode-free", "pi-agy-flash-3.7", "agy-flash"],
             "resolvable": ["claude-sonnet", "pi-opencode-free", "pi-agy-flash-3.7"] },
"panes": [ { "pane_id": "w2:p1", "kind": "claude", "status": "idle",
             "bee_state": "Idle", "bee_feature": "todo-column-collapse",
             "workspace": "waggledance", "tab": "1" } ]
```

- `herding` is `null` for a project with no `.bee/` and for one whose config has no
  `herding` block — one absence, one branch for the consumer.
- `default` is `null` when `agent_command` is the inline-argv form: that form names no
  label, and its tokens are what D2 keeps out.
- A label in `agents` but not in `resolvable` will not start today. bee's object form
  (`{argv, workspace_trust}`, used by `agy-flash`) is the live example.
- `panes` has three shapes: **absent** (terminal family off), **null** beside
  `panes_error` (herdr unreadable), or the array.
- `status` is herdr's, derived from the screen. `bee_state` is bee's own hook record and
  is `null` for a pane no live session claims. Where both exist bee is the trustworthy
  one — that is the field that answers "is this pane really free?".

The tool's own description states all of this, pinned by a test: a published field
nobody can discover is not published.

## Learned

Two lessons, both bought by running the built binary against the real registry rather
than trusting a green suite.

**A lazily-built handle is invisible to tests that supply it.** `read_fleet_panes` read
the orchestration slot directly, but that handle is built lazily and nothing in an
`ask_state`-only session builds it — so `panes` degraded to `null` on every real call
while 875 unit tests stayed green. The D5 test even passed, for the wrong reason: it
handed the function the empty slot it wanted and asserted the shape that bug produced.
The fix routes through `orchestration_handle`, and that test moved onto the pure
`attach_panes` so it no longer depends on whether a socket is live where it runs. See
`patterns/prove-the-whole-path`.

**`project_panes` returns `bee_state`/`bee_feature` empty by contract.** The join lives
in `project_bee_activity` + `apply_bee_activity` and every caller must apply it; a
caller that forgets publishes two fields that can never hold a value. That join also
silently drops any activity record with no `cwd`, or a `cwd` outside the boundary —
which is what a fixture must carry before the join will fire at all.

## Verification

`cargo test -p waggledance` (875) and `-p waggledance-core` (430), green, plus the real
proof: one rollup call against the live daemon returned nine panes across three
projects, with claimed panes carrying their own bee state and feature and unclaimed ones
`null`.

## Open gaps

- `waggledance_dispatch` still resolves `preset` against the global
  `terminal.agent_presets` (empty on this machine) rather than the target project's own
  `herding.agents`, so an orchestrator can see an agent kind it cannot spawn. PBI
  `p-42cbde3a` — the spawn half of this feature's reuse half.
- `herding_agent_argv_from_config` rejects the object form bee itself writes, so
  selecting `agy-flash` makes the board refuse. `resolvable` surfaces the symptom; PBI
  `p-9212cae8` is the fix.
- `waggledance`, `jarvis`, `jarvis-mcp` and `memorypad` declare no `herding` block at
  all, so the board cannot start anything in them.
- README still advertises four MCP tools; seven ship.
