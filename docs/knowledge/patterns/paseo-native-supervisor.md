---
type: bee.pattern
title: "A task handed in from outside Paseo goes to the supervisor, which runs the project's leader as its own subagent"
description: "Practice: an agent outside Paseo, such as aicoworker on OpenClaw, hands a task to the one waggledance supervisor with paseo send; the supervisor opens or reuses the project's leader as a Paseo subagent, hears its turn ends from Paseo itself, routes its questions to an advisor or the user, and reports to Telegram with waggledance supervisor report through the OpenClaw hook. The daemon's leader queue is frozen beside it."
timestamp: 2026-10-17
bee:
  id: paseo-native-supervisor
  lifecycle: active
  areas: [orchestration]
  sources: [docs/history/paseo-native-supervisor/CONTEXT.md, docs/history/paseo-native-supervisor/plan.md]
  polarity: practice
  signature: a task handed in from outside Paseo
---

# A task handed in from outside Paseo goes to the supervisor, which runs the project's leader as its own subagent

Locked by paseo-native-supervisor D1–D13
(`docs/history/paseo-native-supervisor/CONTEXT.md`). D8 freezes the daemon path in
[A request for a project reaches its one Paseo leader](paseo-leader-dispatch.md): it
keeps working for its callers and gains nothing from this feature.

Inside Paseo, agents talk through Paseo's own tools: `create_agent`,
`send_agent_prompt` with `notifyOnFinish`, `archive_agent`, and labels. waggledance is
the supervisor's read layer (projects, `ask_state`, runs), never the pipe between
agents (D1). Classifying a leader's turn and judging whether a leader is free is the
supervisor's call; waggledance exposes the raw fields and decides nothing (D7).

## The three hops

| Hop | From → to | Carried by |
|-----|-----------|------------|
| In | aicoworker → supervisor | `wd-bridge.py handoff`: `waggledance supervisor ensure --json`, then `paseo send <supervisor id> --no-wait --prompt-file <tmp> --json` (D5) |
| Inside | supervisor ↔ leaders and advisors | Paseo subagents the supervisor creates with MCP `create_agent`; Paseo tells the parent when a child's turn ends, errors, or asks for a permission (D3, D4) |
| Out | supervisor → user | `waggledance supervisor report`, which posts to OpenClaw's `POST /hooks/agent` with `deliver: true`, `channel: telegram` (D6) |

The supervisor is one long-lived Paseo agent for every project, on Claude Haiku,
labelled `waggledance.role=supervisor` (D2). Its working directory is
`<data dir>/supervisor/`. Its standing instructions are the `CLAUDE.md` there, written
from `docs/waggledance-supervisor-agent.md`. aicoworker is not a Paseo agent, so Paseo
notifications cannot reach it; that is why the outbound hop goes through the hook.

## Message shapes

aicoworker sends the supervisor exactly four shapes (D12):

| Command | Message |
|---------|---------|
| `handoff` | `[handoff <corr>] <task>`; `handoff` mints `<corr>` as `t-<6 hex>` when none is given |
| `reply` | `[reply <corr>] <text>` |
| `redo` | `[redo <corr>] <text>` |
| `accept` | `[accept <corr>]` |

The supervisor reports with `--kind` one of `result`, `question`, `blocked`, `error`,
`closed`, `progress`. Each report reaches Telegram tagged
`[waggledance#<corr>] <project> — <kind label>: <text>`, the kind label in Vietnamese
(kết quả, câu hỏi, đang chờ quyền, lỗi, đã đóng, tiến độ). A bare "ok" from the user
means `accept` of the newest open item; the OpenClaw skill tells aicoworker so.

## Leaders and advisors

A leader is a Paseo subagent the supervisor creates from inside itself with
`create_agent`, never with `paseo run` from the CLI: only an MCP-created agent is a
Paseo child, and only a child's `notifyOnFinish` returns to the supervisor (D3). It is
placed in the project's workspace with `workspaceId` and carries three labels:
`waggledance.role=leader`, `waggledance.project=<project id>`, and
`waggledance.via=supervisor`. One leader per project.

