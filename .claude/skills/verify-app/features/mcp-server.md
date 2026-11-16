# MCP server

Claude Code and other MCP clients launch `waggledance mcp` and talk JSON-RPC over stdio.
The server answers `initialize`, lists seven `waggledance_*` tools, and runs them against
the same registry the CLI uses.

## Sub-features

- `mcp-initialize` — the handshake returns `serverInfo.name = "waggledance"` at the binary's version.
- `mcp-tools-list` — `tools/list` exposes `waggledance_projects`, `waggledance_ask_state`,
  `waggledance_dispatch`, `waggledance_await`, `waggledance_runs`, `waggledance_task`
  and `waggledance_bee_upgrade`.
- `mcp-projects` — `waggledance_projects` returns the registry, with a text summary and
  `structuredContent`.
- `mcp-ask-state` — `waggledance_ask_state {"project":"repo"}` returns a result.
- `mcp-unknown-tool` — an unknown tool name returns an error, and the server does not crash.

## How to get to it (user POV)

- `waggledance doctor --fix` wires `waggledance mcp` into Claude Code. The client then
  spawns it over stdio.
- By hand, pipe JSON-RPC lines into `waggledance mcp`.

## Driving it with control-waggledance

Preconditions:

- One scratch project `repo` is registered.

- **Session.** The script writes six JSON-RPC lines to `mcp-request.jsonl` (initialize,
  initialized, tools/list, projects, ask_state, a bogus tool) and pipes them into
  `waggledance mcp`. Stdin closes at the end of the file and the server exits.
- **Handshake.** The `"id":1` response names `"name":"waggledance"`.
- **Tools.** The `"id":2` response names all seven tools.
- **Projects.** The `"id":3` response says `1 registered project`.
- **Ask state.** The `"id":4` response is a `result`.
- **Unknown tool.** The `"id":5` response carries an error.
- **Drive it.** `bash .bee/verify/verify-app/control-waggledance drive --fast mcp-server`.

## Gotchas

- The server answers `protocolVersion` `2024-11-05` whatever the client asks for.
  Assert on the tools, not on the protocol echo.
- `waggledance_dispatch`, `_await` and `_task` need a live run. Calling them here proves
  only their refusals. See `leader-mail-and-dispatch`.
- Do not test against the user's MCP wiring. A client spawned from the user's config
  reads `~/.waggledance`.
