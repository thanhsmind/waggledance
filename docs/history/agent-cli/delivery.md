# agent-cli — delivery

Concepts changed, one line each.

- Agent CLI verbs (`projects`, `state`, `dispatch`, `await`, `runs`, `task`): agents in any project drive waggledance from Bash through the same handlers MCP uses, so they pay fewer tokens (D1).
- Output shaping (summary default, `--full`, `--fields`, `--limit`, stderr and exit 1 on error): agents get small JSON by default and the MCP-sized answer on request (D2).
- `schema` and `search`: an agent finds a verb without loading every tool description, built from the clap tree so nothing drifts (D3).
- `waggledance_runs` payload: rows carry `task_summary`, and `run_id` returns one run with its full task, because task text was 488 KB of a 587,737-byte answer (D4).
- `waggledance_ask_state` rollup: drops `sessions` and `recent_decisions` per project and adds `session_count`, because sessions were most of a 324,669-byte answer; the one-project answer is unchanged (D4).
- Crate-visible MCP tool entry: lets the CLI call the one dispatcher instead of a second implementation of any tool (D1).
- Agent and supervisor templates, README and usage guide: lead with the CLI verbs and keep MCP as the fallback, so installed snippets reach every project through doctor and setup (D5).
- `wg` alias: stays an optional user-made symlink and no installer creates it, because of WireGuard (D5).
- `search` verb: live again as verb discovery, so the e2e list of removed viewer commands no longer names it (D3).
- supervisor-daemon area: records the CLI surface and the trimmed MCP shapes with their reasons, anchored to `agent_cli.rs` and `mcp.rs`.
