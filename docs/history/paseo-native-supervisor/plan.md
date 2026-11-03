---
artifact_contract: bee-plan/v2
mode: high-risk
---

# Plan: Paseo-Native Supervisor

## Summary

A single supervisor agent on Haiku lives in Paseo and takes work from aicoworker. It
picks the project, opens or reuses that project's leader as its own Paseo subagent,
hears from Paseo when the leader finishes or asks something, consults an advisor or
sends the question to the user on Telegram, and archives the leader once the user
accepts. waggledance gives the supervisor what it lacks today: a `leaders` view inside
`waggledance_ask_state`, a `waggledance supervisor` CLI with `ensure` (find or start
the supervisor) and `report` (post to aicoworker's webhook so the agent never holds
the token), and a `waggledance setup openclaw` target that installs aicoworker's
`handoff`/`answer` skill and turns the webhook on. The daemon's existing leader queue
is left exactly as it is.

Mode: `high-risk` — 4 risk flags: external systems (Paseo, OpenClaw), public
contract (`ask_state` payload), audit/security (a bearer token), multi-domain (Rust,
agent prompt, Python skill, host config).
Why this is the least workflow that protects the work: the walking skeleton crosses
three processes and one secret, and no cell can be proven without the other two
sides existing. A plan with a live proof at its end is the smallest shape that does
not ship a path nobody walked.

Playbook: `skills/bee-planning/playbooks/feature.md` (route class `feature`), by
pointer. Step 1 (data shape first) lands as the `[supervisor]`/`[openclaw]` config
blocks, the token file and the `leaders` JSON before any verb; step 2 (walking
skeleton) is the whole slice; step 4 (prove the user-visible path) is pns-6; step 5
(sync `docs/knowledge/`) is pns-5. No step is skipped.

Plan check: the hat wave ran at this step with all five seats; its synthesis and the
dismissed findings are in `docs/history/paseo-native-supervisor/reports/hat-wave.md`.

## Requirements (from CONTEXT.md)

- D1: Agent-to-agent communication inside Paseo is Paseo native; waggledance is the read layer, never the pipe.
- D2: One long-lived supervisor agent for all projects, Haiku, label `waggledance.role=supervisor`, find-or-create by label.
- D3: A leader is a Paseo subagent created from inside the supervisor via MCP `create_agent`, in the project workspace, labelled role+project; one per project; clear = archive then create.
- D4: Advisor is a short-lived Paseo subagent of the supervisor without a workspace; forward or escalate.
- D5: Inbound from aicoworker via `paseo send <id> --no-wait`; OpenClaw skill gains `handoff` and `answer`.
- D6: Outbound via OpenClaw `POST /hooks/agent`, Telegram delivery, project name + one-line context; token never in the prompt; hooks enabled with a token.
- D7: Classification of a leader's turn and "really free" is the supervisor's judgment; waggledance exposes raw fields only.
- D8: `ask_state` gains `leaders`; CLI gains `supervisor ensure`; daemon queue/lock/notices frozen.
- D9: Archive a leader only after the originating agent relays the user's acceptance.
- D10: Idle leader with no open question → archive and create fresh on a new task; leader waiting on a reply → queue and tell the user; via label `waggledance.via=supervisor`, reuse only those.
- D11: The hook is a model turn; reports are verbatim-forward instructions in Vietnamese, tagged; accepted limit; boundary names four waggledance additions.
- D12: Message protocol `[handoff|reply|accept|redo <corr>]` in, `report --kind result|question|blocked|error|closed|progress` out; bare "ok" = newest open item.
- D13: Token in `<data dir>/hook.token` owner-only; graceful clear; advisor label; append-only ledger with the map as truth; a 30-minute heartbeat.

## Load-bearing claims

