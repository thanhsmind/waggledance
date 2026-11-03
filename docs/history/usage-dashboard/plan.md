---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Usage Dashboard

## Summary

The home page gets a third tab, **Usage**. It shows how many tokens every bee
project spent over the last 1, 7, 14 or 30 days, split by model and by main
session versus subagents, with new tokens and cache-read tokens apart. Below the
totals it lists the features finished in that window: project, when it closed,
how many cells it capped, how long the agents worked on it, and its tokens by
model. A project filter narrows everything to one project. The same numbers come
out as JSON at `/api/usage`.

Mode: `standard` — 1 risk flag: public-contracts (a new, additive `/api/usage` route)
Why this is the least workflow that protects the work: story-sized new surface over
seven product files; one reader cell, one surface cell, one docs cell, then a live look
at the real daemon.

## Requirements (from CONTEXT.md)

- D1: spend is bucketed by session `ended_ms`; a session ending inside the window counts in full; unclosed features show no spend.
- D2: a new home tab **Usage** beside Board and Agents, over every registered bee project, with a per-project filter.
- D3: tokens only, split into new (`new`) and cache-read (`cached`), per model; no USD.
- D4: windows 1, 7, 14, 30 days back from now; day buckets in the daemon's local time.
- D5: main-session tokens (`models`) and subagent tokens (`subagent_models`) shown separately per model.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command run, output kept).

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | Usage files carry per-session models, subagent models and session times | ran | `head .bee/usage/agent-mark-sprite-paint.json` | `"subagent_models": {},` / `"started_ms": 1788177934167,` / `"ended_ms": 1788178373879,` |
| 2 | Subagent usage is keyed by model with the same token fields | ran | python over `.bee/usage/paseo-leader-dispatch.json` | `"claude-fable-5-1": {"input": 740, "output": 21671, "cache_write": 157906, "cache_read": 1360275, "new": 180317, "cached": 1360275, "total": 1540592}` |
| 3 | Nothing reads `.bee/usage/` today | ran | `rg "bee/usage" crates docs` (gather job-1792724309796-1595099-1) | no match |
| 4 | Home tabs are an enum plus one tab-bar function | read | `crates/waggledance/src/views.rs:173`, `:548-555` | `pub enum HomeTab {` / `HomeTab::Terminals => (mark(false), mark(true)),` |
| 5 | Archived cells for a feature are readable by one core call | read | `crates/waggledance-core/src/bee.rs:2596` | `pub fn read_archived_cells(root: &Path, feature: &str) -> Vec<BeeCell> {` |
| 6 | `main()` runs before any tokio runtime, so a local offset read there succeeds | read | `crates/waggledance/src/main.rs:42-56` | `fn main() {` … `let cli = cli::Cli::parse();` |
| 7 | The `time` crate is built without `local-offset` | read | `Cargo.toml:26` | `time = { version = "0.3", features = ["formatting", "parsing", "serde"] }` |
| 8 | No existing code reads the local timezone | ran | `grep -rn "current_local_offset" crates` | only `bee.rs:3511: let utc = dt.to_offset(time::UtcOffset::UTC);` |

## Discovery

Gather job-1792724309796-1595099-1 mapped the board (`server.rs` router, `index_page`,
`views::home_page`, `HomeTab`, the oneshot test harness) and confirmed `.bee/usage/` has
no reader. 18 usage files exist in this repo; 3 carry subagent models.

## Approach

Recommended:

1. New module `crates/waggledance-core/src/usage.rs` (sync, no comments), registered in
   `lib.rs`. `read_usage(root) -> Vec<FeatureUsage>` parses every
   `.bee/usage/*.json` with schema `bee-usage/v1`; a malformed file is skipped and
   counted, never an error. `FeatureUsage` carries `feature`, `closed_at_ms`,
   `cells_capped` (from `bee::read_archived_cells`, status `capped`), and sessions
   (`started_ms`, `ended_ms`, `models` and `subagent_models` as `model -> {new, cached}`).
   `summarize(projects: &[(id, name, Vec<FeatureUsage>)], now_ms, days, offset_secs) ->
   UsageSummary`: totals (main / subagent / all; new / cached), per project, per model
   (main and subagent apart, D5), per local day for the bar strip (D4), and the feature
   list. Totals use sessions whose `ended_ms` is in `(now - days*86400000, now]` (D1);
   a session with no `ended_ms` falls back to the feature's `closed_at`. A feature row
   appears when its `closed_at` is in the window and shows the feature's whole spend,
   active time (sum of session durations) and wall span (first `started_ms` to
   `closed_at`). Serialize with serde for the JSON route.
2. Local offset (D4): enable `time`'s `local-offset` feature; `main()` reads
   `UtcOffset::current_local_offset()` once before `cli::run` and stores it in a
   `OnceLock` in `usage.rs` (`set_local_offset` / `local_offset_secs`, UTC when unread).
3. `server.rs`: `GET /api/usage?days=N&project=ID` (N in {1,7,14,30}, else 7) reads
   every bee project's usage over `spawn_blocking` and returns the `UsageSummary` JSON.
   `RegisterFlag` learns `tab=usage`, `days`, `project`; `index_page` reads usage only
   when the Usage tab is selected.
