# wd Direct Leader — slice 2 contract

Slice 2 of `docs/history/wd-direct-leader/plan.md` ("Shape"): wd's follow-up commands
(D10), the heartbeat (D8) and the permission relay (D9). This file is the one home for
the names, columns and rules slice-2 cells share; the slice-1 contract in plan.md still
holds.

## Tasks table (D8, D10)

Migration 8 adds `tasks`: `corr` primary key, `project_id`, `leader_id` null, `state`
(`open` or `closed`), `created_at`, `last_delivered_at` null, `last_mail_at` null,
`nudged_at` null, `progress_sent_at` null, `changed_at`, `board_sent_at` null.
`delete_project` deletes a project's tasks and permission requests. The wake loop's hourly prune also deletes closed tasks and closed or answered requests older than 30 days.

Writers:

- A wd hand-off (`openclaw:wd` caller, `request_id` with no `:`) inserts the task `open`.
- Delivery, when it sends a wd run to a leader, sets `leader_id` and `last_delivered_at`
  and clears `nudged_at` and `progress_sent_at`.
- `mail send` of any kind sets `last_mail_at`, so a leader that asked a question is not nudged.
- `accept` sets `state = closed`.
- Every write above sets `changed_at`.

## Follow-up commands (D10)

One new MCP tool, `waggledance_task`, input `{corr, action, text?, req_id?}`:

| action | effect |
|---|---|
| `reply`, `redo` | queues a leader run for the task's project with `request_id = <corr>:<n>` (n = 1 + count of runs whose request_id starts with `<corr>:`; a duplicate return retries with n + 1, never reports success), caller `openclaw:wd`, task = the `wd_task` prefix for `<corr>` + `Reply from the user:` or `Redo requested:` + the text |
| `accept` | closes the task; archives its leader with `paseo archive <leader_id>` only when no queued or working run belongs to that leader and no other open task shares it, otherwise leaves the leader and says why; returns whether it archived |
| `allow`, `deny` | runs `paseo permit allow|deny <leader_id> <req_id>`; without `req_id`, the newest unanswered request of that task; marks the request answered |

An unknown corr, a closed task (except `accept`, which is idempotent), or a task with no
leader yet returns a tool error naming the cause. wd-bridge gains `reply`, `redo`,
`accept`, `permit allow|deny` over this tool, with `WAGGLEDANCE_CALLER=wd`.

## Permission relay (D9)

Migration 8 also adds `permission_requests`: `req_id` primary key, `corr`, `leader_id`,
`tool`, `description`, `seen_at`, `answered_at` null, `closed_at` null.

A watcher in the daemon runs `paseo permit ls --json` every 30 s. For each row whose
`agentId` is the `leader_id` of an open task (several open tasks on one leader: the one with the newest `last_delivered_at`) and whose `id` is not in the table, it
inserts the request and one `permission` inbox message naming project, corr, tool,
description and `req_id`. A known request no longer listed, with `answered_at` null,
gets `closed_at` and one `closed` message saying it vanished unanswered. From slice 2,
delivery's `blocked_then_done` no longer writes `permission` messages: the watcher is the
one writer.

## Heartbeat (D8)

Every 10 minutes over open tasks with a `leader_id`:

1. Leader idle (inspect status not `running`/`initializing` and `pending_permissions`
   0), `last_mail_at` null or older than `last_delivered_at`, `nudged_at` null: under the
   project's `LeaderLock`, re-check idle, then `paseo send <leader> "<nudge>"` asking for
   the done report through `waggledance mail send`, then set `nudged_at`.
2. `nudged_at` set on an earlier beat, still no newer mail, `progress_sent_at` null:
   one `progress` message to wd, set `progress_sent_at`.
3. Any task's `changed_at` newer than the last board: one `board` message (corr
   `board`, project `all`) listing every open task as `<corr> <project> <state of the
   leader>`, then stamp `board_sent_at` on every task.

A `board` message is framed with its own instruction line (a list of open tasks, not a
report on one task). The interval is `[openclaw] heartbeat_secs` (default 600); tests drive one beat directly.
