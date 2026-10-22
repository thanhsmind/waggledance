---
type: bee.pattern
title: "wd hands a task straight to a project's leader, and the leader answers through a waggledance inbox that wakes wd"
description: "Practice: wd, the aicoworker agent on OpenClaw, hands a task to a project's leader with waggledance_dispatch leader: true through wd-bridge.py; the daemon queues it, delivers it to a waggledance.via=wd leader, and carries every result, question, permission request, progress note and board back to wd through a durable SQLite inbox, waking wd's main session with one openclaw agent turn per message and acking it only when that turn ends ok. A 10-minute heartbeat and a 30-second permission watcher run in the daemon as code, not as a model turn."
timestamp: 2026-10-22
bee:
  id: wd-direct-leader
  lifecycle: active
  areas: [orchestration]
  sources: [docs/history/wd-direct-leader/CONTEXT.md, docs/history/wd-direct-leader/plan.md, docs/history/wd-direct-leader/slice-2.md, docs/history/wd-direct-leader/slice-3.md, docs/history/wd-direct-leader/reports/slice-1-live.md, docs/history/wd-direct-leader/reports/slice-2-live.md]
  polarity: practice
  signature: a task handed in from wd on OpenClaw
---

# wd hands a task straight to a project's leader, and the leader answers through a waggledance inbox that wakes wd

Locked by wd-direct-leader D1–D12 (`docs/history/wd-direct-leader/CONTEXT.md`). The
shared names and rules live in the slice-1 contract of
`docs/history/wd-direct-leader/plan.md`, in `slice-2.md` and in `slice-3.md`. This path
replaces the Paseo supervisor
([paseo-native-supervisor](paseo-native-supervisor.md), now retired): no agent sits
between wd and the leader.

wd is the aicoworker agent whose OpenClaw main session talks to the user on Telegram.
wd is the orchestrator. The daemon queues, delivers, watches and wakes. The path ends
at wd's main session: what wd writes on Telegram is wd's own turn.

## The path

| Step | From → to | Carried by |
|------|-----------|------------|
| Hand-off | wd → daemon | `wd-bridge.py handoff --project <p> --task <t>`: `waggledance mcp` with `WAGGLEDANCE_CALLER=wd`, calling `waggledance_dispatch {project, task, leader: true, request_id: <corr>}` |
| Delivery | daemon → leader | the daemon leader queue of [paseo-leader-dispatch](paseo-leader-dispatch.md), to a leader labelled `waggledance.via=wd` |
| Mail | leader → inbox | `waggledance mail send --to wd --corr <corr> --kind <kind>`, body on stdin or from a file |
| Wake | inbox → wd | one `openclaw agent --agent <agent_id> --message <frame> --deliver --channel telegram --json` turn in wd's main session per message |
| Follow-up | wd → daemon | `wd-bridge.py reply`, `redo`, `accept`, `permit allow|deny`, over the `waggledance_task` MCP tool |

The corr is the task id `t-<6 hex>` that `handoff` mints when none is given. It is the
dispatch `request_id`, so a retried hand-off returns the same run.

## Hand-off and leaders

`handle_dispatch` stores the caller `openclaw:wd` when `PASEO_AGENT_ID` is empty and
`WAGGLEDANCE_CALLER` is `wd`. The dispatch tool schema has no wd field.

- A wd run resolves only leaders labelled `waggledance.via=wd`. It creates leaders with
  `waggledance.role=leader`, `waggledance.project=<project id>` and
  `waggledance.via=wd`.
- Every other leader run skips a `via=wd` leader, so the Paseo plugin's request box and
  other MCP callers never deliver into wd's leader (D2, D12).
- `waggledance_ask_state` reports each leader's `via` as `wd` or `daemon`. A leader
  still labelled `waggledance.via=supervisor` reads `daemon`.
- A hand-off (a `request_id` with no `:`) inserts the task row `open`.

The leader receives the task text behind a fixed prefix (`paseo::wd_task`):
`Task waggledance#<corr> from wd.`, the mail rule, the advisor rule, then the task
unchanged. The mail rule tells the leader to run `mail send --kind result` with four
parts: `Done:`, `Proof:`, `Left:`, `Where:`. The advisor rule tells it to ask its own
project's bee advisor (`bee dispatch prepare --kind advisor`) before a `question`, and to
mail only what the advisor cannot decide, quoting the advisor's answer (D7).

The per-project hold is shared: a leader busy with another caller's task parks a wd
task on the same project until it is idle.