4. `views.rs`: `HomeTab::Usage`, a third tab-bar link `/?tab=usage`, and
   `usage_tab(summary, days, project)` HTML: window chips (1d/7d/14d/30d links),
   project filter links, KPI tiles (total, new, cache-read, features done, active time),
   a per-day bar strip, a per-project table, a per-model table with main/subagent
   columns, the feature list; an empty state naming that spend appears only after a
   feature closes. Plain links, no new JS. Styles in `assets/app.css` with existing
   atelier tokens.

Rejected:

- Fold usage into `read_snapshot` and the board cache: the Kanban path would pay for a
  read only the Usage tab needs.
- Window by feature `closed_at`: rejected by D1.
- Day buckets in UTC like `BeeVelocity`: D4 asks for local days; UTC splits a Vietnam
  day at 07:00.

Risk map:

| Component | Risk | Lands in | Proof |
|---|---|---|---|
| Window edge and day bucketing | MEDIUM | ud-1 | unit tests: a session ending just inside and just outside the window; a non-zero offset moves a session across a day boundary |
| Malformed or partial usage files | LOW | ud-1 | test: bad JSON is skipped and counted; missing `subagent_models` reads as empty |
| `HomeTab` match arms across `views.rs` tests | LOW | ud-2 | `cargo test -p waggledance` green |

