# Fleet Read On ask_state — Context

**Feature slug:** ask-state-fleet-read
**Date:** 2026-08-25
**Shaping session:** complete
**Scope:** Quick
**Domain types:** CALL | READ

## Feature Boundary

`waggledance_ask_state` gains two additive, read-only fields per project — the
target project's **herding registry** (agent-kind labels only) and the **live pane
inventory** already contained to that project — so an orchestrator agent can see
which agent kinds a project offers and which panes already exist, and choose to
reuse an idle pane instead of spawning. The feature ends at the read: it grants no
new authority, adds no tool, and puts no reuse-before-spawn policy inside
waggledance.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never
a silent edit.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The two fields are added to the existing `ask_state_digest` payload. **No eighth MCP tool is created.** Both the single-project form and the no-argument rollup form carry them. | `orchestrator-dispatch` D2 locks the dispatch-family tool count; an additive field on an existing tool leaves that decision untouched, a new tool would supersede it. The rollup form is the cross-project view an orchestrator needs most, so it is not the place to economise. |
| D2 | `herding` carries **labels only**: `{default, agents[], resolvable[]}` — the `herding.agent_command` default, every key in `herding.agents`, and the subset whose argv this reader can actually resolve. **The argv itself is never published**, in any form, for any label. | `orchestrator-dispatch` D3 says a caller may name a preset and may never supply argv. Publishing argv invites a caller to paste it back and defeats that decision by convention rather than by code. `resolvable[]` is also how an entry bee wrote in a shape this reader does not understand declares itself, instead of failing silently later. |
| D3 | `panes` is the existing `project_panes` projection, containment-filtered to the project's own boundary exactly as the board already filters it. Fields carried: `pane_id`, `kind`, `status`, `bee_state`, `bee_feature`, `workspace`, `tab`. The pane's `name` and `title` are **not** carried. | `name`/`title` are what the agent is called and what it is doing right now — screen-derived text an orchestrator does not need to route work, and which `badge-title` D1a already treats as a field to keep out of surfaces that do not need it. |
| D4 | One herdr `snapshot()` per tool call, regardless of how many projects the answer covers. The rollup form filters that single snapshot per project; it never calls herdr once per project. | herdr's snapshot is already the whole machine's flat pane list, so per-project calls would be N times the cost for the same bytes. |
| D5 | herdr unreachable → `panes` is `null` with a named reason field beside it, and the bee-state half of the answer is returned in full. **Never a tool error.** | This is the posture `waggledance_search` already takes with its `refresh` flag: return what could be read, flag what could not, never fail the whole call and never go silently stale. |
| D6 | `terminal.enabled` off → the `panes` field is **absent entirely** (not `null`, not empty), and the tool's answer is what it is today. | `ask_state` currently works with no herdr and no terminal family; a live read must not make the tool's behaviour depend on a switch its existing callers never set. |
| D7 | Seeing a pane grants nothing. Per-project `orchestration.enabled` plus `terminal.enabled` remain the only gate on dispatch. This feature adds no write path and no new authority. | `orchestrator-dispatch` D6 is preserved exactly; visibility and write rights stay separate. |
| D8 | **Reuse-before-spawn is a policy in the calling agent, never in waggledance.** waggledance exposes the inventory; the orchestrator decides. | `orchestrator-dispatch` D1: waggledance never decides what to dispatch, it only executes dispatches safely. A reuse rule inside the daemon would be a decision, not an execution. |

### Agent's Discretion

Exact field names and JSON nesting within the two new keys; where the reason
string in D5 sits and how it is worded; whether the herding reader is one function
or two beside `bee::herding_agent_argv`; the visibility change needed to reach
`project_panes` from `mcp.rs` (same crate).

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| herding registry | The `herding.agents` map in a project's own `.bee/config.json`: label → the command that starts that agent kind. |
| label | A key in that map (`claude-sonnet`, `pi-agy-flash-3.7`, `agy-flash`). The only part of the registry this feature publishes. |
| resolvable | A label whose entry this reader can turn into an argv today. An entry in a shape the reader does not accept is present in `agents[]` and absent from `resolvable[]`. |
| pane inventory | The panes herdr reports whose directory resolves inside the project's own boundary — the same set the board's terminal surface shows for that project. |
| reuse-before-spawn | The calling agent's policy of sending work to an existing idle pane rather than starting a new one. Named here only to place it outside this feature. |