## The inbox

The inbox is the `inbox` table in `registry.db` (migration 7): `seq`, `recipient`,
`corr`, `project_id`, `kind`, `body`, `created_at`, `acked_at`, `attempts`, `dead_at`.
Rows survive a daemon restart. `delete_project` deletes a project's rows.

Kinds and their Vietnamese labels: `result` kết quả, `question` câu hỏi, `progress`
tiến độ, `error` lỗi, `permission` xin quyền, `closed` đã đóng, `board` bảng việc.

| Writer | Kinds |
|--------|-------|
| The leader, through `mail send` | `result`, `question`, `progress`, `error` |
| Delivery, when a wd run is `lost` | `error`, quoting the transcript's first line |
| The permission watcher | `permission`, `closed` |
| The heartbeat | `progress`, `board` |

`mail send` refuses a `result` that lacks any of `Done`, `Proof`, `Left` or `Where`.
A part counts when a line, after leading spaces and any of `#`, `-`, `*`, starts with
that word and `:` or ends a heading, and the part under it is not empty. The CLI exits 2
and names every missing part. An unknown corr or kind also exits 2. The corr resolves to
the newest run whose `request_id` is `<corr>` or starts with `<corr>:`. Every accepted
mail stamps the task's `last_mail_at`.

Delivery sends no Paseo caller notice for an `openclaw:` caller. A wd run that blocks
on a permission writes nothing from delivery: the watcher is the one writer of
`permission` messages.

## The wake and its ack rule

The wake is one serial tokio task (`wake::Wake`), active only when `[openclaw]
agent_id` is set. Every 2 seconds it takes the lowest-`seq` row for `wd` with `acked_at`
and `dead_at` both null, and runs one `openclaw agent` turn for it, with `--reply-to`
when `telegram_to` is set and a 900-second timeout. wd never polls.

The message is an instruction line, then the frame:

```text
[waggledance#<corr> #<seq>] <project> — <kind label>: <body>
```

A `board` message carries its own instruction line: it is a list of open tasks, not a
report on one task. The board frame reads `[waggledance#board #<seq>] all — bảng việc: …`.

- A row is acked only when the turn that carried it returns `status == "ok"` (D6).
  Reading never acks.
- A failed or timed-out turn increments `attempts` and backs off 30 seconds. The row is
  carried again, in `seq` order, with the same `#<seq>`.
- At 5 failed attempts the row gets `dead_at`, the daemon logs one warning naming the
  seq, and the next row is woken.
- A row written while a turn runs waits for the next wake.
- Once an hour the wake deletes acked rows older than 30 days, closed tasks older than
  30 days, and closed or answered permission requests older than 30 days.

## Follow-up commands

The `waggledance_task` MCP tool takes `{corr, action, text?, req_id?}`. wd reaches it
through `wd-bridge.py`.

| Action | Effect |
|--------|--------|
| `reply`, `redo` | queues a leader run for the task's project with `request_id = <corr>:<n>`, caller `openclaw:wd`, and the task text behind the same `wd_task` prefix plus `Reply from the user:` or `Redo requested:` |
| `accept` | closes the task, then archives its leader with `paseo archive <leader_id>` only when no queued or working run belongs to that leader and no other open task shares it; otherwise it leaves the leader and says why |
| `allow`, `deny` | runs `paseo permit allow|deny <leader_id> <req_id>`; without `req_id`, it takes the task's newest unanswered request; marks the request answered |

An unknown corr, a closed task, or a task with no leader yet returns a tool error that
names the cause. `accept` on a closed task is idempotent. A leader is archived only
after `accept` (D10).

## The heartbeat

The `tasks` table (migration 8) holds one row per corr: `state` (`open` or `closed`),
`leader_id`, `last_delivered_at`, `last_mail_at`, `nudged_at`, `progress_sent_at`,
`changed_at` and `board_sent_at`. Delivery sets `leader_id` and `last_delivered_at` when
it sends a wd run, and clears `nudged_at` and `progress_sent_at`.

`heartbeat::Heartbeat` beats every `[openclaw] heartbeat_secs` (default 600) over open
tasks that have a leader. The rules are code, not a model turn (D8):

1. When the leader is idle, has sent no mail since its last delivery, and was not yet
   nudged, the daemon takes the project's `LeaderLock`, re-checks idle, sends one
   `paseo send` that asks for the done report, and sets `nudged_at`.
