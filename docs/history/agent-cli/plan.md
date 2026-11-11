# agent-cli — plan

Lane: standard (2 flags: public-contracts, covered-contract-change — new CLI
verbs, and the `waggledance_runs` and `waggledance_ask_state` rollup
payloads change shape). Decisions: `CONTEXT.md` D1-D5.

## Summary

Agents in any project can drive waggledance from Bash:
`waggledance state`, `waggledance dispatch`, `waggledance await`,
`waggledance runs`, `waggledance task` and `waggledance projects` print
compact JSON. By default they print a summary, and `--full` prints what MCP
returns. `waggledance search "<what you want>"` and `waggledance schema` let
an agent find a verb without loading every tool description. MCP stays, but
its two heaviest answers shrink: the run list carries a one-line task
summary, and the all-projects state drops session and decision lists.

Mode: `standard` — 2 risk flags: public-contracts, covered-contract-change
Why this is the least workflow that protects the work: the CLI reuses the
MCP handlers, so the only new logic is output shaping and discovery, and
the MCP payload change is guarded by the existing handler tests.

## Load-bearing claims

Labels: `read` = opened at that line this session; `ran` = command run
this session. Evidence is a verbatim substring of the anchored line.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | MCP builds its engine straight from the registry database, so a CLI verb can do the same without the daemon | read | crates/waggledance/src/mcp.rs:102 | `let engine = Engine::new(SqliteStore::open(&registry_db_path())?, Config::load());` |
| 2 | Every tool goes through one dispatcher a CLI can call | read | crates/waggledance/src/mcp.rs:330 | `fn handle_tool_call(` |
| 3 | The run list returns the full task text per row | read | crates/waggledance/src/mcp.rs:1875 | `"task": r.task,` |
| 4 | A one-line task summary helper already exists | read | crates/waggledance/src/mcp.rs:826 | `fn task_summary(task: &str) -> String {` |
| 5 | Each ask_state digest carries the full session list | read | crates/waggledance/src/mcp.rs:583 | `"sessions": snapshot.sessions.iter().map(|s| json!({` |
| 6 | The rollup answer is the per-project digests | read | crates/waggledance/src/mcp.rs:547 | `"structuredContent": { "projects": digests }` |
| 7 | The ask_state description sends callers to waggledance_runs for a full task, which D4 must keep reachable | read | crates/waggledance/src/mcp.rs:214 | ``judge, read the run's full `task` from `waggledance_runs`.`` |
| 8 | CLI subcommands are a clap derive enum dispatched in cli.rs | read | crates/waggledance/src/cli.rs:156 | `Command::Mcp => crate::mcp::run(),` |
| 9 | doctor installs the agent snippet from the repo template, so editing the template reaches every project | read | crates/waggledance/src/doctor.rs:485 | `const AGENT_TEMPLATE: &str = include_str!("../../../docs/waggledance-agents-template.md");` |
| 10 | The agent snippet currently steers agents to the MCP tools | read | docs/waggledance-agents-template.md:11 | `Waggledance supervises agents across registered projects. Use its MCP tools:` |
| 11 | The rollup and the run list are the token cost | ran | waggledance mcp, tools/call waggledance_ask_state {} and waggledance_runs {} piped to wc -c | `324669` / `587737` |

## Discovery

Read `mcp.rs` tool schemas, `handle_tool_call`, `handle_ask_state`,
`handle_runs`, `task_summary`, `tool_error`, and `cli.rs` `Command` and
`run`. Measured live payloads on 2026-11-11: rollup 324,669 bytes
(`beehive` alone 103 KB, of which `sessions` 91 KB), runs 587,737 bytes for
227 runs (`task` 488 KB). Research brief:
`docs/history/research/wg-cli-agent-surface.md`.

## Approach

Recommended path: first trim the two MCP payloads inside their handlers
(D4) and expose the dispatcher to the rest of the crate; then add the CLI
verbs as a thin layer that builds the engine, calls that dispatcher, and
shapes `structuredContent` (D1-D3); in parallel, point the agent templates
and README at the CLI (D5).

