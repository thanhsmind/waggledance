---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Leader delegated work (hold, live children, late replies)

## Summary

A leader that hands work to a background subagent, a herdr worker or a Paseo child and ends its turn is no longer treated as finished. The leader says so with `waggledance hold`, and waggledance also sees Paseo children on its own. While the wait is live, the run stays working, the leader is not archived and the heartbeat does not nudge it. The wait ends when the leader mails a result, releases it, or after 2 hours. A reply the leader gives after its run already ended is passed on to the caller instead of being dropped.

Mode: `standard` — 2 risk flags: changes behavior an existing test asserts (turn end ends a run done), data model (a new migration).
Why this is the least workflow that protects the work: the change rewires when a run ends, which every caller reads, so it gets a plan and per-cell proof, but it touches no auth or external provider contract.

## Requirements (from CONTEXT.md)

- D1: the leader declares a wait with `waggledance hold` (reason, optional duration) and can release it; a live Paseo child of the leader counts as held work; no timeline inference.
- D2: a hold lasts at most 2 hours by default, configurable; on expiry the old behavior resumes and the caller is told the hold expired.
- D3: while held, the run stays `working` with a note naming the wait; turn end does not end it done, the idle sweep does not archive, the heartbeat does not nudge.
- D4: the hold clears on a `result` mail from that leader or on release.
- D5: a leader reply after its run ended `done` or `lost` reaches the caller as a late-result message.

## Load-bearing claims

Labels: `read` (opened at that line) or `ran` (executed). Evidence is a verbatim substring of the anchor.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | A Claude leader on Paseo reads idle while its background subagent still runs | ran | `paseo inspect d1025a15 --json` at 09:56:26 UTC, 2026-11-11 | `"Status": "idle",` |
| 2 | The same leader woke on its own after the subagent and gave the real answer | ran | `paseo logs d1025a15` at 09:59:34 UTC | `FINAL-AFTER-BG` |
| 3 | Paseo exposes the parent of an agent | ran | `paseo inspect d1025a15 --json` | `"ParentAgentId": "27bea2ad-24d0-49bf-8a57-b7769389fd8a"` |
| 4 | Every Paseo agent has its own id in its environment | ran | `env \| grep -i paseo` inside a Paseo agent | `PASEO_AGENT_ID=27bea2ad-24d0-49bf-8a57-b7769389fd8a` |
| 5 | A turn end ends the run done today | read | `crates/waggledance/src/delivery.rs:674` | `Ok(TurnEnd::Completed) => return self.done(name, run, Some(agent), sent).await,` |
| 6 | The idle sweep archives on idle time alone once the family is closed | read | `crates/waggledance/src/delivery.rs:245-246` | `let limit = self.idle_limit(&carried); / let Some(idle) = idle_for(&state, now, limit) else {` |
| 7 | The heartbeat nudges an idle, silent leader | read | `crates/waggledance/src/heartbeat.rs:195-196` | `if idle(state) && silent(task) { / self.nudge(task, leader).await;` |
| 8 | The last migration is 11 | read | `crates/waggledance-core/src/repository.rs:26` | `(11, migration_11_runs_handoff),` |
| 9 | A leader may mail only four kinds | read | `crates/waggledance/src/mail.rs:9` | `const LEADER_KINDS: [&str; 4] = ["result", "question", "progress", "error"];` |
| 10 | The idle sweep runs every 5 minutes, beside discovery | read | `crates/waggledance/src/discovery.rs:9` | `pub const EVERY: Duration = Duration::from_secs(300);` |
| 11 | Polling status every 5 s never saw the wake turns as running | ran | Monitor `until paseo inspect d1025a15 --json \| grep -q '"Status": "running"'` | `[Monitor expired after 5m with no events delivered.` |

## Discovery

Extends CONTEXT.md's read set with `crates/waggledance/src/delivery.rs` 185-262 (tick, sweep), 500-580 (prompt assembly), 660-716 (`follow_turn`), 924-968 (`end` and caller notice); `crates/waggledance/src/heartbeat.rs` 15-262; `crates/waggledance/src/mail.rs` 1-80; `crates/waggledance/src/paseo.rs` 160-173 (trait), 268-300 (`wd_task`, `task_body`), 960-986 (`wait`, `logs_since`). Finding: claim 11 means a wake turn must be detected by `UpdatedAt` moving, never by seeing `running`.

