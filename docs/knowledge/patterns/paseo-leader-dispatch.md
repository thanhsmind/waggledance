---
type: bee.pattern
title: Each request for a project starts its own Paseo leader through waggledance_dispatch
description: "Practice: with a working Paseo CLI found and a registered project whose .waggledance.toml does not say orchestration = false, waggledance_dispatch with leader: true queues the task; the daemon starts a new labelled Paseo leader for that run, delivers it once, runs of one project in parallel, marks it done when Paseo reports that turn completed, keeps the leader for follow-ups until the idle sweep archives it ([paseo] leader_idle_minutes, default 30), routes a follow_up_of run to the newest leader of its family, lets a leader hand off to a successor with handoff: true, sends a same-cluster running leader a context-only steer: true note, and tells the caller."
timestamp: 2026-10-16
bee:
  id: paseo-leader-dispatch
  lifecycle: active
  areas: [orchestration]
  sources: [docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md, docs/history/leader-delivery/CONTEXT.md, docs/history/leader-delivery/plan.md, docs/history/paseo-auto-wiring/CONTEXT.md, docs/history/paseo-auto-wiring/plan.md, docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md, docs/history/paseo-leader-mode/CONTEXT.md, docs/history/paseo-leader-mode/plan.md, docs/history/paseo-dispatch-default-on/CONTEXT.md, docs/history/paseo-dispatch-default-on/plan.md, docs/history/leader-per-task/CONTEXT.md, docs/history/leader-follow-up/CONTEXT.md, docs/history/leader-cluster-steer/CONTEXT.md]
  polarity: practice
  signature: request for a project running in Paseo
---

# Each request for a project starts its own Paseo leader through waggledance_dispatch

Locked by paseo-leader-dispatch D1/D2/D3/D4/D5/D6/D8/D9/D10
(`docs/history/paseo-leader-dispatch/CONTEXT.md`),
leader-delivery D1/D2/D3/D4/D5/D6/D7/D8 (`docs/history/leader-delivery/CONTEXT.md`), and
paseo-auto-wiring D1/D2/D3/D4/D5/D6/D7 (`docs/history/paseo-auto-wiring/CONTEXT.md`), and
paseo-leader-mode D2/D3/D4 (`docs/history/paseo-leader-mode/CONTEXT.md`), which narrows
paseo-auto-wiring D8 for `mode` only, and paseo-dispatch-default-on D1/D2/D4/D5
(`docs/history/paseo-dispatch-default-on/CONTEXT.md`, decision `fb1b49ab`), which
supersedes paseo-auto-wiring D2 and D5: dispatch is on by default, and
leader-per-task D1/D2 (`docs/history/leader-per-task/CONTEXT.md`, decisions `a29369df`
and `0398ca94`), now superseded by leader-follow-up D2/D3
(`docs/history/leader-follow-up/CONTEXT.md`, decisions `b3c4d87a` and `1ab9a58b`):
every run starts its own leader unless it names `follow_up_of`, and no leader is archived
when its run ends `done`, and leader-idle-close D1 (`docs/history/leader-idle-close/CONTEXT.md`,
decision `af012b0b`), now superseded by leader-lifecycle D5/D6
(`docs/history/leader-lifecycle/CONTEXT.md`, decisions `344e1f58` and `e0f7806a`): a leader
idle longer than `leader_idle_minutes` (default 30) with nothing open in its family is
archived, and leader-lifecycle D1-D4/D7: a leader hands off with `handoff: true`, and
leader-idle-close D2 (decision `12506149`): a wd `reply` or `redo` goes to its task's own
leader while that leader is not archived, and leader-follow-up D1/D4 (decisions
`0f51193e` and `aa16f27d`): `waggledance_dispatch` takes `follow_up_of`, and
`ask_state` leaders list the runs they carry. Leader-per-task D1
superseded leader-delivery D3, which had superseded paseo-leader-dispatch D7: no run waits
for, or goes to, an existing leader, apart from the two follow-ups below.
Paseo-auto-wiring D3 removes paseo-leader-dispatch D8's two-workspace refusal, and
paseo-auto-wiring D4 makes the global `[paseo]` block optional.

## Turn it on

