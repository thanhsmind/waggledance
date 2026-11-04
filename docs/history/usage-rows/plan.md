---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Usage Rows

## Summary

The daemon gets `GET /api/usage/rows`, a flat per-day, per-project,
per-session usage feed that agent-works reads for its two-team Usage page. The
transcript reader stops folding token kinds together, keeps the cost a
transcript records, counts messages, tool calls and errors, and reads opencode's
sqlite store. Herdr-spawned runs remember their agent's session id so a row can
name the preset that started it. The Usage tab and `/api/usage` stay as they are.

Mode: `standard` — 2 risk flags: public-contracts (a new JSON route a sibling
repo builds against), data-migration (one nullable `runs` column).
Why this is the least workflow that protects the work: two independent reader
and store cells in parallel, one surface cell that joins them and owns the
route's contract tests, one docs cell; the live curl against the real daemon is
the whole-path proof.

## Requirements (from CONTEXT.md)

- D1: `UsageEvent` keeps input/output/cache_read/cache_write and the
  transcript's own cost parts; `new`/`cached` become derived.
- D2: exact response shape and field names.
- D3: Asia/Saigon days, inclusive window, 30-day default, 400 `{error}`.
- D4: registered project or `other`; worktrees fold.
- D5: agent = preset label else agent kind; provider; bare model.
- D6: session / parent / preset / runTitle.
- D7: cost null unless recorded; `costSource: "transcript"`; never estimated.
- D8: messages / toolCalls / errors / tokens.
- D9: `runs.session_id` from herdr `agent_session`.
- D10: opencode sqlite reader.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | pi records cost per message, split by token kind | ran | `grep '"usage"' ~/.pi/agent/sessions/*/*.jsonl` | `"cost":{"input":0.0032954000000000004,"output":0.0002088,"cacheRead":0,"cacheWrite":0,"total":0.0035042000000000003}` |
| 2 | pi marks errored turns by stop reason | ran | `grep -o '"stopReason":"[a-zA-Z]*"'` | `160 "stopReason":"error"` |
| 3 | The reader folds token kinds today | read | `crates/waggledance-core/src/agent_usage.rs:156-160` | `new: num(u, "input_tokens") + num(u, "output_tokens") + num(u, "cache_creation_input_tokens"),` |
| 4 | herdr already reports each pane's agent session id | read | `crates/waggledance/src/herdr/wire.rs:70` | `pub session_id: Option<String>,` |
| 5 | opencode keeps sessions, messages and parts in sqlite with JSON `data` | ran | `sqlite3 -readonly ~/.local/share/opencode/opencode.db .schema` | `CREATE TABLE \`message\` (… \`session_id\` text NOT NULL, … \`data\` text NOT NULL` |
| 6 | The dev host's opencode store is empty | ran | `select count(*) from message` | `0` |
| 7 | `runs` grows by `ALTER TABLE … ADD COLUMN` migrations | read | `crates/waggledance-core/src/repository.rs:1079` | `conn.execute("ALTER TABLE runs ADD COLUMN feature TEXT", [])?;` |

## Work breakdown

| Cell | Role | Files | Deps |
|------|------|-------|------|
| ur-1 core reader + row builder | test | `crates/waggledance-core/src/agent_usage.rs`, `crates/waggledance-core/src/usage.rs`, `crates/waggledance-core/src/usage_rows.rs` (new), `crates/waggledance-core/src/lib.rs` | — |
| ur-2 run session id | test | `crates/waggledance-core/src/repository.rs`, `crates/waggledance-core/src/engine.rs`, `crates/waggledance/src/orchestrate.rs` | — |
| ur-3 `/api/usage/rows` route | test | `crates/waggledance/src/server.rs` | ur-1, ur-2 |
| ur-4 knowledge | docs | `docs/knowledge/work/homepage-tabs/usage-tab.md`, `docs/knowledge/index.md` (regenerated) | ur-3 |

ur-1 and ur-2 touch disjoint files and run in parallel.

### ur-1 contract

- `UsageEvent { ts_ms, provider, model, input, output, cache_read, cache_write, cost: Option<Cost>, tool_calls: u32, error: bool }`,
  `Cost { input, output, cache_read, cache_write, total }` (f64), with methods
  `new_tokens()` / `cached_tokens()`; `usage.rs` uses them so the Usage tab
  numbers are unchanged (its existing tests stay green unmodified in value).
