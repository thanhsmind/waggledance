# Paseo-Native Supervisor — Context

**Feature slug:** paseo-native-supervisor
**Date:** 2026-10-17
**Shaping session:** complete
**Scope:** Standard
**Domain types:** CALL | RUN | READ

## Feature Boundary

A long-lived supervisor Paseo agent takes a task from aicoworker (an OpenClaw gateway outside Paseo), picks the project, hands the task to that project's leader as a Paseo subagent, watches the leader through Paseo's own notifications, routes a leader's question to an advisor or back to the user on Telegram, and archives the leader once the user accepts. waggledance becomes the supervisor's map (projects, `ask_state` with a `leaders` field, `supervisor ensure`); the daemon's own leader queue and caller notices are frozen, not removed. The feature ends at the three artifacts: the supervisor agent definition, the two waggledance additions, and the OpenClaw skill and hook wiring on this host.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never
a silent edit.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Agent-to-agent communication inside Paseo is Paseo native: `create_agent` subagents, `send_agent_prompt` with `notifyOnFinish`, `archive_agent`, labels. waggledance is the supervisor's read layer (projects, `ask_state`, `runs`), never the pipe between agents. | Paseo already carries turn end, permission and error to a parent agent; a daemon queue and notice relay duplicate that. Touches leader-delivery D2/D3/D4 without retiring them (see D8). |
| D2 | One long-lived supervisor Paseo agent for every project, on Claude Haiku, labelled `waggledance.role=supervisor`; callers find it by label and create it when absent. | Choosing the project is the step that needs a cross-project seat. |
| D3 | A leader is a Paseo subagent the supervisor creates from inside itself with Paseo MCP `create_agent` (never `paseo run` from the CLI), placed in the project's workspace via `workspaceId`, labelled `waggledance.role=leader` and `waggledance.project=<project id>`. One leader per project by default; "clear session" is `archive_agent` then `create_agent`. The leader runs Claude Code in the project; bee inside it keeps its own workers. | Only an MCP-created agent is a Paseo child, which is what makes `notifyOnFinish` return to the supervisor. Placement never changes parentage. |
| D4 | An advisor is a Paseo subagent of the supervisor with no `workspaceId`, created when a leader asks a question and archived after it answers. If it can decide, the supervisor forwards the answer to the leader; otherwise the supervisor escalates to the originating agent. | The advisor judges the question, not the code, so it needs no repo cwd. |
| D5 | Inbound: aicoworker hands a task to the supervisor with `paseo send <supervisor id> --no-wait` through its exec tool, after find-or-create by label. The OpenClaw skill `waggledance-orchestrator` gains `handoff` (find-or-create, then send) and `answer` (relay the user's reply). | aicoworker is not a Paseo agent, so Paseo notifications cannot reach it; the `paseo` CLI is on its PATH. |
| D6 | Outbound: the supervisor reports by `POST /hooks/agent` on the OpenClaw gateway with `deliver: true`, `channel: telegram`; a result or a leader's question reaches the user on Telegram carrying the project name and a one-line context. The hook token lives in a config file or environment the supervisor reads, never in its prompt. OpenClaw `hooks` must be enabled with a token. | The only inbound door OpenClaw offers. Rejected: aicoworker rephrasing the question first, which adds a model turn that drifts it. |
| D7 | Classifying a leader's turn (result, question, blocked) and judging whether a leader is really free is the supervisor's judgment from the leader's last reply, Paseo status and `ask_state` `waiting_on`. waggledance never classifies; it exposes the raw fields. | orchestrator-dispatch D1 and ask-state-fleet-read D8 hold. |
| D8 | `waggledance_ask_state` gains a `leaders` field per project (Paseo agent id, status, labels, `waiting_on`, last run) read from `paseo ls` and the runs table; the CLI gains `waggledance supervisor ensure`, printing the supervisor's agent id and creating it when absent. The daemon's leader queue, per-project lock and caller notices stay as they are, frozen: working for existing callers, extended by nothing here. | One read for the supervisor instead of label grepping; the Paseo plugin and existing MCP callers keep working. |
| D9 | The supervisor archives a leader only after the originating agent relays the user's acceptance; a finished-but-unaccepted leader stays idle with its context. | User: "Leader xong hết người dùng ok thì close agent leader đó luôn". |
| D10 | A new task for a project whose leader is idle with no open question is taken by archiving that leader and creating a fresh one, whether or not the user has said ok to its last result (the result was already reported). A project whose leader is waiting on a reply queues the new task in the supervisor's ledger and the user is told it is queued. Supervisor-created leaders carry a third label `waggledance.via=supervisor`, and the supervisor reuses only those. | User's rule: a free leader clears its session and takes new work; acceptance only decides when a leader with nothing more to do is closed. The via label keeps the frozen daemon path and the supervisor from sharing one leader (hat wave, 2026-10-17). Added after Gate 1 as a new D-ID. |
| D11 | OpenClaw's `/hooks/agent` runs an isolated model turn, so a report is an instruction to that turn: "forward the following to the user verbatim, in Vietnamese, do not rephrase", tagged `[waggledance#<corr>] <project>`, with the hook's `model`/`thinking` overrides set low. The relay still passes through a model; that is the accepted limit, checked in pns-6 by quoting the Telegram text against the `report` input. The feature boundary reads "the waggledance additions" — `ask_state.leaders`, `supervisor ensure`, `supervisor report` and `setup openclaw` — not "two". | Hat wave: the webhook doc says `message` is "The prompt or message for the agent to process". Added after Gate 1 as a new D-ID; touches D6 without retiring it. |
| D12 | Message protocol. aicoworker → supervisor: `[handoff <corr>] <task>`, `[reply <corr>] <text>`, `[accept <corr>]`, `[redo <corr>] <text>`; `handoff` mints `<corr>` as `t-<6 hex>`. Supervisor → user (via `report --kind`): `result`, `question`, `blocked`, `error`, `closed`, `progress`, each carrying the tag and project. A bare "ok" from the user resolves to the newest open item; the OpenClaw skill tells aicoworker so. | Hat wave: without a tag, "ok" with two projects live is ambiguous, and an answer cannot find its leader. Added after Gate 1 as a new D-ID. |
| D13 | Operational rules. The hook token lives in `<data dir>/hook.token`, owner-only, written by `setup openclaw` the way `telegram.token` is; it is never a `Config` field. "Clear session" is graceful: the supervisor first asks the leader to write a pause handoff and release its session, waits for the finish notification, then archives; it archives outright only when the leader does not answer within the supervisor's wait. The advisor carries `waggledance.role=advisor`. The supervisor's `ledger.md` is append-only and a hint; `ask_state.leaders` and `paseo ls` are the truth re-read at every turn start. The supervisor keeps one Paseo heartbeat (every 30 minutes) that reports `progress` for any leader running longer than an hour and retries any report the hook refused. | Hat wave risk findings. Added after Gate 1 as a new D-ID. |

### Agent's Discretion

Exact JSON shape inside `leaders`; where the hook token file lives under `~/.waggledance/` (or an env var name); the supervisor's prompt wording and the exact label for the advisor; whether `supervisor ensure` is a subcommand of `waggledance setup` or a new verb; how the OpenClaw skill locates the supervisor id (label query via `paseo ls -g --json`).

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| supervisor | The one Paseo agent labelled `waggledance.role=supervisor`, on Haiku, that routes tasks to leaders and reports to aicoworker. |
| leader | A Paseo subagent of the supervisor running Claude Code in one project's workspace, labelled per D3. |
| advisor | A short-lived Paseo subagent of the supervisor that answers a leader's question (D4). |
| originating agent | The agent that handed the task in; here aicoworker, an OpenClaw gateway whose user channel is Telegram. |
| frozen | Code kept working and tested but receiving no new behaviour in this feature (D8). |
| hook | OpenClaw's `POST /hooks/agent` webhook on `127.0.0.1:18789`, bearer-token protected. |

## Specific Ideas And References

- The user's model, verbatim intent: a small-model supervisor sees which project a task belongs to, checks whether a leader is really free (not just its status), clears its session before new work, asks an advisor when the leader asks, falls back to aicoworker to ask the user, and closes the leader when the user is satisfied.
- aicoworker today: OpenClaw gateway in `~/Applications/AICoworker.AppImage`, agent `main` on Gemini flash, Telegram channel, skill `~/aicoworker/openclaw/workspace-main/skills/waggledance-orchestrator/bin/wd-bridge.py` spawning `waggledance mcp` per call (outbound only). `hooks` block in `~/aicoworker/openclaw/openclaw.json` has only `internal.session-memory`; webhook not enabled. Gateway listens on `127.0.0.1:18789`.
- `~/.agents/skills/waggledance-supervisor/` is the deployed copy aicoworker reads of `docs/waggledance-supervisor-skill-template.md`.

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/paseo.rs` — `candidates`, `detect`, `paseo_available`, `resolve_leader` (label lookup over `paseo ls -g --json`), env scrub, fixtures under `testdata/fixtures/paseo-0.11.2/`.
- `crates/waggledance/src/mcp.rs:537-640` — `handle_ask_state`, `ask_state_digest`; the `panes` field is the shape `leaders` follows (degrade with a reason, never a tool error — ask-state-fleet-read D5).
- `crates/waggledance-core/src/bee.rs:222-265` — `waiting_on_live` and `waiting_on` already read from `state.json`.
- `crates/waggledance-core/src/repository.rs` — runs table with `caller`, `request_id`, `pending_notices`; the "last run" per leader comes from here.
- `crates/waggledance/src/setup.rs` — `waggledance setup paseo`, the pattern for a CLI verb that talks to Paseo.
- `docs/waggledance-supervisor-skill-template.md` — the current one-shot supervisor skill; its leader mode section is what the new agent definition replaces.

### Established Patterns

- Degrade with a flag, never fail the call (`waggledance_search` `refresh`, `ask_state` `panes: null` + reason).
- Every Paseo behaviour relied on has a captured fixture (`docs/knowledge/patterns/existence-is-not-evidence.md`).
- Secrets never in prompts or repo: hook-guarded secret-shaped files; the Telegram bot token already sits in `openclaw.json` outside this repo.

### Integration Points

- `crates/waggledance/src/mcp.rs` — `ask_state_digest` gains `leaders`.
- `crates/waggledance/src/main.rs` + `setup.rs` — `supervisor ensure` verb.
- `docs/waggledance-supervisor-skill-template.md` — rewritten for the Paseo-native loop, or a sibling `docs/waggledance-supervisor-agent.md` holding the supervisor's agent prompt; `~/.agents/skills/waggledance-supervisor/` refreshed from it.
- `~/aicoworker/openclaw/workspace-main/skills/waggledance-orchestrator/` — `handoff` and `answer` commands; `~/aicoworker/openclaw/openclaw.json` — `hooks.enabled`, `hooks.token`. Both are host files outside this repo; the repo holds their canonical text under `docs/`.
- `docs/knowledge/patterns/paseo-leader-dispatch.md` — gains the frozen note and a pointer to the supervisor concept.

## Canonical References

- `docs/history/leader-delivery/CONTEXT.md` — D2/D3/D4/D5, the daemon path this feature freezes.
- `docs/history/ask-state-fleet-read/CONTEXT.md` — D5 (degrade), D8 (policy in the caller).
- `docs/history/orchestrator-dispatch/CONTEXT.md` — D1, waggledance never decides.
- `~/.claude/skills/paseo/SKILL.md` — `create_agent`, `send_agent_prompt`, `archive_agent`, heartbeats; agent-scoped creation is always a subagent.
- `~/aicoworker/app/gateway-overlay/openclaw/docs/automation/webhook.md` — `/hooks/agent` payload and auth.

## Outstanding Questions

### Deferred To Planning

- [ ] How the supervisor keeps the task-to-leader-to-caller mapping across its own turns — its own context, `update_agent` labels, or a small file under `~/.waggledance/`; answered by checking what survives a Paseo restart.
- [ ] Whether `paseo ls -g --json` exposes labels in 0.11.2 so `leaders` and `supervisor ensure` can filter by them; answered by capturing a fixture.
- [ ] How the supervisor waits between leader turns: rely on `notifyOnFinish` alone, or add a `create_heartbeat` as a safety net; answered by one live probe of a long leader turn.

## Deferred Ideas

- Several leaders per project when a task is marked independent — backlog; one leader per project holds (D3) until worktree and bee-state collisions are understood.
- Letting Paseo-internal callers (the Waggle Dance plugin request box) route through the supervisor instead of the frozen daemon path.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Planning's Gate 2 shape stage and reviewing use locked decisions for coverage and UAT.