The supervisor reuses only leaders labelled `waggledance.via=supervisor` (D10). For a
new task:

- **No supervisor leader.** It opens one.
- **Idle, nothing open.** It clears that leader and opens a fresh one, whether or not
  the user accepted the last result; the result was already reported (D10).
- **Waiting on a reply.** It queues the task in its ledger and reports `progress`
  saying the task is queued (D10).

Clearing is graceful (D13): the supervisor asks the leader to write a pause handoff and
release its session, waits for the finish notification, then calls `archive_agent`. It
archives outright only when the leader does not answer within its wait. A leader is
archived only after the user's acceptance is relayed as `[accept <corr>]`; a finished
but unaccepted leader stays idle with its context (D9).

An advisor is a short-lived subagent with no `workspaceId`, labelled
`waggledance.role=advisor`, opened when a leader asks a question and archived once it
answers. When it can decide, the supervisor forwards the decision to the leader;
otherwise the supervisor reports `question` to the user (D4).

The supervisor keeps `ledger.md` in its working directory, append-only. The ledger is
a hint; `ask_state.leaders` and `paseo ls` are the truth, re-read at every turn start.
A Paseo heartbeat every 30 minutes reports `progress` for any leader running longer
than an hour and retries any report the hook refused (D13).

## The hook is a model turn

OpenClaw's `/hooks/agent` does not post text to Telegram; it runs an isolated model
turn whose prompt is the `message` (D11). `report` therefore sends an instruction:
"Chuyển nguyên văn cho người dùng, không diễn giải lại, không dịch, không thêm lời:"
followed by the tagged line, with `thinking` set from `[openclaw] hook_thinking`
(default `low`) and `model` from `[openclaw] hook_model` when set. The relay still
passes through a model, so the Telegram text can drift from the report text. That is
the accepted limit; the live proof checks it by quoting one against the other.

## The `leaders` field

`waggledance_ask_state` carries `leaders` per project, read once per tool call from
two Paseo lists: every agent labelled `waggledance.role=leader`, and the subset also
labelled `waggledance.via=supervisor` (D8). The agent list carries no labels, so a
leader belongs to a project when its cwd is the project root or one of its worktrees,
the deepest root winning. Each entry:

```json
{"agent_id": "…", "status": "idle", "cwd": "/…/project", "created": "…",
 "via": "supervisor",
 "labels": {"waggledance.role": "leader", "waggledance.project": "<id>", "waggledance.via": "supervisor"},
 "waiting_on": {"kind": "…", "subject": "…"},
 "last_daemon_run": {"run_id": "…", "status": "done"}}
```

`via` is `supervisor` for a leader carrying the via label and `daemon` otherwise.
`waiting_on` comes from the project's bee state; `last_daemon_run` is the newest run
whose pane id starts with `paseo:`. Entries are ordered oldest first.

| Paseo | `leaders` |
|-------|-----------|
| `[paseo] enabled = false` | key absent |
| unavailable, or a list call fails | `null`, beside `leaders_error` naming the reason |
| read | an array (possibly empty), beside `leaders_skipped`: leaders under no registered project |

A Paseo failure never turns into a tool error; the bee half of the answer is intact.

## Commands

### waggledance supervisor ensure

Finds the supervisor by `paseo ls -g --label waggledance.role=supervisor --json` and
reuses the first one listed. With none, it starts one with
`paseo run -d --json --cwd <data dir>/supervisor --provider <[supervisor] provider>
[--mode] [--thinking] --title <[supervisor] title> --label waggledance.role=supervisor`
and a one-line bootstrap prompt. Either way it writes `CLAUDE.md` when its bytes differ
from the embedded instructions. It refuses when Paseo is unavailable.

```json
{"agent_id": "…", "created": false, "instructions_stale": true, "cwd": "…/supervisor"}
```

`instructions_stale` is true when a running supervisor was found and `CLAUDE.md` had to
be rewritten: that agent read the old copy. `[supervisor]` defaults: provider
`claude/claude-haiku-5-5`, mode `bypassPermissions`, no thinking, title
`waggledance supervisor`. `ensure` takes no lock; two racing callers on a missing
supervisor can start two, and the next `ensure` returns the first listed.

