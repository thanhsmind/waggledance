---
type: bee.pattern
title: "A task handed in from outside Paseo goes to the supervisor, which runs the project's leader as its own subagent"
description: "Retired, superseded by wd-direct-leader. Practice: an agent outside Paseo, such as aicoworker on OpenClaw, hands a task to the one waggledance supervisor with paseo send; the supervisor opens or reuses the project's leader as a Paseo subagent, hears its turn ends from Paseo itself, routes every question to an advisor before the user, and reports with waggledance supervisor report into wd's main OpenClaw session, where wd writes the Telegram message. A heartbeat reconciles its ledger and sends wd a board of open tasks. The daemon's leader queue is frozen beside it."
timestamp: 2026-10-18
bee:
  id: paseo-native-supervisor
  lifecycle: retired
  areas: [orchestration]
  sources: [docs/history/paseo-native-supervisor/CONTEXT.md, docs/history/paseo-native-supervisor/plan.md, docs/history/supervisor-active-loop/plan.md]
  polarity: practice
  signature: a task handed in from outside Paseo
---

# A task handed in from outside Paseo goes to the supervisor, which runs the project's leader as its own subagent

Retired by wd-direct-leader D11: the supervisor and its commands are gone, and wd now hands tasks straight to a project's leader; see [wd hands a task straight to a project's leader](wd-direct-leader.md). The rest of this page records the retired path.

Locked by paseo-native-supervisor D1/D2/D3/D4/D5/D6/D7/D8/D9/D10/D11/D12/D13
(`docs/history/paseo-native-supervisor/CONTEXT.md`). D8 freezes the daemon path in
[A request for a project reaches its one Paseo leader](paseo-leader-dispatch.md): it
keeps working for its callers and gains nothing from this feature.
supervisor-active-loop (`docs/history/supervisor-active-loop/plan.md`, decisions
`15cc4a55`, `c505ac8d`, `83d65d2e`, `4d03d229`, `221af324`) moves the outbound hop
into wd's session and adds the done report, advisor-first and the board.

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
| Out | supervisor → wd → user | `waggledance supervisor report`, which runs `openclaw agent --agent <agent_id> --message <framed report> --deliver --channel telegram --json`: a turn in `agent:<agent_id>:main`, whose reply goes to Telegram (decision `15cc4a55`) |

The supervisor is one long-lived Paseo agent for every project, on Claude Sonnet
(decision `4d03d229`),
labelled `waggledance.role=supervisor` (D2). Its working directory is
`<data dir>/supervisor/`. Its standing instructions are the `CLAUDE.md` there, written
from `docs/waggledance-supervisor-agent.md`. aicoworker is not a Paseo agent, so Paseo
notifications cannot reach it; that is why the outbound hop goes through the OpenClaw
CLI.

## Message shapes

aicoworker sends the supervisor exactly four shapes (D12):

| Command | Message |
|---------|---------|
| `handoff` | `[handoff <corr>] <task>`; `handoff` mints `<corr>` as `t-<6 hex>` when none is given |
| `reply` | `[reply <corr>] <text>` |
| `redo` | `[redo <corr>] <text>` |
| `accept` | `[accept <corr>]` |

The supervisor reports with `--kind` one of `result`, `question`, `blocked`, `error`,
`closed`, `progress`, `board`. Each report reaches wd tagged
`[waggledance#<corr>] <project> — <kind label>: <text>`, the kind label in Vietnamese
(kết quả, câu hỏi, đang chờ quyền, lỗi, đã đóng, tiến độ, bảng việc). wd checks it
against the task it handed off and writes the user one Vietnamese message that keeps
the tag; it never answers a leader's question for the user. A bare "ok" from the user
means `accept` of the newest open item; the OpenClaw skill tells wd so.

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

A turn end is not a task end. Every prompt the supervisor sends a leader carries the
mail rule: end each turn with `[mail waggledance#<corr> <kind>]`, where `<kind>` is
`result`, `question`, `blocked` or `working`. `working` means the leader stopped while
its own advisors, subagents or builds still run. The supervisor reports nothing for a
`working` turn and waits for the next turn end; a reply with no mail line that says
something still runs reads as `working`. A leader whose last mail is `working` is not
free. This is piggery's split between `reply` and `settled`, carried on Paseo's own
finish notification (decision `8558b2b0`, after `t-334d78` reported a leader as done
while its plan check and baseline build still ran).

A `working` mail opens a follow-up that the supervisor owns until the leader sends
`result`, `question` or `blocked` (decision `c6881832`). The finish notification only
arrives when the leader's own next turn ends, and nothing guarantees that wake: in
`t-bd665a` the leader stopped on "Tôi chờ worker" with a Monitor armed on a child
worker. So on each beat the heartbeat nudges once a leader that is idle, whose last
mail was `working`, and that has had no turn end for 15 minutes. If the leader is
still silent on the next beat, the heartbeat reports `progress` once, quoting the
leader's last reply.

A `result` mail carries a done report above its mail line: what was done, the proof
(command and outcome), what is left, and where it lives. A result missing any part goes
back to the leader and is not reported (decision `c505ac8d`).

