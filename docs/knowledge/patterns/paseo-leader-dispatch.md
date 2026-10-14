---
type: bee.pattern
title: A request for a project reaches its one Paseo leader through waggledance_dispatch
description: "Practice: with [paseo] enabled and the project opted into orchestration, waggledance_dispatch with leader: true queues the task for the project's labelled Paseo leader; the daemon delivers it once, in order, only while the leader is idle, marks it done when Paseo reports that turn completed, and tells the caller."
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch
  lifecycle: active
  areas: [orchestration]
  sources: [docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md, docs/history/leader-delivery/CONTEXT.md, docs/history/leader-delivery/plan.md]
  polarity: practice
  signature: request for a project running in Paseo
---

# A request for a project reaches its one Paseo leader through waggledance_dispatch

Locked by paseo-leader-dispatch D1/D2/D3/D4/D5/D6/D8/D9/D10
(`docs/history/paseo-leader-dispatch/CONTEXT.md`) and
leader-delivery D1/D2/D3/D4/D5/D6/D7/D8 (`docs/history/leader-delivery/CONTEXT.md`). Leader-delivery D3
supersedes paseo-leader-dispatch D7: a busy leader no longer refuses a request.

## Turn it on

Add this block to `~/.waggledance/config.toml` on the host that runs the daemon:

```toml
[paseo]
enabled = true

[[paseo.leaders]]
project = "waggledance"
workspace = "wks_0123456789ab"
```

- `[paseo] enabled = true` is required. Without it, a leader dispatch is refused with a
  message that names the config file and the lines `[paseo]` and `enabled = true`.
- `[[paseo.leaders]]` is optional and repeatable. `project` matches a project's id or
  its name. `workspace` is a Paseo workspace id. An entry with an empty `workspace` is
  ignored.
- `turn_timeout_secs` defaults to `3600`. It bounds how long one delivery waits for the
  leader's turn to end.
- `provider`, `mode`, and `thinking` default to empty, which leaves the choice to Paseo's
  own defaults. Each one is passed to `paseo run` only when set.