## Approach

Recommended path:
1. ldw-1, store and command. Migration 12 adds a `leader_holds` table (one row per leader: `leader_id` key, `why`, `set_at`, `until`) and a `runs.late_reply_at` column. `waggledance hold --why <text> [--minutes N]` sets or replaces the hold of the leader named by `PASEO_AGENT_ID`; `waggledance hold --release` clears it; with no `PASEO_AGENT_ID` it exits 2 naming the variable. `--minutes` defaults to and is capped at `[paseo] hold_max_minutes` (default 120). A `mail send --kind result` clears the hold of the task's recorded leader (D4).
2. ldw-2, the daemon honors holds. A helper decides "held" for a leader: an unexpired hold row, or a Paseo child whose `ParentAgentId` is the leader, not archived, status `running` or `initializing`. In `follow_turn`, a completed or idle turn end of a held leader keeps the run `working`, adds the wait note to the run header, and polls `inspect` every `permission_poll` until `UpdatedAt` moves (a new turn ended) or the hold ends; then it re-decides. `Paseo::wait` is not called on an idle leader. When the hold expires, the run ends as today and its transcript carries `hold expired after <N> min (<why>)` (D2). `sweep_leaders` skips a held leader; the heartbeat does not nudge one (D3). Every prompt delivered to a leader, except a steer note, ends with one standing line telling it to run `waggledance hold --why "<what it waits for>"` before ending a turn while handed-out work is still running, and to mail its result or `hold --release` when that work reports.
3. ldw-3, late replies (D5). On each sweep tick, for a non-archived leader whose newest carried run ended `done` or `lost`, has no `late_reply_at`, and whose `UpdatedAt` is later than that run's end, with no newer run sent to it: read `logs_since(run end)`. A non-empty reply is stamped `late_reply_at`, appended to the run's transcript under a `[waggledance] late reply` line, and passed to the caller: an `openclaw:` caller gets a `progress` mail headed `late reply` unless the task already got a mail after the run's end; any other caller gets a run notice naming the late reply.
4. ldw-4, writer. Knowledge consolidation and `delivery.md`.

Rejected alternatives:
- Inferring delegated work from the provider timeline: provider-specific, misses herdr workers (D1).
- A new run status `waiting`: every caller would have to learn it (D3).

Risk map:

| Component | Risk | Lands in | Proof needed |
|---|---|---|---|
| `follow_turn` wait loop | MEDIUM: a busy loop or a run that never ends | ldw-2 | Test: a held leader with a fake that stays idle polls at `permission_poll` and ends when the hold expires (claims 5, 11) |
| Migration 12 | LOW | ldw-1 | Migration round-trip test |
| Late reply duplicates a wd result mail | MEDIUM | ldw-3 | Test: a task with a mail after the run's end gets no late-reply mail |