2. When a nudged leader is still silent on a later beat, the daemon mails wd one
   `progress` message and sets `progress_sent_at`.
3. When any task's `changed_at` is newer than the last board, the daemon mails wd one
   `board` message (corr `board`, project `all`) listing every open task as
   `<corr> <project> <leader state>`, then stamps `board_sent_at` on every task.

## The permission watcher

The same task runs `paseo permit ls --json` every 30 seconds. Rows land in the
`permission_requests` table (migration 8): `req_id`, `corr`, `leader_id`, `tool`,
`description`, `seen_at`, `answered_at`, `closed_at`.

- A row whose `agentId` is the `leader_id` of an open task, with a `req_id` not yet
  stored, is inserted and becomes one `permission` message. The message names the
  project, the corr, the tool, the description and the `req_id`. When several open tasks
  share the leader, the task with the newest `last_delivered_at` owns the request.
- A stored request that is no longer listed and has no `answered_at` gets `closed_at`
  and one `closed` message that says it vanished unanswered.
- The daemon never allows or denies a request on its own. Only wd's
  `permit allow|deny` answers one (D9).

## Known gaps

- A permission message names the tool, but not the command. Paseo's `permit ls --json`
  row carries `name` and `description`, and for a `Bash` request the description is the
  literal `-`. The user is asked to allow a shell command they cannot see. Filed as the
  P2 backlog finding "Permission messages to wd name only the tool (Bash: -), not the
  command the leader wants to run" (`bee backlog findings --feature wd-direct-leader`).
- A board written during a long wd turn is read late. The wake is serial and carries the
  backlog in `seq` order, so a `board` row waits until the running turn ends. In the
  slice-2 walk, two boards sat unacked for minutes behind one wake turn.

## Retired: the Paseo supervisor

`waggledance supervisor` and its subcommands, `SupervisorConfig` and
`docs/waggledance-supervisor-agent.md` are gone (D11). A config file that still holds a
`[supervisor]` table loads. The `waggledance-supervisor` Claude skill stays: its leader
mode hands a task to `wd-bridge.py handoff` from aicoworker and to
`waggledance_dispatch {project, task, leader: true, request_id}` from any other session.
The herdr watchdog (`crates/waggledance/src/supervisor.rs`,
`terminal.supervisor_enabled`) and `bee supervisor` share only the word.

## Pointers

- `crates/waggledance/src/wake.rs` — `Wake`, `kind_label`, `inbox_frame`,
  `report_argv`, `openclaw_cli`, `cli_run`, `cli_status`, `enabled`, and the hourly
  `prune`.
- `crates/waggledance/src/mail.rs` — `waggledance mail send`: `send`, `missing_parts`,
  `part_header`.
- `crates/waggledance/src/heartbeat.rs` — `Heartbeat`: `beat`, `nudge`, `progress`,
  `watch`, `seen`, `vanished`, `nudge_text`.
- `crates/waggledance/src/mcp.rs` — `dispatch_to_leader` with the `WAGGLEDANCE_CALLER`
  read, `handle_task`, `task_action`, `permit_task`, and the `via` field of
  `waggledance_ask_state`.
- `crates/waggledance/src/delivery.rs` — the `openclaw:wd` branch in delivery and the
  `lost` → `error` mail.
- `crates/waggledance/src/paseo.rs` — `WD_LABEL`, `WD_CALLER`, `wd_task`, `wd_corr`,
  `labelled`, `resolve_leader`.
- `crates/waggledance-core/src/repository.rs` — `migration_7_inbox`,
  `migration_8_tasks`, the inbox functions (`insert_inbox_message`,
  `next_inbox_message`, `ack_inbox_message`, `fail_inbox_message`, `prune_inbox`), the
  task functions, and the permission-request functions.
- `crates/waggledance/src/main.rs` — where the daemon starts `Wake` and `Heartbeat`.
- `crates/waggledance-core/src/config.rs` — `OpenClawConfig`: `agent_id`,
  `telegram_to`, `heartbeat_secs`.
- `integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py` — `handoff`,
  `reply`, `redo`, `accept`, `permit allow|deny`.
- `integrations/openclaw/waggledance-orchestrator/SKILL.md` — how wd hands off and how
  it reads a frame and a board.
- `docs/waggledance-supervisor-skill-template.md` — the skill's leader mode.
- `docs/history/wd-direct-leader/reports/slice-1-live.md` and `slice-2-live.md` — the
  live walks of the whole path.
