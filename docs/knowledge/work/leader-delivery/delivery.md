---
type: bee.delivery
title: leader-delivery — delivery
description: "Delivery record proposed by bee knowledge promote for work item leader-delivery: 6 capped cell(s), 18 recorded deviation(s)."
timestamp: 2026-10-14
bee:
  id: leader-delivery-delivery
  lifecycle: active
  required_context: [docs/history/leader-delivery/CONTEXT.md, docs/history/leader-delivery/plan.md]
  sources: [docs/history/leader-delivery/CONTEXT.md, docs/history/leader-delivery/plan.md, .bee/cells/archive/leader-delivery/ld-1.json, .bee/cells/archive/leader-delivery/ld-2.json, .bee/cells/archive/leader-delivery/ld-3.json, .bee/cells/archive/leader-delivery/ld-4.json, .bee/cells/archive/leader-delivery/ld-5.json, .bee/cells/archive/leader-delivery/ld-6.json]
---

# leader-delivery — Delivery

## What shipped

- **ld-1** — Paseo client resolves its binary per call, scrubs PASEO_AGENT_*, parses past startup noise, and reports turn ends; 0.11.2 fixtures committed (3 file(s) changed)
- **ld-2** — Migration 5 adds request_id, caller, attempts and a unique per-project request index; store and engine gain the queue/request functions (2 file(s) changed)
- **ld-3** — Leader dispatch stores a queued run with no Paseo call; request_id dedupes on every target; stale heartbeat warns paused (4 file(s) changed)
- **ld-4** — Daemon delivers queued leader runs one at a time per project, turn-tied done, caller notices, ledger-only await (6 file(s) changed)
- **ld-5** — Paseo panel forwards a per-draft request_id, shows queued/already-sent with warnings, and polls run_status every 10 s until done or lost (5 file(s) changed)
- **ld-6** — Supervisor template, paseo-leader-dispatch concept, README and the runtime-capture lesson describe queued, turn-tied leader delivery (5 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **ld-1** — `cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance-core config` — 31 paseo-filtered tests (incl. mcp/orchestrate/reaper leader tests) and 27 config tests cover every touched function and fixture; clippy -D warnings not clean: send_and_wait and wait are unused in th…
- **ld-2** — `cargo test --profile fast -p waggledance-core runs && cargo test --profile fast -p waggledance-core migrat` — runs and migration tests cover the new columns, idempotent insert, single-winner claim, requeue and reaper exclusion (10 + 9 passed; rest of the core suite not run)
- **ld-3** — `cargo test --profile fast -p waggledance dispatch` — 36 passed, covers the mcp dispatch tests touched (leader queue, request_id repeat on leader and herdr, heartbeat warning); also ran cargo test -p waggledance-core runs_with_one_request_id (1 passed) …
- **ld-4** — `cargo test --profile fast -p waggledance delivery && cargo test --profile fast -p waggledance reaper && cargo test --profile fast -p waggledance await` — FakePaseo delivery suite, reaper and await suites cover the touched modules; also ran the whole waggledance crate (1175 passed, 1 ignored), clippy -D warnings and fmt --check clean; no live Paseo run
- **ld-5** — `cd integrations/paseo && node --test 'test/**/*.test.ts' && npx tsc --noEmit && cd ../.. && cargo test --profile fast -p waggledance setup` — 15/15 plugin tests, tsc clean, setup 9 passed; covers the touched plugin files, composer polling not driven live
- **ld-6** — `cargo test --profile fast -p waggledance doctor` — doctor holds the supervisor template assertions (28 passed); concept and README are prose with no test

## Deviations

- **ld-1** — the no-wait send keeps its existing trait name send instead of send_no_wait — renaming would force edits to mcp.rs, outside this cell, which ld-3 rewrites — found a better route
- **ld-1** — Start::run stays as a thin wrapper over run_and_wait (with the new Start::spec) and AgentState keeps its pending_permissions count beside the new permissions list — mcp.rs, orchestrate.rs and reaper.rs still call these and are outside this cell; ld-3/ld-4 can drop them — hit an unforeseen obstacle
- **ld-1** — PaseoConfig::binary() removed so binary resolution lives in one function, paseo::resolve_binary — the cell asks for one function — found a better route
- **ld-1** — wrapper-systemd-ls.err starts with [ so the document rule finds it; the test pins that it does not parse as a list (stderr stays discarded) — the plan was wrong about a fact
- **ld-2** — insert_run now delegates to insert_run_with_request and the three columns are also added to SCHEMA for fresh databases — follows the migration_3/4 pattern and removes a duplicated INSERT — found a better route
- **ld-3** — added Engine/SqliteStore::set_run_request in waggledance-core (reserved) — the herdr run is inserted inside orchestrate::dispatch_run, which this cell may not touch, so the request id is stamped after the send — hit an unforeseen obstacle
- **ld-3** — heartbeat path is paseo::delivery_heartbeat_path(data_dir) in paseo.rs (reserved) — single home ld-4 must write — followed the leader note
- **ld-3** — added #![allow(dead_code)] at the top of paseo.rs — removing the leader send path left resolve_leader, Start, LeaderLock, TurnEnd, RunSpec etc. unused and clippy -D warnings red until ld-4 consumes them; ld-4 should delete the line — something else had to be fixed first
- **ld-4** — resolve_leader no longer takes the leader lock; delivery acquires LeaderLock for the whole delivery — taking the same flock twice in one process would deadlock — found a better route
- **ld-4** — await on a paseo run returns a new RunStatus::Queued (non-terminal) and mcp handle_await adds age_secs and the paused warning (handle_await gained a data_dir param) — D8 says await reports paused too — found a better route
- **ld-4** — a send failure other than Spawn/PromptFile is treated as post-send (lost, project held) since the send may have reached Paseo — no re-send of a sent run — found a better route
- **ld-4** — caller notices are batched per caller per tick with a 30 s quiet gap after a send, because Paseo inspect still reads idle for seconds after a send (probe) — found a better route
- **ld-4** — FakePaseo run_and_wait now registers the new leader idle, matching a completed first turn — the plan was wrong about a fact
- **ld-5** — the composer falls back to a time+random id when crypto.randomUUID is missing — the Paseo client may run on a JS engine without it — found a better route
- **ld-5** — the sent and status wording live in shared/rpc.ts (describeSent, describeRun) so tests can check them — the composer itself cannot be run under node --test — found a better route
- **ld-5** — an await status of timeout is shown as working — a zero-second await on a live run means nothing has finished yet — something else had to be fixed first
- **ld-6** — the ambiguous-workspace refusal bullet in the template now says the run ends lost with the refusal text — the refusal moved from dispatch to delivery under D6, so the old wording was false — something else had to be fixed first
- **ld-6** — the runtime-capture lesson extends existence-is-not-evidence as a Recurrence entry, so no new pattern file; the index was regenerated only because the concept description changed — followed the plan

## Provenance

Proposed by `bee knowledge promote --work leader-delivery` from 6 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/leader-delivery/CONTEXT.md`, `docs/history/leader-delivery/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.
