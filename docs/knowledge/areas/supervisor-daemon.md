---
type: bee.area
title: The supervisor daemon is headless and MCP serves eight tools
description: "waggledance serve runs the supervisor loops with no HTTP listener; waggledance mcp serves eight tools over stdio; nothing indexes or serves other projects' files"
tags: [supervisor, daemon, mcp]
timestamp: 2026-11-12
bee:
  id: supervisor-daemon
  lifecycle: active
  areas: [orchestration]
  anchors: [crates/waggledance/src/daemon.rs, crates/waggledance/src/mcp.rs, crates/waggledance/src/runtime.rs, crates/waggledance/src/cli.rs, crates/waggledance/src/agent_cli.rs, crates/waggledance/src/intake.rs, crates/waggledance/src/main.rs, crates/waggledance/src/watcher.rs, crates/waggledance-core/src/repository.rs, crates/waggledance-core/src/intake.rs, crates/waggledance-core/src/config.rs, crates/waggledance-core/src/bee_global_config.rs]
  sources: [docs/history/headless-supervisor/CONTEXT.md, docs/history/headless-supervisor/plan.md, docs/history/agent-cli/CONTEXT.md, docs/history/agent-cli/plan.md, docs/history/anphabe-intake/CONTEXT.md, docs/history/anphabe-intake/plan.md]
---

# The supervisor daemon is headless and MCP serves eight tools

Read when: you look for the board, the terminal page, a docs or code viewer, `open`, `refresh`, `/health` or the Paseo sidebar plugin, or you wonder why `status` cannot dial the daemon.

## Boundary

waggledance supervises agents and nothing else. Locked by headless-supervisor D1-D5 (`docs/history/headless-supervisor/CONTEXT.md`).

- `waggledance serve` (`daemon.rs`) hosts the background loops and binds no socket, not even `/health`. Liveness is the lock file's pid, read through `waggledance status` and `waggledance doctor`; see [a TCP health probe reads a listenerless daemon as dead](../patterns/a-tcp-health-probe-reads-a-listenerless-daemon-as-dead.md).
- `waggledance mcp` (`mcp.rs`) is stdio and registers eight tools: `waggledance_projects`, `waggledance_ask_state`, `waggledance_dispatch`, `waggledance_await`, `waggledance_runs`, `waggledance_task`, `waggledance_bee_upgrade`, `waggledance_bee_config`. `waggledance_view_file` and `waggledance_search` are gone.
- Agents are driven through Paseo or herdr directly. Leader mail keeps flowing through `mail` and notify.
- The pi extension and the OpenClaw `wd-bridge` stay because they speak MCP over stdio. The Paseo plugin and the desktop app are gone.

## Rejected

- Keep `/health` as the only listener: D2 forbids any port, and a listener is what the removed web surface hung from.
- Leave the `files`, `links` and `files_fts` tables unused: the old store was the registry lock source (see [the retired FTS5 pitfall](../patterns/an-fts5-delete-by-unindexed-column-scans-the-whole-table.md)); migration drops them. A version-gated migration is why an older binary on a migrated database sees empty recreated tables, not the old rows.
- Keep a `[server]` config section working: the key still parses and is ignored, so existing config files load.

## Pointers

- `crates/waggledance/src/watcher.rs` — the bee status watch trigger and notify keep using; it is not the removed file watcher.
- `crates/waggledance-core/src/repository.rs` — `SCHEMA` and `MIGRATIONS`; the file tables are dropped by migration, not by editing history.

## Agent CLI verbs

Read when: you wonder why `waggledance search` answers again after the viewer was removed, why a CLI verb prints less than its MCP tool, or where the verbs call into MCP.

- `projects`, `state`, `dispatch`, `await`, `runs`, `task`, `schema` and `search` (`agent_cli.rs`) are a thin layer over `mcp::call_tool`: each maps flags to the tool's arguments and shapes `structuredContent`. No tool logic lives in the CLI (agent-cli D1, `docs/history/agent-cli/CONTEXT.md`).
- `search` here is verb discovery ranked from the clap tree (D3), not the removed viewer search; `tests/e2e_daemon.rs` no longer lists it among removed commands.
- Default output is a summary; `--full` is the unchanged `structuredContent`, so a drift between CLI and MCP shows up in the `--full` equality tests (D2). `--fields` and `--limit` narrow rows after the summary, never before it.
- `schema` and `search` read the clap command tree, so a new verb is discoverable the moment it exists; no second list of verbs is kept (D3).
- The CLI builds its engine from the registry database like the stdio loop, so a verb works with the daemon down.
- A tool error goes to stderr with exit 1; stdout stays JSON only.

## Trimmed MCP payloads

Read when: a caller reads `task` from `waggledance_runs` and finds `task_summary`, or reads `sessions` from the all-projects `waggledance_ask_state` and finds `session_count`.

- `waggledance_runs` rows carry `task_summary`; the full `task` comes only from the `run_id` argument. `waggledance_ask_state` without `project` drops `sessions` and `recent_decisions` per project and adds `session_count`; with `project` the answer is unchanged (agent-cli D4, `mcp.rs`).
- Why: on 2026-11-11 the rollup measured 324,669 bytes and the run list 587,737, mostly session lists and task text no caller needed to choose a next step.
- Rejected: trimming only in the CLI left MCP callers paying the full cost; a separate `wg` binary duplicated install and logic. `wg` stays a user-made symlink because the name collides with WireGuard (D5).