An advisor is a short-lived subagent with no `workspaceId`, labelled
`waggledance.role=advisor`, opened for every question, a leader's or the supervisor's
own, and archived once it answers. When it can decide, the supervisor carries out the
decision; otherwise the supervisor appends `advisor-undecided` and only then reports
`question` (D4, decision `83d65d2e`, after `t-39cfa3` put a question to the user with
no advisor step).

The supervisor keeps `ledger.md` in its working directory, append-only. The ledger is
a hint; `ask_state.leaders` and `paseo ls` are the truth, re-read at every turn start.
A Paseo heartbeat every 10 minutes reconciles the ledger with the map, follows
`working` tasks, reports `progress` for any leader running longer than an hour, and
retries any report that failed (D13). A task is open from `handoff` until `closed` or
`stale-closed`; a task whose leader is closed, archived or gone from the map gets
`stale-closed` (after `t-bd665a` stayed on the list once its leader closed). The beat
sends `board` when any task changed state since the last `board-sent`, and every 2
hours while any task is open: every open task with project, step, waiting on whom and
since when, questions waiting on the user included (decision `221af324`).

## The report is a turn in wd's session

OpenClaw's `/hooks/agent` runs an isolated turn with its own session key, so wd never
saw what the supervisor said about the task wd handed off. `report` runs the live
OpenClaw CLI instead, in wd's main session, and the framed message asks wd to check the
report against that task and write the user's message in Vietnamese, quoting numbers,
paths and the leader's words verbatim. The CLI is found from the running
`openclaw-gateway` process: its `APPDIR` environment names the AppImage mount, and the
CLI is `<APPDIR>/resources/openclaw/openclaw.mjs`, run by `node`; `[openclaw] cli`
overrides that. With no gateway running, `report` fails and the supervisor's ledger
retry covers it; it never runs the agent locally. The message still passes through
wd's model, so the Telegram text is wd's wording, not the report's.

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
`claude/claude-sonnet-5-5`, mode `bypassPermissions`, no thinking, title
`waggledance supervisor`. `ensure` takes no lock; two racing callers on a missing
supervisor can start two, and the next `ensure` returns the first listed.

### waggledance supervisor report

`report --kind <kind> --project <name> --corr <corr> --text <text> [--json]` runs
`<cli> agent --agent <[openclaw] agent_id> --message <framed report> --deliver
--channel telegram --json`, adding `--reply-to <[openclaw] telegram_to>` when set, and
never `--local`. It exits non-zero when no gateway runs and no `[openclaw] cli` is set,
when the CLI exits non-zero, or when its JSON status is not ok. An unknown kind is
refused. The heartbeat's board goes as `--kind board --project all --corr board`.

`hook.token` and the hook config fields stay, written by `setup openclaw`, until
nothing reads them; `report` no longer does. The token is owner-only and never a
`Config` field (D13).

### waggledance setup openclaw

Installs aicoworker's side, in this order:

1. Writes `<data dir>/hook.token` (32 random bytes, hex, owner-only) when absent; an
   existing token is never overwritten.
2. Writes the `waggledance-orchestrator` skill into
   the workspace of `[openclaw] agent_id`: its `agents.list[].workspace` in
   `openclaw.json` (relative paths under the OpenClaw home), else
   `<[openclaw] home>/workspace-<agent_id>/skills/`, and `workspace-main` when
   `agent_id` is empty. It moves any unversioned copy to
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
waits on trigger `the-waggle-dance-paseo-plugin-s-request-b__469884e6`.

## Pointers

- `crates/waggledance/src/seat.rs` — `supervisor ensure` and `report`: `ensure_over`,
  `write_doc`, `kind_label`, and the OpenClaw CLI discovery and argv.
- `crates/waggledance/src/setup.rs` — `Target::OpenClaw`: `openclaw_install`,
  `openclaw_remove`, `edit_openclaw_json`, `openclaw_notes`, the embedded skill files.
- `crates/waggledance/src/mcp.rs` — `read_leaders`, `LeadersRead`, `attach_leaders`,
  and the `waggledance_ask_state` description.
- `crates/waggledance/src/paseo.rs` — `parse_listed`, `VIA_LABEL`,
  `role_leaders_args`, `via_leaders_args`, `PaseoCli::list_labelled`.
- `crates/waggledance-core/src/config.rs` — `SupervisorConfig`, `OpenClawConfig`,
  `hook_token_path`, `read_hook_token`, `ensure_hook_token`.
- `integrations/openclaw/` — the `waggledance-orchestrator` skill: `SKILL.md` and
  `bin/wd-bridge.py` with `handoff`, `reply`, `redo`, `accept`, and how wd reads a
  report and a board.
- `docs/waggledance-supervisor-agent.md` — the supervisor's standing instructions.
- `crates/waggledance/testdata/fixtures/paseo-0.11.2/ls-label-leader.out` and
  `npm-run-detached.out` — the captured `ls --label` and `run -d --json` answers.
