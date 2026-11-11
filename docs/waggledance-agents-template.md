# Waggledance Integration (template)

Copy this snippet into your project's `AGENTS.md` or global `~/.claude/CLAUDE.md`
so agents know what waggledance offers. waggledance runs as a supervisor daemon
(`waggledance serve`) and exposes its tools over MCP (`waggledance mcp`).

---

## Waggledance (supervisor)

Waggledance supervises agents across registered projects. Use its MCP tools:

- `waggledance_projects` lists the registered projects.
- `waggledance_ask_state` reads a project's bee state and live panes.
- `waggledance_dispatch` hands work to a project's leader or a pane.
- `waggledance_await`, `waggledance_runs` and `waggledance_task` follow dispatched work.
- `waggledance_bee_upgrade` previews or applies a bee upgrade.

Check the daemon with `waggledance status` and `waggledance doctor`.
