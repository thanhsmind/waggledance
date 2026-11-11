<h1 align="center">Waggle Dance</h1>

<p align="center">
  <strong>A headless supervisor for a colony of AI coding agents.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange?logo=rust&logoColor=white">
  <img alt="Single binary" src="https://img.shields.io/badge/single-binary-brightgreen">
  <img alt="Works with Claude Code" src="https://img.shields.io/badge/works%20with-Claude%20Code%20(MCP)-8A63D2">
</p>

<p align="center">
  When a forager bee finds nectar, it doesn't carry the whole field home — it dances the
  <em>direction and the distance</em> so the rest of the colony can go straight there.
  Waggle Dance does that for your agents: it knows every project you run, hands work to the
  right project's leader, and tells you when something needs your answer.
</p>

---

## What it does

You are running agents across several repositories. Waggle Dance is the one process that
keeps track of them. It has no web page and no window: it is a daemon, a CLI and an MCP
server in one Rust binary.

- **Project registry.** The projects you register, kept in `~/.waggledance/registry.db`.
  Waggle Dance records where a project lives. It does not copy or index the project's files.
- **Dispatch to a project's leader.** A task goes to the one long-lived leader agent a
  project has, or to a fresh pane by preset, or to a pane that is already running. The
  caller gets a `run_id` and can wait on it.
- **Await, runs, task.** Block until a run finishes or needs a human, list runs, and follow
  up on a handed-out task: reply, redo, accept, or answer the leader's permission request.
- **Project state.** `waggledance_ask_state` reads each project's `.bee/` store and reports
  the feature, phase, open and blocked cells, sessions, handoff and what is waiting on you.
- **Bee upgrade.** Preview or apply a bee upgrade across every registered project that uses
  bee.
- **Wake, mail and notify.** Leaders report back with `waggledance mail send`. The daemon
  keeps those answers in an inbox, wakes the orchestrator once per message, nudges a silent
  leader, and sends status changes to Telegram.

