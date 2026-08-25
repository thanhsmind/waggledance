# Dispatch Resolves A Project's Own Agent Kinds — Context

**Feature slug:** dispatch-project-presets
**Date:** 2026-08-25
**Shaping session:** complete
**Scope:** Standard
**Domain types:** CALL | RUN

## Feature Boundary

`waggledance_dispatch`'s `preset` label resolves against the **target project's own**
`herding.agents` registry as well as the global `terminal.agent_presets` list, so an
orchestrator can spawn the agent kinds a project actually declares — the same argv
source the board's Start button already spawns from. The feature ends at label
resolution: no new authority, no new switch, no raw argv from a caller, and no change
to what happens after a preset resolves.

## The Problem This Closes

`ask-state-fleet-read` made a project's agent labels visible to an orchestrator. This
makes them usable. Today the board and the tool disagree on the same machine:

- **The board** resolves the spawn argv from the target project's own
  `.bee/config.json` — `herding.agent_command` through `herding.agents`
  (`board-run-actions` D4, `bee::herding_agent_argv`).
- **`waggledance_dispatch`** resolves `preset` only against the *global*
  `config.terminal.agent_presets`, which is `[]` on this machine — so every
  preset-spawn dispatch refuses with `unknown agent preset`, and only targeting an
  already-running pane works.

A human clicking Start can spawn `claude-sonnet` in beehive; an agent holding the MCP
tools cannot. That asymmetry, not a missing capability, is the whole feature.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never a
silent edit.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | A `preset` label resolves in this order: the global `terminal.agent_presets` list **first**, then the target project's own `herding.agents`. A label present in both resolves to the global one. | Purely additive: every installation that configured global presets keeps the exact meaning it has today, and only labels that refuse today can start resolving. Reversing the order would silently re-point an existing label at a different command. |
| D2 | The project-side argv comes from the target project's own `.bee/config.json`, read by the same resolver the board spawns through. The caller supplies a **label only** — never argv, env or cwd, from either source. | `orchestrator-dispatch` D3 unchanged and unweakened. The trust model is not new: this is the file the board already executes from on a human click. |
| D3 | A label found in neither source is still refused **before any herdr call**, with an error naming the label. The refusal message additionally names the project whose registry was searched. | The existing fail-closed ordering is load-bearing (`dispatch_refuses_an_unknown_preset_label_before_touching_herdr`); naming the project is what makes the refusal actionable now that two sources were searched. |
| D4 | An entry this resolver cannot turn into an argv — bee's object form `{argv, workspace_trust}` — is **not** resolvable, and its refusal says so in those terms rather than reporting the label as unknown. | The label exists; saying "unknown" would send a caller looking for a typo. `ask-state-fleet-read` D2 already publishes exactly this distinction through `resolvable`, and the two must agree. |
| D5 | No new permission, switch or config key. `terminal.enabled` plus the per-project `orchestration.enabled` remain the only gate, unchanged and still default-off. | `orchestrator-dispatch` D6. The project declaring an agent kind in its own file is not consent to dispatch; the opt-in is. |
| D6 | `ask_state`'s `herding.resolvable` and this resolver answer the same question and share one implementation. A label reported resolvable must dispatch, and one reported not resolvable must refuse per D4. | Two copies of "can this label start?" would drift, and the drift would show up as a tool that promises a label and then refuses it. |
| D7 | The run records the label it was given (`Run.preset_label`), whichever source resolved it. The run row does not record which source won. | `orchestrator-dispatch` D7's run state is about recovering the fleet, not about auditing config precedence; the label is what a caller passed and what it will pass again. |

### Agent's Discretion

The resolver's function name and where it sits in `bee.rs`; exact refusal wording within
D3/D4's constraints; whether the two-source lookup is one function or a small chain.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| global presets | `config.terminal.agent_presets` in `~/.waggledance/config.toml` — operator-authored, machine-wide, currently empty. |
| project registry | `herding.agents` in the target project's own `.bee/config.json` — what bee itself uses and what the board spawns from. |
| label | The name a caller passes as `preset`. The only thing a caller may pass. |
| resolvable | A label this resolver can turn into an argv today — the same predicate `ask_state` publishes. |

## Specific Ideas And References

- PBI `p-42cbde3a`, raised at the close of `ask-state-fleet-read`.
- Live evidence: beehive declares `claude-sonnet`, `pi-opencode-free`,
  `pi-agy-flash-3.7` (arrays, resolvable) and `agy-flash` (object form, not resolvable);
  `~/.waggledance/config.toml` has `agent_presets = []`.
- `docs/history/research/swarmforge-platoon-layer.md` — the addendum that named this gap
  as "the keystone", before the reuse half took priority.

## Existing Code Context

### Reusable Assets

- `crates/waggledance-core/src/bee.rs` — `herding_agent_argv` /
  `herding_agent_argv_from_config` (the argv resolver) and
  `herding_registry_from_config` (the labels reader `ask_state` publishes). D6's shared
  predicate belongs beside these two.
- `crates/waggledance/src/mcp.rs` (`handle_dispatch`) — the preset lookup is a single
  `match` block resolved before any herdr call; that ordering is D3's.
- `config::AgentPreset { label, argv }` — the shape `run_dispatch` already consumes, so
  a project-resolved preset needs no new type.

### Established Patterns

- Presets-only: every caller names a command, none supplies one.
- Fail closed before the socket: an unresolvable target never reaches herdr.
- The board reads the target project's own config rather than a machine-wide list —
  this feature brings the tool onto the same footing.

### Integration Points

- `crates/waggledance-core/src/bee.rs` — the by-label resolver.
- `crates/waggledance/src/mcp.rs` — the fallback in `handle_dispatch`.

## Canonical References

- `docs/history/orchestrator-dispatch/CONTEXT.md` — D3 (presets only, never argv), D6
  (per-project opt-in), D7 (durable run state).
- `docs/history/board-run-actions/CONTEXT.md` — D4 (the board spawns from
  `herding.agent_command` resolved through `herding.agents`).
- `docs/knowledge/work/ask-state-fleet-read/delivery.md` — D2's `resolvable` contract,
  which D6 binds this resolver to.

## Outstanding Questions

### Deferred To Planning

- [ ] Whether the by-label resolver replaces `herding_agent_argv`'s body (that function
      is the same lookup with the label fixed to `agent_command`) or sits beside it.
      Answered by reading how many callers `herding_agent_argv` has.

## Deferred Ideas

- Teaching the resolver bee's object form so `agy-flash` becomes spawnable — PBI
  `p-9212cae8`. Deliberately still separate: this feature makes the object form's
  refusal *honest*, and making it *work* is a different change.
- Filling in a `herding` block for `waggledance`, `jarvis`, `jarvis-mcp` and
  `memorypad`, which declare none.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