Rejected alternatives:
- A separate `wg` binary or crate: duplicates install and logic (D1, D5).
- A daemon IPC client: the handlers already read SQLite directly (claim 1).
- Trimming only in the CLI: leaves MCP callers paying 80k-145k tokens (D4).

Risk map:
- Callers that read `task` from the run list break / MEDIUM / ac-1 / the
  new `run_id` path returns the full task; the description at claim 7 is
  rewritten to name it; existing runs tests stay green.
- The CLI drifts from MCP / LOW / ac-2 / `--full` output equals the MCP
  `structuredContent` for the same args, asserted in a test.
- The proof runs a stale binary / MEDIUM / ac-2 / resolve the artifact path
  from cargo's JSON and confirm it by content.

Waves: ac-1 and ac-3 in parallel (disjoint files); ac-2 after ac-1 (both
write `mcp.rs`, and ac-2 calls the dispatcher ac-1 exposes); ac-4 last.

## Cells

```json
[
 {
  "id": "ac-1",
  "feature": "agent-cli",
  "title": "Shrink the MCP run list and state rollup and expose the tool dispatcher to the crate",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance/src/mcp.rs"
  ],
  "action": "Per D4 in crates/waggledance/src/mcp.rs. (1) waggledance_runs: each listed row carries task_summary (the existing task_summary helper) instead of task; add an optional string argument run_id that returns exactly that run as a one-row list with its full task plus task_summary, and a tool error naming the id when no run matches; add run_id to the schema. (2) waggledance_ask_state without project: drop sessions and recent_decisions from each project digest and add session_count (the number of sessions); the one-project answer keeps every field. Update both tool descriptions so they state these shapes, and change the sentence that tells callers to read the full task from waggledance_runs so it names the run_id argument. (3) Make one crate-visible entry that runs a tool by name with JSON arguments and returns its result, building the engine and lazy orchestration the way run() does, so cli.rs can call it; the stdio loop keeps working unchanged. Tests: extend the existing runs and ask_state tests for the new shapes, run_id hit and miss, and the one-project answer still carrying sessions. No code comments.",
  "verify": "cargo test --profile fast -p waggledance mcp::",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [],
  "decisions": ["0066c745-8651-40c7-bca6-86aebef52824", "a18d5070-b194-41f4-b771-963d11e51df2"],
  "read_first": [
   "docs/history/agent-cli/CONTEXT.md",
   "docs/history/agent-cli/plan.md",
   "crates/waggledance/src/mcp.rs"
  ],
  "must_haves": {
   "truths": [
    "waggledance_runs without run_id returns rows with task_summary and no task key",
    "waggledance_runs with run_id returns that one run with its full task",
    "waggledance_ask_state without project returns digests with session_count and without sessions or recent_decisions",
    "waggledance_ask_state with project still returns sessions and recent_decisions"
   ],
   "artifacts": [
    {"path": "crates/waggledance/src/mcp.rs", "substantive": "trimmed runs and rollup shapes, run_id argument, crate-visible tool entry"}
   ],
   "key_links": ["the stdio loop and the new crate entry both call handle_tool_call"],
   "prohibitions": ["No change to dispatch, await, task, projects or bee_upgrade results", "No code comments"],
   "open_choices": ["Name and signature of the crate-visible tool entry"]
  }
 },
 {
  "id": "ac-2",
  "feature": "agent-cli",
  "title": "Add the agent CLI verbs with summary output, field selection and command search",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance/src/cli.rs",
   "crates/waggledance/src/agent_cli.rs",
   "crates/waggledance/src/main.rs",
   "crates/waggledance/src/mcp.rs"
  ],
  "action": "After ac-1, per D1-D3. Add subcommands to the clap Command enum: projects; state [PROJECT]; dispatch --project P (--leader | --preset L | --pane-id ID) with the task from --task, --task-file or stdin, plus --request-id, --follow-up-of, --handoff/--no-handoff, --steer; await RUN_ID [--timeout SECS]; runs [--project P] and runs show RUN_ID; task CORR reply|redo|accept|allow|deny [--text T] [--req-id R]; schema [VERB]; search TEXT. Every data verb maps its flags to the matching MCP tool arguments and calls the crate entry from ac-1. Output per D2: JSON only, pretty when stdout is a TTY and compact otherwise; data verbs print a summary by default (state rollup: per project id, feature, phase, mode, waiting_on_live, cell counts and the leader count; one-project state: the digest without sessions and with at most three recent decisions; runs: id, project_id, status, task_summary, updated_at; await: status and the last lines of delta), --full prints the tool's structuredContent unchanged, --fields a,b keeps those keys on each row, --limit N caps the rows. A tool result with isError prints its text to stderr and exits 1. schema prints every verb or one verb with its flags and help as JSON built by walking the clap command tree; search ranks verbs by term hits on name, help and flag help and prints the top matches with a usage line; the top-level help text names search. Existing commands keep their behavior. Tests: summary and --full for state and runs on an in-memory engine, --fields and --limit, --full equal to the MCP structuredContent for the same args, schema covers every new verb, search ranks dispatch first for a dispatch-shaped query, tool error exits non-zero. Then build with `cargo build --profile fast -p waggledance --message-format=json`, take the executable path from the compiler-artifact line, confirm it by `strings <path> | rg` on a literal only this change contains, and run `<path> state`, `<path> runs --limit 3` and `<path> search dispatch` against the real registry, recording byte counts. No code comments.",
  "verify": "cargo test --profile fast -p waggledance",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["ac-1"],
  "decisions": ["a18d5070-b194-41f4-b771-963d11e51df2", "ca226f61-879d-4519-9a8e-e48dd72855d2", "54105da6-1c35-42b4-a58f-dcf9ad8e7fb4"],
  "read_first": [
   "docs/history/agent-cli/CONTEXT.md",
   "docs/history/agent-cli/plan.md",
   "crates/waggledance/src/cli.rs",
   "crates/waggledance/src/mcp.rs",
   "docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md"
  ],
  "must_haves": {
   "truths": [
    "waggledance state with no project prints one compact JSON summary row per project with no sessions",
    "waggledance runs prints task_summary and never the full task unless runs show is used",
    "--full prints the same structuredContent the MCP tool returns for the same arguments",
    "waggledance search dispatch lists the dispatch verb first",
    "waggledance schema prints JSON naming every new verb and its flags",
    "a tool error prints to stderr and exits with status 1"
   ],
   "artifacts": [
    {"path": "crates/waggledance/src/agent_cli.rs", "substantive": "verb-to-tool mapping, output shaping, schema and search"}
   ],
   "key_links": ["cli.rs run dispatches the new verbs to agent_cli", "agent_cli calls the mcp crate entry from ac-1"],
   "prohibitions": ["No second implementation of any tool's logic", "No change to existing subcommands", "No code comments"],
   "open_choices": ["Module layout inside agent_cli.rs", "The exact summary key list per verb beyond the ones named"]
  }
 },
 {
  "id": "ac-3",
  "feature": "agent-cli",
  "title": "Point the agent templates and README at the CLI verbs",
  "lane": "standard",
  "role": "docs",
  "files": [
   "docs/waggledance-agents-template.md",
   "docs/waggledance-supervisor-skill-template.md",
   "README.md",
   "docs/usage.md"
  ],
  "action": "Per D5 and the verb set in D1-D4. In docs/waggledance-agents-template.md, tell agents to drive waggledance with the CLI verbs (waggledance projects, state, dispatch, await, runs, task; waggledance search and schema to find a verb), compact JSON by default, --full for the MCP-sized answer, and the MCP tools as the fallback when Bash is not available. In docs/waggledance-supervisor-skill-template.md, give each MCP tool step its CLI equivalent beside it, and change the full-task lookup to `waggledance runs show <run_id>` or waggledance_runs with run_id. In README.md and docs/usage.md add the agent CLI verbs, the summary/--full/--fields/--limit rules, and that wg can be a user-made symlink but no installer creates one because of WireGuard. Do not edit CLAUDE.md or AGENTS.md. Run cargo test --profile fast -p waggledance doctor:: setup:: since both include these templates.",
  "verify": "cargo test --profile fast -p waggledance doctor:: setup::",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [],
  "decisions": ["c5bd8ac4-1885-47c4-a868-bfed37d86133", "0066c745-8651-40c7-bca6-86aebef52824"],
  "read_first": [
   "docs/history/agent-cli/CONTEXT.md",
   "docs/waggledance-agents-template.md",
   "docs/waggledance-supervisor-skill-template.md"
  ],
  "must_haves": {
   "truths": [
    "The agent template names the CLI verbs first and MCP as the fallback",
    "The supervisor skill template gives the CLI form beside each MCP tool step",
    "README says wg is an optional user-made symlink"
   ],
   "artifacts": [
    {"path": "docs/waggledance-agents-template.md", "substantive": "CLI-first instructions"}
   ],
   "key_links": ["doctor and setup include the edited templates"],
   "prohibitions": ["No edit to CLAUDE.md, AGENTS.md or any Rust source", "No installer creates a wg link"],
   "open_choices": []
  }
 },
 {
  "id": "ac-4",
  "feature": "agent-cli",
  "title": "Consolidate this feature's knowledge and write its delivery",
  "lane": "standard",
  "role": "writer",
  "files": [
   "docs/knowledge/**",
   "docs/history/agent-cli/delivery.md"
  ],
  "action": "Run `bee knowledge consolidate --feature agent-cli`. Record the agent CLI surface (verbs, summary versus --full, schema and search) and the trimmed MCP runs and rollup shapes in the supervisor-daemon area, anchored to code; never restate code. Write docs/history/agent-cli/delivery.md listing each concept changed and why, one line each. Run `.bee/bin/bee knowledge check` green. One commit.",
  "verify": ".bee/bin/bee knowledge check",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["ac-1", "ac-2", "ac-3"],
  "decisions": ["a18d5070-b194-41f4-b771-963d11e51df2", "ca226f61-879d-4519-9a8e-e48dd72855d2", "54105da6-1c35-42b4-a58f-dcf9ad8e7fb4", "0066c745-8651-40c7-bca6-86aebef52824", "c5bd8ac4-1885-47c4-a868-bfed37d86133"],
  "read_first": [
   "docs/history/agent-cli/CONTEXT.md",
   "docs/history/agent-cli/plan.md"
  ],
  "must_haves": {
   "truths": [
    "The supervisor-daemon area describes the CLI verbs and the trimmed MCP shapes",
    "docs/history/agent-cli/delivery.md lists every concept the feature changed"
   ],
   "artifacts": [
    {"path": "docs/history/agent-cli/delivery.md", "substantive": "the feature's delivery record"}
   ],
   "key_links": ["the delivery lands in the same merge as the code"],
   "prohibitions": ["No restating of code in concepts", "No code comments"],
   "open_choices": []
  }
 }
]
```

## Test matrix

- Happy: `waggledance state` on a registry with two projects. Pass when the
  output is one JSON line with two rows and no `sessions` key.
- Happy: `waggledance runs --limit 1`. Pass when one row prints with
  `task_summary` and no `task`.
- Happy: `waggledance search "send a task to a leader"`. Pass when the
  first match is `dispatch`.
- Edge: `--full` on `state <project>`. Pass when it equals the MCP
  `structuredContent.project` for the same project.
- Edge: MCP `waggledance_runs` with `run_id`. Pass when one row carries the
  full `task`.
- Error: `waggledance runs show no-such-run`. Pass when stderr names the id
  and the exit status is 1.

## Proof at feature end

`bee test` (the full workspace suite) in the worktree, then the fast-build
CLI run from ac-2 against the real registry, the artifact confirmed by
content, with the byte counts beside the 324,669 and 587,737 baselines.

## Open Questions

(none)

## Out of scope

- CLAUDE.md and AGENTS.md: the main checkout has uncommitted owner edits
  to both; doctor refreshes the installed snippet from the template.
- Creating a `wg` symlink in any installer (D5).
- A release and version bump.
