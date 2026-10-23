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
 {"id": "ud-4", "feature": "usage-dashboard", "title": "Start usage windows at local midnight so day bars add up to the totals", "lane": "standard", "role": "test", "status": "open", "deps": ["ud-1", "ud-3"], "decisions": ["D1", "D4", "48a185e4-b226-4299-b283-72b2ae553d74"], "files": ["crates/waggledance-core/src/usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md"], "read_first": ["crates/waggledance-core/src/usage.rs", "docs/knowledge/work/homepage-tabs/usage-tab.md", "docs/history/usage-dashboard/CONTEXT.md"], "affects_skills": [], "affects_specs": [], "action": "Per decision 48a185e4: in usage::summarize, set since_ms to the instant of local midnight (using offset_secs) that begins the oldest of the `days` local dates in per_day (today and the days-1 dates before it), instead of now_ms - days*86_400_000; window stays (since_ms, now_ms] but must include a session ending exactly at that midnight, so make the lower bound inclusive (t >= since_ms). Feature rows and project/model totals use the same bounds. Every session counted in totals must land in a per_day bar. Red-first tests in usage.rs mod tests: with days=7, offset 25200 and now 2026-10-23T05:00Z (12:00 local), a session ending 2026-10-16T18:00Z (local 10-17 01:00) counts and lands in the 2026-10-17 bar, and one ending 2026-10-16T16:00Z (local 10-16 23:00) counts zero; for any fixture the sum of per_day equals totals.all; days=1 covers only today local. Update existing window-edge tests to the new bounds. In docs/knowledge/work/homepage-tabs/usage-tab.md replace the rolling-window and partial-oldest-day text with the calendar-day rule. No code comments.", "must_haves": {"truths": ["A N-day window starts at local midnight of the oldest of N local dates ending today", "The per_day bars sum to totals.all", "1d means today in local time"], "artifacts": [{"path": "crates/waggledance-core/src/usage.rs", "substantive": "since_ms at local midnight, inclusive lower bound, tests"}], "key_links": ["usage-tab.md describes the calendar-day window"], "prohibitions": ["No code comments", "Do not change the per_day length"]}, "verify": "cargo test --profile fast -p waggledance-core usage && cargo test --profile fast -p waggledance usage", "trace": {"worker": null, "outcome": null, "files_changed": [], "deviations": [], "friction": null, "capped_at": null, "behavior_change": true}}
]
```
