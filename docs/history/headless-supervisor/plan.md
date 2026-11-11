# headless-supervisor — plan

Lane: standard (1 flag: covered-contract-change — two MCP tools, three CLI
commands and every HTTP route go away). Decisions: `CONTEXT.md` D1-D5.

## Summary

waggledance stops being a browser cockpit and a document store. After this
change it is one binary that supervises agents: `waggledance serve` runs
the supervisor loops with no web port, `waggledance mcp` serves seven
supervisor tools, and nothing copies other projects' files into
`~/.waggledance/registry.db` any more. The board, the terminal page, the
docs and code reader, the guide, the desktop app and the Paseo sidebar
plugin are deleted.

## Load-bearing claims

Labels: `read` = opened at that line this session; `ran` = command run
this session. Evidence is a verbatim substring of the anchored line.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | MCP is stdio, not an HTTP route, so it survives the listener's removal | read | crates/waggledance/src/mcp.rs:108 | `let stdin = std::io::stdin();` |
| 2 | The only caller of the HTTP server is the serve command | read | crates/waggledance/src/cli.rs:388 | `rt.block_on(crate::server::serve())` |
| 3 | MCP borrows pane helpers that live in server.rs, so they must move before server.rs is deleted | read | crates/waggledance/src/server.rs:5378 | `pub(crate) fn project_panes(` |
| 4 | The bee status watch that triggers and notify use lives in watcher.rs, not in the file watcher watch.rs | read | crates/waggledance/src/trigger.rs:65 | `use crate::watcher::{statuses_from, BeeRoots, StatusCursor};` |
| 5 | The view tool is one of the registered MCP tools | read | crates/waggledance/src/mcp.rs:166 | `"name": "waggledance_view_file",` |
| 6 | Migrations are version-gated, so an older binary skips a newer step and its CREATE TABLE IF NOT EXISTS only recreates empty tables | read | crates/waggledance-core/src/repository.rs:1286-1288 | `for (target, step) in MIGRATIONS { /         if version >= *target { /             continue;` |
| 7 | The files table is part of the base schema | read | crates/waggledance-core/src/repository.rs:1562 | `CREATE TABLE IF NOT EXISTS files (` |
| 8 | The Paseo plugin reads the HTTP board | read | integrations/paseo/server/waggledance.ts:111 | `getJson(base, "/api/board", opts.timeoutMs)` |
| 9 | The pi extension talks to waggledance over MCP stdio | read | integrations/pi/index.ts:8 | `args: ["mcp"],` |
| 10 | doctor installs the viewer skill from a docs template | read | crates/waggledance/src/doctor.rs:681 | `const SKILL_TEMPLATE: &str = include_str!("../../../docs/waggledance-skill-template.md");` |
| 11 | The desktop crate is already outside the workspace build | read | Cargo.toml:4 | `# Desktop shell (Tauri) is excluded from the default workspace build because it` |

## Discovery

Grep of every `crate::server`, `crate::views`, `crate::guide`,
`crate::watch` and `crate::watcher` use outside those modules: only
`cli.rs:388` (serve), `mcp.rs:909-958,2716` (four pane helpers) and
`watcher` users (`main.rs`, `trigger.rs`, `notify/mod.rs`, kept). Core
`indexer` use outside the index path is the `now_rfc3339`,
`rel_path_str`, `walk_would_index` and bounded-scan helpers.

## Approach

Recommended path: move the supervisor pieces that sit inside UI modules
into a kept module, swap `serve` for a loop-only daemon, then delete the UI
and the file store top-down — binary crate first, core crate second, so
each cell leaves a building workspace.

Rejected alternatives:
- Keep `/health` only: D2 forbids any listener.
- Leave the `files` tables in place unused: D4 asks for the migration; the
  old 1 GB store was the lock source in registry-db-lock.
- One cell for everything: the core deletion cannot compile until the
  binary stops importing it, and the docs work is independent.

Risk map:
- Daemon loops stop starting / MEDIUM / hs-1 / a scratch-HOME daemon run
  shows the lock file, `waggledance status` reports it running, no port open.
