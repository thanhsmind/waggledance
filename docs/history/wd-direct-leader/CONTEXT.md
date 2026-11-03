# wd Direct Leader — Context

**Feature slug:** wd-direct-leader
**Date:** 2026-10-22
**Shaping session:** complete
**Scope:** Deep
**Domain types:** CALL | RUN | READ

## Feature Boundary

wd (the aicoworker agent on OpenClaw) hands a task straight to a project's leader
through waggledance's daemon dispatch, and the leader's results, questions,
permission requests and the heartbeat's nudges and board come back to wd through a
durable waggledance inbox that wakes wd's main session. The Paseo supervisor agent
is retired. The feature ends at wd's main session; what wd writes on Telegram stays
wd's own turn.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | wd is the orchestrator. It hands a task with `waggledance_dispatch` `{project, task, leader: true, request_id: <corr>}` through `wd-bridge.py`. The daemon finds the project's leader or creates it, queues per project and delivers only while the leader is idle, as the paseo-leader-dispatch path already does. The freeze on that path (paseo-native-supervisor D8) is lifted. | User 2026-10-22: the wd → supervisor → wd hop is a detour. |
| D2 | Leaders the daemon opens for wd carry three labels: `waggledance.role=leader`, `waggledance.project=<project id>`, `waggledance.via=wd`. A wd task reuses only a `via=wd` leader; a leader without that label (opened by the Paseo plugin or another MCP caller) is never given a wd task. | Keeps wd's leaders apart from other callers' leaders, as `via=supervisor` did. |
| D3 | The return path is a durable inbox in waggledance's existing SQLite database (not DuckDB). Each message has a recipient, the task's `corr`, the project, a kind, a body, a created time and an `acked_at`. Messages survive a daemon restart. | User 2026-10-22: inbox like piggery's. SQLite: many processes write (daemon and leaders' CLI), small row writes, the database already exists. |
| D4 | A leader writes to the inbox with a new CLI, `waggledance mail send --to wd --corr <corr> --kind <kind>`, body from a file or stdin. A `result` message must carry the done report (what was done, the proof command and outcome, what is left, where it lives); one without it is refused and the leader is told why. | Carries decision c505ac8d (done means a detailed mail) to the new path. |
| D5 | The daemon wakes wd for each unacked inbox message by running `openclaw agent --agent <agent_id> --message <framed message> --deliver --channel telegram --json` into wd's main session, the command `waggledance supervisor report` runs today. The frame is `[waggledance#<corr>] <project> — <kind label>: <body>` with the Vietnamese kind label. wd is never asked to poll. | Piggery's rule: mail wakes you, do not poll. Keeps decision 15cc4a55: wd reads the report with the task's context and writes the Telegram message itself. |
| D6 | A message is acked only when the `openclaw agent` turn that carried it ends successfully. A failed or timed-out turn leaves it unacked and it is delivered again on the next wake, in `seq` order. Reading never acks. | Piggery: ack at the end of the turn, never because a participant looks idle. |
| D7 | Before mailing wd a `question`, the leader puts it to the bee advisor of its own project (`bee dispatch prepare --kind advisor`). Only a question the advisor cannot decide is mailed, and the mail quotes the advisor's take. wd runs no advisor step of its own for leader questions. The rule lives in the leader's initial prompt. | Decision logged 2026-10-22 (touches paseo-native-supervisor D4). |
| D8 | The daemon runs a heartbeat every 10 minutes over its open wd tasks: a leader that is idle with no `result` message since its last delivery is nudged once with a `paseo send` asking for the done report; still silent on the next beat, the daemon mails wd one `progress` message; when any task changed state since the last board, it mails wd one `board` message listing the open tasks. The rules are code, not a model turn. | User 2026-10-22: heartbeat is important; supervisor-active-loop rules moved into the daemon. |
| D9 | The daemon watches permission requests with `paseo permit ls --json` about every 30 seconds. A new request (by `req_id`) from a `via=wd` leader becomes one `permission` inbox message naming the project, the corr, the tool or command and the `req_id`. wd answers with `wd-bridge.py permit allow|deny <corr> [req_id]`, and the daemon runs `paseo permit allow|deny <leader> <req_id>`. The daemon never allows or denies on its own. A request that disappears unanswered is reported to wd as closed. | User 2026-10-22: permission requests are important; the user decides every one through wd. |
| D10 | wd's other commands keep their shapes from paseo-native-supervisor D12 but go to the daemon instead of the supervisor: `reply <corr> <text>` and `redo <corr> <text>` are sent to that task's leader; `accept <corr>` closes the task and archives its leader. A leader is archived only after accept (carries paseo-native-supervisor D9). | — |
| D11 | The Paseo supervisor is retired: `waggledance supervisor ensure`, `supervisor report` and the other supervisor subcommands are removed, `docs/waggledance-supervisor-agent.md` and the `waggledance-supervisor` skill's supervisor hand-off go, and the OpenClaw `waggledance-orchestrator` skill and `wd-bridge.py` are rewritten for D1, D9 and D10. A running supervisor agent is archived once, by hand or by `waggledance setup openclaw`. | User agreed 2026-10-22 to the scope list that names dropping the supervisor. |
| D12 | The daemon's existing callers (Paseo plugin request box, MCP callers that are not wd) keep their current behavior and notices. | paseo-leader-dispatch D4 holds. |

