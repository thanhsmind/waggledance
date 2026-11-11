# headless-supervisor — CONTEXT

Owner request (2026-11-11): remove the waggledance frontend, keep MCP and
the core, stop syncing other projects' files into a central store, and
focus waggledance on its supervisor job. The owner accepted all four
proposed scope points the same day.

## Locked decisions

- D1 (1d17ffbc). The MCP tools `waggledance_view_file` and
  `waggledance_search` are removed. MCP keeps `waggledance_projects`,
  `waggledance_ask_state`, `waggledance_dispatch`, `waggledance_await`,
  `waggledance_runs`, `waggledance_task` and `waggledance_bee_upgrade`.
- D2 (b49a9fce). `waggledance serve` becomes a headless supervisor daemon.
  It runs the background loops (heartbeat, reaper, wake, trigger, notify,
  worktree sweep, the bee status watch) and opens no HTTP listener, not
  even `/health`. State is read through `waggledance status` and
  `waggledance doctor`.
- D3 (9478b44c). The browser board, terminal view, web inbox, docs and
  code reader, guide pages and the Tauri desktop crate are removed. Agents
  are driven through Paseo or herdr directly. Leader mail keeps flowing
  through `mail` and notify (Telegram).
- D4 (479035d3, bf8db4a9). Central file sync is removed: no markdown or
  code indexing, no watching of project content, no full-text or
  title search, no rendering. The CLI commands `open`, `search` and
  `refresh` are removed. `setup` and `doctor` stop writing the
  "Documentation Viewing" block and stop installing the viewer skill. A
  migration drops the `files`, `links` and `files_fts` tables from
  existing registry databases. The README is rewritten for the
  supervisor role.
- D5 (f40f990e). The Paseo plugin in `integrations/paseo` is removed with
  the HTTP endpoints it reads. The pi extension and the OpenClaw
  `wd-bridge` stay because they talk to `waggledance mcp` over stdio.

## Read set

### Knowledge

| Concept | Grade at read | Why it matters |
|---------|---------------|----------------|
| docs/knowledge/patterns/an-fts5-delete-by-unindexed-column-scans-the-whole-table.md | unverified | the file store this feature drops was the registry lock source |
| docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md | unverified | the daemon proof must run the binary just built |
| docs/knowledge/patterns/prove-the-whole-path.md | unverified | the headless daemon owes one end-to-end run |

### Code

| Path | What it decides |
|------|-----------------|
| `crates/waggledance/src/server.rs` | `serve` starts the loops and the HTTP listener; holds pane helpers MCP borrows |
| `crates/waggledance/src/mcp.rs` | the MCP tool list and its stdio loop |
| `crates/waggledance/src/watcher.rs` | the bee status watch that trigger and notify keep using |
| `crates/waggledance/src/main.rs` | `TerminalBackground`, the supervisor loop host |
| `crates/waggledance/src/doctor.rs` | the agent snippet and viewer skill install |
| `crates/waggledance-core/src/repository.rs` | `SCHEMA` and `MIGRATIONS` for the file tables |
