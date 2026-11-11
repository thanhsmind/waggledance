# agent-cli — CONTEXT

Owner request (2026-11-11): give agents in every project a CLI to drive
waggledance, modeled on Cloudflare's `cf`, so they spend fewer tokens than
through the current MCP tools. The owner approved the research brief
`docs/history/research/wg-cli-agent-surface.md` the same day.

## Locked decisions

- D1 (a18d5070). New verbs `projects`, `state`, `dispatch`, `await`,
  `runs`, `task`, `schema`, `search` on the `waggledance` binary. Each data
  verb calls the handler its MCP tool calls; one logic path.
- D2 (ca226f61). JSON only: pretty on a TTY, compact otherwise. Summary by
  default; `--full` prints the MCP `structuredContent` unchanged;
  `--fields a,b` keeps those keys per row; `--limit N` caps rows. A tool
  error prints its message to stderr and exits 1.
- D3 (54105da6). `schema [verb]` prints verbs, flags and help as JSON built
  from the clap command tree; `search <text>` ranks verbs by term hits on
  name, help and flag help; top-level help names `search`.
- D4 (0066c745). MCP stays. `waggledance_runs` rows carry `task_summary`
  instead of `task`; optional `run_id` returns one run with its full
  `task`. The `waggledance_ask_state` rollup (no `project`) drops
  `sessions` and `recent_decisions` per project and adds `session_count`;
  the one-project answer is unchanged.
- D5 (c5bd8ac4). The binary stays `waggledance`; `wg` is an optional
  user-made symlink, never installed. Agent templates tell agents to prefer
  the CLI verbs, MCP as fallback.

## Read set

### Knowledge

| Concept | Why it matters |
|---------|----------------|
| docs/knowledge/areas/supervisor-daemon.md | MCP serves the seven tools this feature mirrors |
| docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md | the CLI proof must run the binary just built |
| docs/knowledge/patterns/prove-the-whole-path.md | one end-to-end CLI run owed |

### Code

| Path | What it decides |
|------|-----------------|
| `crates/waggledance/src/mcp.rs` | tool schemas, `handle_*` handlers, `task_summary`, `handle_runs`, `ask_state_digest` |
| `crates/waggledance/src/cli.rs` | clap `Command` enum and `run` dispatch |
| `docs/waggledance-agents-template.md` | agent snippet doctor/setup install |
| `docs/waggledance-supervisor-skill-template.md` | supervisor skill doctor/setup install |