Waves: ldw-1 first (ldw-2 and ldw-3 call its store). ldw-2 then ldw-3, serial because both edit `delivery.rs`. ldw-4 last.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "d4e696dbde7989d6388dacce3fd7057cb3a6935b46024b79698596a46f6f41d3",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "not-applicable", "role": "hat-facts-gaps", "reason": "the gap was reproduced live; claims 1-4 and 11 are run evidence"},
    {"stage": "hat-wave-alternatives", "classification": "not-applicable", "role": "hat-alternatives", "reason": "alternatives were put to the owner and decided in D1 and D3"},
    {"stage": "hat-wave-user-impact", "classification": "not-applicable", "role": "hat-user-impact", "reason": "the user-visible change is named in D2, D3 and D5"},
    {"stage": "hat-wave-risks", "classification": "not-applicable", "role": "hat-risks", "reason": "not high-risk"},
    {"stage": "hat-wave-value", "classification": "not-applicable", "role": "hat-value", "reason": "not high-risk"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "standard lane"},
    {"stage": "test", "classification": "conditional", "role": "test", "reason": "tests ride inside each code cell", "condition": "a cell's tests need a separate pass"},
    {"stage": "docs", "classification": "required", "role": "writer", "reason": "knowledge consolidation cell ldw-4"},
    {"stage": "code", "classification": "required", "role": "code", "reason": "ldw-1, ldw-2, ldw-3"},
    {"stage": "review", "classification": "not-applicable", "role": "review", "reason": "review is user-invoked only"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "no blind lanes"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no deploy in this feature"},
    {"stage": "design", "classification": "not-applicable", "role": "design", "reason": "no rendered UI"},
    {"stage": "docs-role", "classification": "not-applicable", "role": "docs", "reason": "the writer role owns ldw-4"},
    {"stage": "extraction", "classification": "not-applicable", "role": "extraction", "reason": "no narrow lookup planned"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "code cells use the code role"},
    {"stage": "read", "classification": "not-applicable", "role": "read", "reason": "no gather planned"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "no supervision stage"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 | Hold storage and command | Everything else reads it | `waggledance hold --why x` inside a Paseo agent prints the hold | ldw-2, ldw-3 |
| 2 | Daemon honors holds and children | The reported defect | A held leader's run stays working past its turn end | ldw-3 |
| 3 | Late replies reach the caller | D5 | A reply after `done` shows on the run and reaches the caller | close |

## Proof at close

`cargo test --profile fast --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, the three commands CI runs. Whole path: a live Paseo leader that runs `waggledance hold`, starts a background subagent and ends its turn keeps its run working until it mails a result, on the rebuilt daemon confirmed by content.

## Cells — current slice (preview)

```json
[
{"id":"ldw-1","feature":"leader-delegated-work","lane":"standard","role":"code","deps":[],"affects_skills":[],"affects_specs":[],"title":"Store leader holds and add the waggledance hold command","decisions":["D1","D2","D4"],"files":["crates/waggledance-core/src/repository.rs","crates/waggledance-core/src/engine.rs","crates/waggledance-core/src/config.rs","crates/waggledance/src/cli.rs","crates/waggledance/src/hold.rs","crates/waggledance/src/main.rs","crates/waggledance/src/mail.rs"],"read_first":["docs/history/leader-delegated-work/CONTEXT.md","docs/history/leader-delegated-work/plan.md"],"action":"Follow plan.md Approach step 1. Migration 12 adds leader_holds and runs.late_reply_at. Engine gains set_hold, release_hold, live_hold(leader, now) and the late-reply stamp. The hold command reads PASEO_AGENT_ID and exits 2 naming it when absent. hold_max_minutes lives in [paseo] with default 120. A result mail clears the hold of the task's leader.","verify":"cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance hold mail","verify_owner":"worker","must_haves":{"truths":["migration 12 creates leader_holds and runs.late_reply_at and round-trips","hold sets, replaces and releases one row per leader","--minutes above hold_max_minutes is capped at it and the default is 120","live_hold is none after until","hold without PASEO_AGENT_ID exits 2 naming the variable","a result mail clears the hold of the task's leader"],"artifacts":[],"key_links":[]}},
{"id":"ldw-2","feature":"leader-delegated-work","lane":"standard","role":"code","deps":["ldw-1"],"affects_skills":[],"affects_specs":[],"title":"Keep a held leader's run working and its leader unarchived and unnudged","decisions":["D1","D2","D3"],"files":["crates/waggledance/src/delivery.rs","crates/waggledance/src/heartbeat.rs","crates/waggledance/src/paseo.rs"],"read_first":["docs/history/leader-delegated-work/CONTEXT.md","docs/history/leader-delegated-work/plan.md"],"action":"Follow plan.md Approach step 2. Add a Paseo children lookup by ParentAgentId to the trait, the CLI client and the test fake. A held leader's completed or idle turn end keeps the run working with the wait note and polls inspect at permission_poll until UpdatedAt moves or the hold ends, never calling wait on an idle leader. Hold expiry ends the run as before with 'hold expired after N min (why)' in the transcript. sweep_leaders and the heartbeat skip a held leader. Every leader prompt except a steer note ends with the one hold line; task_body still returns the caller's task.","verify":"cargo test --profile fast -p waggledance delivery heartbeat paseo","verify_owner":"worker","must_haves":{"truths":["a held leader's turn end leaves the run working with the wait note","a held run ends done with the reply of the turn after the hold clears","an expired hold ends the run and its transcript says hold expired","a live Paseo child holds like a hold row","the idle sweep keeps a held leader","the heartbeat does not nudge a held leader","no wait call is made on an idle held leader","every non-steer leader prompt carries the hold line and task_body is unchanged"],"artifacts":[],"key_links":[]}},
{"id":"ldw-3","feature":"leader-delegated-work","lane":"standard","role":"code","deps":["ldw-2"],"affects_skills":[],"affects_specs":[],"title":"Pass a leader's late reply on to the caller","decisions":["D5"],"files":["crates/waggledance/src/delivery.rs"],"read_first":["docs/history/leader-delegated-work/CONTEXT.md","docs/history/leader-delegated-work/plan.md"],"action":"Follow plan.md Approach step 3, on the sweep tick.","verify":"cargo test --profile fast -p waggledance delivery","verify_owner":"worker","must_haves":{"truths":["a leader turn after its run ended done appends a late reply to that run once","an openclaw caller gets one progress mail headed late reply","a task that already got a mail after the run's end gets no late-reply mail","another caller gets a run notice naming the late reply","a leader that took a newer run is not read for a late reply","an empty late turn records nothing"],"artifacts":[],"key_links":[]}},
{"id":"ldw-4","feature":"leader-delegated-work","lane":"standard","role":"writer","deps":["ldw-1","ldw-2","ldw-3"],"affects_skills":[],"affects_specs":["docs/knowledge/patterns/paseo-leader-dispatch.md","docs/knowledge/patterns/wd-direct-leader.md"],"title":"Consolidate the knowledge for leader holds and late replies","decisions":["D1","D2","D3","D4","D5"],"files":["docs/knowledge/patterns/paseo-leader-dispatch.md","docs/knowledge/patterns/wd-direct-leader.md","docs/history/leader-delegated-work/delivery.md"],"read_first":["docs/history/leader-delegated-work/CONTEXT.md","docs/history/leader-delegated-work/plan.md"],"action":"Run bee knowledge consolidate --feature leader-delegated-work, then write the deduped knowledge into the two patterns (hold command, held-leader behavior, the 2-hour ceiling, late replies) and docs/history/leader-delegated-work/delivery.md.","verify":".bee/bin/bee knowledge check","verify_owner":"worker","must_haves":{"truths":["paseo-leader-dispatch names waggledance hold, hold_max_minutes and late replies","wd-direct-leader says a result mail clears the hold and the heartbeat skips a held leader","delivery.md exists"],"artifacts":[],"key_links":[]}}
]
```

## Test matrix

| Case | Kind | Pass when |
|---|---|---|
| Held leader ends its turn | happy | run status reads `working` and the header names the wait |
| Hold cleared by result mail, next turn ends | happy | run reads `done` with that turn's reply |
| Hold expires with leader still idle | edge | run ends and its transcript contains `hold expired after` |
| Live Paseo child, no hold row | edge | run reads `working` |
| Idle sweep over a held leader idle 31 min | edge | leader not archived |
| Heartbeat over a held silent idle leader | edge | no nudge sent |
| `hold` with no `PASEO_AGENT_ID` | error | exit 2 and stderr names `PASEO_AGENT_ID` |
| Late turn after `done`, wd caller with no later mail | happy | one `progress` mail headed `late reply` |
| Late turn after `done`, wd task already mailed | edge | no late-reply mail |
| Same scenario on main and head: leader ends turn with no hold | regression | run reads `done` on both |

## Open Questions

(none)

## Out of scope

- Setting Claude Code `crossSessionInbound` (separate backlog item).
- Inferring Claude background subagents from the provider timeline (D1).
