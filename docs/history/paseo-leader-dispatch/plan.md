---
artifact_contract: bee-plan/v1
mode: high-risk
---

# Plan: Paseo leader dispatch

## Summary

Waggle Dance learns to hand a request to a project's leader inside Paseo. A
new `[paseo]` section in `~/.waggledance/config.toml` turns it on and, per
project, names the Paseo workspace the leader lives in. `waggledance_dispatch`
gains a `leader` target: it finds the project's labelled leader and sends it
the task, or starts one in the right workspace when none exists. The run is
stored like any other run, so `waggledance_await`, `waggledance_runs` and the
board report it, and the reaper closes it out when nobody waits on it. The
Waggle Dance panel in Paseo gets a request box that uses the same door, and
the supervisor skill learns the leader mode. External agents (openclaw,
hermes, aicoworker) reach it through `waggledance mcp` like any MCP client.

Mode: `high-risk` — 4 risk flags: external systems (the `paseo` CLI), public
contracts (the `waggledance_dispatch` schema), authorization (a new write path
under the orchestration gate), multi-domain (Rust daemon + TypeScript plugin).
Why this is the least workflow that protects the work: a wrong gate, a wrong
workspace or a false "done" sends or reports real work wrongly, so the shape
carries the full matrix, but it adds no schema migration and no new door.

## Requirements (from CONTEXT.md)

