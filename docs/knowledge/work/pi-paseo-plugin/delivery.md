---
type: bee.delivery
title: pi-paseo-plugin — delivery
description: "Delivery record for work item pi-paseo-plugin: a pi extension that hands every pi session the waggledance MCP tools, a read-only Paseo plugin over a new GET /api/board route, and waggledance setup pi|paseo that installs both from files embedded in the binary — modeled on refs/piggery."
timestamp: 2026-10-06
bee:
  id: pi-paseo-plugin-delivery
  lifecycle: active
  areas: [integrations, orchestration]
  required_context: [docs/history/pi-paseo-plugin/CONTEXT.md]
  sources: [docs/history/pi-paseo-plugin/CONTEXT.md, docs/history/pi-paseo-plugin/plan.md]
---

# pi-paseo-plugin — Delivery

## What shipped

piggery reaches its agents in two hosts: a pi extension and a Paseo plugin, each
installed by one `setup` command from files the binary carries. Waggle Dance now does
the same.

- **pi extension** — `integrations/pi/index.ts` calls `pi.registerMcpServer("waggledance",
  { command: <bin>, args: ["mcp"], exposure: "direct" })`. pi 1.0 has its own MCP client,
  so the extension is a registration, not the 450-line socket client piggery needed for
  pi 0.87. The tool set keeps one home: whatever `waggledance mcp` lists. pi names the
  tools `mcp__waggledance__waggledance_*`; without `exposure: "direct"` pi hides MCP
  tools behind codemode and the model never sees them. `WAGGLEDANCE_DISABLED=1` turns it
  off. pi sessions Paseo launches load it too, from `~/.pi/agent/extensions/waggledance`.
- **`GET /api/board`** — the cross-project board as JSON: `in_progress` (waiting first,
  with the "Waiting on you — …" reason) and `ready_to_merge`, built by
  `views::bee_board_json` from the same `bee_classify_features` the HTML board uses. It
  carries no filesystem path; the project list stays at `/api/projects`.
- **Paseo plugin** — `integrations/paseo/`: a "Waggle Dance" sidebar surface and a
  workspace panel listing Waiting on you, In progress and Live agents, plus a settings
  screen. One RPC, `waggledance.board({dir?})`, fetches `/api/board` and `/api/agents` on
  the Paseo host; with a workspace directory it runs `waggledance list --json` locally to
  find the owning project (longest registered root wins), so root paths never cross HTTP.
  It never starts the daemon and never writes.
- **`waggledance setup <pi|paseo> [--remove] [--json]`** — `crates/waggledance/src/setup.rs`
  embeds both trees with `include_str!`, adds a `VERSION` marker, rewrites `installed.ts`
  to the running binary's path, writes through a sibling temp dir, and refuses any target
  directory without `VERSION`. For Paseo it reads `paseo plugin ls --json` and runs
  install, remove+install, enable, reload or nothing.

## Locked decisions

| ID | Decision |
|----|----------|
| pi-paseo-plugin D1 | Both halves ship: a pi extension and a Paseo plugin. |
| pi-paseo-plugin D2 | The pi tools are exactly what `waggledance mcp` lists; nothing is copied into the extension. |
| pi-paseo-plugin D3 | The Paseo plugin is read-only: sidebar surface plus workspace panel; rows open Waggle Dance in the browser. |
| pi-paseo-plugin D4 | Reading never starts the daemon; a down daemon names `waggledance serve` as the fix. |
| pi-paseo-plugin D5 | `setup pi|paseo` installs embedded files idempotently and refuses a directory it did not write. |
| pi-paseo-plugin D6 | The pi server is registered with `exposure: "direct"`; tools appear as `mcp__waggledance__waggledance_*`. |
| pi-paseo-plugin D7 | The Paseo plugin has a fetch URL (default `http://127.0.0.1:7700`) and a separate link base, so a phone or the tunnel host opens real pages. |

## Gotchas

- A `waggledance` server in `~/.pi/agent/mcp.json` overrides the extension; `setup pi`
  warns and names `pi mcp remove waggledance`.
- `paseo plugin ls` can show `enabled: true` with `status: "disabled"`: Paseo's global
  plugin switch is off. Turn on plugins once in Paseo's settings.
- `installed.ts` pins the binary path `setup` ran from; `setup` warns when that path is a
  cargo `target/` directory that later builds overwrite. Rerun `setup` after an upgrade.
- `bee cells leader-check` cannot see files that exist only in a feature worktree; the
  checks for this feature were recorded on main after merge.

## Pointers

- `integrations/pi/index.ts`, `integrations/pi/installed.ts`
- `integrations/paseo/index.client.tsx`, `integrations/paseo/server/waggledance.ts`, `integrations/paseo/shared/rpc.ts`
- `crates/waggledance/src/setup.rs`, `crates/waggledance/src/views.rs` (`bee_board_json`), `crates/waggledance/src/server.rs` (`api_board`)