### waggledance supervisor report

`report --kind <kind> --project <name> --corr <corr> --text <text> [--json]` reads the
bearer token from `<data dir>/hook.token` and posts the D11 body to
`[openclaw] hook_url` (default `http://127.0.0.1:18789/hooks/agent`) with a 15-second
timeout. A 2xx prints `{status, sent: true}`. A missing token file refuses before any
request and names `waggledance setup openclaw`; any other status exits non-zero with
the status and the first 300 characters of the body; a connection error exits non-zero
naming the URL. An unknown kind is refused.

The token is written owner-only, like `telegram.token`, and is never a `Config` field
(D13). The supervisor never reads it; `report` does.

### waggledance setup openclaw

Installs aicoworker's side, in this order:

1. Writes `<data dir>/hook.token` (32 random bytes, hex, owner-only) when absent; an
   existing token is never overwritten.
2. Writes the `waggledance-orchestrator` skill into
   `<[openclaw] home>/workspace-main/skills/`, moving any unversioned copy to
   `<data dir>/backups/` first.
3. Refreshes `~/.agents/skills/waggledance-supervisor/` from
   `docs/waggledance-supervisor-skill-template.md` the same way.
4. Edits `<[openclaw] home>/openclaw.json`: sets `hooks.enabled`, `hooks.token`,
   `hooks.defaultSessionKey = "hook:waggledance"` and `hooks.allowRequestSessionKey =
   false`, keeps every other key, and leaves an owner-only `openclaw.json.bak-<unix
   seconds>` beside it. It skips the write when all four already match.

It prints what it changed, then two `curl` lines to run: one without the token
(expect 401) and one with it (expect 200), plus the note that the gateway may need a
restart to load `hooks`. setup never restarts the gateway.

`setup openclaw --remove` deletes both skill directories it wrote and restores any
copy it set aside, restores the newest `openclaw.json.bak-*`, and deletes
`hook.token`. It refuses to delete a skill directory it did not write.

## Shared-label limit with the frozen daemon path

Both paths label a leader `waggledance.role=leader` and
`waggledance.project=<project id>`; only the supervisor adds `waggledance.via=supervisor`
and reads it. A project reached through the Paseo plugin's request box and through the
supervisor at once may hold two leaders: one the daemon started (`via: daemon`), which
the supervisor never reuses or archives, and one the supervisor opened
(`via: supervisor`). `leaders` lists both. The daemon's `resolve_leader` filters on the
role and project labels only, so it takes the first unarchived leader of either kind
and may deliver into the supervisor's. Routing the request box through the supervisor
is deferred.

## Pointers

- `crates/waggledance/src/seat.rs` — `supervisor ensure` and `report`: `ensure_over`,
  `write_doc`, `hook_body`, `kind_label`, `post`.
- `crates/waggledance/src/setup.rs` — `Target::OpenClaw`: `openclaw_install`,
  `openclaw_remove`, `edit_openclaw_json`, `openclaw_notes`, the embedded skill files.
- `crates/waggledance/src/mcp.rs` — `read_leaders`, `LeadersRead`, `attach_leaders`,
  and the `waggledance_ask_state` description.
- `crates/waggledance/src/paseo.rs` — `parse_listed`, `VIA_LABEL`,
  `role_leaders_args`, `via_leaders_args`, `PaseoCli::list_labelled`.
- `crates/waggledance-core/src/config.rs` — `SupervisorConfig`, `OpenClawConfig`,
  `hook_token_path`, `read_hook_token`, `ensure_hook_token`.
- `integrations/openclaw/` — the `waggledance-orchestrator` skill: `SKILL.md` and
  `bin/wd-bridge.py` with `handoff`, `reply`, `redo`, `accept`.
- `docs/waggledance-supervisor-agent.md` — the supervisor's standing instructions.
- `crates/waggledance/testdata/fixtures/paseo-0.11.2/ls-label-leader.out` and
  `npm-run-detached.out` — the captured `ls --label` and `run -d --json` answers.
