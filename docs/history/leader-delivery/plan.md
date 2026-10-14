---
artifact_contract: bee-plan/v1
mode: high-risk
---

# Plan: Leader delivery

## Summary

A request for a project's Paseo leader becomes a `queued` run the moment it
is accepted. The waggledance daemon delivers queued requests one at a time per
project, only while the leader is idle, through a Paseo call that waits until
the leader's turn ends, and marks the run done only when Paseo reports that
turn completed. A busy leader no longer refuses: its requests wait. A request
may carry a `request_id`, so a retry never sends twice. When a run ends or
needs a permission, the caller hears about it — through the notify path, and
as a short message to the calling Paseo agent once that agent is idle. If the
daemon is not running, callers are told the queue is paused. Every Paseo
behaviour the code relies on is pinned by a captured fixture.

Mode: `high-risk` — 4 risk flags: external systems (the `paseo` CLI), public
contracts (`waggledance_dispatch` gains `request_id`, runs gain `queued`),
data model (runs migration 5), multi-domain (Rust daemon + TypeScript plugin).
Why this is the least workflow that protects the work: a lost, doubled or
interrupting request reaches a real repo, so the full matrix stays; the herdr
path changes only by honoring `request_id`.

## Requirements (from CONTEXT.md)

- D1: `request_id` makes a repeat dispatch return the original run and send nothing.
- D2: done = the Paseo turn carrying the task ended, as Paseo reports it; Paseo behaviours are pinned by fixtures.
- D3: busy leader → held in a per-leader queue, delivered in order; leader creation serialized.
- D4: on done / blocked / lost the caller is told unasked.
- D5: never send to a running or initializing Paseo agent (leader or caller).
- D6: pre-send failure → back to queued with backoff, lost after 5 attempts; post-send timeout or restart → lost "may still be finishing", project held until leader idle; claimed-never-sent → queued on restart.
- D7: idempotency exact on the leader path, sequential on herdr.
- D8: `queued` is a public status with age; "paused — daemon not running" when the heartbeat is stale; a new leader starts with the task via blocking `run`; turn timeout default 1 h.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command executed); evidence is a verbatim substring of the anchor. Probe captures live in this worktree's `.bee/cache/paseo-probe/` (Paseo 0.11.2, 2026-10-14) and become fixtures in ld-1.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | A blocking `send` returns when the turn ends, status completed (D2) | read | `.bee/cache/paseo-probe/send-blocking.out:3` | `"status": "completed",` |
| 2 | A blocking `run` returns completed at the end of its first turn, so a new leader starts with the task (D8) | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in run-blocking-cli.out) | `"status": "completed",` |
| 3 | A blocking `send` returns early with status permission when the leader waits on one (blocked) | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in send-perm.out) | `"status": "permission",` |
| 4 | `inspect` lists pending permissions with their tool | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in inspect-pending.out) | `"tool": "Write"` |
| 5 | A send into a running turn interrupted it: the long task's reply never appears after the short message (D5) | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in logs-after-busy.out) | `[User] Reply with exactly the word SHORT-DONE.` |
| 6 | Killing a blocking send leaves the leader's turn running (D6) | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in logs-after-kill.out) | `[Task notification] Background command "Wait 20 seconds" completed (exit code 0)` |
| 7 | `send --no-wait` returns at once (caller notice) | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in send-nowait.out) | `"status": "sent",` |
| 8 | `wait` reports idle (used to hold a project until its leader is idle) | read | `.bee/cache/paseo-probe/wait.out:3` | `"status": "idle",` |
| 9 | The PATH wrapper prints multi-line startup objects inside a Paseo agent | read | `.bee/cache/paseo-probe/run-blocking.out:3` | `22:38:47.170 › [login-shell-env] start {` |
| 10 | The PATH wrapper works from a systemd user scope (the daemon's environment), after its noise lines | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in wrapper-systemd-ls.out) | `"shortId": "35eca12",` |
| 11 | An agent created from inside a Paseo agent becomes its child, so the client scrubs `PASEO_AGENT_*` | ran | `bash .bee/cache/paseo-probe/probe.sh` (output kept in inspect-pending.out) | `"ParentAgentId": "35eca12c-e112-4690-bf48-af0222f659e1"` |
| 12 | Paseo names its bundled CLI to its agents | ran | `env \| grep PASEO_CLI` | `PASEO_CLI=/tmp/.mount_Paseo.C0veTi/resources/bin/paseo` |
| 13 | The client falls back to `paseo` on PATH today | read | `crates/waggledance-core/src/config.rs:43` | `"paseo"` |
| 14 | Runs schema is version 4, so new columns are migration 5 | read | `crates/waggledance-core/src/repository.rs:560` | `pub const SCHEMA_VERSION: i64 = 4;` |
| 15 | Migrations are an ordered list the new step appends to | read | `crates/waggledance-core/src/repository.rs:556` | `(4, migration_4_runs_final_transcript),` |
| 16 | Stored terminal statuses are a whitelist; `queued` stays non-terminal | read | `crates/waggledance/src/orchestrate.rs:631` | `pub fn terminal_from_stored(status: &str) -> Option<RunStatus> {` |
| 17 | The final transcript has its own store function | read | `crates/waggledance-core/src/repository.rs:273` | `pub fn set_run_final_transcript(&self, id: &str, transcript: &str) -> Result<()> {` |
| 18 | The reaper's Paseo sweep is what the delivery loop replaces | read | `crates/waggledance/src/reaper.rs:196` | `async fn sweep_paseo(&self) -> Vec<(String, Verdict)> {` |
| 19 | The leader branch of dispatch is one function to rewrite | read | `crates/waggledance/src/mcp.rs:1018` | `fn dispatch_to_leader(` |

## Discovery

The piggery v0.10.2 study and two live probes on Paseo 0.11.2 with throwaway
agents (`wd probe*`, all deleted; `deletedCount: 1` each). Findings: claims
1–12 and CONTEXT D5–D8. Five advisor seats critiqued the first draft
(synthesis: `docs/history/leader-delivery/reports/hat-wave.md`). Every cell's
verify filter was run on the base before the gate (`delivery` has 0 tests
until ld-4 adds them).

## Approach

Playbook: `skills/bee-planning/playbooks/feature.md` (cited, not copied).

**Recommended path.**

- **Client (ld-1).** Binary resolved on every call: `[paseo] binary`, else
  `$PASEO_CLI` when set and the file exists, else `paseo`. Child processes
  drop `PASEO_AGENT_ID` and `PASEO_AGENT_CWD` and keep `PASEO_HOME`. Output
  parsing takes the JSON document starting at the first line beginning `{`
  or `[`. New calls: `run_and_wait(spec)` (blocking `run --json`, no `-d`),
  `send_and_wait(id, text, timeout)` and `wait(id, timeout)`, each returning
  a `TurnEnd` of `Completed`, `Permission` or `Other{status, message}`.
  Fixtures in `crates/waggledance/testdata/fixtures/paseo-0.11.2/` with a
  README, parsed by tests through the production functions (D2).
- **Store (ld-2).** Migration 5: `request_id`, `caller`, `attempts`
  (integer, default 0) on `runs`, unique index `(project_id, request_id)`
  where not null. Repository: insert with request id (conflict returns the
  existing id), find by request id, list queued Paseo runs per project oldest
  first, claim (queued → working, single winner, pane stays
  `paseo:pending`), set pane, requeue with attempts+1, list in-flight Paseo
  runs.
- **Dispatch (ld-3).** Optional `request_id` on every target; a hit returns
  the stored run with `duplicate: true` before any send (D1, D7). The leader
  branch checks the gates and inserts a `queued` run (`paseo:pending`,
  `caller` from `PASEO_AGENT_ID`), returning `{run_id, status: "queued",
  duplicate: false}` plus "paused — waggledance daemon not running" when the
  heartbeat file is older than 30 s (D8). ld-3 does not touch await.
- **Delivery (ld-4).** A daemon task whenever `[paseo] enabled`, writing a
  heartbeat file every tick (2 s). Per project, one delivery at a time under
  the per-project lock file held for the whole delivery. Hold a project while
  its leader is running (D5) or while a previous run is "may still be
  finishing" (D6). Deliver: resolve the leader; existing → `send_and_wait`;
  none → `run_and_wait` with the task (D8). `Permission` → `blocked` +
  notice, then poll `inspect` until no pending permission, then `wait` for
  idle → `done`. `Completed` → `done`, transcript = leader text after the
  last `[User] <task first line>`. Timeout (default 3600 s) or `Other` →
  `lost`. Pre-send failures → requeue with backoff, `lost` after 5 (D6).
  Restart: `working`+`paseo:pending` → `queued`; `working`/`blocked` with an
  agent → `lost` "may still be finishing", project held until idle. Await on
  a `paseo:` run reads the ledger only (`queued` with age, `working`,
  `blocked`, terminal + transcript); the log-text heuristic and the reaper's
  Paseo sweep are removed. Notices (D4): existing notify path, plus one
  `send --no-wait` to `caller` when set, not the leader, and idle (D5),
  retried each tick for up to 1 h, best-effort after the status write.
  Wording: done/lost informational; blocked names the permission to approve
  in the leader's chat.
- **Composer (ld-5).** Per-draft `request_id` (kept on retry, replaced on
  success or text change); shows "Queued for <project> leader", "Already
  sent", or the paused warning; polls `waggledance_await` every 10 s to show
  queued / working / blocked / done for the last request, since the panel is
  not a Paseo agent and gets no caller notice.
- **Docs (ld-6).** Supervisor template, the paseo-leader-dispatch concept,
  and the runtime-capture lesson (extend an existing pattern when one
  covers it).

**Rejected.** A bootstrap turn before the task (claim 2 makes it
unnecessary); refusing a busy leader (D3); sending into a running agent
(claim 5); re-sending after a restart (claim 6: the turn may still be
running); a delivery loop inside `waggledance mcp` (dies with its client);
Paseo `notifyOnFinish` for D4 (it notifies the agent that sent the prompt —
the daemon is not an agent); reserving herdr rows before the send (D7);
reusing `preset_label` for `caller` (overloads a column for five lines).

**SMALLER PATH check.** No bootstrap (cut), no separate await cell (folded
into ld-4), no column reuse (rejected above); dropping the daemon loop breaks
D2/D3/D5. PASS.

**Risk map.**

| Component | Risk | Lands in | Proof |
|---|---|---|---|
| Interrupting a running agent (D5) | HIGH | ld-4 | never sends while inspect says running; fixture of claim 5 |
| Doubled request (D1) | HIGH | ld-2, ld-3 | repeat dispatch returns the same run; unique index |
| False done (D2) | HIGH | ld-4 | done only from `Completed` or permission→idle; fixtures |
| Wrong binary (daemon vs agent env) | HIGH | ld-1 | resolution-order tests; wrapper fixtures for both envs |
| Lost-but-running after restart (D6) | MEDIUM | ld-4 | restart test; project held until idle |
| Silent queue with daemon down (D8) | MEDIUM | ld-3 | stale heartbeat → paused warning |
| Migration on live DB | MEDIUM | ld-2 | v4→v5 keeps rows |

Waves: (ld-1 ∥ ld-2) → ld-3 → (ld-4 ∥ ld-5 ∥ ld-6). ld-3 needs both stores
of truth; ld-4/5/6 need ld-3's contract and touch disjoint files.

## Shape

Epic map — outcome: every accepted leader request is delivered once, in
order, never into a running turn, ends with a true status, and its caller
hears the result.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| Client + fixtures | D2, D5, D8 | the real CLI, pinned | 1 | fixture tests |
| Store | D1, D3, D6 | idempotency, queue, attempts | 1 | migration + repository tests |
| Dispatch | D1, D3, D7, D8 | the contract | 1 | mcp tests |
| Delivery + await | D2–D6, D8 | turn-tied completion, push | 1 | FakePaseo tests |
| Composer + docs | D1, D4, D8 | callers use it | 1 | node tests, doctor tests |

Slice queue: one slice, six cells.

## Cells — current slice (preview)

```json
[
  {
    "id": "ld-1",
    "title": "Call the right Paseo CLI and wait for a leader's turn to end",
    "role": "code",
    "lane": "high-risk",
    "feature": "leader-delivery",
    "deps": [],
    "decisions": ["D2", "D5", "D8"],
    "files": ["crates/waggledance/src/paseo.rs", "crates/waggledance-core/src/config.rs", "crates/waggledance/testdata/fixtures/paseo-0.11.2/*"],
    "read_first": ["docs/history/leader-delivery/CONTEXT.md", "docs/history/leader-delivery/plan.md", "crates/waggledance/src/paseo.rs", "crates/waggledance-core/src/config.rs"],
    "action": "Resolve the binary on every call (one function): config.paseo.binary when non-empty, else PASEO_CLI when set and the file exists, else `paseo`. Child processes drop PASEO_AGENT_ID and PASEO_AGENT_CWD and keep every other variable, PASEO_HOME included. Parsing: JSON commands take the document that starts at the first line beginning with `{` or `[`; `-q` ids take the last non-empty line that is not noise; logs text drops noise lines and the indented continuation lines and closing brace of a multi-line noise object. Add `TurnEnd { Completed, Permission, Other { status, message } }` from the JSON `status` field, and: `run_and_wait(spec, timeout)` = `run --json` WITHOUT -d (returns agent id + TurnEnd), `send_and_wait(id, text, timeout)` = `send <id> --prompt-file <tmp> --json` without --no-wait, `send_no_wait(id, text)`, `wait(id, timeout)` = `wait <id> --timeout <s> --json`. Keep resolve_leader as is except that a Start no longer runs `run -d`: it returns the spec for run_and_wait. Add `turn_timeout_secs` (default 3600) to PaseoConfig. FakePaseo: scriptable TurnEnds, statuses and pending permissions, and records every call. Copy every file from /mnt/data/Projects/goglbe/waggledance--wt--leader-delivery/.bee/cache/paseo-probe/*.out (and wrapper-systemd-ls.err) into crates/waggledance/testdata/fixtures/paseo-0.11.2/ with a README.md naming for each file the exact command, the environment (inside a Paseo agent, or a systemd user scope), the Paseo version 0.11.2 and the date 2026-10-14, and what behaviour it pins. Tests parse every fixture through the production parse functions via include_str!. No code comments.",
    "verify": "cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance-core config",
    "must_haves": {
      "truths": ["binary resolution prefers config, then an existing PASEO_CLI, then paseo, evaluated per call", "child processes never inherit PASEO_AGENT_ID or PASEO_AGENT_CWD and keep PASEO_HOME", "the inside-agent wrapper fixture with multi-line noise and the systemd wrapper fixture both parse", "run-blocking-cli and send-blocking fixtures map to TurnEnd::Completed, send-perm to TurnEnd::Permission", "inspect-pending yields one pending permission with tool Write"],
      "artifacts": [{"path": "crates/waggledance/src/paseo.rs", "substantive": "binary order, env scrub, document parsing, TurnEnd calls, fixture tests"}, {"path": "crates/waggledance/testdata/fixtures/paseo-0.11.2/README.md", "substantive": "command, environment, version, date and pinned behaviour per file"}],
      "key_links": ["fixture tests call the same parse functions as PaseoCli"],
      "prohibitions": ["no shell", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "ld-2",
    "title": "Store a request id, a caller, attempts and a queued state on runs",
    "role": "code",
    "lane": "high-risk",
    "feature": "leader-delivery",
    "deps": [],
    "decisions": ["D1", "D3", "D6", "D7"],
    "files": ["crates/waggledance-core/src/repository.rs", "crates/waggledance-core/src/engine.rs"],
    "read_first": ["docs/history/leader-delivery/plan.md", "crates/waggledance-core/src/repository.rs", "crates/waggledance-core/src/engine.rs"],
    "action": "Migration 5 (append to MIGRATIONS, SCHEMA_VERSION 5): nullable request_id TEXT, nullable caller TEXT, attempts INTEGER NOT NULL DEFAULT 0 on runs, with the has_column + ALTER TABLE pattern of migration_3, and a unique index on (project_id, request_id) WHERE request_id IS NOT NULL. The Run struct stays unchanged (column-only fields, like feature and final_transcript). Repository + Engine pass-throughs: insert_run_with_request(run, feature, request_id, caller) returning the existing run id on a unique conflict; run_by_request(project_id, request_id); run_caller(run_id); run_attempts(run_id); list_queued_paseo_runs(project_id) oldest first (status 'queued' AND pane_id LIKE 'paseo:%'); list_projects_with_queued_paseo_runs(); claim_queued_run(run_id) moving exactly one row queued -> working (pane stays paseo:pending), false when another claim won; set_run_pane(run_id, pane_id); requeue_run(run_id) working -> queued with attempts+1; list_inflight_paseo_runs() (status working or blocked AND pane_id LIKE 'paseo:%'). Tests: a v4 DB with runs migrates to v5 keeping rows; duplicate request id returns the first id; same request id in another project inserts; claim is single-winner; requeue bumps attempts; queued rows never appear in list_unattended_working_runs. No code comments.",
    "verify": "cargo test --profile fast -p waggledance-core runs && cargo test --profile fast -p waggledance-core migrat",
    "must_haves": {
      "truths": ["a v4 database migrates to v5 keeping its runs", "a second insert with the same project and request id returns the first run id", "two claims of one queued run succeed exactly once", "requeue increments attempts", "queued runs are not seen by the herdr reaper query"],
      "artifacts": [{"path": "crates/waggledance-core/src/repository.rs", "substantive": "migration 5 and queue/request functions with tests"}],
      "key_links": ["Engine exposes the new repository functions"],
      "prohibitions": ["no field added to Run", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "ld-3",
    "title": "Queue leader requests and return the same run for a repeated request id",
    "role": "code",
    "lane": "high-risk",
    "feature": "leader-delivery",
    "deps": ["ld-1", "ld-2"],
    "decisions": ["D1", "D3", "D7", "D8"],
    "files": ["crates/waggledance/src/mcp.rs"],
    "read_first": ["docs/history/leader-delivery/plan.md", "crates/waggledance/src/mcp.rs", "crates/waggledance-core/src/repository.rs"],
    "action": "dispatch_schema gains optional string `request_id` for every target, described as 'pass one on every call so a retry never sends twice'. After argument validation and the project lookup, a request_id matching a stored run of that project returns {run_id, status, duplicate: true} with no herdr or paseo call (D1). Herdr paths check before the send and store request_id with the run (sequential idempotency, D7). The leader branch keeps its gate order (orchestration, then [paseo] enabled), makes no paseo call, and inserts via insert_run_with_request a run with pane_id \"paseo:pending\", preset_label None, status \"queued\", caller = PASEO_AGENT_ID of this process when set, returning {run_id, status: \"queued\", duplicate: false, warnings}. When the delivery heartbeat file (path from one shared function in paseo.rs or config: <waggledance data dir>/paseo-delivery.heartbeat) is missing or older than 30 s, warnings include \"paused — waggledance daemon not running (start it with `waggledance serve`)\" (D8). The busy refusal is gone (D3). Do not change await code. No code comments.",
    "verify": "cargo test --profile fast -p waggledance dispatch",
    "must_haves": {
      "truths": ["a repeated request_id returns the first run with duplicate true and no herdr or paseo call", "a leader dispatch stores a queued run and makes no paseo call", "a stale or missing heartbeat adds the paused warning", "a leader dispatch is accepted while that leader is busy"],
      "artifacts": [{"path": "crates/waggledance/src/mcp.rs", "substantive": "request_id and queued leader dispatch with tests"}],
      "key_links": ["dispatch uses run_by_request before any send"],
      "prohibitions": ["no paseo CLI call inside waggledance_dispatch", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "ld-4",
    "title": "Deliver queued leader requests from the daemon and tell the caller when they end",
    "role": "code",
    "lane": "high-risk",
    "feature": "leader-delivery",
    "deps": ["ld-3"],
    "decisions": ["D2", "D3", "D4", "D5", "D6", "D8"],
    "files": ["crates/waggledance/src/delivery.rs", "crates/waggledance/src/main.rs", "crates/waggledance/src/reaper.rs", "crates/waggledance/src/orchestrate.rs"],
    "read_first": ["docs/history/leader-delivery/plan.md", "docs/history/leader-delivery/CONTEXT.md", "crates/waggledance/src/reaper.rs", "crates/waggledance/src/main.rs", "crates/waggledance/src/orchestrate.rs", "crates/waggledance/src/paseo.rs"],
    "action": "Add crates/waggledance/src/delivery.rs, a daemon task started from the same reconcile path as the reaper whenever config.paseo.enabled (independent of terminal.enabled), over `Arc<dyn Paseo>`. Every 2 s tick: write the heartbeat file (ld-3's path); on the first tick run restart recovery — working runs still at paseo:pending go back to queued; working or blocked runs with an agent become lost with transcript 'daemon restarted — the leader may still be finishing; check its chat' (notice sent) and their project is held. Per project at most one delivery, holding the per-project lock file for the whole delivery. A project is held while its leader's inspect status is running or initializing, or while it was marked held, until `wait`/inspect shows idle (D5, D6). For an eligible project, claim its oldest queued run and deliver: resolve_leader; Existing -> send_and_wait; Start -> run_and_wait with the task (no bootstrap). Set pane paseo:<agent-id> as soon as the id is known. TurnEnd::Completed -> done, transcript = the leader text after the last `[User] <task first line>` in logs. TurnEnd::Permission -> blocked (notice), then poll inspect every 10 s until no pending permission, then wait for idle -> done as above. Timeout (config.paseo.turn_timeout_secs) or Other -> lost 'the leader may still be finishing' and hold the project. Failures before the send (resolve error, binary missing, Paseo unreachable) -> requeue_run with backoff (10 s, 30 s, 1 min, 5 min), and on the 5th -> lost with the error (notice). Await on a paseo: run reads the ledger only: queued -> status queued with age_secs, working/blocked as stored, terminal -> stored transcript; remove observe_paseo_run's log-text completion and the reaper's Paseo sweep and wiring (herdr sweep unchanged). Notices: done, lost and blocked go through orchestrate's existing notify path without closing anything, and when the run's caller is set and is not the leader, one `send_no_wait` to the caller — only while the caller's inspect status is idle (D5), retried each tick for up to 1 h, best-effort and logged on failure, after the status write. Wording: '[waggledance] <project> leader finished run <run_id> (<status>) — informational, no action needed; read it with waggledance_await {\"run_id\": \"<run_id>\"}.' and for blocked '[waggledance] <project> leader is waiting for a permission in its Paseo chat (run <run_id>) — approve or deny it there.' Declare the module in main.rs. FakePaseo tests: in-order delivery, one at a time per project, two projects in parallel, no send while the leader is running, completed -> done, permission -> blocked -> done, timeout -> lost and project held, pre-send failure requeues then lost after 5, restart recovery (pending -> queued, sent -> lost), caller notice once and only when idle, no self-notice, await reports queued with age. No code comments.",
    "verify": "cargo test --profile fast -p waggledance delivery && cargo test --profile fast -p waggledance reaper && cargo test --profile fast -p waggledance await",
    "must_haves": {
      "truths": ["queued runs of one project are delivered one at a time in order", "nothing is sent to a leader or caller whose status is running", "a run is done only after a Completed turn end or a permission followed by idle", "a pre-send failure requeues and becomes lost only after 5 attempts", "a restart requeues never-sent runs and marks sent ones lost without re-sending", "the caller gets one notice per terminal or blocked state, only while idle"],
      "artifacts": [{"path": "crates/waggledance/src/delivery.rs", "substantive": "delivery loop with FakePaseo tests"}],
      "key_links": ["main.rs starts the delivery task when [paseo] enabled", "reaper no longer sweeps paseo runs", "await reads the ledger for paseo runs"],
      "prohibitions": ["no re-send of a sent run", "no send to a running agent", "no stop/archive of a leader", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "ld-5",
    "title": "Retry a request from the Paseo panel without sending it twice and watch its status",
    "role": "code",
    "lane": "high-risk",
    "feature": "leader-delivery",
    "deps": ["ld-3"],
    "decisions": ["D1", "D3", "D8"],
    "files": ["integrations/paseo/server/dispatch.ts", "integrations/paseo/shared/rpc.ts", "integrations/paseo/index.server.ts", "integrations/paseo/client/composer.tsx", "integrations/paseo/test/server.test.ts", "crates/waggledance/src/setup.rs"],
    "read_first": ["integrations/paseo/server/dispatch.ts", "integrations/paseo/client/composer.tsx", "integrations/paseo/test/server.test.ts", "crates/waggledance/src/mcp.rs"],
    "action": "The send RPC takes request_id and passes it in the waggledance_dispatch arguments ({project, task, leader: true, request_id}) and returns status, duplicate and warnings. Add an RPC `waggledance.run_status` {run_id} that calls waggledance_await with timeout_seconds 0 through the same stdio MCP client and returns {status, age_secs?}. The composer creates one request_id per draft (crypto.randomUUID), keeps it across retries, replaces it after a success or when the text changes. Success shows 'Queued for <project> leader (run <id>)' or 'Already sent (run <id>)' on duplicate, plus any warning such as the paused text. After a success the composer polls run_status every 10 s and shows queued / working / blocked (\"waiting for a permission in the leader's chat\") / done / lost, stopping at a terminal state. The timeout text says the request may already be queued and retrying is safe. Tests: request_id forwarded, reused on retry, duplicate wording, run_status mapping. If a plugin file is added or removed, update the embedded table in crates/waggledance/src/setup.rs. No code comments.",
    "verify": "cd integrations/paseo && node --test 'test/**/*.test.ts' && npx tsc --noEmit && cd ../.. && cargo test --profile fast -p waggledance setup",
    "must_haves": {
      "truths": ["the tool call carries the draft's request_id", "a retry of the same draft reuses the request_id", "a duplicate answer is shown as already sent", "the panel shows the run's status until it ends"],
      "artifacts": [{"path": "integrations/paseo/client/composer.tsx", "substantive": "per-draft request id and status polling"}],
      "key_links": ["server/dispatch.ts forwards request_id"],
      "prohibitions": ["no HTTP write", "no code comments"]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "ld-6",
    "title": "Document queued leader delivery and the runtime-capture lesson",
    "role": "docs",
    "lane": "high-risk",
    "feature": "leader-delivery",
    "deps": ["ld-3"],
    "decisions": ["D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8"],
    "files": ["docs/waggledance-supervisor-skill-template.md", "docs/knowledge/patterns/paseo-leader-dispatch.md", "docs/knowledge/patterns/*", "README.md"],
    "read_first": ["docs/waggledance-supervisor-skill-template.md", "docs/knowledge/patterns/paseo-leader-dispatch.md", "docs/knowledge/patterns/existence-is-not-evidence.md", "docs/history/leader-delivery/CONTEXT.md", "crates/waggledance/src/doctor.rs"],
    "action": "Supervisor template leader mode: always pass request_id equal to the correlation id; a busy leader no longer refuses — the run starts queued; a 'paused — daemon not running' warning means start `waggledance serve`; the seat may wait for the leader notice instead of looping on await. Update the paseo-leader-dispatch concept: queued status with age, delivery by the daemon, never sending into a running agent, done = the Paseo turn completed (or permission then idle), restart and timeout semantics, caller notices and their wording, request_id (exact on leader, sequential on herdr), binary order (config, an existing $PASEO_CLI, paseo) and why; cite leader-delivery D1/D2/D3/D4/D5/D6/D7/D8; known limit: a human typing to the leader during a delivery interrupts that turn. Record the runtime-capture lesson (piggery rule 9; the stdout noise, ignored --since and send-interrupts findings; fixtures in crates/waggledance/testdata/fixtures/paseo-0.11.2/) by extending an existing pattern in docs/knowledge/patterns/ when one covers it, or as a new pattern with a one-line reason it is a separate fact; run `bee knowledge index` if a pattern file is added. Update README's Paseo section in one or two lines. Keep doctor template assertions green.",
    "verify": "cargo test --profile fast -p waggledance doctor",
    "must_haves": {
      "truths": ["the template passes request_id and treats queued as normal", "the concept states done = Paseo turn completed and that nothing is sent into a running agent"],
      "artifacts": [{"path": "docs/knowledge/patterns/paseo-leader-dispatch.md", "substantive": "updated delivery semantics citing leader-delivery D1-D8"}],
      "key_links": ["the concept cites leader-delivery D1/D2/D3/D4/D5/D6/D7/D8"],
      "prohibitions": ["no change to the spec-drop line format"]
    },
    "affects_skills": [],
    "affects_specs": ["docs/knowledge/patterns/paseo-leader-dispatch.md"]
  }
]
```

## Test matrix

| Dimension | Probe | Pass when |
|---|---|---|
| Idempotency | dispatch twice with one request_id | same run_id, `duplicate: true`, zero sends |
| Idempotency | same request_id while still queued | same run_id |
| Idempotency | same request_id, other project | a new run |
| Queue | three requests, one project | delivered in creation order, one at a time |
| Queue | two projects | delivered in parallel |
| Busy | leader status running (human turn) | nothing sent until idle |
| Completion | `Completed` | `done`, transcript = reply after the task line |
| Completion | `Permission` | `blocked` + notice; cleared then idle → `done` |
| Completion | timeout / `Other` | `lost` "may still be finishing", project held until idle |
| Pre-send failure | resolve or binary error | requeued with attempts+1; 5th → `lost` + notice |
| Ambiguous workspace | two root workspaces | requeued then `lost` with the refusal text |
| Restart | working at `paseo:pending` | back to `queued` |
| Restart | working with an agent | `lost` + notice, never re-sent |
| Push | caller set and idle | one notice per state |
| Push | caller running | notice waits until idle |
| Push | caller is the leader | no notice |
| Liveness | heartbeat stale | dispatch warning "paused — waggledance daemon not running" |
| Await | queued run | `status: queued` with age |
| Binary | config empty, PASEO_CLI set and exists | PASEO_CLI executed |
| Binary | PASEO_CLI set but missing | falls back to `paseo` |
| Env | child process | no PASEO_AGENT_ID, PASEO_HOME kept |
| Parsing | both wrapper fixtures | JSON parsed |
| Migration | v4 DB | v5, rows kept |
| Plugin | retry same draft | same request_id; status shown until terminal |
| Regression | herdr dispatch / await / reaper suites | green |

## Open Questions

(none)

## Out of scope

Stopping or archiving a leader; images in a request; coordinating a human who types to the leader during a delivery (documented as a known limit).
