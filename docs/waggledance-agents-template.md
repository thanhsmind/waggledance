# Waggledance Integration (template)

Copy this snippet into your project's `AGENTS.md` or global `~/.claude/CLAUDE.md`
so agents know what waggledance offers. waggledance runs as a supervisor daemon
(`waggledance serve`); agents reach it through CLI verbs or, as a fallback, its MCP
tools (`waggledance mcp`).

---

## Waggledance (supervisor)

Waggledance supervises agents across registered projects. Drive it with the
`waggledance` CLI from Bash; each verb prints JSON, compact when piped:

- `waggledance projects` lists the registered projects.
- `waggledance state [project]` reads bee state and live panes: one summary row per
  project, or one project's digest.
- `waggledance dispatch --project <id> (--leader | --preset <label> | --pane-id <id>)`
  hands work to a project's leader or a pane; the task comes from `--task`,
  `--task-file` or stdin.
- `waggledance await <run_id>`, `waggledance runs [--project <id>]` and
  `waggledance task <corr> reply|redo|accept|allow|deny` follow dispatched work.
  `waggledance runs show <run_id>` returns one run with its full task.

Answers are a summary by default. Add `--full` for the same answer the MCP tool
returns, `--fields a,b` to keep only those keys per row and `--limit N` to cap the
rows. A tool error prints to stderr and exits 1.

Not sure which verb you need? `waggledance search "<what you want>"` ranks the verbs,
and `waggledance schema [verb]` prints their flags as JSON.

When Bash is not available, use the MCP tools instead: `waggledance_projects`,
`waggledance_ask_state`, `waggledance_dispatch`, `waggledance_await`,
`waggledance_runs` (pass `run_id` for one run's full task), `waggledance_task` and
`waggledance_bee_upgrade`, which previews or applies a bee upgrade.

Check the daemon with `waggledance status` and `waggledance doctor`.
