# Paseo Leader Dispatch — Context

**Feature slug:** paseo-leader-dispatch
**Date:** 2026-10-14
**Shaping session:** complete (four questions answered by the owner)
**Scope:** Standard
**Domain types:** CALL | RUN | SEE

## Feature Boundary

When waggledance runs alongside Paseo, a request for a project reaches that
project's one long-lived leader — a Paseo agent in the project's configured
Paseo workspace — through the existing dispatch door, and the run is durable
and visible like a herdr run. Requests come from the waggledance agent chatting
in Paseo, from a composer in the Waggle Dance Paseo plugin, and from external
agents over waggledance's MCP server. It ends there: waggledance never chooses
the project or rewrites the task, and adds no stop/archive control over Paseo
agents.

Original ask (verbatim): «Tôi đang chạy waggledance trong paseo tôi muốn thêm
config và thêm tính năng hỗ trợ paseo hoàn chỉnh để coa thể nhận yêu cầu rồi
giao cho đúng leader đúng workspace»

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | waggledance carries paseo code again: a Paseo backend for `waggledance_dispatch` inside the daemon that drives the `paseo` CLI. Its runs live in the same durable runs table and are read through the same `await` / `runs` tools as herdr runs. This supersedes `paseo-removal` D1 for the dispatch surface only; the removed `/paseo/*` view routes stay gone (decision 86365acd). | The owner chose the daemon over a skill-only path so runs stay durable and on the Runs board. |
| D2 | Each project has one long-lived leader: a Paseo agent labelled as that project's leader in its target workspace. A request goes to the live leader with `paseo send`; with no leader, or a closed/archived one, a new leader is started with `paseo run` and labelled (decision a675822a). | A leader keeps the project's context across requests. |
| D3 | The target Paseo workspace for a project is named in waggledance config. With none named, the `local_checkout` workspace whose cwd is the project root is used; with neither, a new workspace is created at the project root (decision 52ebd832). | Some projects already have two Paseo workspaces; a guess would be ambiguous. |
| D4 | Three sources, one door: the waggledance agent chatting in Paseo (supervisor skill), a request composer in the Waggle Dance Paseo plugin, and external agents (aicoworker, openclaw, hermes) calling waggledance's MCP server. The caller names the project; waggledance never picks it or edits the task text (decision b2e63d04). | `orchestrator-dispatch` D1 holds unchanged. |
| D5 | The existing gate holds: a Paseo dispatch is refused unless the target project has orchestration enabled. A caller passes a project and a task only — never argv, cwd, provider strings or workspace ids; those come from config. The plugin stays read-only except for sending a request to a project's leader, its first write path (narrows `pi-paseo-plugin` D3). | `orchestrator-dispatch` D3/D6 security posture. |

### Agent's Discretion

- Config shape and key names (global Paseo defaults plus per-project leader
  overrides: workspace, provider/model, mode, thinking), how a caller selects
  the Paseo backend versus herdr, the leader label key, and how run status is
  derived from Paseo agent state (idle / running / needs permission / error).
- How the plugin composer reaches the daemon — constrained by D5 and by the
  HTTP API being unauthenticated.
- How the supervisor skill learns to use the Paseo backend.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| leader | The one Paseo agent per project that receives that project's requests and runs its own bee chain. |
| target workspace | The Paseo workspace a project's leader lives in, resolved per D3. |
| Paseo backend | The dispatch path that drives the `paseo` CLI instead of herdr panes. |

## Specific Ideas And References

- `paseo run --workspace <id> --cwd <path> --provider <p/m> --mode <m> --thinking <id> --label k=v --title <t> -d --json`
- `paseo send <id> --prompt-file <path> --no-wait --json`; `paseo ls -g --label k=v --json`; `paseo inspect <id> --json`; `paseo wait <id> --timeout <s>`; `paseo workspace …`.
- Paseo 0.11.2 on this host; this session itself runs as a Paseo agent.

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/mcp.rs` — the three dispatch tools.
- `integrations/paseo/` — the read-only plugin (board surface, workspace panel, settings).
- `waggledance-supervisor` skill — the seat that opens a lead through dispatch.

### Integration Points

- Dispatch engine and runs table (planning reads them).
- `crates/waggledance-core/src/config.rs` — new Paseo config.
- `integrations/paseo/` — composer + RPC.

## Outstanding Questions

### Deferred To Planning

- [ ] How a Paseo run's `done` / `blocked` maps from Paseo agent status and attention reason.
- [ ] Whether the plugin composer calls a new `waggledance` CLI subcommand or the MCP server.

## Deferred Ideas

- Stopping or archiving a leader from waggledance.
- Images attached to a request.
