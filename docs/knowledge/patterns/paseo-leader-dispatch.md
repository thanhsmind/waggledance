---
type: bee.pattern
title: A request for a project reaches its one Paseo leader through waggledance_dispatch
description: "Practice: with a working Paseo CLI found and the project opted into orchestration (its .waggledance.toml or its settings page), waggledance_dispatch with leader: true queues the task for the project's labelled Paseo leader; the daemon delivers it once, in order, only while the leader is idle, marks it done when Paseo reports that turn completed, and tells the caller."
timestamp: 2026-10-16
bee:
  id: paseo-leader-dispatch
  lifecycle: active
  areas: [orchestration]
  sources: [docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md, docs/history/leader-delivery/CONTEXT.md, docs/history/leader-delivery/plan.md, docs/history/paseo-auto-wiring/CONTEXT.md, docs/history/paseo-auto-wiring/plan.md, docs/history/piggery-lessons/CONTEXT.md, docs/history/piggery-lessons/plan.md]
  polarity: practice
  signature: request for a project running in Paseo
---

# A request for a project reaches its one Paseo leader through waggledance_dispatch

Locked by paseo-leader-dispatch D1/D2/D3/D4/D5/D6/D8/D9/D10
(`docs/history/paseo-leader-dispatch/CONTEXT.md`),
leader-delivery D1/D2/D3/D4/D5/D6/D7/D8 (`docs/history/leader-delivery/CONTEXT.md`), and
paseo-auto-wiring D1/D2/D3/D4/D5/D6/D7 (`docs/history/paseo-auto-wiring/CONTEXT.md`).
Leader-delivery D3 supersedes paseo-leader-dispatch D7: a busy leader no longer refuses a
request. Paseo-auto-wiring D3 removes paseo-leader-dispatch D8's two-workspace refusal,
and paseo-auto-wiring D4 makes the global `[paseo]` block optional.

## Turn it on

Nothing goes in `~/.waggledance/config.toml`, and nothing is written by hand per
workspace (paseo-auto-wiring D1/D4). Three things turn a project's leader path on:

