# The Write Half Of The herdr Protocol Is Four Versions Behind — Context

**Feature slug:** herdr-protocol-20
**Date:** 2026-08-25
**Shaping session:** complete
**Scope:** Standard
**Domain types:** CALL | RUN

## Feature Boundary

waggledance's herdr **write** calls are ported from protocol 16 to the protocol 20 the
installed herdr actually speaks, so starting an agent works again — for MCP dispatch and
for the board's own Start / Run review / Run compound. It ends at the wire: no new
capability, no relaxed check, and the read calls are not touched.

## What Was Found

Evidence is first-party — `herdr api schema --json` from the installed binary
(`herdr 0.8.0`, `protocol: 20`, `schema_version: 1`), not guesswork.

- `crates/waggledance/src/herdr/wire.rs` pins `HERDR_PROTOCOL = 16`.
- waggledance calls seven methods: `session.snapshot`, `pane.read`, `pane.send_input`,
  `pane.send_keys`, `pane.send_text`, `tab.create`, `agent.start`. All seven still
  exist. Five are shape-compatible; **two drifted**, and both are on the spawn path:

| Call | waggledance sends / expects | protocol 20 |
|---|---|---|
| `agent.start` params | `{name, argv, workspace_id, cwd, focus}` | **requires `{name, kind, pane_id}`**, optional `args`, `timeout_ms` |
| `tab.create` response | `{tab:{tab_id}, root_pane:{pane_id}}` | `{type, tab}` only — `TabInfo` is `{tab_id, workspace_id, number, label, focused, pane_count, agent_status}`, **no `root_pane`, no pane id at all** |

So the new `agent.start` cannot create a pane; it starts an agent **into an existing
one**. And `tab.create` no longer tells you which pane it just made.

The live failure, after `spawn-destination-fallback` cleared the destination problem:
`agent start failed: herdr refused the request (invalid_request): missing field 'kind'`.

**Why nobody noticed.** Every read path is unaffected — the snapshot, the pane list,
`ask_state`, the board, the terminal view all work. Only the two write calls that create
things are dead, and the one production caller of `tab.create` (`server.rs`'s open-a-shell
route) parses `root_pane`, so it is broken too.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | `agent.start` sends `{name, kind, pane_id}` plus `args`. **`kind` is argv[0] and `args` is argv[1..]** — the exact split bee itself uses (`bee herding wave`: "splits token 0 into the herdr agent kind and the remaining tokens into the agent's own argv"). | The split is not invented here; it is the convention the argv in `herding.agents` was already written for, which is why those entries lead with `claude` / `pi` / `agy`. |
| D2 | Spawning becomes two calls: `tab.create` for the pane, then `agent.start` into it. The pane id comes from a `session.snapshot` lookup for the pane whose `tab_id` matches the created tab — because protocol 20's `tab_created` carries no pane id and `pane.list` filters only by workspace. | |
| D3 | A `tab.create` that yields a tab but no resolvable pane is a **typed failure**, never a fallback to some other pane. The half-made tab is reported in the error so a human can see it. | Silently adopting a neighbouring pane would start an agent in a pane someone else is using — the precise thing `preflight` exists to prevent. |
| D4 | `HERDR_PROTOCOL` becomes 20. The bump is justified by the audit above — all seven methods checked against the schema — and by nothing else; it is not a blind bump to silence a mismatch. | |
| D5 | Read calls (`session.snapshot`, `pane.read`) and the three send calls are **not touched**. Their params validate against protocol 20 as they stand, and they demonstrably work against the live daemon. | A protocol port is where unrelated "while I'm here" edits do the most damage. |
| D6 | No check is relaxed. `preflight`'s fail-closed status rule, boundary containment, the per-project `orchestration.enabled` opt-in and presets-only all stand exactly as they are. | |
| D7 | `FakeHerdr` moves with the real client, so the protocol it fakes is the protocol that ships. A test double that keeps speaking 16 would turn every green test into a lie. | |

### Agent's Discretion

The agent `name` generated for a spawn; whether the tab→pane lookup lives in
`socket.rs` or beside `resolve_spawn_destination`; error variant names; whether
`timeout_ms` is sent at all.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| kind | herdr's name for the agent program (`claude`, `pi`, `codex`, `agy`) — argv[0]. |
| args | The rest of the command line, argv[1..]. |
| write call | A herdr method that creates or changes something: `tab.create`, `agent.start`, the sends. |

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/herdr/socket.rs` — `agent_start_params`, `tab_create_params`,
  and the two response parsers. The whole change lives here plus the flow in
  `orchestrate.rs`.
- `crates/waggledance/src/herdr/fake.rs` — the double every protocol test runs against.
- `crates/waggledance/src/orchestrate.rs` — `run_dispatch`'s `Spawn` branch, which today
  makes one `agent_start` call and must become tab-then-start.
- `.bee/config.json` `herding.agents` in beehive — real argv to test the split against:
  `["claude","--model","sonnet",...]`, `["pi","-a","--model",...]`.

### Integration Points

- `crates/waggledance/src/server.rs:4325` — the only production `tab_create` caller
  (open a shell in a project); it parses the removed `root_pane` and must follow D2.

## Canonical References

- `herdr api schema --json` from the installed binary — the wire contract.
- `docs/knowledge/work/spawn-destination-fallback/delivery.md` — the open gap this closes.
- `docs/history/orchestrator-dispatch/CONTEXT.md` — D3/D5/D6, unchanged by this work.

## Outstanding Questions

### Deferred To Planning

- [ ] Whether `tab.create` should pass `label` (protocol 20 accepts one) so a spawned
      pane is identifiable in the herdr UI. Answered by looking at what the board's
      existing shell-create shows today.
- [ ] Whether the snapshot lookup after `tab.create` needs a retry: herdr may not list
      the new pane in the very next snapshot. Answered by trying it against the live
      daemon.

## Deferred Ideas

- `agent.wait` / `agent.prompt` / `events.subscribe` exist in protocol 20 and could
  replace the marker-and-poll completion protocol entirely. Out of scope: that would
  supersede `orchestrator-dispatch` D5, which is the user's call, not a port's.
- Auditing the *read* responses field by field against protocol 20. They work; a
  drift there would be a separate, evidenced change.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