Labels are `read` (file opened at that line) or `ran` (command executed, output held); the evidence column is a verbatim substring of the anchored line(s), multi-line joined with " / ".

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | Paseo CLI can filter agents by label, so `leaders` and `ensure` can find labelled agents | ran | `paseo ls --help \| grep -i label` | `--label <key=value>    Filter by label (can be used multiple times) (default:` |
| 2 | `paseo ls -g --json` does NOT return labels, so one project cannot be told from another by the list alone; `leaders` maps by cwd | ran | `paseo ls -g --json 2>/dev/null \| sed -n '/^\[/,$p' \| python3 -c "...print(len(a),'agents; keys:',sorted(a[0].keys()))"` | `14 agents; keys: ['created', 'cwd', 'id', 'name', 'provider', 'shortId', 'status', 'thinking']` |
| 3 | The label-filtered list returns `[]` after the wrapper's noise lines, so `strip_noise`/`parse_document` apply unchanged | ran | `paseo ls -g --json --label waggledance.role=leader 2>&1 \| cat -A` | `[linux-sandbox] disabled: requested by --no-sandbox$ / 18:34:34.046 M-bM-^@M-: [desktop] app startup { version: '0.11.2', platform: 'linux', arch: 'x64', isPackaged: true }$ / 18:34:34.052 M-bM-^@M-: [linux-sandbox] { enabled: false, reason: 'requested by --no-sandbox' }$ / []$` |
| 4 | Paseo MCP `create_agent` takes `labels`, `workspaceId` and `notifyOnFinish`, which is all D3 needs | ran | `ToolSearch select:mcp__paseo__create_agent` | `"labels": {"additionalProperties": {"type": "string"}, "description": "Labels to set on the agent"` / `"workspaceId": {"description": "Existing workspace id.` / `"notifyOnFinish": {"default": true` |
| 5 | The Haiku model id Paseo accepts for the claude provider | ran | `mcp__paseo__list_models provider=claude` | `"id":"claude-haiku-5-5","label":"Haiku 5.5"` |
| 6 | `paseo run` accepts `--cwd`, `--label` and `-d`, so `ensure` can start a top-level supervisor in its own directory | ran | `paseo run --help` | `--label <key=value>               Add label(s) to the agent (can be used` |
| 7 | OpenClaw exposes `POST /hooks/agent` for an isolated agent run with channel delivery | read | `/home/thanhsmind/aicoworker/app/gateway-overlay/openclaw/docs/automation/webhook.md:60` | `### \`POST /hooks/agent\`` |
| 8 | aicoworker's gateway currently has no webhook enabled; its `hooks` block opens with only the internal entry | read | `/home/thanhsmind/aicoworker/openclaw/openclaw.json:888-890` | `  "hooks": { /     "internal": { /       "enabled": true,` |
| 9 | The gateway listens locally on 18789, the port the webhook is served from | ran | `ss -ltnp \| grep 18789` | `127.0.0.1:18789` |
| 10 | The `waggledance` crate already depends on reqwest, so `report` needs no new dependency | read | `crates/waggledance/Cargo.toml:45` | `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }` |
| 11 | The top-level `Config` struct is where `[supervisor]` and `[openclaw]` blocks are added | read | `crates/waggledance-core/src/config.rs:19` | `pub paseo: PaseoConfig,` |
| 12 | The clap `Command` enum in `cli.rs` carries `Setup`, the sibling the new `Supervisor` verb sits beside | read | `crates/waggledance/src/cli.rs:87` | `Setup {` |
| 13 | `ask_state_digest` is the single builder of the per-project answer that `leaders` extends | read | `crates/waggledance/src/mcp.rs:624` | `fn ask_state_digest(project_id: &str, snapshot: &bee::BeeSnapshot) -> Value {` |
| 14 | `BeeWaitingOn` carries `kind` and `subject`, the two fields `leaders` surfaces for D7 | read | `crates/waggledance-core/src/bee.rs:275-278` | `pub struct BeeWaitingOn { /     pub kind: String, /     pub subject: String, / }` |
| 15 | A label query helper already exists and is the pattern for the via/supervisor label queries | read | `crates/waggledance/src/paseo.rs:147` | `pub fn leaders_args(project_id: &str) -> Vec<String> {` |
| 16 | `ListedAgent` parses only `id`; `leaders` needs `cwd`, `status` and `created` added | read | `crates/waggledance/src/paseo.rs:636-638` | `struct ListedAgent { /     id: String, / }` |
| 17 | The module name `supervisor` is taken by the herdr watchdog, so the new module is `seat.rs` | read | `crates/waggledance/src/main.rs:27` | `mod supervisor;` |
| 18 | The runs table exposes a per-project list for the `last_daemon_run` field | read | `crates/waggledance-core/src/repository.rs:305` | `    pub fn list_runs(&self, project_id: &str, limit: usize) -> Result<Vec<Run>> {` |
| 19 | `/hooks/agent` runs a model turn, not a pipe, so the report text is an instruction (D11) | read | `/home/thanhsmind/aicoworker/app/gateway-overlay/openclaw/docs/automation/webhook.md:94` | `- Runs an **isolated** agent turn (own session key)` |
| 20 | The hook's `message` is processed by the agent, which is why the report must say "forward verbatim" (D11) | read | `/home/thanhsmind/aicoworker/app/gateway-overlay/openclaw/docs/automation/webhook.md:80` | `- \`message\` **required** (string): The prompt or message for the agent to process.` |
| 21 | A Haiku agent started by `paseo run` reads the `CLAUDE.md` in its cwd and has the Paseo MCP tools (`create_agent`, `send_agent_prompt`, `archive_agent` among them) | ran | `paseo wait 57a65f50-7559-487f-a929-6634c2e53636 --timeout 90 --json` (probe agent, archived afterwards) | `PROBE-CLAUDEMD-READ\nmcp__paseo__archive_agent, mcp__paseo__archive_workspace` |
| 22 | `paseo run -d --json` on the npm CLI prints one JSON object with `agentId`, which `ensure` parses | ran | `~/.local/share/mise/installs/node/24.19.0/bin/paseo run -d --json --cwd … --provider claude/claude-haiku-5-5 --mode bypassPermissions --title "wd probe pns (delete me)" --label waggledance.role=probe "Follow CLAUDE.md in this directory."` (captured to `.bee/tmp/pns/npm-run-detached.out`) | `"agentId": "c4eb3cbd-22e6-45a8-acc3-c97f5d8b7a46", / "status": "running",` |
| 23 | The AppImage wrapper's `run` prints only log lines and creates no agent, so `ensure` must go through `detect_binary` (npm CLI first) | ran | `paseo run -d --json --cwd … "Follow CLAUDE.md in this directory."` (wrapper; captured to `.bee/tmp/pns/run-detached.out`) | `[login-shell-env] start {` |
| 24 | A `paseo run` from inside a Paseo agent attaches to the caller's workspace and ignores `--cwd` unless `PASEO_AGENT_ID`/`PASEO_AGENT_CWD` are scrubbed | ran | the claim-22 command without `env -u`, output `cwd` | `"cwd": "/home/thanhsmind/Projects/goglbe/waggledance",` |
| 25 | An owner-only credential file pattern already exists for the Telegram token and is reused for `hook.token` (D13) | read | `crates/waggledance-core/src/config.rs:452` | `const NOTIFY_CREDENTIAL_FILE_NAME: &str = "telegram.token";` |
| 26 | The owner-only writer `report`'s token file reuses | read | `crates/waggledance-core/src/config.rs:519` | `fn write_owner_only(path: &Path, bytes: &[u8]) -> Result<()> {` |
| 27 | `paseo ls` hides archived agents unless `-a`, so `ensure`'s list never returns an archived supervisor | ran | `paseo ls --help` | `-a, --all              Include archived agents` |
| 28 | Every `PaseoCli` child process already runs under a timeout, so a hung `paseo ls` becomes `leaders_error`, never a hang | read | `crates/waggledance/src/paseo.rs:589` | `        let output = tokio::time::timeout(timeout, child.wait_with_output())` |
| 29 | `paseo send` has `--no-wait` and `--prompt-file`, the two flags `handoff` uses | ran | `paseo send --help` | `--no-wait              Return immediately without waiting for completion` |
| 30 | `setup.rs` has a `Target` enum with `Pi` and `Paseo`, where `OpenClaw` is added | read | `crates/waggledance/src/setup.rs:6-9` | `pub enum Target { /     Pi, /     Paseo, / }` |

## Discovery

Inspected the Paseo 0.11.2 CLI and MCP surface live (claims 1–6, 21–24, 27, 29),
aicoworker's running OpenClaw gateway and docs (7–9, 19, 20), and the waggledance
crates (10–18, 25, 26, 28, 30). Findings that changed the draft: the agent list
carries no labels (claim 2), so `leaders` maps each labelled agent to the project
whose root or worktree contains its cwd; the hook is a model turn (19, 20), so the
report is an instruction and the limit is recorded (D11); a probe agent proved the
CLAUDE.md-in-cwd mechanism and the MCP tool set (21); the AppImage wrapper cannot
start an agent (23) and an unscrubbed environment ignores `--cwd` (24), both of which
`PaseoCli` already handles; the write guard denies hand edits outside the worktree,
so host installation is a `setup openclaw` target like `setup pi`.

## Approach

Recommended path: the supervisor is a plain Paseo agent whose working directory is
`~/.waggledance/supervisor/`, where `waggledance supervisor ensure` writes a
`CLAUDE.md` rendered from `docs/waggledance-supervisor-agent.md` (embedded with
`include_str!`, parity-tested as `setup.rs` does for the plugin files). Claude Code in
that agent reads the file as its instructions and has the Paseo MCP tools (claim 21),
so D1, D3, D4, D9, D10, D12 and D13 are the prompt's rules, not daemon code (D7).
`ensure` goes through `PaseoCli` (binary detection, env scrub, timeout), lists
`-g --label waggledance.role=supervisor --json`, reuses the first agent, else runs
`run -d --json --cwd <dir> --provider <[supervisor] provider, default
claude/claude-haiku-5-5> [--mode] [--thinking] --title "waggledance supervisor"
--label waggledance.role=supervisor "<bootstrap line>"` and prints `{agent_id,
created, instructions_stale}`. `report --kind <k> --project <name> --corr <id> --text
<t>` builds the D11 instruction body and posts it with the bearer token read from
`<data dir>/hook.token`; it refuses when the file is missing and exits non-zero on a
non-2xx answer with the status and body head. `setup openclaw` writes the orchestrator
skill into aicoworker's skills directory, refreshes `~/.agents/skills/waggledance-supervisor/`
from the template, writes the token file first, then edits `openclaw.json` (`hooks.enabled`,
`hooks.token`, `hooks.defaultSessionKey`, `hooks.allowRequestSessionKey: false`, a
`chmod 600` timestamped backup kept beside it), and prints the curl lines the user or
pns-6 runs to confirm 401 without and 200 with the token. aicoworker's `handoff` is
`ensure` then `paseo send --no-wait --prompt-file` (D5); `reply`, `accept` and `redo`
are the same send with D12's prefixes.

