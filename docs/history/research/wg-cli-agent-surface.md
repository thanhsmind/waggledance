---
artifact_contract: bee-research/v1
topic: wg-cli-agent-surface
depth: standard
date: 2026-11-11
---

## Bottom Line

- Recommendation (ladder rung): reuse + adapt-upstream. Put a `wg` verb surface on the existing `waggledance` binary that calls the same `handle_*` functions MCP calls, and apply the Cloudflare `cf` agent conventions on top.
- Why this is the lightest credible path: the MCP handlers already open the engine straight from SQLite (`mcp.rs:102`) and build a `structuredContent` value. The CLI only has to call those handlers and print that value. It needs no daemon IPC and no second business-logic path.
- Why the next-best rung lost: a separate binary or a Code-Mode MCP (`search()` + `execute()`) would lower the schema cost but not the payload cost. Payload cost is where the tokens go (see Local).
- Settling question: where does the waggledance MCP spend tokens, and which `cf` patterns remove that spend? Status: **settled**. Payload dominates, and `cf`'s condensed JSON, field selection and search address it.
- Confidence: 75%
- Outcome: the owner approved the recommendations on 2026-11-11; they became feature `agent-cli` (`docs/history/agent-cli/CONTEXT.md`, D1-D5).

## Repo Snapshot

- Rust workspace, two crates (`waggledance-core`, `waggledance`), v0.5.5, clap 4 derive CLI, rusqlite store, tokio.
- The existing CLI covers register/list/status/doctor/mail/hold/bee upgrade. Dispatch, state, await, runs and task exist only as MCP tools.

## Findings

### Local

- `tools/list` is 8,915 bytes (about 2.2k tokens) for 7 tools. `ask_state` and `dispatch` carry long prose descriptions that teach the run tiers (`mcp.rs:185-275`). Claude Code defers these schemas behind ToolSearch, so this cost is small there. Hosts without deferral pay it on every session.
- Payloads are the real cost, measured on this host on 2026-11-11:
  - `waggledance_ask_state` with no project: 324,669 bytes (about 80k tokens). One project, `beehive`, makes up 103 KB of that, and 91 KB of it is `sessions`. `recent_decisions` adds another 9.6 KB.
  - `waggledance_runs` with no filter: 587,737 bytes (about 145k tokens) for 227 runs. The full `task` text is 488 KB of that. One run's `task` is about 2.3 KB.
  - `waggledance_projects`: 5.6 KB for 37 projects.
- The `content[0].text` field is a one-line summary ("227 run(s)"). All of the data sits in `structuredContent`, which the agent receives in full.
- Reusable pieces: `handle_projects`, `handle_ask_state`, `handle_dispatch`, `handle_runs`, `handle_task` and the await handler each return a JSON-RPC value whose `result.structuredContent` is the payload a CLI would print. `--json` is already the flag convention on `list`, `status`, `register`, `doctor` and `bee upgrade`.
- What is missing: a CLI path to dispatch/await/runs/task/state, field selection, a summary default, pagination or a limit, and a discovery verb.

### Upstream / Docs (Cloudflare `cf`)

- JSON is the default interface. It is "pretty printed for humans and condensed for agents for maximum context savings". Tables are dropped, and agents filter with `jq`. (Docs: Cloudflare blog "Introducing cf", mirrored 2026-09-28)
- Discovery by intent: `cf cli search "<natural language>"` returns matching commands, and `cf schema` describes one command. The agent is told about search "when it runs `--help` for the first time". (Docs)
- One schema source: a TypeScript schema defines the API, the CLI commands and the arguments, and it also generates OpenAPI. Consistency rules are enforced at the schema layer. For example, it is always `--json` and never `--format`, and one verb (`get`) is used where Wrangler mixed `info`, `get` and `describe`. (Docs: blog "Building a CLI for all of Cloudflare")
- Output states whether a command acts on remote or local resources. (Docs)
- Agents are steered with one line in CLAUDE.md/AGENTS.md ("prefer cf") and with a `cf-cli` skill. Cloudflare keeps an MCP server in Code Mode with two tools, `search()` and `execute()`, at under 1k tokens. It presents the CLI and MCP as complementary, not as replacements. (Docs: developers.cloudflare.com/agent-setup/claude-code)
- Not covered by any source: error shape, dry-run or confirm on mutations, pagination.

### Inference

- Translating `cf` to `wg` gives these rules:
  1. Summary by default and detail on request. `wg state` prints counts and phase per project. `wg state <project> --full` adds sessions and decisions. `wg runs` prints id/status/summary and never the full `task`. `wg runs show <id>` prints one task.
  2. `--json` everywhere, condensed when stdout is not a TTY, plus `--fields a,b` and `--limit N`.
  3. One verb set: `wg projects`, `wg state`, `wg dispatch`, `wg await`, `wg runs [show]`, `wg task <reply|redo|accept|allow|deny>`, `wg bee upgrade`.
  4. `wg search "<intent>"` and `wg schema <verb>`. The long prose on run tiers moves out of the always-loaded schema into `wg schema dispatch` or a skill, read only when needed.
  5. One source for flags and descriptions. clap derive already generates `--help`, and `wg schema` can emit JSON from the clap `Command` tree.
- Fixing the payload defaults would help MCP callers too. Trimming `sessions` and `task` in the MCP handlers removes most of the 80k–145k tokens even if MCP stays.
- `wg` as the name: a symlink or `argv[0]` alias of `waggledance` costs nothing. A separate binary does not.

## Risks, Unknowns, Follow-Ups

- `wg` clashes with WireGuard's `wg` binary, which is common on Linux hosts. Resolved by D5: the binary stays `waggledance`, and `wg` is an optional user-made symlink.
- Leaders running in other projects need the binary on PATH and permission to run it through each host's Bash permission rules. Still open after `agent-cli`.
- `await` blocks for up to 60 s. As a CLI it holds the Bash tool for that long, which is acceptable but should be stated in `--help`.
- Keep MCP or retire it: resolved by D4, MCP stays and its two heaviest payloads shrink.

## Source Pack

- Local: `crates/waggledance/src/cli.rs:20-135`, `crates/waggledance/src/mcp.rs:102,144-335,360-547`, live `waggledance mcp` calls (tools/list, projects, ask_state, runs) on 2026-11-11.
- Docs: https://blog.cloudflare.com/cf-cli-local-explorer/, https://noise.getoto.net/2026/09/28/introducing-cf-the-agentic-cli-for-the-entire-cloudflare-api/ (mirror of the Cloudflare launch post), https://developers.cloudflare.com/agent-setup/claude-code/