- Claude: input=`input_tokens`, output=`output_tokens`,
  cache_write=`cache_creation_input_tokens`, cache_read=`cache_read_input_tokens`,
  provider `anthropic`, tool_calls = `tool_use` items in `message.content`,
  error = `isApiErrorMessage == true`. Codex: input = `input_tokens - cached`,
  cache_read = `cached_input_tokens`, cache_write = `cache_write_input_tokens`,
  provider `openai`; `function_call`/`custom_tool_call` response items add to the
  next token event; an `event_msg` of type `error` is a zero-token error event.
  pi: fields `input/output/cacheRead/cacheWrite`, `cost` object when present,
  provider = message `provider`, `toolCall` content items, `stopReason == "error"`.
- An event is kept when it has tokens or is an error.
- opencode: read-only open of `$XDG_DATA_HOME/opencode/opencode.db`
  (default `~/.local/share/opencode/opencode.db`) — a missing file is no runs.
  Each assistant `message.data` gives `providerID`, `modelID`,
  `tokens.{input,output,cache.read,cache.write}` (reasoning folds into output),
  `cost` (a total only — the parts stay null, total carried), `error`,
  `time.created`; `session` gives `directory`, `title`, `parent_id`; tool calls
  are `part` rows with `data.type == "tool"`. Runs keyed `opencode:<session id>`,
  re-read only when the db/wal mtime or size changes. `AgentHomes` gains
  `opencode: PathBuf`.
- `usage_rows::build(runs: &[(Option<String> project, Arc<AgentRun>)], presets: &HashMap<String session, RunMeta{preset, task}>, from_day, to_day) -> Vec<UsageRow>`
  grouped and summed per D2/D5-D8, days at UTC+7, ordered per CONTEXT
  discretion, serialised with the spec's camelCase names and explicit nulls.
- Proof: `cargo test -p waggledance-core` — new tests for each runtime's split
  and counts, the opencode fixture db, row grouping, null-vs-recorded cost, day
  boundary at 17:00Z.

### ur-2 contract

- Migration `ALTER TABLE runs ADD COLUMN session_id TEXT` beside the others;
  `Engine::set_run_session(run_id, session_id)` writes only when NULL;
  `Engine::run_sessions() -> HashMap<String, RunMeta{preset_label, task}>` for
  every run with a session id. `Run` struct unchanged.
- `dispatch_run`: after the send, one herdr snapshot; the agent on the run's
  pane with a `session_id` is written. The await loop writes it when the
  snapshot shows one and the row has none.
- Proof: `cargo test -p waggledance-core` and `cargo test -p waggledance orchestrate`
  with a fake-herdr test that the session id lands at dispatch and at a later poll.

### ur-3 contract

- Route `GET /api/usage/rows`; query `from`/`to` per D3; 400 `{"error": …}`.
- Builds roots from every registered project (D4), runs from
  `AgentLogCache::runs(homes, since = from 00:00 UTC+7)`, presets from
  `Engine::run_sessions()`, then `usage_rows::build`; response per D2 with
  `generatedAt` RFC3339 UTC.
- Proof: `cargo test -p waggledance usage_rows` — default window, explicit
  window, bad date 400, from>to 400, fixture transcript producing one row with
  every field, preset-labelled row via a seeded run session.

### ur-4 contract