Rejected alternatives:
- Supervisor logic in the daemon (Rust loop): a decision in waggledance, against orchestrator-dispatch D1 and D7.
- Token in the supervisor's environment with a curl template in the prompt: D6 allows it, but the body would be shell-quoted by a model and untestable; `report` gives a deterministic body and a tested bearer header.
- A per-project `paseo ls --label` query for `leaders`: N process spawns per rollup; the cwd mapping is one.
- Reusing the daemon queue to reach the leader: duplicates `notifyOnFinish`; D1.
- Hand-editing host files from a worker: the write guard denies writes outside the worktree; `setup openclaw` is the repo's own pattern and is reversible with `--remove`.
- A rollup `supervisor` field and `unassigned_leaders` in `ask_state`: `ensure` already prints the id; D3 places every supervisor leader in a project workspace.

Risk map:

| Component | Level | Lands in | Proof needed / fact |
|---|---|---|---|
| OpenClaw hook enable on a live gateway | HIGH | pns-4, pns-6 | Fact: the `hooks` block exists with only `internal` (claim 8). Order: token file, then gateway edit, then curl. Proof: curl without the token returns 401 and with it 200; the Telegram text equals the report text. Rollback: `setup openclaw --remove` restores the backup and deletes the token file. |
| The hook rephrases or translates the report | MEDIUM | pns-1, pns-6 | Fact: the hook is a model turn (claims 19, 20). The body says "forward verbatim, in Vietnamese"; pns-6 quotes the Telegram text against the input. Accepted limit (D11). |
| Supervisor and the frozen daemon share a leader | MEDIUM | pns-2, pns-3, pns-5 | D10's via label; `leaders` carries `via`; the pattern doc states the limit: a project reached through the plugin request box and through the supervisor at once may hold one leader of each kind. pns-6 checks exactly one via-labelled leader. |
| Archiving a leader mid-cell orphans bee state | MEDIUM | pns-2 | D13 graceful clear: pause handoff first, archive after the finish notification. pns-6 runs one clear and reads the leader's `.bee/HANDOFF.json` existence through `ask_state` `handoff`. |
| `leaders` mapping by cwd mislabels a leader in a worktree or nested project | MEDIUM | pns-3 | The rollup already lists the project's worktrees; a leader whose cwd is a worktree lands under its project. Unit tests: root cwd, worktree cwd, cwd under no project (dropped, counted in `leaders_skipped`). |
| Paseo list hangs or shape changes | LOW | pns-3 | Timeout already in `PaseoCli` (claim 28) → `leaders_error`. Fixture `ls-label-leader.out` captured today (claim 3); a new Paseo gets a sibling fixture directory. |
| Token at rest | LOW | pns-1, pns-4 | Owner-only file, never a `Config` field (D13, claims 25, 26). Test: the file is written with mode 0600 and `Config` serialisation contains no token. |

