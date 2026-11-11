---
type: bee.area
title: The supervisor daemon is headless and MCP serves seven tools
description: "waggledance serve runs the supervisor loops with no HTTP listener; waggledance mcp serves seven tools over stdio; nothing indexes or serves other projects' files"
tags: [supervisor, daemon, mcp]
timestamp: 2026-11-11
bee:
  id: supervisor-daemon
  lifecycle: active
  areas: [orchestration]
  anchors: [crates/waggledance/src/daemon.rs, crates/waggledance/src/mcp.rs, crates/waggledance/src/runtime.rs, crates/waggledance/src/cli.rs, crates/waggledance/src/watcher.rs, crates/waggledance-core/src/repository.rs]
  sources: [docs/history/headless-supervisor/CONTEXT.md, docs/history/headless-supervisor/plan.md]
---

# The supervisor daemon is headless and MCP serves seven tools

Read when: you look for the board, the terminal page, a docs or code viewer, `search`, `open`, `refresh`, `/health` or the Paseo sidebar plugin, or you wonder why `status` cannot dial the daemon.

## Boundary

waggledance supervises agents and nothing else. Locked by headless-supervisor D1-D5 (`docs/history/headless-supervisor/CONTEXT.md`).

- `waggledance serve` (`daemon.rs`) hosts the background loops and binds no socket, not even `/health`. Liveness is the lock file's pid, read through `waggledance status` and `waggledance doctor`; see [a TCP health probe reads a listenerless daemon as dead](../patterns/a-tcp-health-probe-reads-a-listenerless-daemon-as-dead.md).
- `waggledance mcp` (`mcp.rs`) is stdio and registers seven tools: `waggledance_projects`, `waggledance_ask_state`, `waggledance_dispatch`, `waggledance_await`, `waggledance_runs`, `waggledance_task`, `waggledance_bee_upgrade`. `waggledance_view_file` and `waggledance_search` are gone.
- Agents are driven through Paseo or herdr directly. Leader mail keeps flowing through `mail` and notify.
- The pi extension and the OpenClaw `wd-bridge` stay because they speak MCP over stdio. The Paseo plugin and the desktop app are gone.

## Rejected

- Keep `/health` as the only listener: D2 forbids any port, and a listener is what the removed web surface hung from.
- Leave the `files`, `links` and `files_fts` tables unused: the old store was the registry lock source (see [the retired FTS5 pitfall](../patterns/an-fts5-delete-by-unindexed-column-scans-the-whole-table.md)); migration drops them. A version-gated migration is why an older binary on a migrated database sees empty recreated tables, not the old rows.
- Keep a `[server]` config section working: the key still parses and is ignored, so existing config files load.

## Pointers

- `crates/waggledance/src/watcher.rs` — the bee status watch trigger and notify keep using; it is not the removed file watcher.
- `crates/waggledance-core/src/repository.rs` — `SCHEMA` and `MIGRATIONS`; the file tables are dropped by migration, not by editing history.