Agents themselves run in [Paseo](https://github.com/getpaseo/paseo) or
[herdr](https://github.com/ogulcancelik/herdr). Waggle Dance dispatches to them and watches
them; it never runs a terminal of its own.

---

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/thanhsmind/waggledance/main/install.sh | sh
waggledance doctor --fix     # wire up the MCP server
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/thanhsmind/waggledance/main/install.ps1 | iex
waggledance doctor --fix     # wire up the MCP server
```

Or from source (needs Rust):

```sh
cargo install --git https://github.com/thanhsmind/waggledance waggledance
```

The CLI binary is `waggledance`.

---

## Use in 30 seconds

```sh
waggledance register ~/code/my-app     # add a project to the registry
waggledance serve                      # start the supervisor daemon
waggledance status                     # is it running?
```

`waggledance serve` runs the background loops (heartbeat, reaper, wake, trigger, notify,
worktree sweep and the bee status watch) and opens no network port. Read its state with
`waggledance status` and `waggledance doctor`.

---

## Agent integration (MCP)

```sh
waggledance doctor --fix
```

Registers an MCP server for whichever of **Claude Code, Codex, and Antigravity** it detects
on your machine — it never writes config for a tool you don't have. The server runs over
stdio (`waggledance mcp`) and exposes seven tools:

- **`waggledance_projects()`** → every registered project's `id`, `name`, `root_path` and
  `last_seen_at`.
- **`waggledance_ask_state(project?)`** → parsed `.bee/` state, no file reads required. Omit
  `project` for a rollup across every registered project; pass it for one project's full
  snapshot (feature, phase, mode, open/blocked/stuck cells, recent decisions, sessions,
  handoff, attention). A project with no `.bee/` reports absent, never an error. Also
  carries the project's `herding` agent **labels** (never the command behind a label,
  with the resolvable subset marked) and its own `panes` inventory with bee's state and
  feature joined in, so an orchestrator can reuse an idle pane instead of only ever
  spawning. `panes` is absent when the terminal feature is off, and null-with-a-reason
  when the session host can't be reached.
- **`waggledance_dispatch(project, task, preset? | pane_id? | leader?)`** → send a task to an
  agent pane in a managed project. Every project takes dispatched work unless its
  `.waggledance.toml` says `orchestration = false`. Either spawn a fresh pane by `preset`
  (any agent label the *target project* declares), target an already-running `pane_id`, or
  pass `leader: true` to hand it to the project's Paseo leader; the three are mutually
  exclusive. Returns a `run_id`.
- **`waggledance_await(run_id, timeout_seconds?)`** → block until that run completes,
  blocks on a human, or the timeout elapses (clamped to 60s server-side).
- **`waggledance_runs(project?)`** → list dispatched runs and their current status,
  read-only; narrow to one project with `project`.
- **`waggledance_task(corr, action, text?, req_id?)`** → follow up on a task handed to a
  project's leader. `reply` and `redo` queue the user's words for that leader; `accept`
  closes the task and archives its leader when nothing else uses it; `allow` and `deny`
  answer the leader's permission request.
- **`waggledance_bee_upgrade(apply?, projects?)`** → upgrade bee in the registered projects
  that already use it. Previews by default; `apply: true` runs `bee onboard --apply` and
  commits only bee's files, and requires a non-empty `projects` list the human approved.

Drop the snippet from [`docs/waggledance-agents-template.md`](docs/waggledance-agents-template.md)
into your project's `AGENTS.md` / `CLAUDE.md` so its agents know the supervisor tools exist.

### Agent CLI

Agents with a shell can drive the same tools through CLI verbs, which cost fewer tokens
than the MCP tool descriptions and answers. Each verb runs the handler its MCP tool runs:

```sh
waggledance projects                      # waggledance_projects
waggledance state [project]               # waggledance_ask_state
waggledance dispatch --project <id> (--leader | --preset <label> | --pane-id <id>) --task "..."
waggledance await <run_id> [--timeout N]  # waggledance_await
waggledance runs [--project <id>]         # waggledance_runs
waggledance runs show <run_id>            # one run with its full task
waggledance task <corr> reply|redo|accept|allow|deny [--text ...]
waggledance search "<what you want>"      # rank the verbs for a need
waggledance schema [verb]                 # verbs, flags and help as JSON
```

Output is JSON only: pretty on a terminal, compact when piped. A verb prints a summary by
default. `--full` prints the MCP tool's answer unchanged, `--fields a,b` keeps only those
keys on each row, and `--limit N` caps the rows. A tool error prints its message to stderr
and exits 1. The MCP tools stay as the fallback for agents without Bash; their run list
and all-projects state are trimmed to match (`waggledance_runs` rows carry `task_summary`,
and `run_id` returns one run's full `task`).

The binary is `waggledance`. You can make `wg` a symlink to it yourself, but no installer
creates one: `wg` is also the WireGuard command, and a link would shadow it.

### Paseo leaders

When waggledance runs beside Paseo, each project can have one long-lived leader agent in its
Paseo workspace. waggledance finds the Paseo CLI and registers every Paseo workspace as a
project on its own. `waggledance_dispatch` with `leader: true` queues the task; the running
daemon (`waggledance serve`) delivers it when that leader is idle, or starts one, and tells
the caller when the leader's turn ends. Pass a `request_id` so a retry never sends twice.
The detection order, the workspace rules, and the run statuses are in
[A request for a project reaches its one Paseo leader](docs/knowledge/patterns/paseo-leader-dispatch.md).

wd, the aicoworker agent on OpenClaw, uses the same door: `wd-bridge.py handoff` sends the
task to the project's leader, and the leader answers with `waggledance mail send`. The
daemon keeps those answers in an inbox and wakes wd's main session once per message. It also
nudges a silent leader, sends wd a board of open tasks, and relays each permission request
for wd to allow or deny. `waggledance setup openclaw` installs wd's side, and also installs
the bee skill set from the bee-onboarded repo it runs in into OpenClaw's global skills dir
(`~/.openclaw/skills`), so every OpenClaw agent can run the bee workflow;
`waggledance_ask_state` lists each project's `leaders`. The inbox, the wake, the follow-up
commands and the known gaps are in
[wd hands a task straight to a project's leader](docs/knowledge/patterns/wd-direct-leader.md).

The pi extension in `integrations/pi` and the OpenClaw bridge in `integrations/openclaw`
both talk to `waggledance mcp` over stdio. `waggledance setup pi` and
`waggledance setup openclaw` install them.

### Per-project settings: .waggledance.toml

Every managed project takes dispatched work with no file at all. A `.waggledance.toml`
committed at the repo root tunes it:

```toml
orchestration = false

[paseo]
workspace = "wks_0123456789ab"
```

`orchestration = false` is the only per-project off switch; an absent key or `true`
leaves dispatch on. A file that exists but cannot be read or parsed also turns dispatch
off until it is fixed. `terminal.enabled` still gates pane dispatch. `[paseo]` optionally
names the workspace a new leader starts in, and takes `provider`, `mode`, and `thinking` for
it. `mode` defaults to `bypassPermissions`. `leader_idle_minutes` (default 30, also settable
in the global `[paseo]` block) is how long an idle leader with nothing open in its family
stays before it is archived. Every key is described in
[.waggledance.toml](docs/knowledge/patterns/paseo-leader-dispatch.md#waggledancetoml).

---

## CLI

```sh
waggledance register <dir> [--name ...]   # add a project to the registry
waggledance list                          # list registered projects
waggledance unregister <project-id>       # remove a project (files kept)
waggledance serve                         # run the supervisor daemon
waggledance status                        # is the daemon up?
waggledance stop                          # stop the daemon
waggledance restart                       # restart the daemon (apply config changes)
waggledance doctor [--fix]                # diagnose & repair the integration
waggledance mcp                           # MCP server over stdio (agent tools call this)
waggledance projects | state | dispatch | await | runs | task   # agent verbs, JSON out
waggledance search <text> | schema [verb] # find an agent verb
waggledance setup <pi|openclaw>           # install an integration
waggledance mail send --to ... --corr ... --kind ...   # a leader reports back
waggledance hold --why ... [--minutes N]  # tell the daemon this leader is waiting
waggledance bee upgrade [--apply]         # upgrade bee across registered projects
waggledance config edit                   # edit ~/.waggledance/config.toml in $EDITOR
```

Most commands accept `--json`. Full reference and settings live in the
**[usage guide](docs/usage.md)**.

---

## How it works

One daemon owns the registry (`~/.waggledance/registry.db`) and a lock file
(`~/.waggledance/daemon.lock`) so only one runs at a time. It reads each project's `.bee/`
store for state, talks to Paseo and herdr to start and watch agents, and keeps the
leader mailbox and the dispatched-run list. `waggledance mcp` is a separate stdio process
that agents start on demand; it reads the same registry.

---

## Security

The daemon opens no HTTP listener, so there is no web surface to protect. Dispatching a task
to a pane is arbitrary input to an agent's shell: the terminal family is **off until you
switch it on** in `~/.waggledance/config.toml`, and a second switch is needed before agents
outside every registered project root become visible. Per project,
`orchestration = false` in `.waggledance.toml` turns dispatch off.

---

## Status

Actively developed. The supervisor daemon, the seven MCP tools, the CLI and `doctor` work
end-to-end. The board, the terminal page, the docs and code reader, search and the desktop
app were removed; see [PRD.md](PRD.md) for the original design, which predates that change.

---

## Credits

Waggle Dance grew out of a markdown viewer and kept its ancestry. It also absorbs
[herdr-go](https://github.com/vantt/herdr-go), the agent-terminal experiment that preceded
it, now retired.

## License

MIT — see [LICENSE](LICENSE).