Waves: pns-2 and pns-3 run in parallel (disjoint files). pns-1 follows pns-2 (it
embeds the doc). pns-4 follows pns-1 (it shares `cli.rs`/`config.rs` and calls
`ensure`). pns-5 follows pns-1, pns-3 and pns-4 (it documents all three). pns-6
follows pns-4 and pns-3 (live proof needs every side).

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "30ff876890293b1cea96673b722ea95a7b27780259af61e6c3b2be09a0c2b112",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan; drafted by the leader at the plan step"},
    {"stage": "hat-wave-facts-gaps", "classification": "required", "role": "hat-facts-gaps", "reason": "high-risk: all five seats; ran 2026-10-17"},
    {"stage": "hat-wave-alternatives", "classification": "required", "role": "hat-alternatives", "reason": "high-risk: all five seats; ran 2026-10-17"},
    {"stage": "hat-wave-user-impact", "classification": "required", "role": "hat-user-impact", "reason": "high-risk: all five seats; ran 2026-10-17"},
    {"stage": "hat-wave-risks", "classification": "required", "role": "hat-risks", "reason": "high-risk: all five seats; ran 2026-10-17"},
    {"stage": "hat-wave-value", "classification": "required", "role": "hat-value", "reason": "high-risk: all five seats; ran 2026-10-17"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "the hat wave is the high-risk advisor consult (b34fdea9)"},
    {"stage": "code", "classification": "required", "role": "code", "reason": "pns-1, pns-3, pns-4 write Rust and Python"},
    {"stage": "docs", "classification": "required", "role": "docs", "reason": "pns-2 agent definition, pns-5 knowledge sync"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "pns-6 drives the live path and records green:live"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup it cannot do in its own budget", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "fallback slot; every cell names its role"},
    {"stage": "review", "classification": "conditional", "role": "review", "condition": "the user invokes bee-reviewing", "reason": "agents-review-user-invoked"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no release in this feature; host installation is `setup openclaw` run in pns-6"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "herding cockpit role, not used"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "no blind lanes: one design with repo precedent"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Shape

Epic map. Outcome: a task typed to aicoworker on Telegram reaches a project leader
in Paseo, and its result or question comes back to Telegram, with no human step in
between except the ones the user asked for. Repo-reality basis: claims 1–30.

| Epic | Capability / Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| Supervisor seat | the agent, its instructions, `ensure`, `report` | D1–D4, D6, D9–D13 live here | 1 | `ensure` idempotent; `report` 200 with token, refused without a token file |
| Map for the seat | `ask_state.leaders`, fixtures | D7, D8, D10 | 1 | unit tests over fixtures; live `ask_state` shows the leader pns-6 creates with `via: supervisor` |
| aicoworker bridge | `setup openclaw`: skill, hook enable, token file | D5, D6, D12, D13 | 1 | setup unit tests; curl 401/200; `handoff --dry-run` prints the exact commands |
| Knowledge | concept, frozen note, template section | D8 freeze, playbook step 5 | 1 | `bee knowledge index --check` and `bee knowledge check` green |
| Live path | one task end to end, one question, one accept | playbook step 4; D4, D9 live | 1 | `green:live` evidence quoted in the cap |

Slice queue: one slice; every epic's slice is current. Later work (headlines only, not
cells): several leaders per project (PBI filed); routing the Paseo plugin's request
box through the supervisor; exposing `parent_agent_id` in `leaders` if the via label
proves insufficient.

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| pns-2 | Write the supervisor agent's instructions | `docs/waggledance-supervisor-agent.md` (new) | — | A reader follows, step by step, how the supervisor takes `[handoff]`, picks the project, reuses or opens a via-labelled leader, clears gracefully, handles a question through an advisor or Telegram, reports with tags in Vietnamese, keeps its ledger and heartbeat, and archives on `[accept]` | structure check: every section present and every D1–D13 cited; behaviour proof is pns-6 |
| pns-3 | Add `leaders` to `waggledance_ask_state` | `crates/waggledance/src/mcp.rs`, `crates/waggledance/src/paseo.rs`, `crates/waggledance/testdata/fixtures/paseo-0.11.2/ls-label-leader.out` (new), `crates/waggledance/testdata/fixtures/paseo-0.11.2/README.md` | — | `waggledance_ask_state` for a project answers `leaders: [{agent_id, status, cwd, created, via, labels, waiting_on, last_daemon_run}]`, `leaders: null` + `leaders_error` when Paseo is unavailable, and no `leaders` key when `[paseo] enabled = false` | `cargo test --profile fast -p waggledance leaders && cargo test --profile fast -p waggledance paseo` |
| pns-1 | Add `waggledance supervisor ensure` and `report` | `crates/waggledance/src/cli.rs`, `crates/waggledance/src/main.rs`, `crates/waggledance/src/seat.rs` (new), `crates/waggledance-core/src/config.rs`, `crates/waggledance/testdata/fixtures/paseo-0.11.2/npm-run-detached.out` (new), `crates/waggledance/testdata/fixtures/paseo-0.11.2/README.md` | pns-2 | `waggledance supervisor ensure --json` prints `{agent_id, created: true, instructions_stale: false}` the first time and `created: false` the second; `waggledance supervisor report --kind result --project waggledance --corr t-abc123 --text "…"` posts the D11 body with a bearer header read from `hook.token`, and refuses when that file is missing | `cargo test --profile fast -p waggledance seat && cargo test --profile fast -p waggledance-core config` |
| pns-4 | Install aicoworker's bridge and the webhook with `waggledance setup openclaw` | `crates/waggledance/src/setup.rs`, `crates/waggledance/src/cli.rs`, `crates/waggledance-core/src/config.rs`, `integrations/openclaw/waggledance-orchestrator/SKILL.md` (new), `integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py` (new) | pns-1 | `waggledance setup openclaw` writes the skill files, refreshes the deployed supervisor skill, writes `hook.token` (0600), edits `openclaw.json` with a 0600 backup, prints the two curl checks; `--remove` restores the backup and deletes what it wrote; `wd-bridge.py handoff --task … --dry-run` prints the `ensure` and `paseo send` commands it would run | `cargo test --profile fast -p waggledance setup && python3 -m py_compile integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py && python3 integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py handoff --dry-run --task probe` |
| pns-5 | Record the supervisor path and the frozen daemon path in knowledge | `docs/knowledge/patterns/paseo-native-supervisor.md` (new), `docs/knowledge/patterns/paseo-leader-dispatch.md`, `docs/knowledge/index.md`, `docs/knowledge/patterns/index.md`, `README.md`, `docs/waggledance-supervisor-skill-template.md` | pns-1, pns-3, pns-4 | The knowledge bundle names where the supervisor path lives, what the frozen daemon path still does, which decision froze it, and the shared-label limit; the skill template's leader mode points to `handoff` | `.bee/bin/bee knowledge index --check && .bee/bin/bee knowledge check` |
| pns-6 | Drive one task from aicoworker to a leader and back | `docs/history/paseo-native-supervisor/reports/live-proof.md` (new, the execution record) | pns-3, pns-4 | After `setup openclaw` and `ensure`, a `handoff` for project `waggledance` yields exactly one via-labelled leader; its reply reaches Telegram through the hook with the tag; a second task that makes the leader ask a question yields a `question` on Telegram, a `reply` reaches the leader; `accept` leaves zero via-labelled leaders and zero advisors | `paseo ls -g --json --label waggledance.role=leader --label waggledance.via=supervisor && paseo ls -g --json --label waggledance.role=advisor` |

```json
[
  {
    "id": "pns-2",
    "feature": "paseo-native-supervisor",
    "title": "Write the supervisor agent's instructions",
    "lane": "high-risk",
    "role": "docs",
    "status": "open",
    "deps": [],
    "decisions": ["D1", "D2", "D3", "D4", "D6", "D7", "D9", "D10", "D11", "D12", "D13"],
    "files": ["docs/waggledance-supervisor-agent.md"],
    "read_first": ["docs/history/paseo-native-supervisor/CONTEXT.md", "docs/history/paseo-native-supervisor/plan.md", "docs/waggledance-supervisor-skill-template.md", "docs/knowledge/patterns/paseo-leader-dispatch.md"],
    "affects_skills": [],
    "affects_specs": [],
    "action": "Write docs/waggledance-supervisor-agent.md through the bee-technical-writing skill: it is the CLAUDE.md a Claude Haiku agent reads as its standing instructions when `waggledance supervisor ensure` writes it into ~/.waggledance/supervisor/, so write it TO that agent, in English, in second person, with no bee vocabulary and no reference to this plan. Sections, in order: Who you are (one supervisor for every project, label waggledance.role=supervisor, D2); What arrives (the four message shapes of D12, each with its exact prefix, plus the finish, error and permission notifications Paseo sends for your subagents, plus your own heartbeat); Your map (call waggledance_ask_state with no project for the rollup, read `leaders` per project with its `via`, `status`, `waiting_on` and `last_daemon_run`, and waggledance_projects for roots; the map is the truth you re-read at the start of every turn, D13); Taking a handoff (pick the project from the task text and the map, say which and why in one line; if no project fits, report kind=question asking the user to name one); Finding or opening a leader (reuse only a leader whose `via` is supervisor, D10; judge free as: Paseo status idle AND waiting_on empty AND your ledger shows no open question for it, D7; a leader waiting on a reply means queue the task in the ledger and report kind=progress saying it is queued, D10; an idle leader with nothing open is cleared first, D10; open a leader with create_agent: provider claude/claude-opus-5-5 unless the project's .waggledance.toml names one, workspaceId from `paseo workspace ls --json` matching the project root or created with `paseo workspace create --path <root> --isolation local --json`, labels waggledance.role=leader, waggledance.project=<project id>, waggledance.via=supervisor, title '<project> leader', notifyOnFinish true, initialPrompt = the task with a one-line preamble naming the corr tag, D3); Clearing a session (send_agent_prompt asking the leader to write a pause handoff and release its session, wait for the finish notification, then archive_agent; archive outright only after 10 minutes of silence, D13); When a leader finishes (read its last reply; a result → report kind=result; a question → open an advisor with create_agent, no workspaceId, label waggledance.role=advisor, provider claude/claude-opus-5-5, prompt = the question plus the task context, notifyOnFinish true; when the advisor answers with a decision, send it to the leader and archive the advisor; when the advisor says it cannot decide, report kind=question to the user and archive the advisor, D4; a permission request or an error → report kind=blocked or error, never approve a permission yourself); Reporting (always through Bash `waggledance supervisor report --kind <k> --project <name> --corr <id> --text <t>`; text in Vietnamese, the leader's own words quoted verbatim where they matter, under 3500 characters with a pointer to the leader's Paseo chat beyond that; never read, print or guess the hook token, D6, D11); Replies from the user ([reply] → send_agent_prompt to the leader of that corr; [redo] → the same with the redo text, the leader stays; [accept] → clear the leader gracefully and report kind=closed, D9, D12); Your ledger (ledger.md in your cwd, append-only, one line per event: time, corr, project, leader id, advisor id, state; a hint, never the truth, D13); Your heartbeat (create one create_heartbeat every 30 minutes on first start if none exists; on each beat re-read the map, report kind=progress for any leader running over an hour, and retry any report the hook refused, D13); What you never do (never run a leader's work yourself, never approve a gate or permission, never archive a leader that still has an open question, never send into a running leader). Every rule cites its D-id in parentheses once, at the rule. End with a ten-line quick reference of the commands and tools you use. No code comments apply (it is prose); keep the whole file under 220 lines.",
    "verify": "test -s docs/waggledance-supervisor-agent.md && for d in D1 D2 D3 D4 D6 D7 D9 D10 D11 D12 D13; do rg -q \"\\($d\\b\" docs/waggledance-supervisor-agent.md || { echo missing $d; exit 1; }; done && rg -c '^## ' docs/waggledance-supervisor-agent.md",
    "must_haves": {
      "truths": ["a reader can follow each of the four inbound message shapes to the action it triggers", "the leader is only ever created with the three labels and notifyOnFinish true", "clearing is graceful: pause handoff, finish notification, then archive", "reports go only through waggledance supervisor report and the token is never mentioned as something to read"],
      "artifacts": [{"path": "docs/waggledance-supervisor-agent.md", "substantive": "complete standing instructions, every rule citing its decision id, under 220 lines"}],
      "key_links": ["the heartbeat section names create_heartbeat and the 30-minute cadence", "the map section names waggledance_ask_state leaders.via"],
      "prohibitions": ["no bee vocabulary", "no hook token value or path in the text", "no instruction to run the project's work itself"]
    }
  },
  {
    "id": "pns-3",
    "feature": "paseo-native-supervisor",
    "title": "Add leaders to waggledance_ask_state",
    "lane": "high-risk",
    "role": "code",
    "status": "open",
    "deps": [],
    "decisions": ["D7", "D8", "D10"],
    "files": ["crates/waggledance/src/mcp.rs", "crates/waggledance/src/paseo.rs", "crates/waggledance/testdata/fixtures/paseo-0.11.2/ls-label-leader.out", "crates/waggledance/testdata/fixtures/paseo-0.11.2/README.md"],
    "read_first": ["docs/history/paseo-native-supervisor/CONTEXT.md", "docs/history/paseo-native-supervisor/plan.md", "crates/waggledance/src/mcp.rs", "crates/waggledance/src/paseo.rs", "docs/history/ask-state-fleet-read/CONTEXT.md"],
    "affects_skills": [],
    "affects_specs": [],
    "action": "In crates/waggledance/src/paseo.rs: extend `struct ListedAgent {` (rg hit: `636:struct ListedAgent {`) with `cwd: String`, `status: String` and `created: Option<String>` (serde defaults, every field may be absent) and make a `pub fn parse_listed(stdout) -> Result<Vec<ListedAgent>, String>` beside `pub fn parse_leaders(stdout: &str)` that keeps the whole struct; add `pub const VIA_LABEL: &str = \"waggledance.via=supervisor\"` beside `ROLE_LABEL` and `pub fn role_leaders_args() -> Vec<String>` (`ls -g --label waggledance.role=leader --json`) and `pub fn via_leaders_args()` (the same plus `--label waggledance.via=supervisor`) beside `pub fn leaders_args(project_id: &str)`; give `PaseoCli` a `pub async fn list_labelled(&self, args) -> Result<Vec<ListedAgent>>` using the existing command/timeout path. In crates/waggledance/src/mcp.rs: beside `fn attach_panes(` add `fn attach_leaders(digest, project, rollup, leaders: &LeadersRead)` where `enum LeadersRead { Off, Unreachable(String), Listed { all: Vec<ListedAgent>, via: HashSet<String> } }` is read ONCE per tool call in `handle_ask_state` (both forms, the panes D4 pattern) by: `Off` when `config.paseo.enabled == Some(false)`; `Unreachable(reason)` when `paseo_unavailable(&config.paseo)` is Some or either list call fails (the reason text), never a tool error; else two list calls (role, then via). Mapping: an agent belongs to a project when its canonical cwd equals or is inside the project's root_path, or equals or is inside any path in the rollup's worktree list for that project; when two projects both contain it the deeper root wins; an agent under no project is dropped and counted in the digest's `leaders_skipped`. Each entry: `{agent_id, status, cwd, created, via: \"supervisor\"|\"daemon\", labels: {\"waggledance.role\": \"leader\", \"waggledance.project\": <id>, \"waggledance.via\": \"supervisor\"|null}, waiting_on: {kind, subject}|null from snapshot.state.waiting_on, last_daemon_run: {run_id, status}|null = newest run from engine.list_runs(project, 20) whose pane_id starts with `paseo:`}`, ordered oldest `created` first. `Off` adds no key; `Unreachable` adds `leaders: null` and `leaders_error`; `Listed` adds `leaders: [...]` (possibly empty) and `leaders_skipped`. Update the `waggledance_ask_state` description in ask_state_schema to name `leaders`, `via`, and the three absence states. Copy the real capture `paseo ls -g --json --label waggledance.role=leader` (its bytes are in plan.md claim 3 and must be re-captured from the wrapper, not typed) into crates/waggledance/testdata/fixtures/paseo-0.11.2/ls-label-leader.out and add its README row (command, environment: this host via the AppImage wrapper, date 2026-10-17, what it pins: noise lines then an empty list). Tests, in mcp.rs and paseo.rs: parse_listed over wrapper-systemd-ls.out and ls-label-leader.out; a leader whose cwd is the project root maps to it; a leader whose cwd is a sibling worktree in the rollup maps to it; a leader under no project is skipped and counted; two leaders ordered by created; via set membership yields via supervisor vs daemon; Unreachable yields null plus reason and keeps every existing key; Off yields no leaders key; the existing ask_state tests stay byte-identical for every key they already assert. Build with `cargo build --profile fast -p waggledance`, never --release. No code comments in new code.",
    "verify": "cargo test --profile fast -p waggledance leaders && cargo test --profile fast -p waggledance paseo",
    "must_haves": {
      "truths": ["ask_state for a project with Paseo available answers leaders as a list with via on every entry", "Paseo unavailable yields leaders null plus leaders_error and the bee half intact", "[paseo] enabled = false yields no leaders key", "a worktree-cwd leader maps to its project", "one Paseo list per label per tool call, never per project"],
      "artifacts": [{"path": "crates/waggledance/src/mcp.rs", "substantive": "attach_leaders with the three-state read and tests"}, {"path": "crates/waggledance/testdata/fixtures/paseo-0.11.2/ls-label-leader.out", "substantive": "the real wrapper capture, noise lines then []"}],
      "key_links": ["handle_ask_state reads LeadersRead once and passes it to attach_leaders in both forms", "parse_listed is the function the fixture tests and PaseoCli share"],
      "prohibitions": ["no change to the daemon delivery loop or resolve_leader", "no tool error on a Paseo failure", "no code comments"]
    }
  },
  {
    "id": "pns-1",
    "feature": "paseo-native-supervisor",
    "title": "Add waggledance supervisor ensure and report",
    "lane": "high-risk",
    "role": "code",
    "status": "open",
    "deps": ["pns-2"],
    "decisions": ["D2", "D6", "D8", "D11", "D13"],
    "files": ["crates/waggledance/src/cli.rs", "crates/waggledance/src/main.rs", "crates/waggledance/src/seat.rs", "crates/waggledance-core/src/config.rs", "crates/waggledance/testdata/fixtures/paseo-0.11.2/npm-run-detached.out", "crates/waggledance/testdata/fixtures/paseo-0.11.2/README.md"],
    "read_first": ["docs/history/paseo-native-supervisor/CONTEXT.md", "docs/history/paseo-native-supervisor/plan.md", "docs/waggledance-supervisor-agent.md", "crates/waggledance/src/setup.rs", "crates/waggledance/src/paseo.rs", "crates/waggledance-core/src/config.rs", "crates/waggledance/src/cli.rs"],
    "affects_skills": [],
    "affects_specs": [],
    "action": "Config (crates/waggledance-core/src/config.rs, beside `pub paseo: PaseoConfig,`): add `pub supervisor: SupervisorConfig { provider: String (default \"claude/claude-haiku-5-5\"), mode: String (default \"bypassPermissions\"), thinking: String (default empty), title: String (default \"waggledance supervisor\") }` and `pub openclaw: OpenClawConfig { home: String (default \"~/aicoworker/openclaw\"), hook_url: String (default \"http://127.0.0.1:18789/hooks/agent\"), telegram_to: String (default empty), hook_model: String (default empty), hook_thinking: String (default \"low\") }`, both `#[serde(default)]` like PaseoConfig, with defaults tested; add `pub fn hook_token_path(data_dir) -> PathBuf` (`<data_dir>/hook.token`, beside `const NOTIFY_CREDENTIAL_FILE_NAME: &str = \"telegram.token\";`) and `pub fn read_hook_token(data_dir) -> Option<String>` (trimmed, None when absent or empty); the token is never a Config field. CLI (crates/waggledance/src/cli.rs, beside `Setup {`): `Supervisor { #[command(subcommand)] action: SupervisorAction }` with `Ensure { json: bool }` and `Report { kind: String, project: String, corr: String, text: String, json: bool }`; wire it in main.rs where `Command::Setup` is matched, into a NEW module `crates/waggledance/src/seat.rs` (the name `supervisor` is taken by `mod supervisor;`, the herdr watchdog). seat.rs `ensure`: resolve `~/.waggledance/supervisor/` (data_dir join supervisor), create it, write CLAUDE.md from `include_str!(\"../../../docs/waggledance-supervisor-agent.md\")` only when bytes differ and remember whether a live agent was found before the write (instructions_stale = found && bytes differed); build `PaseoCli::new(&config.paseo)` and refuse with `paseo_unavailable` text when Paseo is not available; list `-g --label waggledance.role=supervisor --json` through the cli (parse with the paseo.rs document parser); when one agent is listed, print `{agent_id, created: false, instructions_stale, cwd}`; else run `run -d --json --cwd <dir> --provider <supervisor.provider> [--mode <supervisor.mode>] [--thinking] --title <supervisor.title> --label waggledance.role=supervisor <bootstrap>` where bootstrap is the one line `Read CLAUDE.md in this directory, create your heartbeat if none exists, then reply ready.`, parse `agentId` from the JSON document and print `{agent_id, created: true, instructions_stale: false, cwd}`; non-JSON output or a missing agentId is an error naming the first non-noise line. The listing and the run go through a `Runner`-style closure like setup.rs's `paseo_listed` so tests script them. seat.rs `report`: read_hook_token or refuse naming `waggledance setup openclaw` as the remedy; build the body with a pure `pub fn hook_body(cfg: &OpenClawConfig, kind, project, corr, text) -> serde_json::Value`: `{message: \"Chuyển nguyên văn cho người dùng, không diễn giải lại, không dịch, không thêm lời: [waggledance#<corr>] <project> — <kind-label>: <text>\", name: \"waggledance\", deliver: true, channel: \"telegram\", wakeMode: \"now\"}` plus `to` when telegram_to is set, `model` when hook_model is set, `thinking` = hook_thinking; kind-label maps result→kết quả, question→câu hỏi, blocked→đang chờ quyền, error→lỗi, closed→đã đóng, progress→tiến độ; refuse any other kind. POST with reqwest (the crate's existing async runtime; `Authorization: Bearer <token>`, `Content-Type: application/json`), 15 s timeout; on 2xx print `{status, sent: true}`; on any other status exit non-zero printing the status and the first 300 bytes of the body; on a connection error exit non-zero naming the url. Copy `.bee/tmp/pns/npm-run-detached.out` (a real capture) to crates/waggledance/testdata/fixtures/paseo-0.11.2/npm-run-detached.out and add its README row (command: npm CLI `paseo run -d --json ...` from inside a Paseo agent on 2026-10-17 without env scrub, which is why its cwd is the caller's workspace; it pins the `run -d --json` document shape). Tests (seat.rs and config.rs): defaults; hook_token_path; read_hook_token absent/empty/trimmed; hook_body exact JSON for each kind and refusal for an unknown kind; ensure over a scripted runner: listed → created false and no run; empty list → run called once with the exact argv and created true parsing the fixture; stale detection; report with no token file refuses before any request; the embedded doc equals the repo file (parity, like `embedded_tables_match_the_integration_sources`). Build with `cargo build --profile fast -p waggledance`; resolve the artifact path from `--message-format=json` if you run it. No code comments in new code.",
    "verify": "cargo test --profile fast -p waggledance seat && cargo test --profile fast -p waggledance-core config",
    "must_haves": {
      "truths": ["ensure is idempotent: a listed supervisor is reused and no run is issued", "ensure writes CLAUDE.md byte-equal to docs/waggledance-supervisor-agent.md", "report refuses without hook.token and never reads the token from Config", "hook_body carries the verbatim-forward instruction, the tag and the Vietnamese kind label"],
      "artifacts": [{"path": "crates/waggledance/src/seat.rs", "substantive": "ensure, report, hook_body and their tests"}, {"path": "crates/waggledance-core/src/config.rs", "substantive": "SupervisorConfig, OpenClawConfig, hook_token_path, read_hook_token with tests"}],
      "key_links": ["main.rs routes Command::Supervisor to seat.rs", "seat.rs uses PaseoCli from paseo.rs for detection, env scrub and timeout"],
      "prohibitions": ["no token field on Config", "no change to delivery.rs", "no code comments"]
    }
  },
  {
    "id": "pns-4",
    "feature": "paseo-native-supervisor",
    "title": "Install aicoworker's bridge and the webhook with waggledance setup openclaw",
    "lane": "high-risk",
    "role": "code",
    "status": "open",
    "deps": ["pns-1"],
    "decisions": ["D5", "D6", "D12", "D13"],
    "files": ["crates/waggledance/src/setup.rs", "crates/waggledance/src/cli.rs", "crates/waggledance-core/src/config.rs", "integrations/openclaw/waggledance-orchestrator/SKILL.md", "integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py"],
    "read_first": ["docs/history/paseo-native-supervisor/CONTEXT.md", "docs/history/paseo-native-supervisor/plan.md", "crates/waggledance/src/setup.rs", "crates/waggledance/src/cli.rs", "crates/waggledance-core/src/config.rs", "docs/waggledance-supervisor-skill-template.md"],
    "affects_skills": ["integrations/openclaw/waggledance-orchestrator/SKILL.md"],
    "affects_specs": [],
    "action": "Skill files, two commits: first commit integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py and SKILL.md as byte-for-byte copies of the host files at /home/thanhsmind/aicoworker/openclaw/workspace-main/skills/waggledance-orchestrator/ (read them; they are the rollback bytes). Second commit extends them: wd-bridge.py gains `handoff --task <text> | --task-file <path> [--corr <id>] [--dry-run]` (runs `waggledance supervisor ensure --json`, mints corr as `t-` plus 6 hex when absent, writes `[handoff <corr>] <task>` to a temp file, runs `paseo send <agent_id> --no-wait --prompt-file <tmp> --json`, prints `{supervisor_id, corr, sent}`; `--dry-run` prints the two commands and exits 0 without running them), `reply --corr <id> --text`, `redo --corr <id> --text`, `accept --corr <id>` (the same send with `[reply <corr>] `, `[redo <corr>] `, `[accept <corr>]`), each failing with a one-line error naming Paseo or waggledance when the subprocess fails; `paseo` resolves to the first of `$PASEO_CLI`, the npm CLI under ~/.local/share/mise/installs/node/*/bin/paseo, or `paseo` on PATH, and the child env drops PASEO_AGENT_ID and PASEO_AGENT_CWD. SKILL.md documents the four commands, the D12 message shapes, what arrives on Telegram (tagged `[waggledance#<corr>] <project>` lines), that a bare `ok` or `được` from the user means `accept` of the newest open corr while a sentence answering a question means `reply`, that `handoff` failing means tell the user Paseo or waggledance is down rather than claiming the task went through, and the limit that two racing handoffs on a missing supervisor may create two supervisors (`ensure` is unlocked; the next `ensure` returns the oldest). Rust: `Target::OpenClaw` in setup.rs (beside `pub enum Target {`) and `openclaw` in the cli.rs Setup target parser; `setup openclaw` does, in this order: (1) ensure `<data_dir>/hook.token` exists, generating 32 random bytes hex with the crate's existing randomness and writing it owner-only via the telegram.token writer, never overwriting an existing file; (2) write the two skill files (embedded with include_str! and parity-tested like PI_FILES) into `<openclaw.home>/workspace-main/skills/waggledance-orchestrator/` using the existing write_tree/is_current/refuse_foreign helpers; (3) write ~/.agents/skills/waggledance-supervisor/SKILL.md from `docs/waggledance-supervisor-skill-template.md` the same way; (4) edit `<openclaw.home>/openclaw.json`: copy it to `openclaw.json.bak-<unix seconds>` with mode 0600 first, then parse as JSON, set `hooks.enabled = true`, `hooks.token = <the token>`, `hooks.defaultSessionKey = \"hook:waggledance\"`, `hooks.allowRequestSessionKey = false`, keep every other key including `hooks.internal`, write back with two-space indentation; skip the write when every value already matches; (5) print `done`, `warnings`, and `notes` holding the two curl lines (`curl -s -o /dev/null -w '%{http_code}' -X POST <hook_url> -H 'Content-Type: application/json' -d '{\"message\":\"x\"}'` expecting 401, and the same with `-H \"Authorization: Bearer $(cat <token path>)\"` and the body `{\"message\":\"Chuyển nguyên văn cho người dùng: [waggledance] hook online\",\"deliver\":true,\"channel\":\"telegram\"}` expecting 200) and the note that the gateway may need a restart to pick up `hooks`, which only the user does. `--remove` deletes the skill directory it wrote (refuse_foreign guard), restores the newest `openclaw.json.bak-*` and deletes the token file. Tests in setup.rs over a temp openclaw home: the json edit keeps unknown keys and `hooks.internal`, sets the four hook keys, writes the 0600 backup, is idempotent; the token file is created 0600 and never overwritten; embedded skill files equal the repo files; the npm-first paseo resolution order in wd-bridge.py is covered by `python3 bin/wd-bridge.py handoff --dry-run --task probe` printing both commands. Build with `cargo build --profile fast -p waggledance`. No code comments in new Rust; the Python file keeps only its `#!` line.",
    "verify": "cargo test --profile fast -p waggledance setup && python3 -m py_compile integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py && python3 integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py handoff --dry-run --task probe",
    "must_haves": {
      "truths": ["setup openclaw writes the token before it touches openclaw.json", "the openclaw.json edit keeps hooks.internal and every unknown key and leaves a 0600 backup", "handoff --dry-run prints the ensure and paseo send commands without running them", "--remove restores the backup and deletes the token file and the skill directory"],
      "artifacts": [{"path": "crates/waggledance/src/setup.rs", "substantive": "Target::OpenClaw install, remove, json edit, tests"}, {"path": "integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py", "substantive": "handoff, reply, redo, accept with dry-run and env scrub"}, {"path": "integrations/openclaw/waggledance-orchestrator/SKILL.md", "substantive": "the four commands, the message shapes, the ok rule, the failure wording, the race limit"}],
      "key_links": ["setup.rs embeds the two skill files and the supervisor template with include_str! and a parity test", "wd-bridge.py handoff calls waggledance supervisor ensure --json"],
      "prohibitions": ["never overwrite an existing hook.token", "never restart the gateway", "no code comments in Rust"]
    }
  },
  {
    "id": "pns-5",
    "feature": "paseo-native-supervisor",
    "title": "Record the supervisor path and the frozen daemon path in knowledge",
    "lane": "high-risk",
    "role": "docs",
    "status": "open",
    "deps": ["pns-1", "pns-3", "pns-4"],
    "decisions": ["D1", "D8", "D10", "D11"],
    "files": ["docs/knowledge/patterns/paseo-native-supervisor.md", "docs/knowledge/patterns/paseo-leader-dispatch.md", "docs/knowledge/index.md", "docs/knowledge/patterns/index.md", "README.md", "docs/waggledance-supervisor-skill-template.md"],
    "read_first": ["docs/history/paseo-native-supervisor/CONTEXT.md", "docs/history/paseo-native-supervisor/plan.md", "docs/knowledge/patterns/paseo-leader-dispatch.md", "docs/waggledance-supervisor-agent.md", "crates/waggledance/src/seat.rs", "crates/waggledance/src/setup.rs", "integrations/openclaw/waggledance-orchestrator/SKILL.md"],
    "affects_skills": ["docs/waggledance-supervisor-skill-template.md"],
    "affects_specs": ["docs/knowledge/patterns/paseo-native-supervisor.md", "docs/knowledge/patterns/paseo-leader-dispatch.md"],
    "action": "Write docs/knowledge/patterns/paseo-native-supervisor.md as a bee.pattern (frontmatter like paseo-leader-dispatch.md: type, title, description, timestamp 2026-10-17, bee.id paseo-native-supervisor, lifecycle active, areas [orchestration], sources = this feature's CONTEXT.md and plan.md, polarity practice, signature 'a task handed in from outside Paseo'), written through bee-technical-writing: the three hops (aicoworker → supervisor by paseo send; supervisor ↔ leaders and advisors by Paseo subagents; supervisor → user by waggledance supervisor report through the OpenClaw hook), the D12 message shapes, the D10 reuse/clear/queue rules and the via label, the D11 limit that the hook is a model turn, D13's token file and graceful clear, the `leaders` field with its three absence states, `supervisor ensure` and `setup openclaw` with their outputs and `--remove`, the shared-label limit with the frozen daemon path (a project reached through the plugin request box and the supervisor at once may hold a daemon leader and a supervisor leader), and a Pointers section naming seat.rs, setup.rs, mcp.rs attach_leaders, paseo.rs parse_listed, integrations/openclaw/, docs/waggledance-supervisor-agent.md and the fixtures. In docs/knowledge/patterns/paseo-leader-dispatch.md add a short section 'Frozen' at the top of Dispatch stating that per paseo-native-supervisor D8 the queue, lock, turn-end detection and caller notices stay as documented and receive no new behaviour, that the supervisor path is the one for external agents, and linking the new pattern; update the Sources section's external-agent bullet to point to the new pattern. In docs/waggledance-supervisor-skill-template.md rewrite the 'Leader mode' section to say that a project in Paseo is reached through aicoworker's `handoff` (or `paseo send` to the supervisor found by label) and that `waggledance_dispatch leader: true` is the frozen daemon door kept for the plugin request box. Add one README.md paragraph under the orchestration section linking the new pattern. Regenerate the indexes with `.bee/bin/bee knowledge index` (never hand-edit the generated index files) and run `.bee/bin/bee knowledge check`. Use bee-unslop on the prose.",
    "verify": ".bee/bin/bee knowledge index --check && .bee/bin/bee knowledge check",
    "must_haves": {
      "truths": ["the bundle names the supervisor path, the frozen daemon path and the decision that froze it", "every Pointers path exists in the worktree", "the template's leader mode no longer tells an agent to dispatch leader: true as the first choice"],
      "artifacts": [{"path": "docs/knowledge/patterns/paseo-native-supervisor.md", "substantive": "a complete practice pattern with frontmatter and Pointers"}, {"path": "docs/knowledge/patterns/paseo-leader-dispatch.md", "substantive": "a Frozen section citing D8 and the shared-label limit"}],
      "key_links": ["docs/knowledge/patterns/index.md lists the new pattern (regenerated)", "README.md links the new pattern"],
      "prohibitions": ["no hand edits to generated index files", "no change to decision texts"]
    }
  },
  {
    "id": "pns-6",
    "feature": "paseo-native-supervisor",
    "title": "Drive one task from aicoworker to a leader and back",
    "lane": "high-risk",
    "role": "test",
    "status": "open",
    "deps": ["pns-3", "pns-4"],
    "decisions": ["D3", "D4", "D5", "D6", "D9", "D10", "D11", "D12"],
    "files": ["docs/history/paseo-native-supervisor/reports/live-proof.md"],
    "read_first": ["docs/history/paseo-native-supervisor/CONTEXT.md", "docs/history/paseo-native-supervisor/plan.md", "docs/waggledance-supervisor-agent.md", "integrations/openclaw/waggledance-orchestrator/SKILL.md", "docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md"],
    "affects_skills": [],
    "affects_specs": [],
    "action": "Live proof of the whole path; the only repo file written is docs/history/paseo-native-supervisor/reports/live-proof.md, the execution record the post-ship walkthrough reads: one dated section per step below with the command and its output verbatim, red or green. Steps: (1) build with `cargo build --profile fast -p waggledance --message-format=json` and take the `executable` path from the compiler-artifact line; confirm by `strings <path> | rg 'hook.token'`. (2) Run `<bin> setup openclaw --json`; record its output; run the two curl lines it printed and record 401 then 200; if the second is not 200, record it, tell the human in the cap that the gateway needs a restart to load hooks, and stop here with [BLOCKED]. (3) Run `<bin> supervisor ensure --json` twice; record created true then false and the same agent_id. (4) From the deployed skill run `python3 ~/aicoworker/openclaw/workspace-main/skills/waggledance-orchestrator/bin/wd-bridge.py handoff --task 'Project waggledance: reply with exactly PNS-LIVE-OK and nothing else, then stop.'`; record corr; wait with `paseo wait <supervisor id> --timeout 300 --json` as needed; then `paseo ls -g --json --label waggledance.role=leader --label waggledance.via=supervisor` must show exactly one agent whose cwd is the waggledance project root; `<bin>` MCP `waggledance_ask_state` for project waggledance (run `printf '<jsonrpc tools/call>' | <bin> mcp`) must list that leader with via supervisor. (5) Read the supervisor's `paseo logs` for the `waggledance supervisor report --kind result` call and record the Telegram-bound text; the human confirms the Telegram line in the uat stop, so record the hook response code from the supervisor's log line. (6) Second handoff: `--task 'Project waggledance: before doing anything, ask the user one question: which colour should the badge be, red or blue? Then stop and wait.'`; record that a `report --kind question` was issued (advisor could not decide) or that the advisor answered (then record its answer and that no question report was sent); run `wd-bridge.py reply --corr <corr2> --text 'xanh'` and record the leader's next turn end. (7) `wd-bridge.py accept --corr <corr2>`; wait; `paseo ls -g --json --label waggledance.role=leader --label waggledance.via=supervisor` must be `[]` and `paseo ls -g --json --label waggledance.role=advisor` must be `[]`; record the archived leader id. (8) Check ~/.waggledance/supervisor/ledger.md exists and has one line per event seen. Any step that fails is recorded as red with its output and the cell finishes [BLOCKED] naming the step; never patch code in this cell. Proof line: `green:live` with the eight recorded outputs.",
    "verify": "paseo ls -g --json --label waggledance.role=leader --label waggledance.via=supervisor && paseo ls -g --json --label waggledance.role=advisor",
    "must_haves": {
      "truths": ["exactly one via-labelled leader exists after the first handoff and zero after accept", "the result report text is recorded with its hook status", "the question path produced either an advisor answer or a Telegram question and the reply reached the leader", "zero advisors remain"],
      "artifacts": [{"path": "docs/history/paseo-native-supervisor/reports/live-proof.md", "substantive": "one section per step with the command and its verbatim output"}],
      "key_links": ["the deployed wd-bridge.py calls the freshly built waggledance binary's ensure", "the supervisor's report call appears in its Paseo log"],
      "prohibitions": ["no repo file other than the live-proof record", "no code patched in this cell", "no gateway restart"]
    }
  }
]
```

## Test matrix

High-risk: the 12 edge dimensions of `edge-dimensions.md`, probes only where a dimension applies.

| Dimension | Probe | Cell | Pass when |
|---|---|---|---|
| Empty / absent | Paseo unavailable during `ask_state` | pns-3 | `leaders` is `null`, `leaders_error` names the cause, the bee half of the answer is intact |
| Empty / absent | `[paseo] enabled = false` | pns-3 | no `leaders` key in the payload |
| Empty / absent | no supervisor agent exists | pns-1 | `ensure` runs `paseo run` once and prints `created: true` |
| Empty / absent | no `hook.token` | pns-1 | `report` refuses before any request and names `waggledance setup openclaw` |
| Idempotence | `ensure` twice | pns-1 | second call runs no `paseo run`, prints the same `agent_id`, `created: false` |
| Idempotence | `setup openclaw` twice | pns-4 | second run writes nothing, token unchanged, no second backup |
| Boundary | a leader whose cwd is a project root exactly | pns-3 | mapped to that project |
| Boundary | a leader whose cwd is a sibling worktree listed in the rollup | pns-3 | mapped to that project |
| Boundary | a leader whose cwd is under no registered root | pns-3 | dropped and counted in `leaders_skipped` |
| Ordering | two leaders for one project (fake state) | pns-3 | both listed, oldest `created` first |
| Auth | hook called without bearer | pns-6 | HTTP 401 recorded |
| Auth | hook called with bearer | pns-6 | HTTP 200 recorded and the Telegram text equals the report text |
| Secrets | `Config` serialisation | pns-1 | contains no token; `hook.token` written 0600 |
| Secrets | `openclaw.json` backup | pns-4 | mode 0600 |
| Concurrency | two `handoff` calls racing on a missing supervisor | pns-4 | the limit is written in SKILL.md |
| Failure path | hook returns 5xx | pns-1 | `report` exits non-zero with the status and body head |
| Failure path | `paseo ls` hangs | pns-3 | the `PaseoCli` timeout turns it into `leaders_error` |
| Failure path | Paseo down at `handoff` | pns-4 | `handoff` exits non-zero with a one-line error naming Paseo |
| Behaviour on main vs head | `ask_state` for a project with no leader | pns-3 | main: no `leaders` key; head: `leaders: []`; every existing key byte-identical |
| Live | task → result on Telegram | pns-6 | tagged line recorded with hook 200 |
| Live | question → advisor or Telegram → reply reaches the leader | pns-6 | recorded turn end after `reply` |
| Live | accept → graceful clear | pns-6 | zero via-labelled leaders, zero advisors |

## Open Questions

- Whether the OpenClaw gateway reloads `hooks` on config write or needs a restart; pns-6 finds out by curl after `setup openclaw` and stops with a message for the user if a restart is needed (their live assistant; never restarted by an agent).
- Whether `ledger.md` and the heartbeat survive a Paseo restart; pns-6 records what it sees, a later slice fixes what does not.

## Out of scope

- Several leaders per project (PBI filed).
- Removing or rewriting the daemon's leader queue, lock, or notices (frozen per D8).
- Routing the Waggle Dance Paseo plugin's request box through the supervisor.
- Any change to bee's own worker dispatch inside a leader.
- `parent_agent_id` in `leaders`.
