# waggledance — Usage Guide

Practical guide to installing and running the waggledance supervisor on Linux/macOS.
waggledance has no web page and no window: it is a daemon, a CLI and an MCP server in one
binary.

---

## 1. Install

### From a release (recommended, once published)
```bash
curl -fsSL https://raw.githubusercontent.com/thanhsmind/waggledance/main/install.sh | sh
```
The script detects your OS/arch, downloads the `waggledance` binary, and puts it on
your PATH. Check the latest release at
<https://github.com/thanhsmind/waggledance/releases>.

### From source (works anytime; needs Rust)
```bash
cargo install --git https://github.com/thanhsmind/waggledance waggledance
# or, inside a clone:
cargo build --release -p waggledance     # binary at target/release/waggledance
```

**Iterating on the code?** Use the `fast` profile instead. `release` is tuned for
the shipped binary — fat LTO plus `codegen-units = 1` force the final codegen onto
a single thread, so a one-line change costs ~43 s pinning one core. `fast` spreads
the work across every core:
```bash
cargo build --profile fast -p waggledance    # binary at target/fast/waggledance
```
Measured on a 16-core machine: ~43 s -> ~1 s for a one-line change (~39 s from a
cold target). The binary is larger (21 MB vs 14 MB) and marginally slower at
runtime, which is why releases still go out on `release`.

---

## 2. Run the supervisor

```bash
waggledance register /path/to/project     # add a project to the registry
waggledance list                          # list registered projects
waggledance serve                         # start the supervisor daemon
waggledance status                        # is it running?
```

`waggledance serve` runs the background loops — heartbeat, reaper, wake, trigger, notify,
worktree sweep and the bee status watch — and opens no network port. It writes
`~/.waggledance/daemon.lock`; `waggledance status` reads that file. Use
`waggledance doctor` for a full check.

Registering a project records where it lives. waggledance does not copy, index or watch the
project's files.

---

## 3. Agent integration (MCP)

```bash
waggledance doctor --fix     # register the MCP server for every agent tool present
waggledance doctor           # re-check: PATH, config, daemon, MCP per tool
```

`doctor --fix` sets up the MCP server for **whichever agent tools it detects** on
your machine — and never touches one you don't have (reported `SKIP`):

| Tool | Detected by | MCP config it writes (backed up first) |
|---|---|---|
| Claude Code | `~/.claude.json` / `~/.claude/` / `claude` on PATH | `~/.claude.json` (`mcpServers`) |
| Codex | `~/.codex/` / `codex` on PATH | `~/.codex/config.toml` (`[mcp_servers.waggledance]`, format-preserving) |
| Antigravity | `~/.gemini/config/` / `antigravity` on PATH | `~/.gemini/config/mcp_config.json` (`mcpServers`) |

The server exposes seven tools: `waggledance_projects`, `waggledance_ask_state`,
`waggledance_dispatch`, `waggledance_await`, `waggledance_runs`, `waggledance_task` and
`waggledance_bee_upgrade`. Each is described in the [README](../README.md#agent-integration-mcp).

Drop the snippet from [`waggledance-agents-template.md`](waggledance-agents-template.md)
into your project's `AGENTS.md` / `CLAUDE.md` so its agents know the tools exist. The
snippet tells agents to use the CLI verbs below first and the MCP tools when Bash is not
available.

The pi extension and the OpenClaw bridge reach the same tools over stdio:

```bash
waggledance setup pi
waggledance setup openclaw
```

---

## 4. CLI reference

```bash
waggledance serve                                    # run the supervisor daemon
waggledance register <dir> [--name "My App"]         # add a project to the registry
waggledance list                                     # list projects
waggledance unregister <project-id>                  # remove a project (files kept)
waggledance status                                   # daemon status
waggledance stop                                     # stop the daemon
waggledance restart                                  # restart the daemon (apply config changes)
waggledance doctor [--fix] [--json] [--dry-run]      # diagnose & repair integration
waggledance mcp                                      # MCP server over stdio
waggledance projects                                 # agent verb: registered projects
waggledance state [<project>]                        # agent verb: bee state and panes
waggledance dispatch --project <id> (--leader | --preset <label> | --pane-id <id>) [--task "..." | --task-file <path>]
waggledance await <run_id> [--timeout <secs>]        # agent verb: wait on one run
waggledance runs [--project <id>] [--limit N]        # agent verb: dispatched runs
waggledance runs show <run_id>                       # one run with its full task
waggledance task <corr> reply|redo|accept|allow|deny [--text "..."] [--req-id <id>]
waggledance search "<what you want>"                 # rank the verbs for a need
waggledance schema [<verb>]                          # verbs, flags and help as JSON
waggledance setup <pi|openclaw> [--remove]           # install or remove an integration
waggledance mail send --to <wd> --corr <id> --kind <kind> [--body-file <path>]
waggledance hold --why "<reason>" [--minutes N]      # this leader is waiting; --release ends it
waggledance bee upgrade [--apply] [--project <id>]   # upgrade bee in registered projects
waggledance config edit                              # edit ~/.waggledance/config.toml in $EDITOR
```
Most commands accept `--json` for scripting.

The agent verbs (`projects`, `state`, `dispatch`, `await`, `runs`, `task`) print JSON only,
pretty on a terminal and compact when piped. They print a summary by default; `--full`
prints the MCP tool's answer unchanged, `--fields a,b` keeps those keys on each row and
`--limit N` caps the rows. A tool error prints to stderr and exits 1. `schema` and
`search` help an agent find a verb without loading every description.

`wg` can be a symlink to `waggledance` that you make yourself. No installer creates one,
because `wg` is also the WireGuard command.

---

## 5. Settings

Run `waggledance config edit`. It opens `~/.waggledance/config.toml` in your
`$VISUAL`/`$EDITOR` (falling back to `vi`, or `notepad` on Windows), pre-filled with the
current values. On save it validates the TOML and warns if it's broken (an invalid file is
ignored — waggledance falls back to defaults until you fix it).

Changes apply after a daemon restart — `waggledance restart` (or
`waggledance stop && waggledance serve`).

Per-project dispatch settings live in a `.waggledance.toml` at the project root; see the
[README](../README.md#per-project-settings-waggledancetoml).

---

## 6. Where things live

- `~/.waggledance/registry.db` — project registry, dispatched runs and the leader mailbox (SQLite)
- `~/.waggledance/config.toml` — configuration
- `~/.waggledance/daemon.lock` — the single-daemon coordination file

waggledance never writes to your project files, except `bee upgrade --apply`, which commits
bee's own files in the projects you name.
