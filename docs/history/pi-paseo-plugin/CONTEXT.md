# pi + Paseo plugins — Context

**Feature slug:** pi-paseo-plugin
**Date:** 2026-10-06
**Shaping session:** complete
**Scope:** Standard
**Domain types:** CALL | SEE | RUN

## Feature Boundary

Waggle Dance gains two host integrations modeled on `refs/piggery`: a pi
extension that gives a pi session the `waggledance_*` tools natively, and a
Paseo plugin that shows the Waggle Dance board inside the Paseo app. Both are
installed by `waggledance setup pi` and `waggledance setup paseo` from files
embedded in the binary. It ends there: no new agent control, no write path
from Paseo, no pi session presence reporting.

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Ship both halves: a pi extension and a Paseo plugin (decision 8a563125). | The user chose "both". |
| D2 | The pi extension's tools are exactly what `waggledance mcp` lists: names, descriptions and input schemas are read from that server at load, and each call is forwarded to it (decision 69271b7e). | pi has no MCP client; one home for the tool set. |
| D3 | The Paseo plugin is read-only: a sidebar surface over all projects and a workspace panel for the project that owns the workspace directory. Each lists what is waiting on you, what is in progress, and the live agents; every row opens Waggle Dance in the browser (decision 8e6d261d). | Mirrors piggery's read-only view and the conduct-not-edit stance. |
| D4 | Reading never starts the waggledance daemon. A down daemon shows a notice naming `waggledance serve` / any `waggledance open` as the fix (decision e40989d8). | piggery's `--no-start` rule. |
| D5 | `waggledance setup pi` and `waggledance setup paseo` install the embedded files idempotently and refuse to replace a directory they did not write (a `VERSION` marker) (decision 6734ab90). | No npm build step for users. |
| D6 | The pi extension registers the server with `exposure: "direct"`; the model sees the tools as `mcp__waggledance__waggledance_*`, pi's own naming for MCP tools (decision 476c5184). | pi 1.0 hides MCP tools behind codemode by default. D2's "pi has no MCP client" premise held for pi 0.87, not 1.0; `pi.registerMcpServer` satisfies D2's substance. |
| D7 | The Paseo plugin has a fetch URL (daemon side, default `http://127.0.0.1:7700`) and a separate link base for opening rows (defaults to the fetch URL) (decision 711efdab). | A phone cannot open 127.0.0.1; the daemon is also served through a Cloudflare Access tunnel. |

### Agent's Discretion

- Where the plugin sources live in the repo, how the board is exposed to the
  plugin (CLI or HTTP), poll cadence, and visual layout — planning decides,
  following piggery's shape where it fits.
- The pi half stays an extension, not a doctor-written `mcp.json` entry (decision f2901848).

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| board | The cross-project roll-up the front page shows: Waiting on you, In progress, plus live agents. |
| surface | Paseo's full-screen plugin view reached from the sidebar. |
| workspace panel | Paseo's per-workspace side panel. |

## Specific Ideas And References

- `refs/piggery/extensions/pi/` — pi extension shape: `pi.registerTool`, `Type.Unsafe(schema)`, `package.json` `pi.extensions`, reload-safe process state.
- `refs/piggery/extensions/paseo/` — Paseo plugin shape: `paseo-plugin.json`, `index.server.ts` (`defineRpc`, `server.handle`, settings), `index.client.tsx` (`addSurface`, `addSidebarItem`, `addWorkspacePanel`, `addSettingsScreen`), `server/installed.ts` rewritten by setup with the binary's absolute path.
- `refs/piggery/internal/cli/setup_paseo.go` — install flow: write to `~/.<app>/paseo`, then `paseo plugin install|enable|reload` by `paseo plugin ls --json` state.

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/mcp.rs` — stdio line-delimited JSON-RPC MCP server, seven tools, engine in-process.
- `crates/waggledance/src/server.rs` — `/api/agents` (JSON, live agents), `cross_project_rollup` (board data), `project_summary_json` (omits `root_path` on purpose: the HTTP API is unauthenticated).
- `crates/waggledance/src/doctor.rs` — existing integration wiring for Claude Code.

### Integration Points

- `crates/waggledance/src/cli.rs` / `main.rs` — new `setup` subcommand.
- Installed tools on this host: pi 1.0.0, paseo 0.10.3, daemon on 7700.

## Outstanding Questions

### Deferred To Planning

- [ ] Board transport: a `waggledance board --json` CLI vs a new `/api/board` route — must keep root paths off the unauthenticated HTTP API.
- [ ] The repo's `commands.test` points at `packages/bee-rs`, which this repo does not have — planning names the real proof commands.