- `usage-tab.md` gains the rows feed: route, fields, cost rule, session column;
  `bee knowledge index` regenerates the index.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "094d64e24e68c0b6fabaed19ae6619f3ac5e32bac9e9da2ab51728125c724d9b",
  "stages": [
    {
      "stage": "plan",
      "classification": "required",
      "role": "plan",
      "reason": "this plan, drafted by the leader"
    },
    {
      "stage": "hat-wave-facts-gaps",
      "classification": "not-applicable",
      "role": "hat-facts-gaps",
      "reason": "owner-approved field-level spec; skip logged"
    },
    {
      "stage": "hat-wave-alternatives",
      "classification": "not-applicable",
      "role": "hat-alternatives",
      "reason": "SMALLER PATH checked inline"
    },
    {
      "stage": "hat-wave-user-impact",
      "classification": "not-applicable",
      "role": "hat-user-impact",
      "reason": "additive route, Usage tab unchanged"
    },
    {
      "stage": "hat-wave-risks",
      "classification": "not-applicable",
      "role": "hat-risks",
      "reason": "not high-risk"
    },
    {
      "stage": "hat-wave-value",
      "classification": "not-applicable",
      "role": "hat-value",
      "reason": "not high-risk"
    },
    {
      "stage": "advisor",
      "classification": "not-applicable",
      "role": "advisor",
      "reason": "standard lane"
    },
    {
      "stage": "test",
      "classification": "required",
      "role": "test",
      "reason": "ur-1..ur-3 Rust code plus tests on the native Rust-and-tests seat, precedent a66a46c0"
    },
    {
      "stage": "docs",
      "classification": "required",
      "role": "docs",
      "reason": "ur-4 knowledge sync"
    },
    {
      "stage": "code",
      "classification": "not-applicable",
      "role": "code",
      "reason": "Rust cells run on the test seat (a66a46c0)"
    },
    {
      "stage": "read",
      "classification": "conditional",
      "role": "read",
      "reason": "gathers only",
      "condition": "a worker needs a multi-file lookup"
    },
    {
      "stage": "extraction",
      "classification": "conditional",
      "role": "extraction",
      "reason": "cheap reader",
      "condition": "a single-fact lookup is delegated"
    },
    {
      "stage": "generation",
      "classification": "not-applicable",
      "role": "generation",
      "reason": "every cell names its role"
    },
    {
      "stage": "review",
      "classification": "conditional",
      "role": "review",
      "reason": "agents-review-user-invoked",
      "condition": "the user invokes bee-reviewing"
    },
    {
      "stage": "deploy",
      "classification": "not-applicable",
      "role": "deploy",
      "reason": "no release"
    },
    {
      "stage": "supervisor",
      "classification": "not-applicable",
      "role": "supervisor",
      "reason": "not used"
    },
    {
      "stage": "design",
      "classification": "not-applicable",
      "role": "design",
      "reason": "no UI"
    },
    {
      "stage": "lane-1",
      "classification": "not-applicable",
      "role": "lane-1",
      "reason": "one design"
    },
    {
      "stage": "lane-2",
      "classification": "not-applicable",
      "role": "lane-2",
      "reason": "no blind lanes"
    },
    {
      "stage": "lane-3",
      "classification": "not-applicable",
      "role": "lane-3",
      "reason": "no blind lanes"
    }
  ]
}
```

## Hat wave

Skipped, recorded as a decision: the spec is owner-approved field-for-field, the
change is a read-only additive route plus one nullable column, and the
SMALLER PATH check is inline above (no cheaper shape keeps the field contract).

## Revisions

- rev 1: cell verify fields become the bare commands the cap proof quotes (rev 0 carried prose the cap door cannot match).

## Proof at close

- `cargo test -p waggledance-core` and `cargo test -p waggledance` green.
- `cargo build --profile fast -p waggledance`, run the built daemon on a spare
  port, `curl '127.0.0.1:<port>/api/usage/rows?from=…&to=…'` against real
  transcripts, and a bad-date curl showing 400.
- The repo's recorded `commands.test` targets `packages/bee-rs`, which this repo
  does not have; the cargo commands above are the real suite (named gap).

## Cells — current slice (preview)

| id | title | files | deps | proof |
|---|---|---|---|---|
| ur-1 | Split token kinds and costs in the transcript reader, add the opencode reader and the row builder | `crates/waggledance-core/src/agent_usage.rs`, `crates/waggledance-core/src/usage.rs`, `crates/waggledance-core/src/usage_rows.rs`, `crates/waggledance-core/src/lib.rs` | — | cargo test -p waggledance-core |
| ur-2 | Remember the transcript session id on herdr-spawned runs | `crates/waggledance-core/src/repository.rs`, `crates/waggledance-core/src/engine.rs`, `crates/waggledance/src/orchestrate.rs` | — | cargo test -p waggledance-core -p waggledance |
| ur-3 | Serve GET /api/usage/rows | `crates/waggledance/src/server.rs` | ur-1, ur-2 | cargo test -p waggledance-core -p waggledance |
| ur-4 | Document the usage rows feed | `docs/knowledge/work/homepage-tabs/usage-tab.md`, `docs/knowledge/index.md` | ur-3 | bee knowledge check |

```json
[
 {
  "id": "ur-1",
  "feature": "usage-rows",
  "title": "Split token kinds and costs in the transcript reader, add the opencode reader and the row builder",
  "lane": "standard",
  "role": "test",
  "files": [
   "crates/waggledance-core/src/agent_usage.rs",
   "crates/waggledance-core/src/usage.rs",
   "crates/waggledance-core/src/usage_rows.rs",
   "crates/waggledance-core/src/lib.rs"
  ],
  "read_first": [
   "docs/history/usage-rows/CONTEXT.md",
   "docs/history/usage-rows/plan.md",
   "docs/knowledge/work/homepage-tabs/usage-tab.md"
  ],
  "affects_skills": [],
  "affects_specs": [],
  "decisions": [],
  "action": "Implement the ur-1 contract in docs/history/usage-rows/plan.md section 'ur-1 contract' exactly: UsageEvent split (input/output/cache_read/cache_write, provider, cost Option<Cost>, tool_calls, error) with new_tokens()/cached_tokens() so usage.rs and the Usage tab keep identical numbers; per-runtime mapping for Claude, Codex, pi; opencode sqlite reader (read-only, missing db = no runs, AgentHomes.opencode); new module usage_rows with build() producing UsageRow serialised with the spec field names of CONTEXT D2 (camelCase, explicit nulls), days at fixed UTC+7, grouped by (day, project, agent, provider, model, session) and summed. No code comments (repo rule).",
  "must_haves": {
   "truths": [
    "Usage tab totals unchanged: existing usage.rs and agent_usage.rs tests pass with unchanged expected values",
    "pi rows carry transcript cost parts; Claude/Codex rows carry null cost",
    "opencode fixture db yields runs with tokens, cost total, tool calls and errors",
    "A message at 2026-11-03T17:00:00Z lands on day 2026-11-04"
   ],
   "artifacts": [
    "crates/waggledance-core/src/usage_rows.rs"
   ]
  },
  "verify": "cargo test -p waggledance-core",
  "deps": [],
  "status": "open"
 },
 {
  "id": "ur-2",
  "feature": "usage-rows",
  "title": "Remember the transcript session id on herdr-spawned runs",
  "lane": "standard",
  "role": "test",
  "files": [
   "crates/waggledance-core/src/repository.rs",
   "crates/waggledance-core/src/engine.rs",
   "crates/waggledance/src/orchestrate.rs"
  ],
  "read_first": [
   "docs/history/usage-rows/CONTEXT.md",
   "docs/history/usage-rows/plan.md"
  ],
  "affects_skills": [],
  "affects_specs": [],
  "decisions": [],
  "action": "Implement the ur-2 contract in docs/history/usage-rows/plan.md: nullable runs.session_id column via the existing ALTER TABLE migration pattern (fresh-schema CREATE TABLE too), Engine::set_run_session (write only when NULL), Engine::run_sessions() returning session -> RunMeta{preset_label: Option<String>, task: String} defined public in engine.rs or repository.rs, dispatch_run writes it from a post-send herdr snapshot, the await loop backfills it. Run struct unchanged. No code comments.",
  "must_haves": {
   "truths": [
    "A spawned dispatch whose pane reports agent_session stores it",
    "An await poll fills a still-empty session id",
    "Existing DBs migrate without loss"
   ],
   "artifacts": []
  },
  "verify": "cargo test -p waggledance-core -p waggledance",
  "deps": [],
  "status": "open"
 },
 {
  "id": "ur-3",
  "feature": "usage-rows",
  "title": "Serve GET /api/usage/rows",
  "lane": "standard",
  "role": "test",
  "files": [
   "crates/waggledance/src/server.rs"
  ],
  "read_first": [
   "docs/history/usage-rows/CONTEXT.md",
   "docs/history/usage-rows/plan.md"
  ],
  "affects_skills": [],
  "affects_specs": [],
  "decisions": [],
  "action": "Implement the ur-3 contract in docs/history/usage-rows/plan.md: route, D3 window parsing with 400 {error}, every registered project as roots (D4), projects list plus other, presets from Engine::run_sessions(), usage_rows::build, response per D2 with generatedAt. Contract tests in server.rs test module. No code comments.",
  "must_haves": {
   "truths": [
    "Default window is the 30 days ending today in UTC+7",
    "Bad date and from>to answer 400 with an error field",
    "A fixture transcript yields a row carrying every D2 field",
    "A run with a stored session id makes agent and preset its preset label"
   ],
   "artifacts": []
  },
  "verify": "cargo test -p waggledance-core -p waggledance",
  "deps": [
   "ur-1",
   "ur-2"
  ],
  "status": "open"
 },
 {
  "id": "ur-4",
  "feature": "usage-rows",
  "title": "Document the usage rows feed",
  "lane": "standard",
  "role": "docs",
  "files": [
   "docs/knowledge/work/homepage-tabs/usage-tab.md",
   "docs/knowledge/index.md"
  ],
  "read_first": [
   "docs/history/usage-rows/CONTEXT.md",
   "crates/waggledance/src/server.rs"
  ],
  "affects_skills": [],
  "affects_specs": [
   "docs/knowledge/work/homepage-tabs/usage-tab.md"
  ],
  "decisions": [],
  "action": "Extend usage-tab.md with the /api/usage/rows feed: route, window rule, every field and its source, the cost rule, runs.session_id, opencode source; Pointers name the files. Regenerate the index with bee knowledge index.",
  "must_haves": {
   "truths": [
    "usage-tab.md describes every D2 field"
   ],
   "artifacts": [
    "docs/knowledge/work/homepage-tabs/usage-tab.md"
   ]
  },
  "verify": "bee knowledge check",
  "deps": [
   "ur-3"
  ],
  "status": "open"
 }
]
```