Nothing goes in `~/.waggledance/config.toml`, and nothing is written by hand per
workspace (paseo-auto-wiring D1/D4). Three things turn a project's leader path on:

1. **A working Paseo CLI.** waggledance finds it itself; see
   [Which paseo binary runs](#which-paseo-binary-runs). Paseo support is on as soon as
   one candidate answers `ls -g --json` with a JSON list.
2. **The project is registered.** The daemon registers every Paseo workspace root it
   finds; see [Workspace discovery](#workspace-discovery).
3. **The project has not turned dispatch off.** Every registered project takes
   dispatched work by default. `orchestration = false` in its `.waggledance.toml` is
   the only per-project off switch, and a file that exists but cannot be read or
   parsed also counts as off (decision `4b745240`). The Orchestration section of the
   settings page lists each project as on or off and why. An off project is refused
   before anything is stored. Paseo leader dispatch is not gated by
   `terminal.enabled`; that switch gates pane dispatch only.

The waggledance daemon (`waggledance serve`) must be running. It discovers workspaces
and delivers queued requests; `waggledance mcp` only accepts them.

### .waggledance.toml

The per-project settings file sits at the repo root and is committed with the code
(paseo-auto-wiring D2). Every key is optional:

```toml
orchestration = false

[paseo]
workspace = "wks_0123456789ab"
provider = "claude"
mode = "bypassPermissions"
thinking = "high"
```

- `orchestration` — `false` turns dispatched work off for this project, triggers and
  board actions included. Absent or `true` leaves it on. The settings page shows the
  result and no longer switches it.
- `[paseo] workspace` — the Paseo workspace id a new leader starts in. It wins over
  every other rule in [Workspace resolution](#workspace-resolution).
- `[paseo] provider`, `mode`, `thinking` — passed to `paseo run` for a new leader. A
  value set here wins over the same key in the global `[paseo]` block; an unset one
  falls back to it. `mode` unset in both defaults to `bypassPermissions`; `mode = ""` in
  the global block leaves it to Paseo when the project file sets none, and skips the
  [leader mode check](#leader-mode-check) (paseo-leader-mode D2). `thinking` unset is
  left to Paseo. `provider` is never
  left unset, because `paseo run` refuses without one (`MISSING_PROVIDER`): with no
  provider set anywhere, waggledance uses the first provider `paseo provider ls --json`
  reports as available and enabled, and Paseo picks that provider's default model
  (paseo-auto-wiring D8). Paseo reports such CLI errors as a JSON `error` object on
  stderr; waggledance shows its code and message.
- `[paseo] leader_idle_minutes` — how long an idle leader of this project, with nothing
  open in its family, stays before the idle sweep archives it. It wins over the same key
  in the global `[paseo]` block, whose default is 30 (leader-lifecycle D5).

The file is re-read when its modification time changes, so an edit applies on the next
dispatch without a restart. A file that exists but cannot be read or parsed turns
dispatch off, never fails open. An unknown key is ignored with a warning; the rest of
the file still counts. The refusal for an off project reads:

> project `<id>` has turned off orchestrator dispatch with `orchestration = false` in
> `<path>` — remove that line to allow it

and for a broken file:

> project `<id>` has a .waggledance.toml at `<path>` that could not be read, so dispatch
> is off: `<warning>`

**Accepted risk (paseo-dispatch-default-on D1):** every agent that can reach the MCP
surface can dispatch into every registered project that has not turned dispatch off.
The owner chose default-on; nothing is written into any repo to opt it out.

### The global [paseo] block

The `[paseo]` block in `~/.waggledance/config.toml` is optional and only overrides (D4):

- `enabled = false` turns Paseo support off, and a leader dispatch is refused naming
  that line. `enabled = true` is no longer needed.
- `binary` names a `paseo` CLI to try first; see
  [Which paseo binary runs](#which-paseo-binary-runs).
- `turn_timeout_secs` defaults to `3600`. It bounds one blocking `paseo run`, `send` or
  `wait` call. When it passes, waggledance checks the leader and waits again; it never
  ends the run on the timeout alone.
- `provider`, `mode`, and `thinking` are host-wide defaults beneath the project file's.
- `[[paseo.leaders]]` (`project`, `workspace`) is a deprecated fallback kept for hosts
  that already carry it. Use `[paseo] workspace` in the project's `.waggledance.toml`.

`waggledance setup paseo` prints the CLI it chose, every rejected candidate with why,
and a warning for each global line that is no longer needed.

`terminal.enabled` does not gate the leader path, and the leader path never talks to
herdr.

## Dispatch

### wd is a caller

wd-direct-leader D1 lifts the freeze paseo-native-supervisor D8 put on this path. wd,
the aicoworker agent on OpenClaw, hands its tasks through this door with caller
`openclaw:wd`. Each wd run starts its own leader, which also carries the label
`waggledance.via=wd`, and that leader stays until wd's `accept` archives it. A wd run sends no caller notice: its outcome reaches
wd through the waggledance inbox. See
[wd hands a task straight to a project's leader](wd-direct-leader.md).

A caller sends `{project, task, leader: true, request_id}` to `waggledance_dispatch`.
Exactly one of `preset`, `pane_id`, and `leader: true` is accepted. The task text is
sent unchanged. `request_id` is optional; pass one on every call.

1. If a run of this project already carries the same `request_id`, the call returns
   `{run_id, status, duplicate: true}` for that run and sends nothing (D1).
2. The project's dispatch switch and Paseo availability are checked, in that order.
3. The request is stored as a run with status `queued`, pane id `paseo:pending`, and the
   caller's `PASEO_AGENT_ID` when the dispatch came from a Paseo agent. No `paseo`
   command runs inside the call.
4. The call returns `{run_id, status: "queued", duplicate: false, warnings}`. The run
   gets its own new leader, so nothing it waits on is busy (leader-per-task D1).

When the daemon's delivery heartbeat (`<data dir>/paseo-delivery.heartbeat`) is missing
or older than 30 seconds, `warnings` holds "paused — waggledance daemon not running
(start it with `waggledance serve`)" (D8). The request is still stored and is delivered
once the daemon runs.

### request_id

`request_id` is exact on the leader path and sequential on the herdr path (D7).

- **Leader path.** The queued row is the reservation. A unique index on
  `(project, request_id)` means two racing calls with one id get one run.
- **Herdr path.** The id is checked before the send and stored with the run after a
  confirmed send. A herdr run is stored only after a send succeeds, so two calls racing
  with one id can both send.

The same `request_id` in another project is a new request.

### follow_up_of

`follow_up_of` names a run in the same project — its `run_id` or its `request_id` — whose
leader takes this task instead of a new one (leader-follow-up D1). It goes only with
`leader: true`: beside `preset` or `pane_id`, or without `leader`, the call is refused.
A name that matches no run in the project is refused and nothing is stored. The
`request_id` duplicate check runs first, so a retry still answers with the first run.
The resolved run id is stored on the new run as `runs.follow_up_of`; the task text is
sent unchanged. `waggledance_runs` rows and the dispatch answer carry `follow_up_of`.

Before every leader dispatch, read `waggledance_ask_state` → `leaders[].runs`. Every
caller does this, leaders included (leader-cluster-steer D1). The rows are the runs
whose pane is `paseo:<agent id>`, newest first, at most 5, each `{run_id, request_id,
status, follow_up_of, steer, task_summary}`. `task_summary` is the first task line that
is not a `spec-drop <corr> from waggledance@<sha>` line, the wd `Task waggledance#<corr>
from wd.` preamble or the steer marker, cut to 200 characters (leader-follow-up D4).
`steer` is `true` for a steer note (leader-cluster-steer D7), and the cluster judgment
skips those rows. When a summary is not enough to judge, read the run's full `task`
from `waggledance_runs`. No feature slug or path list is stored for this judgment,
because waggledance knows neither (D8). Then pick one of three tiers:

1. **Same work** (the same feature, the same files, the same output): dispatch with
   `follow_up_of` set to that run's id and no fresh correlation id.
2. **Same cluster** (the same area or module or overlapping paths, a different
   feature): open a new leader whose task carries `Running nearby: <run_id> —
   <task_summary>`, then send the running leader a steer note (see § steer) naming the
   new request, its area, the new `run_id` and what to watch for (D2, D4, D5).
3. **Not related**: mint a new correlation id, and a new leader opens. No steer.

A leader that opens a new leader for tier 2 or tier 3 passes `handoff: false`.
Otherwise the implicit handoff below takes its dispatch as a handoff of its own run
(D9).

Each `leaders[]` entry also names `family_root` — the root run of the family of the
newest run that leader carries — and `current`: `true` when it is that family's newest
leader (leader-lifecycle D7). A small follow-up ask on a live lane always goes through
`follow_up_of` naming a run of the `current` leader; a new leader opens only for new,
unrelated work.

### steer

A steer note is `{leader: true, follow_up_of: <a run of the running leader>, steer:
true}` (leader-cluster-steer D2). `steer: true` without `leader` or `follow_up_of`, or
with `handoff: true`, is refused and nothing is stored. The daemon stores the task with
the fixed marker line `[waggledance steer] Context only, not new work: do not build it,
claim cells for it, or reply with a plan.` above the caller's text (`paseo::STEER_MARKER`
in `crates/waggledance/src/paseo.rs`), so no caller can misspell it. A steer run is a
normal `follow_up_of` run in the family: it waits while the leader is mid-turn and lands
once the leader is idle. A steer whose family has no live leader ends `lost` with
`steer note for run <of> not delivered: its leader <id> is gone, and a steer never
opens a leader`, and no leader starts (D3). The dispatch answer carries `steer`.

The receiving leader treats a message whose first line is the marker as context only:
it does not build it, claim cells for it or reply with a plan, keeps clear of the area
the note names, and ends the turn (D6). The supervisor template says so, and its skill
description loads on that marker.

### handoff

A leader that reaches its context mark hands its work to a successor with
`{leader: true, follow_up_of: <its run>, handoff: true}` (leader-lifecycle D1).
`handoff: true` without `follow_up_of`, or without `leader`, is refused. The run is
stored in the referenced run's family with `runs.handoff` set. Delivery never waits on
the family and never sends to an existing leader: it starts a fresh leader with the
handoff task text, and the transcript header reads `[waggledance] handoff of run <of>:
takes over from leader <old>` (D2). As soon as the successor's agent id is known, every
older leader of the family is polled and archived once it is idle with no `queued`,
`working` or `blocked` run on its pane; when that does not happen within
`turn_timeout_secs`, the idle sweep takes it.

A `leader: true` dispatch with no `follow_up_of` from a Paseo agent (`PASEO_AGENT_ID`)
that is the pane of a run in the same project is treated as a handoff of that agent's
newest run, and the answer's `warnings` tells it to pass `follow_up_of` with
`handoff: true` (D4). `handoff: false` opts out.

## Delivery

The daemon runs the delivery loop when it finds Paseo available, and writes the
heartbeat on every tick. It delivers queued requests oldest first, and the runs of one
project are delivered in parallel, one leader each (leader-per-task D1). Delivery never
looks up, reuses or waits on an existing leader, with two exceptions: a wd follow-up run
(leader-idle-close D2) and a run that names `follow_up_of` (leader-follow-up D2).

Each delivery resolves the workspace and starts a new leader with a blocking `paseo run
--json --workspace <id> --cwd <project root> --title "<project name> leader" --label
waggledance.role=leader --label waggledance.project=<project id> --label
waggledance.run=<run id> -- <task>`. A wd run adds `--label waggledance.via=wd`. The
task is the leader's first turn; there is no bootstrap turn (D8).

### The follow-up exceptions

Two kinds of run go to an existing leader instead of starting their own; every other run
starts a new leader, always.

- **A `follow_up_of` run** (leader-follow-up D2), from any caller. It belongs to a
  family: the root of its `follow_up_of` chain and every run that follows up any run in
  it. Its leader is the paseo pane of the newest run that has one among the earlier
  family runs plus every handoff run of the family, so a chain of follow-ups follows the
  newest leader and a handoff successor wins even over runs queued before the handoff
  (leader-lifecycle D3). While one of those runs is still `queued`, `working` or
  `blocked`, the run stays `queued`, so two follow-ups never land
  in one leader at once. When a `follow_up_of` run is also a wd run, `follow_up_of`
  decides.
- **A wd follow-up run** (leader-idle-close D2): one whose `request_id` is
  `<corr>:<n>`, queued by `reply` or `redo`. Its leader is the task's recorded leader,
  `leader_id` on the `tasks` row.

Both then take the same three branches:

- The leader is not archived and is `running` or `initializing`: the run goes back to
  `queued` and is retried on a later tick. Nothing is sent into a turn already running.
- The leader is not archived and is idle: the run's task is sent to it with a blocking
  `paseo send <id> --prompt-file <tmp> --json`, its mode is checked first, and its
  outcome is handled as any other turn's.
- The leader is archived, cannot be inspected, or (for `follow_up_of`) was never
  started: a new leader starts as for any run. For a wd follow-up its prompt is the
  original task's text (the run whose `request_id` is the bare corr), a blank line, then
  the follow-up's own text, and the task records the new leader. For a `follow_up_of`
  run its prompt is the task texts of the chain from the root to the referenced run,
  each followed by a blank line, then this task; the run's transcript opens with a line
  naming the leader that was gone, and its pane id names the new one.

The run's pane id becomes `paseo:<agent id>` as soon as the agent id is known.

No leader is archived when its run ends `done`, `lost` or `blocked`: it stays so that a
follow-up sent soon after lands on the same leader with its context (leader-lifecycle
D6). A leader is archived on three paths only: wd `accept`, handoff retirement, and the
idle sweep. Every 5 minutes the delivery loop archives a waggledance leader, wd leaders
included, only when it is not archived, not `running` or `initializing`, has no pending
permission, its Paseo `UpdatedAt` is strictly older than the limit, it carries no
`queued`, `working` or `blocked` run, and no run in the family of any run it carries is
`queued`, `working` or `blocked` (leader-lifecycle D5). The limit is `[paseo]
leader_idle_minutes` (default 30) from the global config, or from the `.waggledance.toml`
of the project its newest carried run belongs to. A leader with no `UpdatedAt` is never
archived. `waggledance doctor` shows the limit in force on its `leader idle close` row.

### Leader mode check

A leader runs in the mode waggledance asked for, checked from the agent's runtime mode
(paseo-leader-mode D3/D4). The check runs while a new leader's first turn runs, as
soon as its agent id appears.

1. waggledance reads `runtimeInfo.modeId` from Paseo's agent record,
   `<PASEO_HOME>/agents/<cwd-slug>/<id>.json`, read only. It reads once a second for
   up to 10 seconds until the value equals the asked mode.
2. When it still differs, waggledance runs `paseo agent mode <id> <mode> --json` once
   and reads again the same way.
3. When it differs after that, the daemon log carries the warning "delivery leader runs
   in the wrong mode", and the run's reply opens with `[waggledance] leader <id> runs in
   mode <runtime>, asked <mode>; set it in Paseo`. An unreadable record reports the
   runtime mode as `unknown`.

The task is delivered either way. With no mode asked, nothing is read or set.

The check reads the agent record because `paseo inspect` prints `Mode` from
`currentModeId`, the configured mode, and that value can lie. The leader `ff24c29b` was
started with no `--mode`: it showed `currentModeId=bypassPermissions` while its
`runtimeInfo.modeId` was `default`, and it asked the human for every Bash call for about
9 hours.

### When a run is done

A run is done when the Paseo turn that carried its task ends, as Paseo reports it (D2).
waggledance never infers done from an idle status or from log text.

| Paseo answer | Run status |
|--------------|------------|
| the blocking call returns `completed` | `done` |
| the blocking call returns `permission` | `blocked`; once no permission is pending, `paseo wait` decides: `idle` → `done`, `permission` → `blocked` again |
| `turn_timeout_secs` passes, the call reports `timeout`, or the call fails after the send | the run stays `working` while `paseo inspect` shows the leader not archived, and `paseo wait` runs again; `lost`, "the leader <id> is gone without a result", once the leader is archived or three reads in a row fail |
| any other status, such as `error` | `lost`, "the leader may still be finishing" |

The `idle` that `paseo wait` reports after a permission is a turn end, not a guess from
idle state. After `permit allow`, the agent stays `running`, and `wait` blocks until the
turn ends. When the same turn asks for a second permission, `wait` reports `permission`
instead (piggery-lessons D5). The fixtures `inspect-after-allow.out`,
`wait-after-allow.out` and `wait-second-permission.out` pin all three answers.

The transcript of a `done` run is the leader's text after the last `[User] <first line of
the prompt sent>` in `paseo logs`. The prompt sent is the run's task, or the earlier
tasks plus this one when a follow-up starts a fresh leader. Paseo 0.11.2 ignores
`logs --since`, so the reply is found by that line, not by time. When no reply is
found, the transcript says so and names the leader, so a `done` with an empty
transcript cannot happen.

The run row records its leader (`paseo:<id>`) as soon as the leader is found, even
when the blocking `paseo run` call fails or times out. A follow-up therefore sees that
the earlier run started a leader, waits while that run is still `working`, and then
goes to the same leader.

### Failures, timeouts, and restarts

- **Before the send.** A failure to resolve the leader, a missing binary, or an
  unreachable Paseo puts the run back to `queued` with backoff (10 s, 30 s, 1 min, 5 min).
  The fifth failure marks it `lost` with the error (D6).
- **After the send.** A timeout keeps the run `working` while its leader is alive (see
  the table above), and the leader is left alone. Killing a blocking
  send does not end the leader's turn, so the task is never sent again (D6). Only a
  failure to start the binary or to write the prompt file counts as "before the send";
  any other send error is treated as sent, so a request is never delivered twice.
- **Daemon restart.** A run claimed but not yet sent (still `paseo:pending`) goes back to
  `queued`. A run already sent to a known leader is watched again: waggledance checks
  the leader and waits for its turn end, so it ends `done` or `lost` by the same rules,
  and is never re-sent (D6). A `blocked` run with no recorded leader becomes `lost` with
  "daemon restarted — the leader may still be finishing; check its chat".

## Run status

`waggledance_await` and `waggledance_runs` read a leader run from the runs table only;
they run no `paseo` command.

| Status | Meaning |
|--------|---------|
| `queued` | accepted, waiting for delivery; `waggledance_await` adds its age in seconds (D8) |
| `working` | delivered, or being delivered; the leader's turn has not ended |
| `blocked` | the leader waits on a permission in its Paseo chat; the human answers it there |
| `done` | the turn ended; the transcript is the leader's reply |
| `lost` | the leader is gone without a result, or Paseo reported an error status; the transcript says why |

The board shows a leader that needs the human (piggery-lessons D6). A leader run that
is `blocked`, or `lost` within the last hour, is a leader wait. `GET /api/board` lists
these waits as `leader_waiting: [{project_id, project_name, run_id, status, since,
url}]`, newest first, and the project's home rail row shows one pill:
"Leader waiting — permission" or "Leader lost — may still be finishing". When both kinds
are present, the blocked pill is the one shown. Both read the runs table only.

## Caller notices

When a run becomes `done`, `lost`, or `blocked`, waggledance tells the caller without
being asked (D4):

- through the existing notify path, and
- when the run has a caller that is not the leader itself and not an `openclaw:`
  caller, with one `paseo send --no-wait` to that agent. The notice goes out only while the caller is idle (D5),
  and is retried for up to an hour. Notices for one caller are combined into a single
  message per tick, followed by 30 seconds of quiet, because Paseo still reports a
  just-messaged agent as idle for a few seconds.

The notice is stored on its run row, not held in memory (piggery-lessons D7):

- The row's `notice`, `notice_state` and `notice_until` columns hold it (runs
  migration 6).
- The run's status, final transcript and notice are written in one statement.
- A notice is `pending` until it is sent (`sent`) or its hour runs out (`dropped`), so
  a daemon restart keeps it and an expired one leaves a record.
- A newer notice for the same run replaces one that is still pending. A `blocked`
  notice that was not delivered before the run ended is therefore superseded by the
  `done` notice.
- Only the 30-second quiet per caller stays in memory.

The notice text:

- done or lost: `[waggledance] <project> leader finished run <run_id> (<status>) —
  informational, no action needed; read it with waggledance_await {"run_id": "<run_id>"}.`
- blocked: `[waggledance] <project> leader is waiting for a permission in its Paseo chat
  (run <run_id>) — approve or deny it there.`

A caller that is a Paseo agent can wait for this notice instead of polling
`waggledance_await`. The Paseo plugin's request box is not a Paseo agent; it polls.

## Workspace discovery

The daemon registers every Paseo workspace root as a waggledance project
(paseo-auto-wiring D1). Discovery runs inside the delivery task when it starts and every
5 minutes after. It reads `paseo workspace ls --json` and Paseo's own store,
`~/.paseo/projects/workspaces.json`, read only.

A workspace root is registered only when all of these hold (D6):

- its `isolation` is `local`, and it is neither archived nor a Paseo-owned worktree;
- it is a git repo whose `.git` is a directory, so a git worktree is skipped;
- it is not `$HOME`, not `/`, and not under `/tmp`;
- no registered project's root contains it or sits inside it.

A discovered project takes dispatched work at once, unless its `.waggledance.toml`
says `orchestration = false` (paseo-dispatch-default-on D1).

## Workspace resolution

A workspace is resolved each time a new leader starts, which is once per run. The first
rule that matches wins (paseo-auto-wiring D3/D7):

1. `[paseo] workspace` in the project's `.waggledance.toml` — reason `file`.
2. A deprecated `[[paseo.leaders]]` entry whose `project` equals the project's id or
   name — reason `deprecated-entry`.
3. Among the workspaces from `paseo workspace ls --json` whose `isolation` is `local`
   and whose real cwd equals the project root's real path: the oldest by `createdAt` in
   Paseo's store — reason `oldest`; with no `createdAt` known, the lowest workspace id —
   reason `lowest-id`.
4. With no match, a new workspace from `paseo workspace create --path <project root>
   --isolation local` — reason `created`.

Several workspaces at one root are never refused. The run's transcript opens with
`[waggledance] leader started in Paseo workspace <id> (<reason>)`.

## Which paseo binary runs

waggledance tries candidates in this order and takes the first that answers
`paseo ls -g --json` with a JSON list (paseo-auto-wiring D4/D7):

1. `[paseo] binary`, when it is set and is not an AppImage wrapper.
2. The npm CLI (`@getpaseo/cli`) on `PATH`, then under
   `~/.local/share/mise/installs/node/<version>/bin/`, newest version first.
3. `[paseo] binary` when it is an AppImage wrapper and an npm CLI was found — demoted
   below the npm CLI.
4. `$PASEO_CLI`, when it names an existing file. Paseo sets it to its bundled CLI inside
   every agent it runs.
5. `paseo` on `PATH`.

A candidate that resolves to the same file as an earlier one is tried once. A successful
<!-- bee:not-a-deferral: describes the detection cache behaviour, not a promise to act later -->
detection is cached for 10 minutes; a failed one is not cached, so a CLI installed later
is found on the next call.
<!-- /bee:not-a-deferral --> With no candidate working, a leader dispatch is refused
naming every rejected candidate with why, and how to install the CLI.

The AppImage wrapper is demoted because its `run` printed nothing from the daemon's
systemd scope while the npm CLI worked (paseo-auto-wiring D4).

Every `paseo` child process drops `PASEO_AGENT_ID` and `PASEO_AGENT_CWD` and keeps every
other variable, `PASEO_HOME` included. With those two set, a new agent becomes a child of
the calling agent, and `paseo run` attaches it to the caller's workspace.

## A held leader keeps its run working

Read when: a leader ends its turn while work it handed out still runs, and its run reads done too soon, its leader is archived, or the heartbeat nudges it.

The leader declares its own wait with `waggledance hold --why <reason> [--minutes N]` (it reads `PASEO_AGENT_ID`, exits 2 naming the variable when absent) and releases it with `--release`. A hold lasts at most `[paseo] hold_max_minutes` (default 120, the 2-hour ceiling) and `--minutes` is capped to it; waggledance never infers delegated work from a provider timeline (D1, D2). A `result` mail from the leader also clears the hold (D4).

A leader is held while it has an unexpired `waggledance hold` row or a Paseo child whose `ParentAgentId` is the leader, running or initializing (`paseo::held_work`). `paseo ls` has no parent field, so the children lookup lists running agents and inspects each; it runs only when a decision needs it, and a hold row decides first.

A held leader's completed or idle turn end leaves the run `working` and polls `inspect` every `permission_poll` until `UpdatedAt` moves. Paseo's `wait` is never called on an idle leader, and a wake turn is never caught by seeing `running` (polling status every 5 s missed both). When the hold ends the run ends as before; an expiry adds `hold expired after N min (why)` to the transcript header. The idle sweep and the heartbeat skip a held leader. Every leader prompt but a steer note ends with the hold line, appended at send time so the stored task stays the caller's.

A leader turn after its run ended `done` or `lost` is a late reply (D5), found by the five-minute sweep (`pass_on_late_reply`): the newest run the leader carries, an unarchived settled leader whose `UpdatedAt` is later than that run's end, no `late reply` line in the transcript yet. `late_reply_at` is stamped but never read back, because the engine has no reader and the transcript line is the once-only mark. An empty turn records nothing and is read again next sweep. An `openclaw:` caller gets one `progress` mail headed `late reply`, skipped when the task's `last_mail_at` is after the run's end so a result the leader already mailed is not repeated; any other caller gets a run notice naming the late reply.

## Known limit

A message a person types into a leader's Paseo chat after its run ended `done` gets a reply that is passed on once as a late reply, not as a fresh run: the caller reads it as a late reply notice, and the person's own chat still shows the reply.

A human who types to the leader while a delivery is in flight interrupts that turn —
Paseo's own behaviour for a message sent into a running turn. The blocking send then
returns `completed` for the turn that ended, and the run is marked `done` with whatever
the leader wrote. waggledance never interrupts a turn itself (D5).

## Sources

Each source names the project itself:

- **The Waggle Dance Paseo plugin.** The request box in the workspace panel spawns
  `waggledance mcp` over stdio and calls `waggledance_dispatch` with the panel's project,
  the typed task, `leader: true`, and one `request_id` per draft.
- **External MCP agents.** Any MCP client that runs `waggledance mcp` can still call the
  same tool with the same arguments.
- **wd.** `wd-bridge.py handoff` runs `waggledance mcp` with `WAGGLEDANCE_CALLER=wd` and
  passes the task's corr as `request_id`; see
  [wd hands a task straight to a project's leader](wd-direct-leader.md).

## Pointers

- `crates/waggledance-core/src/config.rs` — `PaseoConfig` (`enabled`,
  `turn_timeout_secs`), `PaseoLeader`, `workspace_for`.
- `crates/waggledance-core/src/project_settings.rs` — `.waggledance.toml`: keys, parse,
  warnings. `Engine::orchestration_allowed` and `Engine::project_settings` in
  `crates/waggledance-core/src/engine.rs` turn it into the effective dispatch switch.
  `dispatch_off` in `crates/waggledance/src/mcp.rs` writes the refusal.
- `crates/waggledance/src/discovery.rs` — workspace discovery and its filters.
- `crates/waggledance/src/setup.rs` — `waggledance setup paseo` and its leftover-line
  warnings.
- `crates/waggledance-core/src/repository.rs` — runs migration 5: `request_id`,
  `caller`, `attempts`, the queue and claim functions. Migration 6 and its functions:
  `notice`, `notice_state`, `notice_until`, `finish_run`, `pending_notices`,
  `set_notice_state` and `list_leader_waits`.
- `crates/waggledance/src/server.rs` — `api_board` with `leader_waiting`, and
  `leader_waits_by_project` for the home rail. `views.rs` holds `proj_row_bee` and
  `leader_wait_label`.
- `crates/waggledance/src/paseo.rs` — the `paseo` CLI client, `candidates`, `detect`,
  `paseo_available`, the environment scrub, `TurnEnd`, labels, lock, `resolve_leader`,
  `pick_workspace`, `delivery_heartbeat_path`, and the mode check: `leader_mode`,
  `record_mode`, `ensure_mode`, `ModeCheck`.
- `crates/waggledance/src/mcp.rs` — `request_id`, `dispatch_to_leader`, and the paused
  warning.
- `crates/waggledance/src/delivery.rs` — the delivery loop, restart recovery, caller
  notices, and `check_mode`, which reports a leader still in the wrong mode.
- `crates/waggledance/testdata/fixtures/paseo-0.11.2/` — the captured Paseo answers the
  client is tested against, with a README per file.
- `integrations/paseo/server/dispatch.ts` and `integrations/paseo/client/composer.tsx` —
  the plugin's request box.
- `docs/waggledance-supervisor-skill-template.md` — the supervisor skill's leader mode,
  which hands a task through this door.
- `docs/knowledge/patterns/existence-is-not-evidence.md` — why every Paseo behaviour the
  client relies on has a capture.