- `binary` picks the `paseo` CLI. See [Which paseo binary runs](#which-paseo-binary-runs).

The project must also be opted into orchestration from its settings page. A project that
is not opted in is refused before anything is stored. `terminal.enabled` does not gate
the leader path, and the leader path never talks to herdr.

The waggledance daemon (`waggledance serve`) must be running. It delivers queued
requests; `waggledance mcp` only accepts them.

## Dispatch

A caller sends `{project, task, leader: true, request_id}` to `waggledance_dispatch`.
Exactly one of `preset`, `pane_id`, and `leader: true` is accepted. The task text is
sent unchanged. `request_id` is optional; pass one on every call.

1. If a run of this project already carries the same `request_id`, the call returns
   `{run_id, status, duplicate: true}` for that run and sends nothing (D1).
2. The orchestration opt-in and `[paseo] enabled` are checked, in that order.
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

The daemon runs the delivery loop whenever `[paseo] enabled` is set, and writes the
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
| the blocking call returns `permission` | `blocked`; once no permission is pending and the leader is idle, `done` |
| `turn_timeout_secs` passes, or any other status | `lost`, "the leader may still be finishing" |

The transcript of a `done` run is the leader's text after the last `[User] <first line of
the task>` in `paseo logs`. Paseo 0.11.2 ignores `logs --since`, so the reply is found by
that line, not by time.

### Failures, timeouts, and restarts

- **Before the send.** A failure to resolve the leader, a missing binary, or an
  unreachable Paseo puts the run back to `queued` with backoff (10 s, 30 s, 1 min, 5 min).
  The fifth failure marks it `lost` with the error (D6). Two matching workspaces with no
  `[[paseo.leaders]]` entry fail this way, and the `lost` run carries the refusal text.
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

## Caller notices

When a run becomes `done`, `lost`, or `blocked`, waggledance tells the caller without
being asked (D4):

- through the existing notify path, and
- when the run has a caller that is not the leader itself, with one `paseo send
  --no-wait` to that agent. The notice goes out only while the caller is idle (D5),
  after the status is written, and is retried for up to an hour. Notices for one caller
  are combined into a single message per tick, followed by 30 seconds of quiet, because
  Paseo still reports a just-messaged agent as idle for a few seconds.

The notice text:

- done or lost: `[waggledance] <project> leader finished run <run_id> (<status>) —
  informational, no action needed; read it with waggledance_await {"run_id": "<run_id>"}.`
- blocked: `[waggledance] <project> leader is waiting for a permission in its Paseo chat
  (run <run_id>) — approve or deny it there.`

A caller that is a Paseo agent can wait for this notice instead of polling
`waggledance_await`. The Paseo plugin's request box is not a Paseo agent; it polls.

## Workspace resolution

A workspace is resolved only when a new leader starts. The first rule that matches wins:

1. A `[[paseo.leaders]]` entry whose `project` equals the project's id or name.
2. The single workspace from `paseo workspace ls --json` whose `isolation` is `local`
   and whose real cwd path equals the project root's real path.
3. With no match, a new workspace from `paseo workspace create --path <project root>
   --isolation local`.

More than one match in rule 2 with no `[[paseo.leaders]]` entry is refused. The
refusal names every matching workspace id and the `[[paseo.leaders]]` lines to add.
waggledance never guesses between two workspaces.

## Which paseo binary runs

The binary is chosen on every call, first match wins:

1. `[paseo] binary` when it is not empty.
2. `$PASEO_CLI` when it is set and names an existing file.
3. `paseo` on `PATH`.

Paseo sets `$PASEO_CLI` to its bundled CLI inside every agent it runs. The
`~/.local/bin/paseo` wrapper that launches the Paseo AppImage, run from inside an agent,
prints only startup logs and creates no agent; from the daemon's systemd scope it works.
So `waggledance mcp` started by a Paseo agent uses the bundled CLI, and the daemon falls
back to `paseo`.

Every `paseo` child process drops `PASEO_AGENT_ID` and `PASEO_AGENT_CWD` and keeps every
other variable, `PASEO_HOME` included. With those two set, a new agent becomes a child of
the calling agent, and `paseo run` attaches it to the caller's workspace.

## Known limit

A human who types to the leader while a delivery is in flight interrupts that turn —
Paseo's own behaviour for a message sent into a running turn. The blocking send then
returns `completed` for the turn that ended, and the run is marked `done` with whatever
the leader wrote. waggledance never interrupts a turn itself (D5).

## Sources

Three sources reach the same door, and each one names the project itself:

- **The waggledance agent in Paseo.** The `waggledance-supervisor` skill dispatches with
  `leader: true` and its correlation id as `request_id` in its leader mode.
- **The Waggle Dance Paseo plugin.** The request box in the workspace panel spawns
  `waggledance mcp` over stdio and calls `waggledance_dispatch` with the panel's project,
  the typed task, `leader: true`, and one `request_id` per draft.
- **External MCP agents.** openclaw, hermes, aicoworker, and any other MCP client that
  runs `waggledance mcp` call the same tool with the same arguments.

## Pointers

- `crates/waggledance-core/src/config.rs` — `PaseoConfig` (`turn_timeout_secs`),
  `PaseoLeader`, `workspace_for`.
- `crates/waggledance-core/src/repository.rs` — runs migration 5: `request_id`,
  `caller`, `attempts`, the queue and claim functions.
- `crates/waggledance/src/paseo.rs` — the `paseo` CLI client, `resolve_binary`, the
  environment scrub, `TurnEnd`, labels, lock, `resolve_leader`, and
  `delivery_heartbeat_path`.
- `crates/waggledance/src/mcp.rs` — `request_id`, `dispatch_to_leader`, and the paused
  warning.
- `crates/waggledance/src/delivery.rs` — the delivery loop, restart recovery, and caller
  notices.
- `crates/waggledance/testdata/fixtures/paseo-0.11.2/` — the captured Paseo answers the
  client is tested against, with a README per file.
- `integrations/paseo/server/dispatch.ts` and `integrations/paseo/client/composer.tsx` —
  the plugin's request box.
- `docs/waggledance-supervisor-skill-template.md` — the supervisor skill's leader mode.
- `docs/knowledge/patterns/existence-is-not-evidence.md` — why every Paseo behaviour the
  client relies on has a capture.