- D1: a Paseo backend for `waggledance_dispatch` in the daemon, driving the `paseo` CLI; runs in the same table, read by the same `await`/`runs` tools.
- D2: one long-lived labelled leader per project — `paseo send` to it, `paseo run` + labels when none exists.
- D3: workspace from config, else the local workspace at the project root, else a new one at the root.
- D4: three sources, one door; the caller names the project; the task text is never edited.
- D5: orchestration opt-in required; callers pass project + task only; the plugin's one write is a request to a leader.
- D6: the leader path is gated by orchestration + `[paseo] enabled`, not `terminal.enabled`; refusals name the config line.
- D7: a busy leader refuses; leader creation is serialized per project.
- D8: more than one root workspace and no config entry refuses, naming the ids.
- D9: the composer calls `waggledance mcp` over stdio from the workspace panel; no CLI subcommand, no HTTP write.
- D10: provider/mode/thinking default empty; leader title `<project name> leader`; labels `waggledance.role=leader`, `waggledance.project=<id>`.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command executed); evidence is a verbatim substring of the anchor.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | `paseo run` targets an existing workspace; it must always be passed, since its default is the caller's workspace | ran | `paseo run --help` | `--workspace <id>                  Run in an existing workspace (defaults to` |
| 2 | `paseo run` takes the working directory explicitly (else the caller's cwd), so `--cwd <root>` is always passed | ran | `paseo run --help` | `--cwd <path>                      Working directory (default: current)` |
| 3 | `paseo run` labels an agent | ran | `paseo run --help` | `--label <key=value>               Add label(s) to the agent (can be used` |
| 4 | `paseo run` takes the prompt as a positional argument, so the task goes after `--` | ran | `paseo run --help` | `Usage: paseo run [options] <prompt>` |
| 5 | A leader is found again by label | ran | `paseo ls --help` | `--label <key=value>    Filter by label (can be used multiple times) (default:` |
| 6 | `paseo ls` is directory-scoped unless `-g` is passed | ran | `paseo ls --help` | `-g, --global           List agents across all directories` |
| 7 | IDs-only output avoids parsing run/workspace JSON | ran | `paseo --help` | `-q, --quiet                      minimal output (IDs only)` |
| 8 | `paseo send` reads the message from a file and returns without waiting | ran | `paseo send --help` | `--no-wait              Return immediately without waiting for completion` |
| 9 | Status, archive state and pending permissions come from `paseo inspect --json` | ran | `paseo inspect 68b5798 --json` | `"PendingPermissions": [],` |
| 10 | Logs can be cut at the dispatch time, giving a per-run delta | ran | `paseo logs --help` | `--since <time>         Show logs since timestamp` |
| 11 | Log text is plain text with a `[User]` prefix for the user's turn, even under `--json` | ran | `paseo logs 68b5798 --tail 6 --filter text` | `[User] Cài đặt bản mới nhất cho waggledance` |
| 12 | A workspace can be created at a path for D3's last fallback | ran | `paseo workspace create --help` | `--path <path>                 Local directory or source checkout (default:` |
| 13 | Two local workspaces can share one root, so D8's refusal is real | ran | `paseo workspace ls --json` | `"isolation": "local",` |
| 14 | The CLI prints startup noise on stderr; stdout alone is parsed | ran | `paseo ls -g --json 2>/dev/null` | `"shortId": "35eca12",` |
| 15 | The orchestration gate already exists in `handle_dispatch` and is reused (D5, D6) | read | `crates/waggledance/src/mcp.rs:944` | `if !engine.orchestration_allowed(&project) {` |
| 16 | `terminal.enabled` is checked before the orchestration gate today, so the leader branch must sit before it (D6) | read | `crates/waggledance/src/mcp.rs:937` | `if !engine.config.terminal.enabled {` |
| 17 | The reaper's query takes only runs with a preset label, so `preset_label = None` keeps leader runs out of the herdr sweep | read | `crates/waggledance-core/src/repository.rs:233` | `FROM runs WHERE status='working' AND preset_label IS NOT NULL` |
| 18 | `finish` closes only runs with a preset label, so a leader run is never closed | read | `crates/waggledance/src/orchestrate.rs:892` | `if completion == Completion::Declared && run.preset_label.is_some() && transcript_stored {` |
| 19 | The board judges a working run alive by herdr pane id, so it needs a `paseo:` branch | read | `crates/waggledance/src/server.rs:3501` | `p.pane_id == run.pane_id);` |
| 20 | `pane_id` is free text, so `paseo:<agent-id>` needs no migration | read | `crates/waggledance-core/src/repository.rs:707` | `pane_id TEXT NOT NULL,` |
| 21 | SQLite runs in WAL with a busy timeout, so `waggledance mcp` processes write runs beside the daemon safely | read | `crates/waggledance-core/src/repository.rs:37` | `conn.pragma_update(None, "busy_timeout", 5000).ok();` |
| 22 | `waggledance mcp` is an existing stdio door the plugin can spawn (D9) | read | `crates/waggledance/src/cli.rs:84` | `/// Run the MCP server over stdio (used by Claude Code).` |
| 23 | The supervisor template's dispatch step is where leader mode is added | read | `docs/waggledance-supervisor-skill-template.md:69` | `3. **Dispatch through the door.** Call `waggledance_dispatch` with `{project, task,` |
| 24 | The plugin already has a workspace panel for the composer | read | `integrations/paseo/index.client.tsx:9` | `client.addWorkspacePanel({ id: "waggledance", title: "Waggle Dance"` |

## Discovery

A gather read the dispatch engine, runs store, reaper, trigger, MCP tools and
config. Reality touches on Paseo 0.11.2 (claims 1–14). Five advisor seats
critiqued the first draft; their accepted findings became D6–D10 and the
changes below (synthesis: `docs/history/paseo-leader-dispatch/reports/hat-wave.md`).

## Approach

Playbook: `skills/bee-planning/playbooks/feature.md` (cited, not copied).

**Recommended path.**

- **Paseo client (pld-1).** A `paseo` module in the `waggledance` crate, beside
  `herdr/`: a `Paseo` trait (leaders by label with `ls -g --label`, `inspect`,
  workspaces, create workspace, run, send, logs since) with a CLI
  implementation (tokio process, timeout, stdout only, `-q` for ids) and a
  `FakePaseo`. `[paseo]` config: `enabled`, `binary`, `provider`, `mode`,
  `thinking` (empty defaults, D10), `[[paseo.leaders]] {project, workspace}`.
  `resolve_leader` per D2/D3/D7/D8, under a per-project lock file.
- **Dispatch + await (pld-2).** `waggledance_dispatch` accepts `leader: true`
  as a third exclusive target. The leader branch runs before the
  `terminal.enabled` check and never builds the herdr handle (D6); gates:
  project → orchestration → `[paseo] enabled`. A live idle leader gets
  `paseo send --prompt-file --no-wait`; a busy one refuses (D7); none →
  `paseo run -d -q --workspace --cwd <root> --title --label … -- <task>`. The
  run stores `pane_id = paseo:<agent-id>`, `preset_label = None` (so the
  herdr reaper and `finish` never touch it, claims 17–18), and the dispatch
  timestamp in `baseline`. `await_run` on a `paseo:` run reads `inspect`:
  archived → `lost`; pending permission → `blocked`; idle with assistant text
  in `logs --since <baseline>` on two consecutive polls → `done` with that
  text as the delta; anything else → `working` (unknown status adds a
  warning).
- **Closing unattended runs (pld-3).** The reaper gets a Paseo sweep over
  working `paseo:` runs using the same mapping, so a plugin dispatch nobody
  awaits still ends; the board treats a working `paseo:` run as alive.
- **Composer (pld-4).** The panel's request box spawns `waggledance mcp`,
  sends `initialize` and one `tools/call` with `{project, task, leader:true}`,
  and shows "sent to <project> leader (new|reused, workspace …)" or the
  refusal text; the task text stays on failure; a timeout says it may have
  been sent.
- **Supervisor + docs (pld-5).** Leader mode in the template, a knowledge
  concept with a copy-paste `[paseo]` block, README pointer.

**Rejected.** A `backend` column (a migration for what the `pane_id` prefix
carries); a `waggledance dispatch` CLI subcommand (D9); an HTTP POST route
(the HTTP API is unauthenticated); a surface composer with a project picker
(D9); queueing on a busy leader (D7).

**SMALLER PATH check.** Skill-only routing breaks D1; dropping the reaper
sweep leaves panel dispatches `working` forever; everything else was already
cut at the hat wave. PASS.

**Risk map.**

| Component | Risk | Lands in | Proof |
|---|---|---|---|
| Gate on the leader path (D5, D6) | HIGH | pld-2 | refusals before any paseo call; terminal off still dispatches |
| Wrong workspace or cwd (D3, D8) | HIGH | pld-1 | resolver tests incl. two-root refusal; argv has `--workspace` and `--cwd <root>` |
| False `done` (D1) | HIGH | pld-2 | idle-before-reply poll stays `working`; done needs two polls with assistant text |
| Duplicate leaders (D7) | MEDIUM | pld-1 | concurrent resolves start one leader |
| Runs left working (D1) | MEDIUM | pld-3 | reaper ends an unawaited paseo run |
| Plugin write path (D5, D9) | MEDIUM | pld-4 | node test: only `leader:true` + project + task reach the tool call |

Waves: pld-1 → pld-2 → (pld-3 ∥ pld-4 ∥ pld-5). pld-2 needs pld-1's trait;
the last three need pld-2's leader target and touch disjoint files.

## Shape

Epic map — outcome: a request for a project lands on that project's leader in
the right Paseo workspace from any of the three sources, and its run ends with
a true status.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| Paseo client + config | D2, D3, D7, D8, D10 | find/start the right leader once | 1 | resolver + fake tests |
| Dispatch + await | D1, D4, D5, D6 | one door, true status | 1 | mcp + orchestrate tests |
| Unattended runs | D1 | runs end without a waiter | 1 | reaper + board tests |
| Composer | D4, D5, D9 | request from Paseo | 1 | node tests + tsc |
| Supervisor + docs | D4 | chat and external sources | 1 | doctor test |

Slice queue: one slice, five cells — all three sources are the ask.

## Cells — current slice (preview)

```json
[
  {
    "id": "pld-1",
    "title": "Find or start a project's Paseo leader through the paseo CLI",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-leader-dispatch",
    "deps": [],
    "decisions": ["D2", "D3", "D7", "D8", "D10"],
    "files": ["crates/waggledance/src/paseo.rs", "crates/waggledance/src/main.rs", "crates/waggledance-core/src/config.rs"],
    "read_first": ["docs/history/paseo-leader-dispatch/CONTEXT.md", "docs/history/paseo-leader-dispatch/plan.md", "crates/waggledance-core/src/config.rs", "crates/waggledance/src/herdr/mod.rs", "crates/waggledance/src/herdr/fake.rs"],
    "action": "Add PaseoConfig to waggledance-core Config as `paseo` (serde default, hand-written Default): enabled=false, binary=\"\" (empty means `paseo` on PATH), provider/mode/thinking=\"\" (empty means the flag is not passed, per D10), leaders: Vec<PaseoLeader{project, workspace}> matched by project id or name. Add crates/waggledance/src/paseo.rs and declare it in main.rs: an async_trait `Paseo` with leaders(project_id) using `ls -g --label waggledance.role=leader --label waggledance.project=<id> --json`, inspect(id) parsing Status/Archived/PendingPermissions/UpdatedAt, workspaces() from `workspace ls --json` (workspaceId, isolation, cwd), create_workspace(path) via `workspace create --path <root> --isolation local -q`, run(RunSpec) via `run -d -q --workspace <id> --cwd <root> --title <t> --label ... [--provider/--mode/--thinking when non-empty] -- <task>`, send(id, text) via `send <id> --prompt-file <tmp> --no-wait`, logs_since(id, ts) via `logs <id> --since <ts> --filter text` as plain text. `PaseoCli` uses tokio::process + tokio::time::timeout (15s), reads stdout only, never a shell; errors are typed and name the binary, the timeout, or the unparsable output. Add `FakePaseo` recording every call. Add `resolve_leader(paseo, config, project)`: inspect each labelled agent; a non-archived one is the leader — Existing{id, busy} where busy is status running/initializing (D7: the caller refuses busy). With none: the workspace is the [[paseo.leaders]] entry's, else the single workspace with isolation local whose realpath cwd equals the realpath project root; more than one such workspace refuses naming every id and the `[[paseo.leaders]] workspace` key (D8); none creates one at the root. Result Start{workspace, cwd: root, title: \"<project name> leader\", labels (D10)}. Serialize lookup-through-start per project with an exclusive lock file under the waggledance data dir (D7). No code comments.",
    "verify": "cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance-core config",
    "must_haves": {
      "truths": ["a non-archived labelled leader is returned as Existing", "an archived leader is ignored", "a configured workspace wins over a root workspace", "two root workspaces with no config entry refuse naming both ids", "no workspace at the root creates one there", "run argv always carries --workspace, --cwd <root>, both labels, the title and -- before the task", "empty provider/mode/thinking pass no flag", "a missing binary errors naming the binary"],
      "artifacts": [{"path": "crates/waggledance/src/paseo.rs", "substantive": "Paseo trait, PaseoCli, FakePaseo, resolve_leader, lock, tests"}],
      "key_links": ["config.paseo is read by resolve_leader"],
      "prohibitions": ["no shell interpolation of task text", "no change to herdr code paths", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "pld-2",
    "title": "Dispatch a task to a project's leader through waggledance_dispatch",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-leader-dispatch",
    "deps": ["pld-1"],
    "decisions": ["D1", "D2", "D4", "D5", "D6", "D7"],
    "files": ["crates/waggledance/src/mcp.rs", "crates/waggledance/src/orchestrate.rs"],
    "read_first": ["docs/history/paseo-leader-dispatch/plan.md", "crates/waggledance/src/paseo.rs", "crates/waggledance/src/mcp.rs", "crates/waggledance/src/orchestrate.rs"],
    "action": "In mcp.rs dispatch_schema add boolean `leader`; exactly one of preset, pane_id, leader:true is required. In handle_dispatch, after the project lookup and before the terminal.enabled check, branch on leader (D6): refuse when orchestration_allowed is false (existing wording), then when config.paseo.enabled is false with text naming ~/.waggledance/config.toml and the lines `[paseo]` / `enabled = true` — both before any paseo call. The leader branch never builds the herdr Orchestration handle; it builds a PaseoCli from config.paseo. Call resolve_leader: Existing busy -> refuse naming the leader id (D7); Existing idle -> send; Start -> run with the task. The task text is sent unchanged with no marker (D4). Store a Run with pane_id \"paseo:<agent-id>\", preset_label None, baseline = the dispatch time as RFC3339 taken before the send, marker empty, status working; return {run_id, warnings, agent_id, workspace, leader: \"reused\"|\"started\"}. In orchestrate.rs await_run, a run whose pane_id starts with \"paseo:\" polls inspect: Archived -> Lost; PendingPermissions non-empty -> Blocked; status idle and logs_since(baseline) holds a line not starting with \"[User]\" on two consecutive polls -> Done with that text as the delta (Completion::Observed); otherwise Working, adding a warning for a status other than idle/running/initializing; deadline -> Working. Put the mapping in one function the reaper (pld-3) can call. Herdr dispatch and await behavior is unchanged. No code comments.",
    "verify": "cargo test --profile fast -p waggledance dispatch && cargo test --profile fast -p waggledance await",
    "must_haves": {
      "truths": ["leader dispatch with orchestration off is refused and FakePaseo saw zero calls", "leader dispatch with [paseo] enabled=false is refused naming the config file and line", "leader dispatch with terminal.enabled=false still dispatches", "a busy leader refuses naming its id and no send happens", "an idle leader receives the unchanged task through send and a run row paseo:<id> with preset_label NULL is stored", "await on an idle leader whose logs since dispatch hold only the [User] line returns working", "await returns done only after two polls with assistant text", "await with a pending permission returns blocked", "existing herdr dispatch tests stay green"],
      "artifacts": [{"path": "crates/waggledance/src/mcp.rs", "substantive": "leader target in schema and handler with refusal tests"}],
      "key_links": ["handle_dispatch calls paseo::resolve_leader", "await_run branches on the paseo: prefix through a shared mapping function"],
      "prohibitions": ["caller cannot pass workspace, provider, cwd or argv", "no close/stop/archive of a Paseo agent", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "pld-3",
    "title": "End unawaited leader runs and show them alive on the board",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-leader-dispatch",
    "deps": ["pld-2"],
    "decisions": ["D1"],
    "files": ["crates/waggledance/src/reaper.rs", "crates/waggledance-core/src/repository.rs", "crates/waggledance/src/server.rs"],
    "read_first": ["crates/waggledance/src/reaper.rs", "crates/waggledance/src/orchestrate.rs", "crates/waggledance/src/server.rs"],
    "action": "Add repository list_working_paseo_runs (status='working' AND pane_id LIKE 'paseo:%'). In reaper.rs sweep_once, after the herdr sweep and only when config.paseo.enabled, judge each such run with pld-2's paseo status mapping and store done (with its transcript) or lost through the same finish path, which never closes a paseo run; a blocked run stays working. A paseo CLI failure leaves the run untouched and logs a warning. In server.rs, where the board computes pane_alive for working runs, a run whose pane_id starts with \"paseo:\" counts as alive. The herdr sweep is unchanged. No code comments.",
    "verify": "cargo test --profile fast -p waggledance reaper && cargo test --profile fast -p waggledance-core runs && cargo test --profile fast -p waggledance board",
    "must_haves": {
      "truths": ["an unawaited paseo run whose leader replied is stored done by one sweep", "a paseo run whose leader is archived is stored lost", "the herdr sweep never sees a paseo run", "a working paseo run is alive on the board"],
      "artifacts": [{"path": "crates/waggledance/src/reaper.rs", "substantive": "paseo sweep with tests"}],
      "key_links": ["reaper calls the shared paseo mapping from orchestrate.rs"],
      "prohibitions": ["no pane close for paseo runs", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "pld-4",
    "title": "Send a request to a project's leader from the Waggle Dance Paseo panel",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-leader-dispatch",
    "deps": ["pld-2"],
    "decisions": ["D4", "D5", "D9"],
    "files": ["integrations/paseo/index.server.ts", "integrations/paseo/shared/rpc.ts", "integrations/paseo/server/waggledance.ts", "integrations/paseo/server/dispatch.ts", "integrations/paseo/client/board.tsx", "integrations/paseo/client/composer.tsx", "integrations/paseo/test/server.test.ts"],
    "read_first": ["integrations/paseo/server/waggledance.ts", "integrations/paseo/client/board.tsx", "integrations/paseo/test/server.test.ts", "crates/waggledance/src/mcp.rs", "docs/history/pi-paseo-plugin/CONTEXT.md"],
    "action": "Add RPC `waggledance.send` {project_id, task} -> {ok:true, run_id, agent_id, workspace, leader} | Failure. server/dispatch.ts spawns the configured waggledance binary with argv [\"mcp\"], writes newline-delimited JSON-RPC `initialize`, then `tools/call` name waggledance_dispatch with arguments exactly {project: project_id, task, leader: true}, reads the matching response, and kills the child. A tool result with isError maps to {ok:false, code:\"failed\", error:<the tool's text>}; a missing binary names the binary; a 60s timeout says the request may have been sent and to check Runs before retrying. Add client/composer.tsx: multiline input and a Send button, disabled while empty or sending; on success show 'Sent to <project> leader (<started|reused>, workspace <id>)' and clear the input; on failure show the error and keep the text. Render it in the workspace panel only when the panel resolved a project (D9). Extend server.test.ts with node:test cases against a fake binary script speaking the stdio protocol: the exact tool arguments, isError mapping, missing binary, timeout wording. No code comments.",
    "verify": "cd integrations/paseo && node --test test/ && npx tsc --noEmit",
    "must_haves": {
      "truths": ["the tool call carries only project, task and leader:true", "a refusal from the tool is shown verbatim and the text is kept", "the board stays otherwise read-only"],
      "artifacts": [{"path": "integrations/paseo/client/composer.tsx", "substantive": "composer used by the workspace panel"}, {"path": "integrations/paseo/server/dispatch.ts", "substantive": "stdio MCP client for one dispatch call"}],
      "key_links": ["index.server.ts handles waggledance.send", "board.tsx panel renders the composer"],
      "prohibitions": ["no HTTP write to the daemon", "no workspace, provider or cwd in the call", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "pld-5",
    "title": "Teach the supervisor skill to hand requests to a project's Paseo leader",
    "role": "docs",
    "lane": "high-risk",
    "feature": "paseo-leader-dispatch",
    "deps": ["pld-2"],
    "decisions": ["D4", "D6", "D10"],
    "files": ["docs/waggledance-supervisor-skill-template.md", "docs/knowledge/patterns/paseo-leader-dispatch.md", "README.md"],
    "read_first": ["docs/waggledance-supervisor-skill-template.md", "crates/waggledance/src/doctor.rs", "docs/history/paseo-leader-dispatch/CONTEXT.md"],
    "action": "In the supervisor template add a leader mode: when the target project runs in Paseo (`[paseo] enabled`), dispatch with {project, task, leader: true} instead of a preset; the spec-drop first line and the relay/execute contract are unchanged; await/runs are used the same way; `blocked` means the leader waits on a permission in Paseo; a busy-leader refusal means retry after its current run ends. Add a knowledge concept with a copy-paste first-run block (`[paseo]` enabled, optional `[[paseo.leaders]]` project + workspace), the orchestration opt-in, the leader labels and title, the workspace resolution order and the two-workspace refusal, the status mapping, and the three sources (the WD agent, the panel composer, external MCP agents such as openclaw, hermes and aicoworker running `waggledance mcp`), with a Pointers list of the files. Add a short README section linking it. Keep any doctor assertion pinning the template green.",
    "verify": "cargo test --profile fast -p waggledance doctor",
    "must_haves": {
      "truths": ["the template names leader: true", "the concept's [paseo] block parses as the config the code reads"],
      "artifacts": [{"path": "docs/knowledge/patterns/paseo-leader-dispatch.md", "substantive": "config, resolution, status mapping, sources"}],
      "key_links": ["README links the concept"],
      "prohibitions": ["no change to the spec-drop line format"]
    },
    "affects_skills": [],
    "affects_specs": ["docs/knowledge/patterns/paseo-leader-dispatch.md"]
  }
]
```

## Test matrix

High-risk dimensions that apply; each writer authors only what existing tests do not pin.

| Dimension | Probe | Pass when |
|---|---|---|
| Authorization | leader dispatch, orchestration off | error names the project; FakePaseo call log is empty |
| Authorization | leader dispatch, `[paseo] enabled=false` | error contains `config.toml` and `enabled = true`; no paseo call |
| Authorization | leader dispatch, `terminal.enabled=false`, others on | run stored |
| Input validation | `preset` + `leader` together | error says exactly one of preset, pane_id, leader |
| Input validation | task beginning with `-` | run argv has `--` immediately before the task |
| Happy path | idle leader exists | one `send` with the unchanged task; run `paseo:<id>`, preset_label NULL |
| Happy path | no leader | `run` argv has `--workspace`, `--cwd <root>`, both labels, the title |
| Concurrency | busy leader | refusal names the leader id; no send |
| Concurrency | two resolves at once, no leader | exactly one `run` |
| Config precedence | two root workspaces + config entry | configured workspace chosen |
| Ambiguity | two root workspaces, no entry | refusal lists both ids and `[[paseo.leaders]]` |
| Fallback | no workspace at the root | `create_workspace(root)`, then `run` in it |
| State mapping | idle, logs since dispatch only `[User] …` | `working` |
| State mapping | idle with assistant text, two polls | `done`, delta is that text |
| State mapping | pending permission | `blocked` |
| State mapping | archived | `lost` |
| Failure | binary missing / timeout | typed error naming the binary / the timeout; no run stored |
| Lifecycle | unawaited run, leader replied | one reaper sweep stores `done` |
| Regression | herdr dispatch, await, reaper suites | green on head |
| Plugin | send RPC against a fake stdio binary | tool arguments equal `{project, task, leader:true}`; isError text surfaces; timeout says "may have been sent" |

## Open Questions

(none)

## Out of scope

Stopping or archiving a leader; queueing requests on a busy leader; images in a request; a surface composer with a project picker; a `waggledance dispatch` CLI; the herdr path's behavior.