## Intake ledger

Read when: an anphabe item is dispatched twice, a held item is announced again, or `intake_items` is missing from a fresh registry.

- `intake_items` (migration 15, `repository.rs`) is keyed by `<source>-<id>` and holds one verdict per item: `dispatched`, `held`, `skipped` (not code) or `covered` (twin of a dispatched key), the enum in `waggledance-core/src/intake.rs`; `insert_intake_item` is insert-if-absent and returns false on a repeat, so the first verdict and title stay and a retry cannot send twice (anphabe-intake D4).
- The table is created only by the migration, which also runs on a fresh database, the way `inbox` and `tasks` are; the cell asked for it in `SCHEMA` too, and a second definition would have been a duplicate home.
- `[intake]` in `config.rs` is off by default, and `user_email` stays empty in code: the pass refuses to run until the host sets it (anphabe-intake D2, D7).

## Intake pass

Read when: an intake item was dispatched to the wrong project, a pass records nothing, or you wonder why the model never calls `waggledance_dispatch`.

- One pass (`intake.rs`) runs Haiku through a runner trait, parses a JSON array, and decides each item in Rust: not code is skipped, a twin of a dispatched key is covered, a registered project at or above `min_confidence` is dispatched through `mcp::call_tool_on` with `leader: true` and `request_id` `intake-<key>`, everything else is held. Only code work is brought in, and a dispatched item is the leader's to do, not to file (anphabe-intake D1). The model only classifies (anphabe-intake D4).
- Unparseable output fails the pass before any ledger write; the error carries the first 300 characters. A dispatch error holds the item with the error as its reason, and a later pass skips it because the key is already in the ledger.
- The prompt is stdin because `--allowedTools` swallows a trailing prompt argument; the allowed list has only the three mcp-gateway tools, but `call_tool` proxies every anphabe tool, writes included, so keeping writes out (anphabe-intake D6) rests on the prompt until mcp-gateway offers a read-only profile.
- The daemon loop (`intake::run`, slot in `TerminalBackground` in `main.rs`) runs only with `[intake] enabled` and a non-empty `user_email`, first pass 60 seconds after start, then every `interval_minutes` (floor one minute), missed ticks skipped. The pass runs under `spawn_blocking` because the claude subprocess blocks; a failed pass logs a warning and the loop goes on (anphabe-intake D3, D5).
- Held items with `notified` false go out as one `intake` notifier message and are marked notified only after the send succeeds, so a failed send is retried on the next tick and a delivered item is never repeated. With no Telegram credentials the notifier is the null one, which only logs.
- `intake run|list` are agent verbs listed in `schema` and `search`; an empty `[intake] user_email` exits 1 naming it.

## Bee global config

Read when: a bee setting should apply to every project, `waggledance bee config` refuses a key, or you wonder why the MCP tool cannot write it.

- `waggledance bee config path|show|get|set|unset|explain|prune` (`cli.rs`, logic in `bee_global_config.rs`) edits bee's user-global file (and `prune` cleans project layers). The path is fixed by beehive D12: `$BEEHIVE_CONFIG_DIR/config.json`, else `$XDG_CONFIG_HOME/beehive/config.json`, else `~/.config/beehive/config.json`, an empty variable counting as unset. The directory also holds `performance.jsonl`, which this code never touches.
- The document has the keys of a project's `.bee/config.json`, written as 2-space pretty JSON plus a newline through `config::write_atomic`, key order kept. `set` parses the value as JSON and stores a string when it does not parse. A missing file reads as `{}`; a malformed or non-object file is an error naming the path and is never overwritten.
- `guards`, `gate_bypass` and `hooks` are per-project only. `set` of one is refused and `save` refuses any document still holding one; `unset` of one is allowed so a hand-edited file can be cleaned.
- `explain` runs each project's `.bee/bin/bee config explain --key <key> --json` through `run_with_deadline`; a project whose bee fails or lacks the verb reports the error on its own row. Precedence: built-in default < global file < project `.bee/config.json` < `.bee/team.json` < `.bee/config.local.json`.
- `prune <key> [--project <id>]... [--force] [--apply] [--json]` removes a dotted key (such as `team` or `herding`) and any parent objects the removal empties from each registered project's tracked (`.bee/config.json`), local (`.bee/config.local.json`) and team (`.bee/team.json`) layers so the global value applies. It refuses when the key is `guards`, `gate_bypass` or `hooks` or when the global file does not set it; a layer whose value differs from global is reported as `differs` and kept unless `--force`. Dry run unless `--apply`, writes atomically with 2-space pretty JSON and key order kept, and never commits in target repos.
- `waggledance_bee_config` (`mcp.rs`) is read-only (`show|get|explain`). Writes stay on the owner's CLI: an agent changing a setting for every project is the owner's call.

## Pointers (bee global config)

- `crates/waggledance-core/src/bee_global_config.rs` — path resolver, `KeyPath`, forbidden-key check, load/set/unset/save, `explain`, `prune`.
- `crates/waggledance/src/cli.rs` — `BeeConfigAction` and `cmd_bee_config`.
- `crates/waggledance/src/mcp.rs` — `bee_config_schema` and `handle_bee_config`.
