---
type: bee.pattern
title: A request for a project reaches its one Paseo leader through waggledance_dispatch
description: "Practice: with [paseo] enabled and the project opted into orchestration, waggledance_dispatch with leader: true sends the task to the project's labelled Paseo leader, or starts one in the configured workspace, and tracks it as a durable run read through waggledance_await and waggledance_runs."
timestamp: 2026-10-14
bee:
  id: paseo-leader-dispatch
  lifecycle: active
  areas: [orchestration]
  sources: [docs/history/paseo-leader-dispatch/CONTEXT.md, docs/history/paseo-leader-dispatch/plan.md]
  polarity: practice
  signature: request for a project running in Paseo
---

# A request for a project reaches its one Paseo leader through waggledance_dispatch

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
- `binary` defaults to `paseo` on `PATH`. `provider`, `mode`, and `thinking` default to
  empty, which leaves the choice to Paseo's own defaults. Each one is passed to
  `paseo run` only when set.

The project must also be opted into orchestration from its settings page. A project that
is not opted in is refused before any `paseo` command runs. `terminal.enabled` does not
gate the leader path, and the leader path never talks to herdr.

## Dispatch

A caller sends `{project, task, leader: true}` to `waggledance_dispatch`. Exactly one of
`preset`, `pane_id`, and `leader: true` is accepted. The task text is sent unchanged.

1. waggledance takes a per-project lock file in `<data dir>/paseo-locks/` and holds it
   from the lookup through `paseo run`, so two requests cannot start two leaders.
2. It lists agents with `paseo ls -g --label waggledance.role=leader --label
   waggledance.project=<project id> --json` and inspects each one. The first agent that
   is not archived is the leader.
3. If the leader's status is `running` or `initializing`, the call is refused with the
   leader's id: "is busy — refusing to interrupt its current run; retry after it ends".
   Nothing is queued.
4. If the leader is idle, the task goes to it through `paseo send <id> --prompt-file
   <temp file> --no-wait`. The result says `leader: "reused"` and `workspace` is null.
5. If there is no leader, waggledance resolves the workspace and starts one with
   `paseo run -d -q --workspace <id> --cwd <project root> --title "<project name>
   leader" --label waggledance.role=leader --label waggledance.project=<project id> --
   <task>`. The result says `leader: "started"` and names the workspace.

The call returns `{run_id, warnings, agent_id, workspace, leader}`. The run is stored in
the same runs table as herdr runs, with pane id `paseo:<agent id>` and no preset label.

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

## Run status

`waggledance_await` and `waggledance_runs` read a leader run the same way as a herdr
run. Each poll inspects the agent with `paseo inspect <id> --json`:

| Paseo state | Run status |
|-------------|------------|
| archived | `lost` |
| one or more pending permissions | `blocked` — the leader waits on a permission in Paseo |
| `idle`, with leader text after the last `[User] <task>` line, on two polls in a row | `done`, and that text is the delta |
| `idle` with no reply yet, `running`, or `initializing` | `working` |
| any other status | `working`, with a warning that names the status |
| the await timeout (at most 60 s) passes first | `working` |

The reply is read from `paseo logs <id> --filter text`. Paseo 0.11.2 ignores
`--since`, so the reply is the text after the last `[User] <task>` line that matches the
dispatched task. A leader run also ends on its own when nobody awaits it.

## Sources

Three sources reach the same door, and each one names the project itself:

- **The waggledance agent in Paseo.** The `waggledance-supervisor` skill dispatches with
  `leader: true` in its leader mode.
- **The Waggle Dance Paseo plugin.** The request box in the workspace panel spawns
  `waggledance mcp` over stdio and calls `waggledance_dispatch` with the panel's project,
  the typed task, and `leader: true`.
- **External MCP agents.** openclaw, hermes, aicoworker, and any other MCP client that
  runs `waggledance mcp` call the same tool with the same three arguments.

## Pointers

- `crates/waggledance-core/src/config.rs` — `PaseoConfig`, `PaseoLeader`,
  `workspace_for`.
- `crates/waggledance/src/paseo.rs` — the `paseo` CLI client, labels, lock, and
  `resolve_leader`.
- `crates/waggledance/src/mcp.rs` — the `leader` argument, `dispatch_to_leader`, and
  `leader_dispatch`.
- `crates/waggledance/src/orchestrate.rs` — `observe_paseo_run` and `await_paseo_run`.
- `crates/waggledance/src/reaper.rs` — the sweep that ends leader runs nobody awaits.
- `integrations/paseo/server/dispatch.ts` and `integrations/paseo/client/composer.tsx` —
  the plugin's request box.
- `docs/waggledance-supervisor-skill-template.md` — the supervisor skill's leader mode.
- `docs/history/paseo-leader-dispatch/CONTEXT.md` — the locked decisions.
