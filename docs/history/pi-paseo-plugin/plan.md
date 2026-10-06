---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: pi + Paseo plugins

## Summary

Waggle Dance gets two plugins, the way piggery has them. In pi, a small
extension hands every session the `waggledance_*` tools by registering the
existing `waggledance mcp` server, so the tool set still lives in one place.
In the Paseo app, a "Waggle Dance" sidebar screen and a workspace panel show
what is waiting on you, what is in progress, and the live agents, each row
opening the Waggle Dance page in the browser. One command installs each:
`waggledance setup pi` and `waggledance setup paseo`.

Mode: `standard` — 3 risk flags: external systems (pi, Paseo), public contracts (a new `/api/board` JSON route and a new CLI verb), multi-domain (Rust + two TypeScript hosts).
Why this is the least workflow that protects the work: four disjoint cells, each provable on its own, with one live check per host at the end.

Playbook: `skills/bee-planning/playbooks/feature.md` (cited, not copied).

## Requirements (from CONTEXT.md)

- D1: both halves ship — a pi extension and a Paseo plugin.
- D2: the pi tools are exactly what `waggledance mcp` lists; no second copy of names or schemas.
- D3: the Paseo plugin is read-only — sidebar surface over all projects, workspace panel for the project owning the workspace directory; lists waiting on you, in progress, live agents; rows open Waggle Dance.
- D4: reading never starts the daemon; a down daemon shows a notice with the fix.
- D5: `waggledance setup pi|paseo` installs embedded files idempotently and refuses a directory it did not write.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command executed); evidence is a verbatim substring of the anchor.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | pi 1.0 lets an extension register an MCP server, so D2 needs no hand-written MCP client | ran | `sed -n 190p ~/.local/share/mise/installs/pi/latest/pi/docs/extensions.md` | `pi.registerMcpServer(name, config)` adds an MCP server for the current session. |
| 2 | pi loads a directory extension through its `index.ts` | ran | `sed -n 46p ~/.local/share/mise/installs/pi/latest/pi/docs/extensions.md` | Pi loads direct TypeScript or JavaScript files and subdirectories containing an `index.ts` or `index.js` entry point. |
| 3 | The installed pi is 1.0.0 under the package name the extension types import | ran | `head -4 ~/.local/share/mise/installs/pi/latest/pi/package.json` | "name": "@earendil-works/pi-coding-agent", |
| 4 | `waggledance mcp` answers line-delimited JSON-RPC and lists its tools | ran | `printf initialize+tools/list \| waggledance mcp` | {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"waggledance_view_file" |
| 5 | Feature classification for the board is plain data, separable from HTML | read | crates/waggledance/src/views.rs:5008 | fn bee_classify_features( |
| 6 | No JSON route exposes the board today, so `/api/board` is new | read | crates/waggledance/src/server.rs:427 | .route("/api/status", get(status)) |
| 7 | The HTTP API deliberately omits project root paths, so the workspace match must come from the local CLI | read | crates/waggledance/src/server.rs:1307 | fn project_summary_json(id: &str, name: &str, file_count: usize) -> serde_json::Value { |
| 8 | `waggledance list --json` gives root paths locally | read | crates/waggledance/src/cli.rs:368 | serde_json::json!({ |
| 9 | Live agents already have a JSON feed | read | crates/waggledance/src/server.rs:433 | .route("/api/agents", get(api_agents)) |
| 10 | Paseo CLI is installed and can install a local plugin directory | ran | `paseo plugin --help` | install\|add [options] <source>  Trust and install a plugin from a directory, |
| 12 | pi names MCP tools by server and hides them behind codemode unless exposure is direct | ran | `sed -n 169p ~/.local/share/mise/installs/pi/latest/pi/docs/mcp.md` | Each server tool is registered as `mcp__<server>__<tool>` |
| 13 | The Paseo SDK opens external URLs through its own helper | ran | `npm pack @getpaseo/plugin@0.10.3` then package/dist/client/index.d.ts:21 | export declare function openExternalUrl(url: string): Promise<void>; |
| 11 | Plugins are stored beside waggledance's own data in `~/.waggledance` | read | crates/waggledance-core/src/config.rs:258 | pub fn data_dir() -> PathBuf { |

## Discovery

Read `refs/piggery/extensions/{pi,paseo}` and `internal/cli/setup_{pi,paseo}.go`.
piggery's pi extension is 450 lines because pi 0.87 had no MCP client; pi 1.0
has `pi.registerMcpServer`, so ours is a registration with `exposure: "direct"` (D6). piggery's Paseo plugin
shells out to `piggery ps --view`; Waggle Dance already serves JSON over HTTP,
so the plugin server fetches `/api/board` and `/api/agents` and runs
`waggledance list --json` only to map a workspace directory to a project.
Plan check (three seats) added D6 (direct exposure, `mcp__waggledance__` names),
D7 (separate link base), one RPC instead of two, empty-state copy, and setup's
warnings; the cheaper "doctor writes pi mcp.json" path was rejected (decision f2901848).

## Approach

- `/api/board` (D3): serialize `bee_classify_features` output per project — in-progress cards (title, done/total, waiting reason, URL) and ready-to-merge — no filesystem paths.
- pi (D1, D2): `integrations/pi/index.ts` calls `pi.registerMcpServer("waggledance", { command: <bin>, args: ["mcp"], exposure: "direct", description })` (D6); `<bin>` comes from `installed.ts`, rewritten by setup, else `waggledance` on PATH.
- Paseo (D1, D3, D4): `integrations/paseo/` with one RPC `waggledance.board({dir?})` (daemon side fetches the two routes with a timeout, never starts the daemon; with `dir`, maps it to a project via `waggledance list --json`, longest root containing the realpath, and filters). Client: surface + sidebar item + workspace panel + settings screen (fetch URL, link base per D7, binary path). Polls every 5 s while something is waiting or working, else 30 s, as piggery does.
- Setup (D5): `crates/waggledance/src/setup.rs` embeds both directories with `include_str!`, writes to `~/.pi/agent/extensions/waggledance` and `~/.waggledance/paseo` with a `VERSION` marker, then `paseo plugin install|enable|reload` by `paseo plugin ls --json` state.

Rejected: a hand-written JSON-RPC client in the pi extension (claim 1 makes it dead weight); a `waggledance board --json` CLI (would recompute the rollup outside the daemon's cache); writing pi's `mcp.json` (not a plugin, and pi's `/mcp` edits that file).

Risk map: `/api/board` contract / MEDIUM / ppp-1 / route test pins the JSON shape. Paseo SDK drift (0.9 → 0.10) / MEDIUM / ppp-3 / `tsc --noEmit` against `@getpaseo/plugin@0.10.3` and a live install. Setup clobbering a user's directory / MEDIUM / ppp-4 / refusal test.

Waves: ppp-1 ∥ ppp-2; ppp-3 after ppp-1 (consumes the route); ppp-4 after ppp-2 and ppp-3 (embeds their files).

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 | `/api/board` JSON + pi extension | data contract and the cheap half first | `curl /api/board`; a pi session lists `mcp__waggledance__*` tools | the Paseo client |
| 2 | Paseo plugin | needs the route | the plugin typechecks and its server tests pass | setup |
| 3 | `waggledance setup pi\|paseo` | embeds finished files | `paseo plugin ls` shows `waggledance` enabled; pi loads it | uat |

## Cells — current slice (preview)

```json
[
  {
    "id": "ppp-1",
    "title": "Serve the cross-project board as JSON at /api/board",
    "role": "code",
    "lane": "standard",
    "feature": "pi-paseo-plugin",
    "deps": [],
    "decisions": [
      "D3"
    ],
    "files": [
      "crates/waggledance/src/server.rs",
      "crates/waggledance/src/views.rs"
    ],
    "read_first": [
      "crates/waggledance/src/views.rs",
      "crates/waggledance/src/server.rs",
      "docs/history/pi-paseo-plugin/CONTEXT.md"
    ],
    "action": "Add GET /api/board returning {\"in_progress\":[{\"project_id\",\"project_name\",\"feature\",\"title\",\"done\",\"total\",\"waiting\",\"reason\",\"last_activity\",\"url\"}],\"ready_to_merge\":[{\"project_id\",\"project_name\",\"feature\",\"title\",\"url\"}]} (project list stays at /api/projects). Build it from cross_project_rollup over bee projects plus a pub(crate) data function in views.rs that runs bee_classify_features (views.rs:5008) and maps the private BeeHubPlacement enum (views.rs:4842): InProgress -> in_progress, ReadyToMerge -> ready_to_merge; Finished, Review, Compound and Todo are dropped. Never parse HTML. title is docs.title, else the feature slug. waiting is true when the card's reason is set; in_progress sorts waiting first, then newest last_activity. url is /p/<id>/_bee/feature/<feature>, not HTML-escaped. No root_path or any absolute path in the output. No code comments.",
    "verify": "cargo test --profile fast -p waggledance api_board",
    "must_haves": {
      "truths": [
        "GET /api/board returns 200 JSON with projects, in_progress, ready_to_merge arrays",
        "a gate-stopped feature appears with waiting=true and a reason starting 'Waiting on you'"
      ],
      "artifacts": [
        {
          "path": "crates/waggledance/src/server.rs",
          "substantive": "route /api/board and a test module case named api_board_*"
        }
      ],
      "key_links": [
        "/api/board reuses bee_classify_features"
      ],
      "prohibitions": [
        "no root_path or absolute path in the response",
        "no change to existing routes' output"
      ]
    }
  },
  {
    "id": "ppp-2",
    "title": "Give pi sessions the waggledance tools through a pi extension",
    "role": "code",
    "lane": "standard",
    "feature": "pi-paseo-plugin",
    "deps": [],
    "decisions": [
      "D1",
      "D2",
      "D6"
    ],
    "files": [
      "integrations/pi/index.ts",
      "integrations/pi/installed.ts",
      "integrations/pi/package.json",
      "integrations/pi/index.test.ts"
    ],
    "read_first": [
      "/home/thanhsmind/Projects/refs/piggery/extensions/pi/index.ts",
      "/home/thanhsmind/.local/share/mise/installs/pi/latest/pi/docs/extensions.md",
      "/home/thanhsmind/.local/share/mise/installs/pi/latest/pi/docs/mcp.md",
      "docs/history/pi-paseo-plugin/CONTEXT.md"
    ],
    "action": "Create integrations/pi: package.json {name waggledance-pi, private, type module, pi.extensions [./index.ts]}; installed.ts exporting const waggledancePath = \"\" on one line (setup rewrites it); index.ts default-exports (pi: ExtensionAPI) => void that, unless process.env.WAGGLEDANCE_DISABLED === \"1\", calls pi.registerMcpServer(\"waggledance\", { command: waggledancePath || \"waggledance\", args: [\"mcp\"], exposure: \"direct\", description: \"Waggle Dance: view docs in the browser, search docs and bee state across projects, dispatch work to agent panes\" }). Type imports only from @earendil-works/pi-coding-agent. index.test.ts (node --test, type stripping) drives the default export with a fake pi and asserts the registration and the disabled case. No code comments.",
    "verify": "node --test integrations/pi/index.test.ts",
    "must_haves": {
      "truths": [
        "the extension registers exactly one MCP server named waggledance running `<bin> mcp` with exposure direct",
        "WAGGLEDANCE_DISABLED=1 registers nothing"
      ],
      "artifacts": [
        {
          "path": "integrations/pi/index.ts",
          "substantive": "calls pi.registerMcpServer"
        }
      ],
      "key_links": [
        "index.ts imports waggledancePath from ./installed.ts"
      ],
      "prohibitions": [
        "no tool names or schemas copied into the extension"
      ]
    }
  },
  {
    "id": "ppp-3",
    "title": "Show the Waggle Dance board inside Paseo",
    "role": "code",
    "lane": "standard",
    "feature": "pi-paseo-plugin",
    "deps": [
      "ppp-1"
    ],
    "decisions": [
      "D1",
      "D3",
      "D4",
      "D7"
    ],
    "files": [
      "integrations/paseo/paseo-plugin.json",
      "integrations/paseo/package.json",
      "integrations/paseo/tsconfig.json",
      "integrations/paseo/.gitignore",
      "integrations/paseo/index.server.ts",
      "integrations/paseo/index.client.tsx",
      "integrations/paseo/server/installed.ts",
      "integrations/paseo/server/waggledance.ts",
      "integrations/paseo/shared/rpc.ts",
      "integrations/paseo/shared/board.ts",
      "integrations/paseo/client/board.tsx",
      "integrations/paseo/client/settings.tsx",
      "integrations/paseo/test/server.test.ts"
    ],
    "read_first": [
      "/home/thanhsmind/Projects/refs/piggery/extensions/paseo/index.server.ts",
      "/home/thanhsmind/Projects/refs/piggery/extensions/paseo/index.client.tsx",
      "/home/thanhsmind/Projects/refs/piggery/extensions/paseo/shared/rpc.ts",
      "/home/thanhsmind/Projects/refs/piggery/extensions/paseo/server/piggery.ts",
      "/home/thanhsmind/Projects/refs/piggery/extensions/paseo/client/poll.ts",
      "/home/thanhsmind/.claude/skills/paseo-plugin/SKILL.md",
      "docs/history/pi-paseo-plugin/CONTEXT.md"
    ],
    "action": "Create a Paseo plugin id `waggledance`, requirements paseo >=0.10.0, devDependencies pinned exactly: @getpaseo/plugin, @getpaseo/client, @getpaseo/protocol 0.10.3, react ~19.1.0, react-native 0.81.5, zod ^4.4.3, @tanstack/react-query, typescript, @types/node, @types/react as piggery's package.json does. Settings (host scope): fetch URL (default http://127.0.0.1:7700), link base (empty = fetch URL; settings help says a phone needs a reachable host such as the tunnel URL), binary path. One RPC waggledance.board({dir?}) in shared/rpc.ts: the daemon side fetches <fetch>/api/board and <fetch>/api/agents with a 5 s timeout and returns {ok:true, board, agents, agents_note?, link_base, project?} or {ok:false, code: down|auth|failed, error}. Connection refused -> down naming `waggledance serve` on the Paseo host; a non-JSON or 3xx reply -> auth saying to use the local URL for fetching and the tunnel only as link base. /api/agents 404 -> agents [] with note 'agent terminal is disabled'; 502 -> note 'herdr is not running'. With dir: run `<bin> list --json` (bin: setting, else installed.ts path, else waggledance; execFile, no shell, 5 s timeout), pick the project whose root_path contains the realpath of dir, longest root wins, filter board and agents to it; none -> project null. Never starts the daemon (D4). Client: addSurface + addSidebarItem (title Waggle Dance) with sections Waiting on you (in_progress with waiting), In progress (the rest plus ready_to_merge), Live agents; addWorkspacePanel (context workspace, dir from useWorkspace(id, w => w.directory)) with the same, or 'This workspace is not a Waggle Dance project' when project is null; addSettingsScreen. Empty copy: no features -> 'Nothing in progress'; no agents -> 'No live agents' or the agents note. Each row opens link_base+url with openExternalUrl from @getpaseo/plugin/client. Keep the last good board on a failed poll and mark it stale. Poll 5 s while anything waits or an agent is working, else 30 s. Text colours from theme.colors only. Pure logic (dir matching, error mapping, section split) lives in server/waggledance.ts and shared/board.ts with no SDK import so test/server.test.ts runs under node --test. No code comments.",
    "verify": "node --test integrations/paseo/test/server.test.ts && (cd integrations/paseo && npm install --no-audit --no-fund && npx tsc --noEmit)",
    "must_haves": {
      "truths": [
        "a daemon that refuses the connection yields code down naming `waggledance serve`",
        "an HTML reply yields code auth",
        "agents 404 and 502 keep the board and set distinct notes",
        "a workspace inside a registered root maps to that project, the longest root wins",
        "the plugin typechecks against @getpaseo/plugin 0.10.3"
      ],
      "artifacts": [
        {
          "path": "integrations/paseo/index.client.tsx",
          "substantive": "registers surface, sidebar item, workspace panel, settings screen"
        }
      ],
      "key_links": [
        "shared/rpc.ts is the only definition of the RPC and settings, used by server and client"
      ],
      "prohibitions": [
        "no write call to waggledance",
        "never spawns `waggledance serve` or `open`"
      ]
    }
  },
  {
    "id": "ppp-4",
    "title": "Install both plugins with waggledance setup pi|paseo",
    "role": "code",
    "lane": "standard",
    "feature": "pi-paseo-plugin",
    "deps": [
      "ppp-2",
      "ppp-3"
    ],
    "decisions": [
      "D5",
      "D2",
      "D3",
      "D6"
    ],
    "files": [
      "crates/waggledance/src/setup.rs",
      "crates/waggledance/src/cli.rs",
      "crates/waggledance/src/main.rs"
    ],
    "read_first": [
      "/home/thanhsmind/Projects/refs/piggery/internal/cli/setup_paseo.go",
      "/home/thanhsmind/Projects/refs/piggery/internal/cli/setup_pi.go",
      "crates/waggledance/src/cli.rs",
      "crates/waggledance/src/doctor.rs",
      "docs/history/pi-paseo-plugin/CONTEXT.md"
    ],
    "action": "Add `waggledance setup <pi|paseo> [--remove] [--json]`. Embed every shipped file of integrations/pi (index.ts, installed.ts, package.json) and integrations/paseo (manifest, entries, client/, server/, shared/, package.json, tsconfig.json; never test/, node_modules/, lockfile) with include_str!, plus a VERSION file holding the crate version. Rewrite installed.ts to the current_exe absolute path. pi target: ~/.pi/agent/extensions/waggledance (PI_CODING_AGENT_DIR overrides the agent dir when set). paseo target: <data_dir>/paseo, then `paseo plugin ls --json` and install | remove+install (installed from another path) | enable | reload (files changed) | nothing. Write atomically through a sibling temp dir; when the target exists without VERSION, refuse naming the path. --remove deletes only a directory carrying VERSION (and `paseo plugin remove waggledance`). Second run with nothing changed prints that nothing changed. Output after setup pi: where it lives, that running pi sessions need /reload, and a warning when <agent dir>/mcp.json has a server named waggledance or waggle_dance (it overrides the extension; `pi mcp remove waggledance`). Output after setup paseo: 'Waggle Dance' appears in the Paseo sidebar, and turn on plugins in Paseo settings once. Both warn when current_exe sits under a target/ or .cache/cargo-target directory (a dev build path that later builds overwrite). Register `mod setup;` in main.rs. Unit tests over the file-writing core with a temp dir: fresh write, idempotent rerun, refuse foreign dir, installed.ts rewritten. No code comments.",
    "verify": "cargo test --profile fast -p waggledance setup::",
    "must_haves": {
      "truths": [
        "setup pi then a pi session lists mcp__waggledance__waggledance_view_file (live proof recorded at cap)",
        "setup paseo then `paseo plugin ls --json` shows waggledance enabled (live proof recorded at cap)",
        "a foreign directory at the target is refused untouched",
        "an mcp.json waggledance entry is reported"
      ],
      "artifacts": [
        {
          "path": "crates/waggledance/src/setup.rs",
          "substantive": "embedded file table, write/refuse/remove, paseo CLI driving, tests"
        }
      ],
      "key_links": [
        "cli.rs Command::Setup dispatches to setup::run",
        "main.rs declares mod setup"
      ],
      "prohibitions": [
        "never deletes a directory without VERSION",
        "never writes test files or node_modules into the install"
      ]
    }
  }
]
```

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| ppp-1 | Serve the board as JSON | server.rs, views.rs | — | `curl :7700/api/board` lists what waits on you, with no paths | `cargo test -p waggledance api_board` |
| ppp-2 | pi extension | integrations/pi/* (new) | — | a pi session's model can call `mcp__waggledance__waggledance_view_file` directly | `node --test integrations/pi/index.test.ts` |
| ppp-3 | Paseo plugin | integrations/paseo/* (new) | ppp-1 | a "Waggle Dance" screen and workspace panel in Paseo | node tests + `tsc --noEmit` |
| ppp-4 | `waggledance setup pi\|paseo` | setup.rs (new), cli.rs, main.rs | ppp-2, ppp-3 | one command installs each plugin; rerun says nothing changed | `cargo test -p waggledance setup::` + live install |

## Test matrix

- Happy: board JSON for a project with a gate-stopped feature — Pass when the row has `"waiting":true` and `reason` starts `Waiting on you`.
- Happy: `setup paseo` on a clean host — Pass when `paseo plugin ls --json` lists `waggledance` with `enabled: true`.
- Happy: `setup pi` — Pass when a `pi -p` session lists `mcp__waggledance__waggledance_view_file` among its direct tools (live, ppp-4 cap).
- Edge: rerun of either setup — Pass when it prints that nothing changed and no file mtime moves.
- Edge: workspace dir under two nested registered roots — Pass when the longer root's project is chosen.
- Error: `/api/agents` 404 (terminal off) or 502 (herdr down) — Pass when the board still renders and the agents section says which.
- Edge: workspace directory in no registered project — Pass when the panel says the workspace is not a Waggle Dance project.
- Error: daemon down — Pass when the board RPC returns `code: "down"` and the text names `waggledance serve`.
- Error: target dir exists without VERSION — Pass when setup exits non-zero naming the path and the dir is untouched.

## Open Questions

(none)

## Out of scope

- Fixing `commands.test` in `.bee/config.json` (it points at `packages/bee-rs`); cells prove with `cargo test -p waggledance` filters and node tests, named on each cap.

- Reporting pi session presence to Waggle Dance's Live list (agents still come from herdr and Paseo).
- Any action from Paseo (reply, approve a gate); D3 keeps it read-only.
- Publishing either plugin to npm.
