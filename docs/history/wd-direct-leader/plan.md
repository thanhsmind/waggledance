---
artifact_contract: bee-plan/v2
mode: high-risk
---

# Plan: wd Direct Leader

## Summary

wd on Telegram hands a task straight to a project's leader, and everything the leader
says back reaches wd without a middle agent. The daemon keeps an inbox in its own
database, wakes wd's main session once per message, and marks a message read only after
wd's turn finished. A leader stuck on a permission or a lost run also lands in that inbox
from the first slice. Later slices add wd's follow-up commands (reply, redo, accept), the
10-minute heartbeat, the full permission relay with allow and deny, and the removal of the
Paseo supervisor. Nothing reaches main until all three slices are done.

Mode: `high-risk` — 6 risk flags: data-model, external-systems, public-contracts,
covered-contract-change, proof-weakening, multi-domain
Why this is the least workflow that protects the work: the change adds a table, rewires an
outside client (OpenClaw) and deletes a working path with its tests, so each slice ends on
a live walk before the next one removes anything.

Playbook: `skills/bee-planning/playbooks/feature.md` — data shape first (wdl-1), walking
skeleton end to end in slice 1, current slice only, user-visible path proven (wdl-5),
knowledge sync in the same change (slice 3, plus each cell's own concept lines).

## Requirements (from CONTEXT.md)

- D1: wd hands a task with `waggledance_dispatch {project, task, leader: true, request_id: <corr>}`; the daemon finds or creates the leader and queues per project. The D8 freeze is lifted.
- D2: wd leaders carry `waggledance.via=wd`; a wd task reuses only a `via=wd` leader.
- D3: durable SQLite inbox: recipient, corr, project, kind, body, created, `acked_at`; survives restart.
- D4: `waggledance mail send --to wd --corr <corr> --kind <kind>`; a `result` without the done report is refused.
- D5: the daemon wakes wd for each unacked message with `openclaw agent`; frame `[waggledance#<corr>] <project> — <kind label>: <body>`; wd never polls.
- D6: ack only when the wake turn ends ok; a failed turn redelivers in `seq` order; reading never acks.
- D7: a leader asks its own project's bee advisor before mailing a `question`; the rule lives in the leader's initial prompt.
- D8: 10-minute heartbeat: nudge once, then `progress` once, `board` on change (slice 2).
- D9: permission watcher over `paseo permit ls --json`, `permit allow|deny` through wd, never auto-answered (slice 2; slice 1 carries the minimal `permission` message).
- D10: `reply`/`redo` to the task's leader; `accept` closes and archives (slice 2).
- D11: the Paseo supervisor is removed; OpenClaw skill and `wd-bridge.py` rewritten (slice 3; hand-off part in slice 1).
- D12: existing non-wd callers keep their behavior and notices.

## Load-bearing claims

Labels: `read` (opened at the anchor), `ran` (command run, output kept). No `guessed` row.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | The MCP leader branch only writes a durable `queued` run; the daemon tick delivers it | read | crates/waggledance/src/mcp.rs:1290 | `let stored = match engine.insert_run_with_request(&run, None, request_id, caller) {` |
| 2 | Leader creation builds the label list in one place | read | crates/waggledance/src/paseo.rs:998 | `labels: vec![ROLE_LABEL.to_string(), project_label(&project.id)]` |
| 2b | Leader reuse lists leaders through `leaders_args`, filtered on role and project only | read | crates/waggledance/src/paseo.rs:148-158 | `pub fn leaders_args(project_id: &str) -> Vec<String> {` |
| 3 | The store adds tables through an append-only migration list over `user_version`, now at 6 | read | crates/waggledance-core/src/repository.rs:756 | `pub const SCHEMA_VERSION: i64 = 6;` |
| 4 | The store opens SQLite in WAL with a busy timeout, so a CLI process and the daemon can both write | read | crates/waggledance-core/src/repository.rs:32-37 | `journal_mode=WAL` … `busy_timeout=5000` |
| 5 | The `openclaw agent` argv builder exists and can move | read | crates/waggledance/src/seat.rs:258-283 | `"agent", "--agent", cfg.agent_id.as_str(), "--message", message, "--deliver", "--channel", "telegram", "--json",` |
| 6 | `cli_run` takes a runtime it blocks on, so it cannot be called from a daemon task as is | read | crates/waggledance/src/seat.rs:286 | `fn cli_run(rt: &tokio::runtime::Runtime, argv: &[String]) -> Result<String> {` |
| 7 | The daemon wd target is a config field, `openclaw.agent_id` | read | crates/waggledance-core/src/config.rs:83 | `agent_id` |
| 8 | Background loops are tokio tasks the `TerminalBackground` reconciler starts and stops | read | crates/waggledance/src/main.rs:215 | `reconcile(` |
| 9 | `paseo permit ls --json` rows carry the request id, agent id, tool name and description | ran | `sed -n 13,20p ~/.local/share/mise/installs/node/24.19.0/lib/node_modules/@getpaseo/cli/dist/commands/permit/ls.js` | `id: permission.id, agentId: agent.id, … name: permission.name, description: permission.description ?? "-"` |
| 10 | `paseo permit ls --json` runs on this host and returns a JSON list | ran | `paseo permit ls --json` | `[]` |
| 11 | wd-bridge reaches waggledance by spawning `waggledance mcp` per call and sends to the supervisor through `ensure` | read | integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py:12-13,72-80 | `subprocess.Popen(["waggledance", "mcp"]` / `ENSURE = ["waggledance", "supervisor", "ensure", "--json"]` |
| 12 | Delivery already detects a leader turn that stops for a permission | read | crates/waggledance/src/delivery.rs:358-386 | `blocked_then_done` polls `inspect` until `pending_permissions == 0` |
| 13 | `delete_project` deletes each child table by hand | read | crates/waggledance-core/src/repository.rs:92-96 | `c.execute("DELETE FROM runs WHERE project_id=?1", params![id])?;` |

## Discovery

Two read-only gathers mapped the leader queue, the store, the daemon loops and every
supervisor surface (main checkout `.bee/mailbox/job-1792675953884-1053492-1/report-1.md`,
`.bee/mailbox/job-1792675960262-1055063-1/report-1.md`). "supervisor" also names the
herdr watchdog (`supervisor.rs`, `terminal.supervisor_enabled`) and `bee supervisor`;
neither is touched. The paseo CLI source fixes the `permit ls` row shape (claim 9). The
hat wave's findings and where each landed: `docs/history/wd-direct-leader/reports/hat-wave.md`.

## Approach

Recommended path: extend the daemon's leader queue rather than add a second one (D1).
A wd hand-off is an ordinary leader run whose `request_id` is the corr and whose `caller`
is `openclaw:wd`. The inbox is one new table. The wake loop is one serial tokio task,
separate from the delivery tick, so a long wd turn never stalls the leader queue.

Rejected:
- A separate wd queue beside the daemon's: two queues can deliver one task twice (piggery-lessons research).
- The runs notice columns as the inbox: one slot per run, dropped after an hour, no kind (breaks D3, D6, D8).
- Batching several messages into one wake: D5 wakes wd per message.
- A `via` field on the public dispatch schema: wd marks itself on its own `waggledance mcp` process instead.
- DuckDB: one writing process only (D3).

Risk map:

| Component | Risk | Lands in | Proof needed |
|---|---|---|---|
| Migration 7 on a live `registry.db` | MEDIUM | wdl-1 | migration test from a v6 database keeps every run row; `user_version` 7 |
| Wake turn talking to the real OpenClaw gateway | HIGH | wdl-4, wdl-5 | live walk: one real message reaches wd and its row is acked; a forced failure leaves it unacked |
| Leader label filter changing which leader a task gets | MEDIUM | wdl-2 | resolve tests both ways: wd skips non-wd leaders, non-wd skips `via=wd` leaders |
| Non-wd callers unchanged (D12) | MEDIUM | wdl-2 | existing delivery and mcp leader tests stay green unmodified |
| Nested runtime panic when the report code moves | HIGH | wdl-4 | the wake test runs inside `#[tokio::test]` |

Waves: wdl-1 alone (every cell reads the inbox) → wdl-2 and wdl-3 in parallel (disjoint
files) → wdl-4 (shares `main.rs` with wdl-3) → wdl-5 (drives the whole path).

## Slice-1 contract

Shared by wdl-2 (leader prompt, daemon-written messages), wdl-3 (CLI), wdl-4 (frame) and
wdl-5 (SKILL). One home: this section.

- **wd caller.** `wd-bridge.py` runs `waggledance mcp` with env `WAGGLEDANCE_CALLER=wd`.
  `handle_dispatch` reads it when `PASEO_AGENT_ID` is empty and stores `caller = "openclaw:wd"`.
  The tool schema does not change.
- **Leader labels.** A run whose caller is `openclaw:wd` resolves only leaders labelled
  `waggledance.via=wd` and creates leaders with `waggledance.role=leader`,
  `waggledance.project=<id>`, `waggledance.via=wd`. Every other leader run skips leaders
  labelled `waggledance.via=wd`. `ask_state.leaders[].via` gains the value `wd`.
- **Leader prompt.** For an `openclaw:wd` run, `handle_dispatch` stores the task as:
  `Task waggledance#<corr> from wd.` + a blank line + the mail rule + the advisor rule + a
  blank line + the task text unchanged. Mail rule: when done, run
  `waggledance mail send --to wd --corr <corr> --kind result` with the body on stdin,
  carrying four parts `Done:`, `Proof:`, `Left:`, `Where:`. Advisor rule: before
  `--kind question`, ask this project's bee advisor (`bee dispatch prepare --kind advisor`);
  mail only what it cannot decide, quoting its answer.
- **Done-report grammar.** A `result` body is accepted when, for each of `Done`, `Proof`,
  `Left`, `Where`, some line, after leading spaces and any of `#`, `-`, `*`, starts with
  that word (case-insensitive) followed by `:` or the end of a heading line, and the part
  under it is not empty. Otherwise the CLI exits 2 and names every missing part.
- **Kinds and labels.** `result` kết quả, `question` câu hỏi, `progress` tiến độ, `error`
  lỗi, `permission` xin quyền, `closed` đã đóng, `board` bảng việc. A leader may send
  `result`, `question`, `progress`, `error`; the daemon writes `permission`, `error`,
  `closed`, `board`.
- **Corr lookup.** `mail send --corr <c>` resolves the project from the newest run whose
  `request_id` is `<c>` or starts with `<c>:`. None found: exit 2 naming the corr.
- **Daemon-written messages (slice 1).** In `delivery.rs` `end`, for a run whose caller is
  `openclaw:wd`: `blocked` writes one `permission` message naming the leader and each
  pending permission id and tool; `lost` writes one `error` message with the transcript's
  first line. No Paseo caller notice is attempted for `openclaw:` callers.
- **Inbox row.** `seq` integer primary key, `recipient`, `corr`, `project_id`, `kind`,
  `body`, `created_at`, `acked_at` null, `attempts` 0, `dead_at` null.
- **Wake.** One serial task, every 2 s, active only when `openclaw.agent_id` is set. It
  takes the lowest-`seq` row with `acked_at` and `dead_at` null for recipient `wd`, frames
  it as the existing Vietnamese instruction followed by
  `[waggledance#<corr> #<seq>] <project> — <kind label>: <body>`, runs the `openclaw agent`
  argv (with `--reply-to` when `telegram_to` is set) asynchronously with the 900 s timeout,
  and on `status == "ok"` sets `acked_at`. On failure it increments `attempts` and backs
  off 30 s; at 5 attempts it sets `dead_at` and logs one warning naming the seq. Acked
  rows older than 30 days are deleted once an hour.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "30ff876890293b1cea96673b722ea95a7b27780259af61e6c3b2be09a0c2b112",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "required", "role": "hat-facts-gaps", "reason": "high-risk: all five seats; ran 2026-10-22"},
    {"stage": "hat-wave-alternatives", "classification": "required", "role": "hat-alternatives", "reason": "high-risk: all five seats; ran 2026-10-22"},
    {"stage": "hat-wave-user-impact", "classification": "required", "role": "hat-user-impact", "reason": "high-risk: all five seats; ran 2026-10-22"},
    {"stage": "hat-wave-risks", "classification": "required", "role": "hat-risks", "reason": "high-risk: all five seats; ran 2026-10-22"},
    {"stage": "hat-wave-value", "classification": "required", "role": "hat-value", "reason": "high-risk: all five seats; ran 2026-10-22"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "the hat wave is the high-risk advisor consult"},
    {"stage": "code", "classification": "required", "role": "code", "reason": "wdl-5 rewrites wd-bridge.py and SKILL.md"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "Rust cells wdl-1..wdl-4 are red-first test-plus-code work; precedent a66a46c0 moved Rust cells to this seat after two flash timeouts"},
    {"stage": "docs", "classification": "conditional", "role": "docs", "condition": "slice 3 knowledge sync", "reason": "no docs-only cell in slice 1"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "every cell names its role"},
    {"stage": "review", "classification": "conditional", "role": "review", "condition": "the user invokes bee-reviewing", "reason": "agents-review-user-invoked"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no release"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "herding cockpit role, not used"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "one design with repo precedent"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Shape

Three slices; only slice 1 has cells.

- **Slice 1 — walking skeleton (cells below).** Hand-off from wd → leader via the daemon
  → `mail send` → inbox → wake wd → ack, plus the daemon's `permission` and `error`
  messages for a wd run.
- **Slice 2 — follow-up, heartbeat, permissions.** A `tasks` table keyed by corr;
  `reply`/`redo`/`accept` through a new MCP tool (D10, archive on accept); the 10-minute
  heartbeat (D8); the 30-second `paseo permit ls` watcher and `permit allow|deny` (D9).
- **Slice 3 — retire the supervisor and sync knowledge.** Remove `seat.rs` ensure, the
  `supervisor` CLI, `docs/waggledance-supervisor-agent.md`, the supervisor door in the
  skill template's leader mode; rewrite `paseo-native-supervisor.md` into the wd pattern,
  lift the Frozen section of `paseo-leader-dispatch.md`, update specs (D11).

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| wdl-1 | Store wd inbox messages in the registry database | crates/waggledance-core/src/repository.rs, crates/waggledance-core/src/engine.rs, crates/waggledance-core/src/domain.rs | — | a v6 database opens as v7 with an `inbox` table and every run row intact; deleting a project deletes its messages | `cargo test --profile fast -p waggledance-core` green |
| wdl-2 | Hand wd tasks to via=wd leaders and mail wd when a wd run blocks or is lost | crates/waggledance/src/mcp.rs, crates/waggledance/src/paseo.rs, crates/waggledance/src/delivery.rs | wdl-1 | a dispatch from `WAGGLEDANCE_CALLER=wd` opens or reuses a `waggledance.via=wd` leader and sends it the task with the mail and advisor rules; a blocked wd run leaves a `permission` message | `cargo test --profile fast -p waggledance` green incl. new resolve, dispatch and delivery tests |
| wdl-3 | Let a leader mail wd with waggledance mail send | crates/waggledance/src/cli.rs, crates/waggledance/src/mail.rs (new), crates/waggledance/src/main.rs | wdl-1 | `waggledance mail send --to wd --corr t-abc123 --kind result` stores a message; a result missing a part exits 2 naming it | `cargo test --profile fast -p waggledance mail` green |
| wdl-4 | Wake wd's main session once per unacked inbox message | crates/waggledance/src/wake.rs (new), crates/waggledance/src/seat.rs, crates/waggledance/src/main.rs | wdl-1, wdl-3 | the daemon runs one `openclaw agent` turn per message and marks it read only when the turn returns ok | `cargo test --profile fast -p waggledance wake` green |
| wdl-5 | Route wd-bridge hand-off through the daemon and walk it live | integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py, integrations/openclaw/waggledance-orchestrator/SKILL.md, docs/history/wd-direct-leader/reports/slice-1-live.md (new) | wdl-2, wdl-3, wdl-4 | `wd-bridge.py handoff --project <p> --task ...` returns a corr and run id; the leader's done mail reaches wd's main session | `green:live` — the walk's commands and outputs in `docs/history/wd-direct-leader/reports/slice-1-live.md` |

Slices 1 (wdl-1..wdl-5) and 2 (wdl-6..wdl-9) are capped. Plan rev 2 carries slice 3: contract in `docs/history/wd-direct-leader/slice-3.md`, risks consult in `docs/history/wd-direct-leader/reports/slice-3-risks.md`. Current slice cells:

```json
[
 {"id":"wdl-10","feature":"wd-direct-leader","title":"Remove the Paseo supervisor command, config and agent instructions","lane":"high-risk","role":"test","status":"open","deps":["wdl-9"],
  "decisions":["D11","D12","50943778-7dd4-4505-b797-aed7f88a0015","288802b5-6ac2-4e6e-a5b9-3ec25f7e985e"],
  "files":["crates/waggledance/src/cli.rs","crates/waggledance/src/seat.rs","crates/waggledance/src/main.rs","crates/waggledance/src/wake.rs","crates/waggledance/src/paseo.rs","crates/waggledance/src/mcp.rs","crates/waggledance/src/setup.rs","crates/waggledance/src/doctor.rs","crates/waggledance-core/src/config.rs","docs/waggledance-supervisor-agent.md","docs/waggledance-supervisor-skill-template.md"],
  "read_first":["docs/history/wd-direct-leader/slice-3.md","crates/waggledance/src/seat.rs","crates/waggledance/src/cli.rs","docs/waggledance-supervisor-skill-template.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"Follow docs/history/wd-direct-leader/slice-3.md sections 'Remove' and 'Keep' exactly. Delete the supervisor command and seat.rs with its tests, keeping every helper wake.rs owns; delete SupervisorConfig and the Config.supervisor field so that a config file with a [supervisor] table still loads (add a test for that); delete docs/waggledance-supervisor-agent.md and every include_str of it; remove VIA_LABEL and via_leaders_args so ask_state via reads wd or daemon, updating the tool description; rewrite only the leader-mode section of docs/waggledance-supervisor-skill-template.md to the two doors the contract names, keeping the template's frontmatter phrases for leader requests. Do not touch the herdr watchdog supervisor.rs or terminal.supervisor_enabled. Delete the tests that pin removed items (the c.supervisor config tests, the via_leaders_args test, the via supervisor test) and adjust setup.rs and doctor.rs tests only where they pinned removed text. Run the built binary once: waggledance supervisor must be an unknown command.",
  "must_haves":{"truths":["waggledance supervisor is an unknown command","A config file with a [supervisor] table still loads","ask_state via is wd or daemon"],"artifacts":[{"path":"docs/waggledance-supervisor-skill-template.md","substantive":"leader mode routes to wd-bridge handoff or waggledance_dispatch leader true"}],"key_links":["wake.rs keeps the openclaw helpers it uses"],"prohibitions":["The herdr watchdog and terminal.supervisor_enabled are untouched","No change to the inbox, wake, heartbeat or task tool behaviour"]},
  "verify":"cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"wdl-11","feature":"wd-direct-leader","title":"Describe the wd direct leader path in the knowledge layer","lane":"high-risk","role":"docs","status":"open","deps":["wdl-10"],
  "decisions":["D1","D11","50943778-7dd4-4505-b797-aed7f88a0015","288802b5-6ac2-4e6e-a5b9-3ec25f7e985e"],
  "files":["docs/knowledge/patterns/wd-direct-leader.md","docs/knowledge/patterns/paseo-native-supervisor.md","docs/knowledge/patterns/paseo-leader-dispatch.md","docs/knowledge/patterns/index.md","docs/specs/doctor.md","docs/specs/reading-map.md","README.md"],
  "read_first":["docs/history/wd-direct-leader/slice-3.md","docs/history/wd-direct-leader/CONTEXT.md","docs/history/wd-direct-leader/plan.md","docs/history/wd-direct-leader/slice-2.md","docs/knowledge/patterns/paseo-native-supervisor.md"],
  "affects_skills":[],"affects_specs":["docs/specs/doctor.md","docs/specs/reading-map.md"],
  "action":"Follow docs/history/wd-direct-leader/slice-3.md section 'Knowledge'. Write docs/knowledge/patterns/wd-direct-leader.md in the frontmatter shape of the existing pattern files (type bee.pattern, bee.id, lifecycle active, areas orchestration, sources the feature's CONTEXT, plan, slice-2 and slice-3 files): how wd hands off, the inbox and its ack rule, the wake frame, the follow-up tool, the heartbeat rules, the permission watcher, and Pointers naming wake.rs, mail.rs, heartbeat.rs, mcp.rs, delivery.rs, paseo.rs, repository.rs and wd-bridge.py. Mark paseo-native-supervisor.md lifecycle retired with one line pointing to it; lift the Frozen section of paseo-leader-dispatch.md; add the new pattern to patterns/index.md; fix any line in README.md and docs/specs (including the doctor row) that promises supervisor ensure or supervisor report. Load the bee-technical-writing skill first. Proof: every path the new pattern's Pointers names exists, and rg finds no remaining supervisor ensure or supervisor report promise outside docs/history.",
  "must_haves":{"truths":["A reader finds how wd reaches a leader from the patterns index"],"artifacts":[{"path":"docs/knowledge/patterns/wd-direct-leader.md","substantive":"the wd path with Pointers"}],"key_links":["patterns/index.md links the new pattern"],"prohibitions":["docs/history files are not rewritten"]},
  "verify":"pointer check: each Pointers path exists and rg 'supervisor (ensure|report)' docs README.md --glob '!docs/history/**' finds only retired-pattern mentions",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":false}}
]
```

## Test matrix

High-risk: the applicable edge dimensions. Each writer judges existing coverage first; a
row whose behavior already exists (idempotency) cites the existing test instead of a new one.

| Dimension | Probe | Cell | Pass when |
|---|---|---|---|
| Happy path | hand-off → leader → result mail → wake → ack | wdl-5 | the message row has `acked_at` set and wd's session shows the frame |
| Data migration | open a v6 database with runs | wdl-1 | `user_version` is 7 and the run count is unchanged |
| Delete | delete a project that has messages | wdl-1 | its inbox rows are gone, other projects' rows remain |
| Idempotency | the same corr dispatched twice | wdl-2 | existing test `mcp.rs` repeated `request_id` still passes: one run |
| Isolation | a non-wd leader exists; a wd task arrives | wdl-2 | a new leader starts with `waggledance.via=wd` |
| Isolation (reverse) | a `via=wd` leader exists; a non-wd task arrives | wdl-2 | the `via=wd` leader is not used |
| Backward compatibility | existing dispatch with `leader: true` and no wd caller | wdl-2 | existing delivery and mcp tests pass unmodified |
| Blocked wd run | turn ends on a permission | wdl-2 | one `permission` message naming the permission id and tool |
| Validation | `result` body missing `Proof` and `Where` | wdl-3 | exit 2, stderr names `Proof` and `Where`, no row written |
| Validation | unknown `--kind` / unknown `--corr` | wdl-3 | exit 2 naming the accepted kinds / the corr |
| Failure | `openclaw agent` exits non-zero or `status != ok` | wdl-4 | the row stays unacked, `attempts` is 1, the next wake carries it again |
| Poison | a message fails 5 times | wdl-4 | `dead_at` set, one warning logged, the next message is woken |
| Ordering | three messages pending | wdl-4 | three wakes in `seq` order |
| Concurrency | a message lands while a wake turn runs | wdl-4 | it waits for the next wake, not acked by the running one |
| Delivered then timed out | the CLI posts and the 900 s timeout fires | wdl-4 | the row redelivers with the same `#<seq>` in its frame |
| Runtime | the wake runs inside a tokio runtime | wdl-4 | the test under `#[tokio::test]` passes, no nested-runtime panic |
| Config absent | `openclaw.agent_id` empty | wdl-4 | no wake runs; rows stay unacked |
| Binary skew | leader's `waggledance` vs the daemon's | wdl-5 | both resolve to the same built artifact, confirmed by a literal only this change contains |

## Open Questions

(none)

## Out of scope

- The per-project hold is shared by wd and non-wd runs: a busy plugin leader parks a wd task on the same project until it is idle. Accepted limit (hat wave, risks seat).
- DuckDB analytics over the store.
- The herdr watchdog (`supervisor.rs`) and `bee supervisor`, which share only the word.