## Specific Ideas And References

- The user's operating rule, verbatim: *"thường tôi luôn chạy 1 herdr trong máy,
  mọi action nếu có pane nên dùng nó."* One herdr per machine is already the
  architecture; the second half is what this feature makes possible for agents.
- The payload shape agreed in conversation:
  `herding: {default, agents[], resolvable[]}` and
  `panes: [{pane_id, kind, status, bee_state, bee_feature, workspace, tab}]`.
- `docs/history/research/swarmforge-platoon-layer.md` — the distill this came out
  of; its two addendum sections carry the evidence for every constraint above.

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `crates/waggledance-core/src/bee.rs:2007-2042` — `herding_agent_argv` /
  `herding_agent_argv_from_config`: already opens the target project's
  `.bee/config.json` and walks `herding.agent_command` → `herding.agents`. The
  labels reader is its sibling, over the same parsed value.
- `crates/waggledance/src/server.rs:4419-4444` — `project_panes(snapshot,
  boundary)`: the whole D3 projection, containment already enforced. Currently
  private; `mcp.rs` is the same crate.
- `crates/waggledance/src/mcp.rs:39-43` — `Orchestration` already holds a
  `SocketHerdr` and a tokio runtime in the MCP process; this is the handle
  `dispatch`/`await` use and the one D4's single snapshot needs.
- `crates/waggledance/src/mcp.rs:589-601` — `ask_state_digest`, the single place
  both the per-project and rollup answers are built.

### Established Patterns

- Degrade-with-a-flag rather than fail the call — `waggledance_search`'s
  `structuredContent.refresh`, which reports a project whose re-index failed while
  still returning hits. D5 is the same shape.
- Absent is not an error — a project with no `.bee/` reports `present: false`
  rather than failing (`mcp.rs` rollup test). D6 extends that habit to a switch.
- Containment before anything else — every pane-scoped route validates through the
  project boundary before it reads or writes.

### Integration Points

- `crates/waggledance/src/mcp.rs` — `ask_state_digest` and the `ask_state` handler
  (which must reach `Orchestration` for the herdr snapshot; today it takes only
  `&Engine`).
- `crates/waggledance-core/src/bee.rs` — the new labels reader.
- `crates/waggledance/src/server.rs` — visibility of `project_panes`.

## Canonical References

- `docs/history/orchestrator-dispatch/CONTEXT.md` — D1 (waggledance never decides),
  D2 (three dispatch-family tools), D3 (presets only, never argv), D6 (per-project
  opt-in). This feature is written to preserve all four.
- `README.md:139-152` — herdr is the only transport; waggledance never runs a
  terminal of its own.
- `docs/specs/bee-cockpit.md` — the cockpit's read-only posture.

## Outstanding Questions

### Deferred To Planning

- [ ] What `herding` reads for a project with no `.bee/`, or a `.bee/config.json`
      with no `herding` block — `null` versus an empty registry. Answered by
      matching what `ask_state_digest` already emits for an absent project, so the
      two absences read the same way to a consumer.
- [ ] Whether `status` (herdr, screen-derived) and `bee_state` (bee's own hook
      record) are both carried raw, or pre-resolved by the `pane_tone` rule where
      both exist. Answered by reading `pane_tone` and deciding which one an
      orchestrator can act on without knowing that rule.

## Deferred Ideas

Out-of-scope ideas captured during shaping. Not lost, not planned.

- Letting `waggledance_dispatch` resolve a `preset` against the target project's
  own `herding.agents` instead of the global `terminal.agent_presets` list — the
  spawn half of the same problem. Deferred: this feature makes the reuse path
  reachable, which is the half the user's operating rule needs first.
- Filling in a `herding` block for `waggledance`, `jarvis`, `jarvis-mcp` and
  `memorypad`, none of which have one — config, not code.
- Teaching `herding_agent_argv_from_config` bee's object form
  (`{argv, workspace_trust}`, used by `agy-flash` in beehive), which today resolves
  to `None` and makes the board refuse. D2's `resolvable[]` surfaces the symptom;
  the fix is its own change.
- A swarm manifest declaring which registered projects form one group. Deferred:
  `waggledance_projects()` already lists them, so an orchestrator can be told its
  membership; the manifest is only persistence across sessions.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Planning's Gate 2 shape stage and reviewing use locked decisions for coverage and UAT.