- Old running binaries meet the new schema / LOW / hs-2 / claim 6; test
  that a v13 database migrates and keeps `projects` and `runs` rows.
- Config files with `[server]` stop loading / LOW / hs-2 / test that a
  config with `[server]` still loads.

Waves: hs-1 and hs-3 in parallel (disjoint files); hs-2 after hs-1 (the
binary must stop importing core modules first); hs-4 last.

## Cells

```json
[
 {
  "id": "hs-1",
  "feature": "headless-supervisor",
  "title": "Run the supervisor daemon without a web server and drop the viewer surfaces from the binary",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance/src/**",
   "crates/waggledance/assets/**",
   "crates/waggledance/tests/**",
   "crates/waggledance/Cargo.toml",
   "docs/waggledance-skill-template.md",
   "docs/waggledance-agents-template.md",
   "docs/waggledance-supervisor-skill-template.md"
  ],
  "action": "In crates/waggledance only. (1) Move the supervisor helpers out of server.rs into a new kept module (project_panes, herdr_session_ids, project_bee_activity, apply_bee_activity, telegram_credentials, open_notify_store and whatever they need) and point mcp.rs and main.rs at it. (2) Replace server::serve with a daemon entry that builds the engine, starts TerminalBackground::reconcile and the existing background loops exactly as serve does today, writes the daemon lock, waits for ctrl-c, removes the lock, and binds no socket; drop the startup index reconcile, reclaim_legacy_index_at_startup, the agent-log warmup and the file watcher. waggledance status, stop and restart keep working off the lock file and pid; no URL is printed. (3) Delete server.rs, views.rs, guide.rs, watch.rs, assets/, tests/e2e_open.rs and every web-only helper and test. (4) MCP: remove waggledance_view_file and waggledance_search from the tool list, the dispatcher and the tests; rewrite the server instructions to describe the supervisor tools. (5) CLI: remove open, search and refresh; register/list/unregister stay and no longer index files. (6) doctor and setup stop installing the viewer skill (delete docs/waggledance-skill-template.md and its install check; when ~/.claude/skills/waggledance/SKILL.md byte-matches a template waggledance shipped, doctor --fix removes it) and the agent snippet in docs/waggledance-agents-template.md loses the Documentation Viewing section. (7) Drop crate dependencies the binary no longer uses (axum, tower-http, minijinja and any other unused). The core crate is untouched in this cell. No code comments.",
  "verify": "cargo test --profile fast -p waggledance",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [],
  "decisions": [
   "1d17ffbc-84b2-4593-a497-aeb9adfad65c",
   "b49a9fce-7652-45a3-92d0-f76641e6d5f0",
   "9478b44c-82dd-4eca-a25a-dc6ada3c77af",
   "479035d3-ccc4-44d5-b307-d6437266f1a9",
   "bf8db4a9-8b22-4df7-8b2d-c42f5363ad5a"
  ],
  "read_first": [
   "docs/history/headless-supervisor/CONTEXT.md",
   "docs/history/headless-supervisor/plan.md",
   "docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md",
   "docs/knowledge/patterns/a-second-daemon-proves-it-without-taking-the-users.md"
  ],
  "must_haves": {
   "truths": [
    "MCP tools/list returns exactly projects, ask_state, dispatch, await, runs, task and bee_upgrade",
    "waggledance serve under a scratch HOME writes the daemon lock, opens no TCP port, and waggledance status reports it running",
    "the heartbeat, reaper, wake, trigger, notify and worktree-sweep loops still start from the daemon",
    "waggledance open, search and refresh are unknown subcommands"
   ],
   "artifacts": [
    {"path": "crates/waggledance/src/server.rs", "substantive": "deleted"},
    {"path": "crates/waggledance/src/views.rs", "substantive": "deleted"},
    {"path": "crates/waggledance/assets", "substantive": "deleted"}
   ],
   "key_links": [
    "cli.rs serve calls the new daemon entry",
    "mcp.rs imports the moved pane helpers from the kept module"
   ],
   "prohibitions": [
    "No HTTP listener of any kind",
    "No edit under crates/waggledance-core",
    "No code comments"
   ],
   "open_choices": [
    "Name and placement of the kept module for the moved helpers"
   ]
  }
 },
 {
  "id": "hs-2",
  "feature": "headless-supervisor",
  "title": "Remove the central file store from the core crate and drop its tables",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance-core/**",
   "crates/waggledance/src/**",
   "Cargo.toml",
   "Cargo.lock"
  ],
  "action": "After hs-1. Delete the core modules that only serve the file store and the viewer: indexer (move now_rfc3339 and any helper the supervisor code still calls into a kept core module and update callers), render, doc_links, link_resolver, code_source, git_diff, fuzzy, short_link, and the Engine/Repository methods and domain types for files, links, search, refresh and reclaim. Add the next migration step that drops files_fts, links and files (and their indexes) and runs no VACUUM; remove those tables from SCHEMA. Keep the [server] section of config.toml parseable so existing config files still load. Remove workspace dependencies nothing uses any more (comrak, syntect, ammonia, ignore, notify, notify-debouncer-full, nucleo-matcher, axum, tower-http, minijinja, and any other now unused) and the desktop exclude comment and entry in Cargo.toml. Update tests: delete tests of removed code; add a migration test that a database at the previous user_version with files, links, files_fts, projects and runs rows ends with the three tables gone and the projects and runs rows kept. No code comments.",
  "verify": "cargo test --profile fast -p waggledance-core -p waggledance",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["hs-1"],
  "decisions": [
   "479035d3-ccc4-44d5-b307-d6437266f1a9",
   "bf8db4a9-8b22-4df7-8b2d-c42f5363ad5a"
  ],
  "read_first": [
   "docs/history/headless-supervisor/CONTEXT.md",
   "docs/history/headless-supervisor/plan.md",
   "docs/history/registry-db-lock/CONTEXT.md"
  ],
  "must_haves": {
   "truths": [
    "A registry at the previous user_version migrates with files, links and files_fts dropped and every projects and runs row kept",
    "A config.toml containing a [server] section still loads",
    "No source file references files_fts, the files table or the removed modules"
   ],
   "artifacts": [
    {"path": "crates/waggledance-core/src/indexer.rs", "substantive": "deleted"},
    {"path": "crates/waggledance-core/src/render.rs", "substantive": "deleted"}
   ],
   "key_links": [
    "MIGRATIONS gains one step that drops the file tables"
   ],
   "prohibitions": [
    "No VACUUM inside the migration",
    "No change to the runs, inbox, tasks, leader_holds or permission_requests tables",
    "No code comments"
   ],
   "open_choices": []
  }
 },
 {
  "id": "hs-3",
  "feature": "headless-supervisor",
  "title": "Remove the desktop app and the Paseo plugin and rewrite the docs for the supervisor role",
  "lane": "standard",
  "role": "docs",
  "files": [
   "crates/waggledance-desktop/**",
   "integrations/paseo/**",
   "integrations/pi/index.ts",
   "README.md",
   "CLAUDE.md",
   "AGENTS.md",
   "PRD.md",
   "docs/usage.md",
   "docs/mermaid-demo.md",
   "docs/assets/**",
   ".github/workflows/release.yml",
   "install.sh",
   "install.ps1"
  ],
  "action": "Delete crates/waggledance-desktop and integrations/paseo. Change the pi extension description to name the supervisor tools only. Rewrite README.md for the supervisor role: what waggledance does now (project registry, dispatch to a project's leader, await/runs/task, ask_state, bee upgrade, wake/mail/notify), the seven MCP tools, the CLI commands that remain, install; no board, viewer, search, desktop or screenshots. Remove the Documentation Viewing section from CLAUDE.md and AGENTS.md (keep Building). Update or delete docs/usage.md and docs/mermaid-demo.md so no doc tells a reader to open a browser view; drop screenshot assets only the old README used. Remove desktop and plugin mentions from release.yml and the install scripts. Touch nothing under crates/waggledance or crates/waggledance-core or Cargo.toml.",
  "verify": "! rg -n 'waggledance_view_file|waggledance_search|waggledance open|/api/board|waggledance-desktop' README.md CLAUDE.md AGENTS.md docs/usage.md integrations .github install.sh install.ps1",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [],
  "decisions": [
   "9478b44c-82dd-4eca-a25a-dc6ada3c77af",
   "f40f990e-6913-482e-a421-a28f834641eb",
   "479035d3-ccc4-44d5-b307-d6437266f1a9"
  ],
  "read_first": [
   "docs/history/headless-supervisor/CONTEXT.md",
   "README.md"
  ],
  "must_haves": {
   "truths": [
    "README describes waggledance as a headless supervisor with seven MCP tools",
    "No doc in the verify list mentions the removed tools, commands or endpoints"
   ],
   "artifacts": [
    {"path": "crates/waggledance-desktop", "substantive": "deleted"},
    {"path": "integrations/paseo", "substantive": "deleted"}
   ],
   "key_links": [
    "integrations/pi and integrations/openclaw keep calling waggledance mcp"
   ],
   "prohibitions": [
    "No edit under crates/waggledance, crates/waggledance-core or Cargo.toml",
    "No edit to docs/history or docs/knowledge"
   ],
   "open_choices": []
  }
 },
 {
  "id": "hs-4",
  "feature": "headless-supervisor",
  "title": "Consolidate this feature's knowledge and write its delivery",
  "lane": "standard",
  "role": "writer",
  "files": [
   "docs/knowledge/**",
   "docs/history/headless-supervisor/delivery.md"
  ],
  "action": "Run `bee knowledge consolidate --feature headless-supervisor`. Retire or rewrite the knowledge concepts that describe the removed board, viewer, search, file index, desktop app and Paseo plugin; record the headless daemon and the seven-tool MCP surface, anchored to code; never restate code. Write docs/history/headless-supervisor/delivery.md listing each concept changed and why, one line each. Run `.bee/bin/bee knowledge check` green. One commit.",
  "verify": ".bee/bin/bee knowledge check",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["hs-1", "hs-2", "hs-3"],
  "decisions": [
   "1d17ffbc-84b2-4593-a497-aeb9adfad65c",
   "b49a9fce-7652-45a3-92d0-f76641e6d5f0",
   "9478b44c-82dd-4eca-a25a-dc6ada3c77af",
   "479035d3-ccc4-44d5-b307-d6437266f1a9",
   "f40f990e-6913-482e-a421-a28f834641eb"
  ],
  "read_first": [
   "docs/history/headless-supervisor/CONTEXT.md",
   "docs/history/headless-supervisor/plan.md"
  ],
  "must_haves": {
   "truths": [
    "No knowledge concept describes the removed surfaces as live",
    "docs/history/headless-supervisor/delivery.md lists every concept the feature changed"
   ],
   "artifacts": [
    {"path": "docs/history/headless-supervisor/delivery.md", "substantive": "the feature's delivery record"}
   ],
   "key_links": ["the delivery lands in the same merge as the code"],
   "prohibitions": ["No restating of code in concepts", "No code comments"],
   "open_choices": []
  }
 }
]
```

## Test matrix

- Happy: MCP `tools/list` names the seven tools. Pass when the list equals
  them exactly.
- Happy: scratch-HOME `waggledance serve`, then `waggledance status`. Pass
  when status reports the daemon running and `ss -ltnp` shows no socket
  owned by its pid.
- Edge: a registry at the previous `user_version` with file rows. Pass when
  `files`, `links`, `files_fts` are gone and `projects`/`runs` counts are
  unchanged.
- Edge: a `config.toml` carrying `[server]`. Pass when `Config::load` returns
  it without error.
- Error: `waggledance open x.md`. Pass when clap reports an unrecognized
  subcommand.

## Proof at feature end

`bee test` (the full workspace suite) in the worktree, then the
scratch-HOME daemon run from the test matrix on the fast build, the
artifact confirmed by content.

## Open Questions

(none)

## Out of scope

- A release and version bump.
- Removing the `[server]` keys from users' existing config files.
- Cleaning `~/.waggledance/registry.db` beyond the migration (no VACUUM).