1. **A working Paseo CLI.** waggledance finds it itself; see
   [Which paseo binary runs](#which-paseo-binary-runs). Paseo support is on as soon as
   one candidate answers `ls -g --json` with a JSON list.
2. **The project is registered.** The daemon registers every Paseo workspace root it
   finds; see [Workspace discovery](#workspace-discovery).
3. **The project opts in.** Commit a `.waggledance.toml` with `orchestration = true` at
   the repo root, or turn orchestration on from the project's settings page (D2). A
   project that is not opted in is refused before anything is stored.

The waggledance daemon (`waggledance serve`) must be running. It discovers workspaces
and delivers queued requests; `waggledance mcp` only accepts them.

### .waggledance.toml

The per-project settings file sits at the repo root and is committed with the code
(paseo-auto-wiring D2). Every key is optional:

```toml
orchestration = true

[paseo]
workspace = "wks_0123456789ab"
provider = "claude"
mode = "bypassPermissions"
thinking = "high"
```

- `orchestration` — `true` opts the project into dispatched work with exactly the scope
  of the settings-page opt-in, triggers included: the project is opted in when either
  the settings page or the file says so (D5). Absent or `false` leaves the settings page
  as the only switch.
- `[paseo] workspace` — the Paseo workspace id a new leader starts in. It wins over
  every other rule in [Workspace resolution](#workspace-resolution).
- `[paseo] provider`, `mode`, `thinking` — passed to `paseo run` for a new leader. A
  value set here wins over the same key in the global `[paseo]` block; an unset one
  falls back to it. `mode` and `thinking` unset are left to Paseo. `provider` is never
  left unset, because `paseo run` refuses without one (`MISSING_PROVIDER`): with no
  provider set anywhere, waggledance uses the first provider `paseo provider ls --json`
  reports as available and enabled, and Paseo picks that provider's default model
  (paseo-auto-wiring D8). Paseo reports such CLI errors as a JSON `error` object on
  stderr; waggledance shows its code and message.

The file is re-read when its modification time changes. A file that fails to parse is
ignored as a whole, and the not-opted-in refusal names the settings page, the file path,
and the parse error. An unknown key is ignored with a warning; the rest of the file
still counts.

**Accepted risk (D5):** anyone who can change a repo checkout can opt that project into
dispatched work by writing this file. The owner chose in-repo consent; there is no
second confirmation on the host.

### The global [paseo] block

The `[paseo]` block in `~/.waggledance/config.toml` is optional and only overrides (D4):

- `enabled = false` turns Paseo support off, and a leader dispatch is refused naming
  that line. `enabled = true` is no longer needed.
- `binary` names a `paseo` CLI to try first; see
  [Which paseo binary runs](#which-paseo-binary-runs).
- `turn_timeout_secs` defaults to `3600`. It bounds how long one delivery waits for the
  leader's turn to end.
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
`openclaw:wd`. Its runs go only to leaders labelled `waggledance.via=wd`, and every other
caller's runs skip those leaders. A wd run sends no caller notice: its outcome reaches
wd through the waggledance inbox. See
[wd hands a task straight to a project's leader](wd-direct-leader.md).

A caller sends `{project, task, leader: true, request_id}` to `waggledance_dispatch`.
Exactly one of `preset`, `pane_id`, and `leader: true` is accepted. The task text is
sent unchanged. `request_id` is optional; pass one on every call.

1. If a run of this project already carries the same `request_id`, the call returns
   `{run_id, status, duplicate: true}` for that run and sends nothing (D1).
2. The orchestration opt-in and Paseo availability are checked, in that order.
3. The request is stored as a run with status `queued`, pane id `paseo:pending`, and the
   caller's `PASEO_AGENT_ID` when the dispatch came from a Paseo agent. No `paseo`
   command runs inside the call.
4. The call returns `{run_id, status: "queued", duplicate: false, warnings}`. A busy
   leader is not a refusal: the request waits its turn (D3).

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

## Delivery

The daemon runs the delivery loop when it finds Paseo available, and writes the
heartbeat on every tick. For each project it delivers at most one request at a time,
oldest first, holding the per-project lock file in `<data dir>/paseo-locks/` for the
whole delivery so two deliveries cannot start two leaders (D3).

waggledance never sends to a Paseo agent whose status is `running` or `initializing`
(D5). A project waits while its leader is running, including a turn a human started,
and its next request goes out once the leader is idle.

A delivery finds the leader by its labels (`waggledance.role=leader`,
`waggledance.project=<project id>`), the first agent that is not archived:

- **A leader exists.** The task goes to it through a blocking `paseo send <id>
  --prompt-file <temp file> --json`, which returns when the leader's turn ends.
- **No leader.** waggledance resolves the workspace and starts one with a blocking
  `paseo run --json --workspace <id> --cwd <project root> --title "<project name>
  leader" --label waggledance.role=leader --label waggledance.project=<project id> --
  <task>`. The task is the leader's first turn; there is no bootstrap turn (D8).

The run's pane id becomes `paseo:<agent id>` as soon as the agent id is known.

### When a run is done

A run is done when the Paseo turn that carried its task ends, as Paseo reports it (D2).
waggledance never infers done from an idle status or from log text.

| Paseo answer | Run status |
|--------------|------------|
| the blocking call returns `completed` | `done` |
| the blocking call returns `permission` | `blocked`; once no permission is pending, `paseo wait` decides: `idle` → `done`, `permission` → `blocked` again |
| `turn_timeout_secs` passes, or any other status | `lost`, "the leader may still be finishing" |

The `idle` that `paseo wait` reports after a permission is a turn end, not a guess from
idle state. After `permit allow`, the agent stays `running`, and `wait` blocks until the
turn ends. When the same turn asks for a second permission, `wait` reports `permission`
instead (piggery-lessons D5). The fixtures `inspect-after-allow.out`,
`wait-after-allow.out` and `wait-second-permission.out` pin all three answers.

The transcript of a `done` run is the leader's text after the last `[User] <first line of
the task>` in `paseo logs`. Paseo 0.11.2 ignores `logs --since`, so the reply is found by
that line, not by time.

### Failures, timeouts, and restarts

- **Before the send.** A failure to resolve the leader, a missing binary, or an
  unreachable Paseo puts the run back to `queued` with backoff (10 s, 30 s, 1 min, 5 min).
  The fifth failure marks it `lost` with the error (D6).
- **After the send.** A timeout marks the run `lost` "the leader may still be
  finishing", and the project's queue holds until its leader is idle. Killing a blocking
  send does not end the leader's turn, so the task is never sent again (D6). Only a
  failure to start the binary or to write the prompt file counts as "before the send";
  any other send error is treated as sent, so a request is never delivered twice.
- **Daemon restart.** A run claimed but not yet sent (still `paseo:pending`) goes back to
  `queued`. A run already sent becomes `lost` with "daemon restarted — the leader may
  still be finishing; check its chat", is never re-sent, and its project holds until the
  leader is idle (D6).

## Run status

`waggledance_await` and `waggledance_runs` read a leader run from the runs table only;
they run no `paseo` command.

| Status | Meaning |
|--------|---------|
| `queued` | accepted, waiting for delivery; `waggledance_await` adds its age in seconds (D8) |
| `working` | delivered, or being delivered; the leader's turn has not ended |
| `blocked` | the leader waits on a permission in its Paseo chat; the human answers it there |
| `done` | the turn ended; the transcript is the leader's reply |
| `lost` | the turn's end was not seen; the transcript says why |

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

A discovered project is not opted in. It receives dispatched work only once its
`.waggledance.toml` or its settings page turns orchestration on (D2).

## Workspace resolution

A workspace is resolved only when a new leader starts; an existing leader is always
reused first. The first rule that matches wins (paseo-auto-wiring D3/D7):

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

## Known limit

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
  `crates/waggledance-core/src/engine.rs` combine it with the settings-page flag.
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
  `pick_workspace`, and `delivery_heartbeat_path`.
- `crates/waggledance/src/mcp.rs` — `request_id`, `dispatch_to_leader`, and the paused
  warning.
- `crates/waggledance/src/delivery.rs` — the delivery loop, restart recovery, and caller
  notices.
- `crates/waggledance/testdata/fixtures/paseo-0.11.2/` — the captured Paseo answers the
  client is tested against, with a README per file.
- `integrations/paseo/server/dispatch.ts` and `integrations/paseo/client/composer.tsx` —
  the plugin's request box.
- `docs/waggledance-supervisor-skill-template.md` — the supervisor skill's leader mode,
  which hands a task through this door.
- `docs/knowledge/patterns/existence-is-not-evidence.md` — why every Paseo behaviour the
  client relies on has a capture.
