# Leader Delivery — Context

**Feature slug:** leader-delivery
**Date:** 2026-10-14
**Shaping session:** complete (owner approved all four piggery lessons: «Làm hết»)
**Scope:** Standard
**Domain types:** CALL | RUN | SEE

## Feature Boundary

A request for a project's Paseo leader is accepted once, held until the
leader is free, delivered in order, marked done when the Paseo turn that
carried it ends, and reported back to the caller without being asked. It
builds on `paseo-leader-dispatch` (merged at d526bc0) and changes only the
leader path; herdr dispatch keeps its behavior except that it also honors a
`request_id`.

Origin: the piggery v0.10.2 study (this session), recommendations 1–4.

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | `waggledance_dispatch` accepts a caller-chosen `request_id`; a repeat call with the same project and `request_id` returns the original run and sends nothing. | A re-send after a timeout must be safe (piggery `client_msg_id`). |
| D2 | A leader run is done when the Paseo turn that carried its task ends, as Paseo reports it — never by reading idle state or log text. Every Paseo CLI behaviour the code relies on has a recorded capture under `testdata/fixtures/paseo-<version>/`. | Piggery acks only on the matching turn end and keeps per-version fixtures. |
| D3 | A request to a busy leader is accepted and held in a per-leader queue, delivered in order when the leader is free; never refused for being busy. Leader creation stays serialized per project. Supersedes `paseo-leader-dispatch` D7. | Piggery holds mail for a busy recipient. |
| D4 | When a leader run ends (done, blocked, lost), waggledance tells the caller unasked: through the existing notify path, and directly to the calling Paseo agent when the dispatch came from one. | Piggery wakes the lead with the reply instead of making it poll. |

Inherited unchanged: `paseo-leader-dispatch` D1–D6, D8–D10; `orchestrator-dispatch` D1 (waggledance never chooses what to dispatch).

### Agent's Discretion

Where the queue and the delivery loop live, the run status vocabulary for a
held request, how a daemon restart treats a turn that was in flight, and the
shape of the notice sent to a calling Paseo agent.

## Findings From The Reality Probe (2026-10-14, Paseo 0.11.2)

Captured in `.bee/cache/paseo-probe/` and to be committed as fixtures:

- `paseo send <id> --prompt-file f --json` without `--no-wait` blocks until the turn ends and prints `{"agentId", "status": "completed", "message": "Agent completed processing the message"}` — the turn-tied completion D2 needs.
- `send --no-wait` prints `{"status": "sent"}`; two seconds later `inspect` still read `"Status": "idle"` — idle state lags a fresh send.
- `paseo wait <id> --json` prints `{"agentId", "status": "idle", "message": "Agent is idle.\nLast 5 activity items:\n…"}`.
- Requests sent one after another were processed in order (`logs`: PONG, SECOND, DONE-A, THIRD-B).
- `~/.local/bin/paseo` is a wrapper that runs the Paseo AppImage. Called from inside a Paseo agent, its `run` exits 0, prints only startup logs (including multi-line `[login-shell-env]` objects that the current noise filter does not drop), and creates no agent. The bundled CLI named by `$PASEO_CLI` works and prints clean JSON.
- Called from inside a Paseo agent (`PASEO_AGENT_ID` set), `paseo run --cwd <dir>` without `--workspace` attached the new agent to the caller's workspace and ignored `--cwd`.

## Existing Code Context

- `crates/waggledance/src/paseo.rs` — Paseo client, `resolve_leader`, noise filter.
- `crates/waggledance/src/mcp.rs` — leader branch of `waggledance_dispatch`.
- `crates/waggledance/src/orchestrate.rs` — `observe_paseo_run`, `await_paseo_run` (the log-text heuristic D2 retires).
- `crates/waggledance/src/reaper.rs` — Paseo sweep; `crates/waggledance/src/main.rs` — reaper wiring.
- `crates/waggledance-core/src/repository.rs` — runs table (schema version 4).
- `integrations/paseo/` — panel composer; `docs/waggledance-supervisor-skill-template.md`.

## Outstanding Questions

### Resolved In Planning

- [ ] Whether a blocking `paseo send` returns `completed` when a human is also typing to the same leader — planning decides how the delivery loop tolerates it.