### Agent's Discretion

- Table and column names, the CLI flag spellings beyond those named above, and how
  the 10-minute and 30-second timers are scheduled inside the daemon.
- Whether `board` and `progress` are composed from the same open-task query.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| wd | The aicoworker agent on OpenClaw whose main session talks to the user on Telegram. |
| inbox | The SQLite table of messages addressed to wd, each acked only at a successful wake turn's end. |
| wake | One `openclaw agent` turn the daemon starts in wd's main session to carry inbox messages. |
| corr | The task id `t-<6 hex>` minted at hand-off; it is the dispatch `request_id`. |

## Specific Ideas And References

- piggery (`/mnt/data/Projects/refs/slp/piggery`): messages table with `acked_at`,
  delivery batches acked at turn end (`internal/core/mail.go:400-475`), wake not poll
  (`internal/cli/skills.md:29`).

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/paseo.rs:953-1037` — `resolve_leader` and `pick_workspace`: find or create a leader, pick the workspace.
- `crates/waggledance/src/delivery.rs:289-336` — the per-project leader queue and turn-end wait.
- `crates/waggledance/src/seat.rs` — supervisor ensure (to remove) and the `openclaw agent` report call (to reuse for the wake).
- `crates/waggledance/src/mcp.rs:844-985` — `ask_state.leaders`.
- `crates/waggledance-core` — `rusqlite` database and repository.

### Integration Points

- `integrations/openclaw/waggledance-orchestrator/SKILL.md` and `bin/wd-bridge.py`.
- `docs/knowledge/patterns/paseo-native-supervisor.md`, `docs/knowledge/patterns/paseo-leader-dispatch.md`.

## Canonical References

- `docs/history/paseo-native-supervisor/CONTEXT.md` — D2, D3, D4, D5, D7, D8 superseded or touched here.
- `docs/history/supervisor-active-loop/plan.md` — the heartbeat and board rules D8 moves into the daemon.

## Outstanding Questions

### Deferred To Planning

- [ ] How `delivery.rs` notices map to inbox messages for wd tasks (turn end without a mail, error, lost) — read the notice kinds in `delivery.rs`.
- [ ] Whether `paseo permit ls --json` rows carry the agent id and a stable `req_id` — run it while a leader waits on a permission.
- [ ] Where the wake loop runs so two wakes for wd never overlap — one serial worker in the daemon.

## Deferred Ideas

- DuckDB for history analytics over the SQLite file — not needed for the inbox.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable.
