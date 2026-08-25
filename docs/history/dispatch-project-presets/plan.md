---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Dispatch Resolves A Project's Own Agent Kinds

Mode: `standard` — 1 risk flag: covered-contract-change (public MCP contract, on the
process-spawn path).
Why this is the least workflow that protects the work: the code is one fallback plus a
resolver, but the label a caller passes decides which process starts, so the resolution
rule earns a written shape and a gate rather than a merged one-liner. It is not
`high-risk`: no validation is removed, no authorization changes, and the argv source is
the file the board already spawns from.

## Requirements (from CONTEXT.md)

- **D1** — global `terminal.agent_presets` first, then the target project's
  `herding.agents`; a label in both resolves to the global one.
- **D2** — the project-side argv comes from that project's own `.bee/config.json`; the
  caller supplies a label only, never argv/env/cwd.
- **D3** — a label in neither source is refused before any herdr call, naming the label
  and the project whose registry was searched.
- **D4** — bee's object form (`{argv, workspace_trust}`) is not resolvable and refuses
  in those terms, never as "unknown".
- **D5** — no new permission, switch or config key; `terminal.enabled` +
  `orchestration.enabled` stay the only gate.
- **D6** — `ask_state`'s `herding.resolvable` and this resolver share one
  implementation of "can this label start?".
- **D7** — the run records the label it was given, not which source won.

## Discovery

- `handle_dispatch`'s preset lookup is a single `match` over
  `engine.config.terminal.agent_presets`, resolved before the `orchestration_handle`
  call — so D3's fail-closed ordering is already structural and needs preserving, not
  building (`crates/waggledance/src/mcp.rs`, the `let preset = match preset_label` block).
- `herding_agent_argv` has exactly **two** production callers, both the board's run
  actions (`server.rs:2280`, `server.rs:2307`), plus five test call sites. That answers
  CONTEXT.md's deferred-to-planning question: a by-label resolver can sit underneath it
  rather than replacing it, and `herding_agent_argv` becomes "resolve the label
  `agent_command` names" — no caller changes. Evidence:
  `rg -n 'herding_agent_argv\(' crates/`.
- `herding_registry_from_config` already computes resolvability per label, inline, with
  `matches!(entry, Value::Array(tokens) if argv_tokens(tokens).is_some())`. D6 is
  satisfied by making that expression call the new resolver instead of restating it.

## Approach

**Recommended path.** Add one pure by-label resolver in `bee.rs` beside the existing
pair, make both existing readers call it, and give `handle_dispatch` a fallback that
uses it when the global list misses (D1, D2, D6). Refusals are shaped by D3/D4 at the
call site, which is the only place that knows the project.

*Rejected alternatives.*
- Replace the global preset list with the project registry — breaks every installation
  that configured global presets, and D1 exists to avoid exactly that.
- Copy the board's resolution into `mcp.rs` — a second copy of "can this label start?"
  that D6 forbids, and the copy would drift from what `ask_state` publishes.
- Let the caller pass argv when no label matches — directly overturns
  `orchestrator-dispatch` D3; not on the table.

*Risk map.*

| Component | Risk | Proof needed |
|---|---|---|
| The resolver (`bee.rs`) | LOW — pure, no I/O beyond one already-read config | Unit cases per entry shape, including the object form |
| The fallback (`mcp.rs`) | MEDIUM — decides which process starts | Order test (global wins a collision), refusal tests before herdr is touched |
| Refusal wording (D3/D4) | LOW — but it is the whole actionability of the feature | Assert both messages name what a caller must fix |
| Existing board callers | LOW — same lookup, one indirection deeper | The existing `herding_agent_argv` suite must stay green unchanged |

## Shape

One slice; the feature is a single capability with no milestone to stage.

| Epic | Capability | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| Label resolution | A dispatch caller can name any agent kind the target project declares | The board and the tool disagree on the same machine; only a human click can spawn today | 1 (this one) | Both refusal paths proved before any herdr call; the global-wins order pinned |

**Current slice — two cells, sequential (both touch `bee.rs`, then `mcp.rs`):**

1. **The shared resolver.** `herding_argv_for_label(config, label)` in `bee.rs`;
   `herding_agent_argv_from_config` and `herding_registry_from_config`'s resolvability
   test both call it (D6). No behaviour change anywhere — the existing suite is the
   proof that this is a pure refactor.
2. **The dispatch fallback.** `handle_dispatch` tries the global list, then the target
   project's registry, with D3's and D4's refusals. Still before any herdr call.

## Test matrix

*Happy path.* A label only the project declares (`pi-agy-flash-3.7` shape) resolves and
reaches `run_dispatch` with that project's argv.

*Edge cases.* A label present in BOTH sources resolves to the global one (D1 — the
collision is the decision, so it is pinned, not assumed). A project with no `herding`
block behaves exactly as today. The existing `herding_agent_argv` cases stay green
unchanged, which is what proves cell 1 changed no behaviour.

*Error paths.* A label in neither source refuses, names the label and the project, and
**touches no herdr call** — extending the existing
`dispatch_refuses_an_unknown_preset_label_before_touching_herdr` rather than replacing
it. A label whose entry is bee's object form refuses in D4's terms, and the message is
asserted not to say "unknown", because that is the wrong instruction to a caller.
`ask_state`'s `resolvable` and the dispatch refusal are asserted to agree on the same
config (D6) — the one test that would catch the two implementations drifting apart.

## Out of scope

- Making the object form spawnable (PBI `p-9212cae8`) — this feature makes its refusal
  honest, not unnecessary.
- Filling in a `herding` block for the four projects that declare none.
- Any change to what happens after a preset resolves: preflight, marker, baseline,
  wait semantics and run state are untouched.