Waves: ud-1, then ud-2 (uses ud-1's types), then ud-3 (docs describe the shipped
surface), then a live look at the rebuilt daemon.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "30ff876890293b1cea96673b722ea95a7b27780259af61e6c3b2be09a0c2b112",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "not-applicable", "role": "hat-facts-gaps", "reason": "spec locked from three direct user answers; the data source is read and quoted (logged)"},
    {"stage": "hat-wave-alternatives", "classification": "not-applicable", "role": "hat-alternatives", "reason": "SMALLER PATH checked inline"},
    {"stage": "hat-wave-user-impact", "classification": "not-applicable", "role": "hat-user-impact", "reason": "read-only view, additive tab"},
    {"stage": "hat-wave-risks", "classification": "not-applicable", "role": "hat-risks", "reason": "not high-risk"},
    {"stage": "hat-wave-value", "classification": "not-applicable", "role": "hat-value", "reason": "not high-risk"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "standard lane, no high-risk consult"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "ud-1 and ud-2 are red-first Rust test-plus-code work; precedent a66a46c0 moved Rust cells to this seat"},
    {"stage": "docs", "classification": "required", "role": "docs", "reason": "ud-3 knowledge sync"},
    {"stage": "code", "classification": "not-applicable", "role": "code", "reason": "Rust cells run on the test seat"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "every cell names its role"},
    {"stage": "review", "classification": "conditional", "role": "review", "condition": "the user invokes bee-reviewing", "reason": "agents-review-user-invoked"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no release"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "herding cockpit role, not used"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "one design"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Shape

One slice. Revision 1 adds ud-4 (decision 48a185e4): the goal-check judge found the rolling N*24h window drops the partial oldest day from the per-day bars while the totals count it; windows become calendar days in local time.

SMALLER PATH: a JSON route alone would be cheaper, but D2 asks for a tab on the board;
dropping per-day buckets would skip the offset work, but the 7/14/30-day views need a
daily strip to read as a trend. No cheaper shape honors D1–D5.

## Revision 2 (uat feedback)

At uat the owner saw spend for one project only: bee writes `.bee/usage/` at close, so
unclosed and non-bee work was invisible. D6 and D7 (decisions fc8c944a, eac8b0f9) move
token spend to the runtimes' own logs and add agent runs as tasks.

Load-bearing claims for this revision (all `ran`):

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 9 | Claude Code lines carry model, usage, timestamp, cwd | ran | first usage line of `~/.claude/projects/-mnt-data-Projects-goglbe-waggledance/e6bb5ec7-….jsonl` | `claude-opus-5-5 {'input_tokens': 2, 'cache_creation_input_tokens': 35196, 'cache_read_input_tokens': 12002, 'output_tokens': 245` |
| 10 | Claude Code subagents have their own transcript and a meta file naming the job | ran | `cat …/subagents/agent-a0e2f9843ab04ceff.meta.json` | `{"agentType":"bee-review","description":"review (opus)",…,"model":"opus"` |
| 11 | pi lines carry provider, model and usage | ran | `grep -m1 usage ~/.pi/agent/sessions/…/2026-10-23T02-58-34-663Z_….jsonl` | `"provider":"hive","model":"deepseek-ai/deepseek-v4.1-flash","usage":{"input":29891,"output":74,"cacheRead":0,"cacheWrite":0` |
| 12 | Codex reports only cumulative totals | ran | `grep token_count ~/.codex/sessions/…` | `"total_token_usage":{"input_tokens":0,…,"total_tokens":563497}` |
| 13 | Paseo agent records name the task, provider session and parent | ran | python over `~/.paseo/agents/*/*.json` labels | `{'bee_job': …, 'bee_agent': 'paseo-pi-flash', 'paseo.parent-agent-id': '03dd107e-…'}` |
| 14 | 30 days of logs are about 1.9 GB, so reads must be cached per file | ran | `find … -mtime -30 -printf '%s'` | `801 files, 1574MB` / `303 files, 195MB` / `214 files, 144MB` |
| 15 | `/home/thanhsmind/Projects` is a symlink to `/mnt/data/Projects`, so paths must be canonicalized | ran | `readlink -f /home/thanhsmind/Projects` | `/mnt/data/Projects` |

Steps: (1) `agent_usage.rs` parsers + per-file cache + project mapping, and `summarize`
fed by run events (ud-5); (2) server cache, warm-up and the Agent runs table (ud-6);
(3) docs (ud-7). Risk: first read of ~1.9 GB — mitigated by the mtime horizon, the
per-file cache and a startup warm-up (ud-6).

| ud-5 | Read token use from Claude Code, pi and Codex session logs and Paseo agent records | `crates/waggledance-core/src/agent_usage.rs`, `usage.rs`, `lib.rs` | ud-4 | numbers exist for every project with agent activity | `cargo test --profile fast -p waggledance-core usage` green |
| ud-6 | Show agent-log token use and per-run tasks on the Usage tab | `server.rs`, `views.rs`, `app.css` | ud-5 | today's spend for every active project and an Agent runs table | `cargo test --profile fast -p waggledance` green |
| ud-7 | Describe the agent-log data source and agent runs | `docs/knowledge/work/homepage-tabs/usage-tab.md` | ud-6 | the doc names the sources and the join | pointer check |

## Revision 3 (judge finding)

The goal-check judge measured 1322 log files / 1.9 GB in 30 days and found the cache
held its lock while reading and parsing, re-reading about 27 MB of live agent logs in
full on every Usage view (decision baa1d239). ud-8 makes reads incremental and lock-free.

| ud-8 | Read agent logs incrementally outside the cache lock | `crates/waggledance-core/src/agent_usage.rs`, `crates/waggledance/src/server.rs` | ud-6, ud-7 | the Usage tab stays fast while agents run | `cargo test --profile fast -p waggledance-core usage` green |

## Revision 4 (uat feedback)

The owner asked to drop the repeated Project column and to fold subtasks into their
parent task the way Paseo lists subagents (decision 4ddb1112).

| ud-9 | Group agent runs by project and fold subagents into their parent task | `crates/waggledance/src/views.rs`, `app.css`, `usage-tab.md` | ud-8 | per-project groups; each task opens to its subagents | `cargo test --profile fast -p waggledance usage` green |

## Revision 5 (uat feedback)

The owner saw Codex runs in anphabe-goglbe without using Codex. The Codex Desktop app
batch-imports Claude Code sessions into `~/.codex/sessions` (files created in the same
second, `[external_agent_tool_call` markers, zero input/output tokens), so Claude work
was counted twice (decision b14cca1b).

| ud-10 | Skip Codex sessions that are imported copies of other agents' history | `crates/waggledance-core/src/agent_usage.rs`, `usage-tab.md` | ud-9 | no fake codex runs | `cargo test --profile fast -p waggledance-core usage` green |

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| ud-1 | Read bee usage files and summarize token spend over a window | `Cargo.toml`, `crates/waggledance-core/src/lib.rs`, `crates/waggledance-core/src/usage.rs`, `crates/waggledance/src/main.rs` | — | nothing yet; the numbers exist in a tested core call | `cargo test --profile fast -p waggledance-core usage` green |
| ud-2 | Show token usage on a Usage tab and at /api/usage | `crates/waggledance/src/server.rs`, `crates/waggledance/src/views.rs`, `crates/waggledance/assets/app.css` | ud-1 | a Usage tab with window chips, project filter, totals, per-day bars, per-project and per-model tables, finished features | `cargo test --profile fast -p waggledance` green |
| ud-3 | Describe the Usage tab and its data source | `docs/knowledge/work/homepage-tabs/index.md` | ud-2 | the docs name the tab, the windows, D1 bucketing and the usage file source | pointer check: every named fn and route exists |
| ud-4 | Start usage windows at local midnight so day bars add up to the totals | `crates/waggledance-core/src/usage.rs`, `docs/knowledge/work/homepage-tabs/usage-tab.md` | ud-1, ud-3 | 1d shows today; the day bars sum to the totals | `cargo test --profile fast -p waggledance-core usage` green |

```json
[
 {"id":"ud-1","feature":"usage-dashboard","title":"Read bee usage files and summarize token spend over a window","lane":"standard","role":"test","status":"open","deps":[],
  "decisions":["D1","D3","D4","D5","892cf93d-c49c-48f8-9f83-25db64e68737","be26628f-ec17-41a8-bcbd-868d2501a550"],
  "files":["Cargo.toml","crates/waggledance-core/src/lib.rs","crates/waggledance-core/src/usage.rs","crates/waggledance/src/main.rs"],
  "read_first":["docs/history/usage-dashboard/CONTEXT.md","docs/history/usage-dashboard/plan.md",".bee/usage/paseo-leader-dispatch.json","crates/waggledance-core/src/bee.rs"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach steps 1-2. Create crates/waggledance-core/src/usage.rs and register `pub mod usage;` in lib.rs. Types (serde Serialize, Clone, Debug): Tokens {new: u64, cached: u64} with total(); SessionUsage {started_ms: Option<i64>, ended_ms: Option<i64>, models: BTreeMap<String,Tokens>, subagent_models: BTreeMap<String,Tokens>}; FeatureUsage {feature: String, closed_at_ms: Option<i64>, cells_capped: usize, sessions: Vec<SessionUsage>}. pub fn read_usage(root: &Path) -> (Vec<FeatureUsage>, usize) reads <root>/.bee/usage/*.json, keeps schema == \"bee-usage/v1\", parses closed_at RFC3339 to ms with the time crate, reads per-model `new` and `cached` (missing map or field = empty/0), counts cells with status capped via crate::bee::read_archived_cells(root, feature); a malformed file is skipped and counted in the usize. pub fn summarize(projects: &[ProjectUsageInput], now_ms: i64, days: u32, offset_secs: i32) -> UsageSummary where ProjectUsageInput {id, name, features: Vec<FeatureUsage>}. Window = (now_ms - days*86_400_000, now_ms]. A session counts when its ended_ms (fallback: the feature's closed_at_ms; neither = skipped) is in the window. UsageSummary {days, since_ms, now_ms, totals: SpendSplit, projects: Vec<ProjectSpend>, models: Vec<ModelSpend>, per_day: Vec<DaySpend>, features: Vec<FeatureRow>} with SpendSplit {main: Tokens, subagents: Tokens, all: Tokens}; ProjectSpend {id, name, spend: SpendSplit, features_done: usize, active_ms: i64} sorted by all.total() desc; ModelSpend {model, main: Tokens, subagents: Tokens} sorted by total desc; DaySpend {date: \"YYYY-MM-DD\" in local offset, spend: Tokens} one entry per day of the window oldest first, zero days included; FeatureRow {project_id, project_name, feature, closed_at_ms, cells_capped, active_ms (sum of ended-started over the feature's sessions), span_ms (closed_at minus earliest started_ms, None if unknown), spend: SpendSplit, models: Vec<ModelSpend>} for features whose closed_at_ms is in the window, newest first, showing the whole feature's spend. Local offset: add the `local-offset` feature to the workspace `time` dep in Cargo.toml; in usage.rs a static OnceLock<i32> with pub fn set_local_offset_secs(i32) and pub fn local_offset_secs() -> i32 (0 when unset); in crates/waggledance/src/main.rs before cli::run call waggledance_core::usage::set_local_offset_secs(time::UtcOffset::current_local_offset().map(|o| o.whole_seconds()).unwrap_or(0)). Tests in usage.rs mod tests with temp dirs: read_usage parses a real-shaped file with models and subagent_models; malformed file skipped and counted; window edge in/out; session without ended_ms falls back to closed_at; offset 25200 moves a session ending 2026-10-22T20:00Z onto 2026-10-23; per_day has exactly `days` entries; feature row only for closed_at inside window. No code comments.",
  "must_haves":{"truths":["A session ending inside the window counts in full and one ending outside counts zero","Main and subagent tokens stay separate per model","per_day has one entry per day of the window in local time","A malformed usage file never fails the read"],"artifacts":[{"path":"crates/waggledance-core/src/usage.rs","substantive":"read_usage, summarize, local offset holder, tests"}],"key_links":["main.rs sets the local offset before cli::run"],"prohibitions":["Do not read usage inside bee::read_snapshot","No USD or price data","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance-core usage && cargo build --profile fast -p waggledance",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"ud-2","feature":"usage-dashboard","title":"Show token usage on a Usage tab and at /api/usage","lane":"standard","role":"test","status":"open","deps":["ud-1"],
  "decisions":["D1","D2","D3","D4","D5","5239a98b-970a-4470-9be4-1e4aedc1ef76","be26628f-ec17-41a8-bcbd-868d2501a550"],
  "files":["crates/waggledance/src/server.rs","crates/waggledance/src/views.rs","crates/waggledance/assets/app.css"],
  "read_first":["docs/history/usage-dashboard/CONTEXT.md","docs/history/usage-dashboard/plan.md","crates/waggledance-core/src/usage.rs","docs/knowledge/work/homepage-tabs/index.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach steps 3-4. server.rs: add GET /api/usage handled by api_usage with query days (1|7|14|30, anything else 7) and optional project (a project id; unknown id = empty summary). It lists projects via the engine, keeps bee projects (is_bee_project), reads waggledance_core::usage::read_usage per root inside tokio::task::spawn_blocking, then summarize(inputs filtered by project, now_ms, days, usage::local_offset_secs()) and returns Json. Factor the read into one async fn shared with index_page. RegisterFlag: accept tab=usage (HomeTab::Usage), days and project, parsed with the same defensive visitor (bad values fall back, never 400). index_page reads usage only when tab is Usage and passes the rendered HTML into views::home_page. views.rs: add HomeTab::Usage; home_tabbar gets a third link `/?tab=usage` labelled Usage with a new TABBAR_ICON_USAGE in the same svg idiom, placed after Agents; fix every HomeTab match. pub fn usage_tab(summary: &UsageSummary, days: u32, project: Option<&str>, projects: &[(String,String)]) -> String renders: window chips 1d/7d/14d/30d as links keeping project; project filter links (All + each bee project) keeping days; KPI tiles (all tokens, new, cache-read, subagent share, features done, active time as h/m); a per-day bar strip (div heights proportional to new+cached, title attr with date and numbers); a per-project table; a per-model table with Main new, Main cache-read, Subagent new, Subagent cache-read; a finished-features table (project, feature, closed local date-time, cells, active time, span, tokens all, per-model breakdown). Format big numbers compactly (1.2M, 34.5k). Empty state when totals are zero: `No closed features in this window — token use is recorded when a feature closes.` Escape every project/feature/model string with the existing html escape helper. app.css: .usage-* classes on existing atelier variables, responsive (tables scroll horizontally on narrow screens). Tests: server.rs bee_route_tests — /api/usage?days=7 over a fresh_root project with a .bee/usage file returns totals and the feature row; days=99 falls back to 7; /?tab=usage renders the Usage tab marked aria-current and the feature name; views.rs tests — usage_tab empty state, and per-model main vs subagent columns. No code comments.",
  "must_haves":{"truths":["The home tab bar shows Board, Agents, Usage","/?tab=usage&days=30 shows 30-day numbers and the 30d chip as current","Project filter narrows every table to one project","Main and subagent tokens appear in separate columns per model","/api/usage returns the same summary as JSON"],"artifacts":[{"path":"crates/waggledance/src/views.rs","substantive":"HomeTab::Usage, usage_tab"},{"path":"crates/waggledance/src/server.rs","substantive":"api_usage route and tab=usage wiring"}],"key_links":["index_page and api_usage share one usage read fn"],"prohibitions":["No new JS dependency","No USD","Kanban and Agents tabs do not read usage files","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"ud-3","feature":"usage-dashboard","title":"Describe the Usage tab and its data source","lane":"standard","role":"docs","status":"open","deps":["ud-2"],
  "decisions":["D1","D2","D3","D4","D5"],
  "files":["docs/knowledge/work/homepage-tabs/index.md"],
  "read_first":["docs/history/usage-dashboard/CONTEXT.md","crates/waggledance-core/src/usage.rs","docs/knowledge/work/homepage-tabs/index.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"Add a Usage tab section to docs/knowledge/work/homepage-tabs/index.md: the third tab, its windows and project filter, the data source .bee/usage/<feature>.json written at bee close (unclosed features show nothing), D1 session-end bucketing, D4 local-day strip, D3 new vs cache-read, D5 main vs subagent, the /api/usage route and its query params; add Pointers to crates/waggledance-core/src/usage.rs, server.rs api_usage, views.rs usage_tab. Match the file's existing style.",
  "must_haves":{"truths":["Every fn and route the doc names exists in the code"],"artifacts":[{"path":"docs/knowledge/work/homepage-tabs/index.md","substantive":"Usage tab section with pointers"}],"key_links":[],"prohibitions":["No USD claims"]},
  "verify":"rg -n 'fn read_usage|fn summarize|fn api_usage|fn usage_tab' crates",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":false}},
 {"id": "ud-4", "feature": "usage-dashboard", "title": "Start usage windows at local midnight so day bars add up to the totals", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-1", "ud-3"], "decisions": ["D1", "D4", "48a185e4-b226-4299-b283-72b2ae553d74"], "files": ["crates/waggledance-core/src/usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md"], "read_first": ["crates/waggledance-core/src/usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md", "docs/history/usage-dashboard/CONTEXT.md"], "affects_skills": [], "affects_specs": [], "action": "Per decision 48a185e4: in usage::summarize, set since_ms to the instant of local midnight (using offset_secs) that begins the oldest of the `days` local dates in per_day (today and the days-1 dates before it), instead of now_ms - days*86_400_000; window stays (since_ms, now_ms] but must include a session ending exactly at that midnight, so make the lower bound inclusive (t >= since_ms). Feature rows and project/model totals use the same bounds. Every session counted in totals must land in a per_day bar. Red-first tests in usage.rs mod tests: with days=7, offset 25200 and now 2026-10-23T05:00Z (12:00 local), a session ending 2026-10-16T18:00Z (local 10-17 01:00) counts and lands in the 2026-10-17 bar, and one ending 2026-10-16T16:00Z (local 10-16 23:00) counts zero; for any fixture the sum of per_day equals totals.all; days=1 covers only today local. Update existing window-edge tests to the new bounds. In docs/knowledge/work/homepage-tabs/usage-tab.md replace the rolling-window and partial-oldest-day text with the calendar-day rule. No code comments.", "must_haves": {"truths": ["A N-day window starts at local midnight of the oldest of N local dates ending today", "The per_day bars sum to totals.all", "1d means today in local time"], "artifacts": [{"path": "crates/waggledance-core/src/usage.rs", "substantive": "since_ms at local midnight, inclusive lower bound, tests"}], "key_links": ["usage-tab.md describes the calendar-day window"], "prohibitions": ["No code comments", "Do not change the per_day length"]}, "verify": "cargo test --profile fast -p waggledance-core usage && cargo test --profile fast -p waggledance usage", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}},
 {"id": "ud-5", "feature": "usage-dashboard", "title": "Read token use from Claude Code, pi and Codex session logs and Paseo agent records", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-4"], "decisions": ["D6", "D7", "fc8c944a-8719-435c-a976-fa5744486371", "eac8b0f9-8b3d-4b5d-a667-5239c3845cfe"], "files": ["crates/waggledance-core/src/lib.rs", "crates/waggledance-core/src/agent_usage.rs", "crates/waggledance-core/src/usage.rs"], "read_first": ["docs/history/usage-dashboard/CONTEXT.md", "docs/history/usage-dashboard/plan.md", "crates/waggledance-core/src/usage.rs"], "affects_skills": [], "affects_specs": [], "action": "Follow plan.md Revision 2 step 1. New sync module crates/waggledance-core/src/agent_usage.rs (pub mod in lib.rs). Types (serde Serialize): UsageEvent {ts_ms, model, new: u64, cached: u64}; AgentRun {key, provider (\"claude\"|\"pi\"|\"codex\"), session_id, cwd, title, parent_key: Option<String>, subagent: bool, started_ms, ended_ms, events: Vec<UsageEvent>}. Parsers, each pure over one file's text so tests feed strings: (a) Claude Code: every `*.jsonl` under <claude_home>/projects/*/ (main session, key `claude:<sessionId>`) and */<sessionId>/subagents/agent-*.jsonl (subagent, key `claude:<sessionId>/<agent file stem>`, parent `claude:<sessionId>`, title from the sibling .meta.json `description`, model fallback meta `model`). A line counts when it has message.usage and a timestamp; new = input_tokens + output_tokens + cache_creation_input_tokens, cached = cache_read_input_tokens, model = message.model; dedupe by (message.id, requestId) within a file keeping the last occurrence; cwd from the first line carrying `cwd`; title for main sessions from the first `summary`/`aiTitle` line if any else empty. (b) pi: <pi_home>/agent/sessions/*/*.jsonl; header line type session gives id and cwd; message lines with message.usage give new = input + output + cacheWrite, cached = cacheRead, model = message.provider + \"/\" + message.model, ts from the line timestamp; key `pi:<id>`. (c) Codex: <codex_home>/sessions/**/*.jsonl; session_meta payload gives id and cwd; model from the first `\"model\":` string in a turn_context or session_meta payload; event_msg token_count with info.total_token_usage: emit the positive delta of total_tokens versus the previous token_count as new, cached = delta of cached_input_tokens (subtract it from new), ts from the line; key `codex:<id>`. (d) Paseo: <paseo_home>/agents/*/*.json: map persistence.sessionId+provider to the run key and override title with labels.bee_cell (if set) else title; parent_key = key of the Paseo agent named by labels[\"paseo.parent-agent-id\"] (resolve through that agent's provider+sessionId); subagent = true when a parent exists. Homes default to $HOME/.claude, $HOME/.pi, $HOME/.codex, $PASEO_HOME or $HOME/.paseo, overridable through a pub struct AgentHomes for tests. Only files with mtime inside the requested horizon (since_ms) are read. pub struct AgentLogCache (Mutex<HashMap<PathBuf,(mtime,len,parsed)>>) with pub fn runs(&self, homes, since_ms) -> Vec<AgentRun> that reparses only files whose mtime or len changed; a malformed line is skipped, an unreadable file is skipped. Project mapping: pub fn project_for(cwd, roots: &[(id,name,PathBuf)]) canonicalizes both (std::fs::canonicalize, fallback the raw path), strips a `--wt--<feature>` suffix from the first path component that carries one, and picks the longest root that is a prefix; no match = None (Other). In usage.rs extend summarize: add an input of agent runs (Vec<(Option<project_id>, AgentRun)>); when runs are supplied, totals, projects (plus an `other` pseudo project id \"\" named \"Other\"), models (main vs subagents by run.subagent) and per_day are computed from run events by event ts inside the window (D6), while features still come from bee usage files; add UsageSummary.runs: Vec<RunRow {key, project_id, project_name, provider, title, model (most-used), parent_key, subagent, started_ms, duration_ms, spend: Tokens}> for runs with any event in the window, ordered by started_ms desc. Keep the old bee-file path when no runs are supplied so existing tests stay meaningful. Tests with fixture strings for each parser (claude main + subagent with meta, pi, codex delta, paseo label join and parent), dedupe, project_for with a --wt-- worktree and a symlinked root, Other bucket, window bucketing by event ts, cache reparses only a changed file. No code comments.", "must_haves": {"truths": ["Claude Code, pi and Codex messages all count toward project, model and day totals", "A Claude Code subagent transcript and a Paseo agent with a parent count as subagent", "A feature worktree cwd folds into its main project and an unmatched cwd lands in Other", "A run takes its title from the bee_cell label, the Paseo title or the subagent description", "An unchanged log file is not reparsed"], "artifacts": [{"path": "crates/waggledance-core/src/agent_usage.rs", "substantive": "parsers for claude/pi/codex/paseo, AgentLogCache, project_for, tests"}], "key_links": ["usage::summarize builds totals from run events when runs are supplied"], "prohibitions": ["No USD", "No code comments", "Never write to any provider or Paseo file"]}, "verify": "cargo test --profile fast -p waggledance-core usage", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}},
 {"id": "ud-6", "feature": "usage-dashboard", "title": "Show agent-log token use and per-run tasks on the Usage tab", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-5"], "decisions": ["D6", "D7", "fc8c944a-8719-435c-a976-fa5744486371", "eac8b0f9-8b3d-4b5d-a667-5239c3845cfe"], "files": ["crates/waggledance/src/server.rs", "crates/waggledance/src/views.rs", "crates/waggledance/assets/app.css"], "read_first": ["docs/history/usage-dashboard/CONTEXT.md", "docs/history/usage-dashboard/plan.md", "crates/waggledance-core/src/agent_usage.rs", "crates/waggledance-core/src/usage.rs"], "affects_skills": [], "affects_specs": [], "action": "Follow plan.md Revision 2 step 2. AppState holds one Arc<agent_usage::AgentLogCache>; at serve start spawn_blocking one warm-up call over 30 days so the first Usage view is fast. usage_summary (shared by index_page and api_usage) now lists every registered project (bee or not) as roots for project_for, reads runs from the cache for the window inside spawn_blocking, maps each run to a project, applies the project filter (\"other\" selects the Other bucket), and passes runs into summarize; bee usage files still feed the completed-features table. views::usage_tab: project filter chips come from projects that have spend in the window plus Other; add an Agent runs table after the model table: task (title, else short session id), project, provider, model, a subagent marker with the parent task title when known, start local date-time, duration, new, cache-read; subagent rows indented under their parent when the parent is in the list, otherwise flat; cap at 300 rows with a line naming how many more. Empty-state text becomes `No agent activity in this window.` KPI 'features done' stays from bee files. Tests: server route test with AgentHomes pointed at a fixture dir (expose a way to inject homes into AppState for tests) proving /api/usage counts a Claude Code session in a non-bee registered project and a pi subagent with a Paseo parent; views test for the runs table and subagent marker. No code comments.", "must_haves": {"truths": ["Today's spend shows for every project that had agent activity today", "The Agent runs table names each task with its provider, model and tokens", "Subagent runs are marked and tied to their parent task", "/api/usage carries the runs list"], "artifacts": [{"path": "crates/waggledance/src/views.rs", "substantive": "Agent runs table in usage_tab"}, {"path": "crates/waggledance/src/server.rs", "substantive": "AgentLogCache in AppState, warm-up, usage_summary wiring"}], "key_links": ["usage_summary passes cached runs into summarize"], "prohibitions": ["No USD", "No new JS dependency", "Kanban and Agents tabs do not read agent logs", "No code comments"]}, "verify": "cargo test --profile fast -p waggledance", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}},
 {"id": "ud-7", "feature": "usage-dashboard", "title": "Describe the agent-log data source and agent runs on the Usage tab", "lane": "standard", "role": "docs", "status": "open", "deps": ["ud-6"], "decisions": ["D6", "D7"], "files": ["docs/knowledge/work/homepage-tabs/usage-tab.md"], "read_first": ["docs/history/usage-dashboard/CONTEXT.md", "crates/waggledance-core/src/agent_usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md"], "affects_skills": [], "affects_specs": [], "action": "Update docs/knowledge/work/homepage-tabs/usage-tab.md for D6 and D7: the three log sources and their paths, how each counts new vs cache-read (Codex reports only totals), the Paseo join for titles, bee_cell and parents, project mapping with worktree folding and Other, event-time day bucketing, the Agent runs table, the cache and warm-up, and that bee usage files now only feed completed features. Pointers to agent_usage.rs (parsers, AgentLogCache, project_for). Regenerate indexes with bee knowledge index if the concept list changes.", "must_haves": {"truths": ["Every fn the doc names exists in the code"], "artifacts": [{"path": "docs/knowledge/work/homepage-tabs/usage-tab.md", "substantive": "agent-log source and runs section"}], "key_links": [], "prohibitions": ["No USD claims"]}, "verify": "rg -n 'pub fn project_for|struct AgentLogCache|fn usage_tab' crates", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": false}},
 {"id": "ud-8", "feature": "usage-dashboard", "title": "Read agent logs incrementally outside the cache lock", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-6", "ud-7"], "decisions": ["D6", "baa1d239-b01f-48f1-a8c6-0e389225a66c"], "files": ["crates/waggledance-core/src/agent_usage.rs", "crates/waggledance/src/server.rs"], "read_first": ["crates/waggledance-core/src/agent_usage.rs", "docs/history/usage-dashboard/plan.md"], "affects_skills": [], "affects_specs": [], "action": "Per decision baa1d239. In AgentLogCache::runs: (1) take the lock only to snapshot which paths are fresh and to insert results; stat, read and parse with the lock released, so a Usage request never waits behind the startup warm-up or another request's parse (two concurrent parses of the same file are allowed; the later insert wins only if its (len, mtime) is newer). (2) For Claude Code, pi and Codex .jsonl files keep per-file parse state (byte offset of the last complete line, the run built so far, and whatever each parser needs to continue: Claude dedupe keys, Codex previous cumulative totals, header-derived id/cwd/model); when a file grew and its prefix is unchanged (len >= offset and mtime moved), read only the bytes from the offset with File::seek and feed the new complete lines into the same parser state; a file that shrank or whose first line changed is reparsed from zero. Refactor the parsers to an incremental shape (state + feed(lines)) while keeping the existing pure-string test entry points working. Paseo .json records stay whole-file reads with the mtime+len check. (3) Store each file's run as Arc<AgentRun> and return Vec<Arc<AgentRun>> (adjust usage::summarize_runs and server.rs callers to take &[Arc<AgentRun>] or derefs) so a request does not deep-copy events. (4) Remove cache entries whose path no longer exists or whose mtime is older than the requested horizon minus one day. (5) In server.rs fix the flaky api_usage_counts_agent_logs_in_a_non_bee_project_with_paseo_parents: place fixture events at a fixed instant inside today's window regardless of wall clock (e.g. now minus one minute but clamped to after local midnight, or pass days=7). Tests: an appended jsonl grows by one line and only that line is parsed (assert via a parse counter or by appending a line whose usage changes totals while the earlier bytes were made unreadable to a full reparse, e.g. truncated-prefix trick is not allowed — prefer an exposed parse-bytes counter on the cache for tests); a Claude duplicate message id across two appends is still deduped; Codex cumulative delta continues across appends; shrink triggers full reparse; evicted entries disappear; a concurrent call from another thread completes while a slow parse holds no lock (use a test hook or just assert the lock is not held during parse by calling runs from two threads). No code comments.", "must_haves": {"truths": ["The cache lock is never held while a file is read or parsed", "A growing log is parsed only from its last offset", "Dedupe and Codex deltas stay correct across appends", "Old entries are evicted", "The route test no longer depends on the wall clock"], "artifacts": [{"path": "crates/waggledance-core/src/agent_usage.rs", "substantive": "incremental per-file state, Arc runs, eviction, tests"}], "key_links": ["server.rs usage_summary uses Arc runs"], "prohibitions": ["No code comments", "Never write to any provider or Paseo file"]}, "verify": "cargo test --profile fast -p waggledance-core usage && cargo test --profile fast -p waggledance usage && cargo fmt --check", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}},
 {"id": "ud-9", "feature": "usage-dashboard", "title": "Group agent runs by project and fold subagents into their parent task", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-8"], "decisions": ["D7", "4ddb1112-ee95-49a5-a464-5ae5c607ab91"], "files": ["crates/waggledance/src/views.rs", "crates/waggledance/assets/app.css", "docs/knowledge/work/homepage-tabs/usage-tab.md"], "read_first": ["crates/waggledance/src/views.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md", "docs/history/usage-dashboard/CONTEXT.md"], "affects_skills": [], "affects_specs": [], "action": "Per decision 4ddb1112, rework views::usage_runs_table (the Agent runs section of usage_tab). (1) Drop the Project column. When no project filter is active, group runs under one heading per project (project name plus that project's run total tokens), projects ordered by total desc, Other last; with a filter, no headings. (2) Render each top-level run (no parent in the list) as a <details class=\"usage__run\"> closed by default; its <summary> is one row: task title (else short session id), provider, model, start local date-time, duration, new, cache-read, where tokens are the run's own plus all its descendants', and when it has subagents a muted 'N subagents' count. Inside, list every descendant subagent (flatten nested levels, ordered by start) one per item styled like Paseo's subagent list: line 1 the subagent title; line 2 muted `provider · model · duration · <new> new · <cache-read> cache` with compact numbers. A run with no subagents renders as a plain row with the same columns (no disclosure marker). Remove the 'subagent of <parent>' text. A subagent whose parent is not in the list is a top-level row with a small 'subagent' tag. Keep the 300-row cap counting top-level rows and the 'N more runs not shown.' line. No new JS; use details/summary. CSS in app.css on existing atelier variables; keep it readable on narrow screens. Update the views.rs tests for the runs table (grouping headings without filter, none with filter, parent summary totals include subagents, subagent lines inside details, no Project column header, orphan tag). Update docs/knowledge/work/homepage-tabs/usage-tab.md's Agent runs section to the new layout. No code comments.", "must_haves": {"truths": ["The Agent runs list has no Project column", "Without a project filter runs are grouped under per-project headings", "Subagents are folded inside their parent task, closed by default", "A parent's tokens include its subagents'", "Subagent lines show provider, model, duration and tokens"], "artifacts": [{"path": "crates/waggledance/src/views.rs", "substantive": "grouped, collapsible usage_runs_table and tests"}], "key_links": ["usage-tab.md describes the grouped layout"], "prohibitions": ["No new JS dependency", "No code comments"]}, "verify": "cargo test --profile fast -p waggledance usage && cargo fmt --check", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}},
 {"id": "ud-10", "feature": "usage-dashboard", "title": "Skip Codex sessions that are imported copies of other agents' history", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-9"], "decisions": ["D6", "b14cca1b-0815-4ad3-a7e4-6afc280f373a"], "files": ["crates/waggledance-core/src/agent_usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md"], "read_first": ["crates/waggledance-core/src/agent_usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md"], "affects_skills": [], "affects_specs": [], "action": "Per decision b14cca1b. In the Codex parser state (incremental LogState path and the pure parse_codex entry point): (1) mark the session as imported when any line's payload text (event_msg agent_message message, or response_item content text) contains `[external_agent_tool_call` or `[external_agent_tool_result`; an imported session yields no run at all (AgentLogCache::runs and parse_codex drop it), and once a file is marked imported later appends keep it dropped. (2) A token_count event adds spend only when info.last_token_usage has input_tokens > 0 or output_tokens > 0; new = input_tokens + output_tokens + cache_write_input_tokens - cached_input_tokens clamped at 0 is NOT required: use new = (input_tokens - cached_input_tokens).max(0) + output_tokens + cache_write_input_tokens and cached = cached_input_tokens from last_token_usage, instead of total_tokens deltas; a token_count whose last_token_usage is all zero except total_tokens adds nothing. (3) Model: take the session's model from turn_context payload.model, else session_meta payload.model, else the first \"model\": string; never the literal \"codex\" fallback when a model string exists. Tests with fixture strings: an imported session (agent_message with [external_agent_tool_call ...]) yields no run, also when the marker arrives in an appended chunk; a real session with last_token_usage {input 1000, cached 400, output 50} yields new 650 cached 400; a total-only token_count adds nothing; update the existing codex delta tests to the new rule. Update usage-tab.md's Codex row: imported sessions are skipped, spend comes from last_token_usage, and why (Codex Desktop imports Claude Code history). No code comments.", "must_haves": {"truths": ["A Codex session carrying external_agent_tool_call markers produces no run", "A Codex token_count with zero input and output adds no spend", "Real Codex usage counts from last_token_usage with cache split"], "artifacts": [{"path": "crates/waggledance-core/src/agent_usage.rs", "substantive": "import detection, last_token_usage counting, tests"}], "key_links": ["usage-tab.md Codex row describes the import skip"], "prohibitions": ["No code comments", "Never write to any provider file"]}, "verify": "cargo test --profile fast -p waggledance-core usage && cargo fmt --check", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}}
]
```
