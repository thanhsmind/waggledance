promote proposal for work item "usage-dashboard" (docs/history/usage-dashboard/CONTEXT.md + docs/history/usage-dashboard/plan.md) — 10 capped cell(s): ud-1, ud-2, ud-3, ud-4, ud-5, ud-6, ud-7, ud-8, ud-9, ud-10
anchor: history — docs/history/usage-dashboard/CONTEXT.md, docs/history/usage-dashboard/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/usage-dashboard/delivery.md

---
type: bee.delivery
title: usage-dashboard — delivery
description: "Delivery record proposed by bee knowledge promote for work item usage-dashboard: 10 capped cell(s), 24 recorded deviation(s)."
timestamp: 2026-10-23
bee:
  id: usage-dashboard-delivery
  lifecycle: active
  areas: [work/homepage-tabs]
  required_context: [docs/history/usage-dashboard/CONTEXT.md, docs/history/usage-dashboard/plan.md]
  sources: [docs/history/usage-dashboard/CONTEXT.md, docs/history/usage-dashboard/plan.md, .bee/cells/ud-1.json, .bee/cells/ud-2.json, .bee/cells/ud-3.json, .bee/cells/ud-4.json, .bee/cells/ud-5.json, .bee/cells/ud-6.json, .bee/cells/ud-7.json, .bee/cells/ud-8.json, .bee/cells/ud-9.json, .bee/cells/ud-10.json]
---

# usage-dashboard — Delivery

## What shipped

- **ud-1** — waggledance_core::usage reads .bee/usage files and summarizes token spend per project, model (main vs subagent), local day and closed feature; main() sets the local offset before cli::run (5 file(s) changed)
- **ud-2** — Usage tab and /api/usage share one usage read; window chips, project filter, KPIs, per-day bars, project/model/feature tables (3 file(s) changed)
- **ud-3** — Usage tab concept written at docs/knowledge/work/homepage-tabs/usage-tab.md with pointers; knowledge indexes regenerated (4 file(s) changed)
- **ud-4** — Usage windows start at local midnight of the oldest per-day date with an inclusive lower bound; per-day bars now sum to totals (2 file(s) changed)
- **ud-5** — agent_usage.rs parses Claude Code/pi/Codex logs and Paseo records behind a per-file cache; usage::summarize_runs builds totals, projects with Other, models, days and a runs list from run events (3 file(s) changed)
- **ud-6** — Usage tab and /api/usage read cached agent logs for every registered project plus Other, and list agent runs with subagents under their parent (3 file(s) changed)
- **ud-7** — usage-tab.md documents the D6 log sources, per-source new/cache-read counting, Paseo join, project mapping with worktree folding and Other, event-time days, the Agent runs table, the cache and warm-up, and that .bee/usage files feed only finished features (1 file(s) changed)
- **ud-8** — Agent logs are read incrementally per file with the cache lock released; runs shared as Arc<AgentRun>; stale and deleted entries evicted; route test no longer wall-clock dependent (3 file(s) changed)
- **ud-9** — Agent runs list grouped per project with no Project column; subagents folded into closed details under their parent, parent totals include descendants (3 file(s) changed)
- **ud-10** — Codex parser drops imported sessions and counts spend from last_token_usage with the cache split (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **ud-1** — `cargo test --profile fast -p waggledance-core usage && cargo build --profile fast -p waggledance` — 9 usage tests in usage.rs pass and the waggledance binary builds; cargo fmt --check and the no_web_framework_dependency_declared guard also green; full suite not run
- **ud-2** — `cargo test --profile fast -p waggledance` — cell verify over the crate touched: 1281+3+5 passed, 2 ignored; new tests api_usage_returns_totals_and_the_closed_feature, api_usage_falls_back_to_seven_days_and_empties_on_an_unknown_project, usage_…
- **ud-3** — `rg -n 'fn read_usage|fn summarize|fn api_usage|fn usage_tab' crates` — every fn the doc names exists (usage.rs:138,248; server.rs:1371 plus route at :430; views.rs:652); also ran bee knowledge index --check (120 indexes, 0 stale) and bee knowledge check (0 OKF/profile e…
- **ud-4** — `cargo test --profile fast -p waggledance-core usage && cargo test --profile fast -p waggledance usage` — the cell verify: 12 core usage tests plus 5 server usage route tests; full suite not run
- **ud-5** — `cargo test --profile fast -p waggledance-core usage` — 21 tests over agent_usage and usage, the two modules touched; clippy clean on waggledance-core; cargo check -p waggledance (non-test) green; cargo check -p waggledance --tests is RED (views.rs test l…
- **ud-6** — `cargo test --profile fast -p waggledance` — the cell verify over the whole waggledance crate, 1283+3+5 passed and 2 ignored; new tests api_usage_counts_agent_logs_in_a_non_bee_project_with_paseo_parents and usage_tab_lists_agent_runs_with_suba…
- **ud-7** — `rg -n 'pub fn project_for|struct AgentLogCache|fn usage_tab' crates` — docs pointer check; also rg-resolved every other fn/const the doc names in crates/
- **ud-8** — `cargo test --profile fast -p waggledance-core usage && cargo test --profile fast -p waggledance usage && cargo fmt --check` — the cell verify over the touched usage modules (26 core + 7 server tests); clippy on waggledance-core clean; full suite not run
- **ud-9** — `cargo test --profile fast -p waggledance usage && cargo fmt --check` — 7 usage-filtered tests incl. the rewritten runs-table test, fmt clean, plus 3 css-filtered tests; full suite not run and no live browser render, so the 720px narrow layout is unverified visually
- **ud-10** — `cargo test --profile fast -p waggledance-core usage && cargo fmt --check` — 29 usage/agent_usage tests incl. new codex import and last_token_usage tests; fmt clean; filtered to usage, rest of suite not run

## Deviations

- **ud-1** — Committed Cargo.lock beside the named files — enabling the local-offset feature on time pulls in num_threads, so the lockfile changes — hit an unforeseen obstacle
- **ud-1** — Wrote the module and its tests together, no separate red run first — the spec named every signature, so a stub-only red run would have proven nothing extra — found a better route
- **ud-1** — A session on the partial oldest local date of the window counts in totals but in no per_day bucket, since per_day has exactly days entries ending today — the plan did not say how to treat the partial day — the plan was wrong about a fact
- **ud-1** — A file whose schema is not bee-usage/v1 is counted as skipped, same as malformed JSON — the plan did not say whether it counts — something else had to be fixed first
- **ud-2** — Added a Usage anchor beside Orchestrator in the home topbar (views::home_page) besides the tab bar link — the handset tab bar is display:none above 900px, so on desktop the tab would only be reachable by typing the URL — the plan was wrong about a fact
- **ud-2** — Widened the handset tab bar from five to six columns (app.css grid repeat(6)) and renamed the views test that pinned five to the_handset_tab_bar_keeps_the_six_destinations_it_was_widened_to — that test guarded the dcfbda20 five-column decision and D2 asks for the new tab-bar entry — something else had to be fixed first
- **ud-2** — api_usage reuses the RegisterFlag query extractor instead of a separate query struct — one defensive parser for days/project serves both routes and repeated keys never 400 — found a better route
- **ud-2** — Removed the stale five-destinations CSS comment above the tab bar grid rather than rewriting it — no-code-comments rule — something else had to be fixed first
- **ud-2** — Red-first not shown as a separate red run: the new tests could only fail by not compiling before the types existed — found a better route
- **ud-3** — wrote the section as a new concept usage-tab.md and regenerated index.md instead of hand-editing index.md — index.md is generated by bee knowledge index; leader accepted via decision 8aca8532 — the plan was wrong about a fact
- **ud-3** — the regen also rewrote concept counts in docs/knowledge/index.md and docs/knowledge/work/index.md, reserved and committed with the cell — the index generator rewrites parent indexes when a folder gains a concept — hit an unforeseen obstacle
- **ud-5** — Added summarize_runs beside summarize instead of changing summarize signature — server.rs (out of scope) calls summarize with four args and must keep compiling; ud-6 calls summarize_runs — found a better route
- **ud-5** — Paseo records are read regardless of the since_ms mtime horizon — a parent agent record can be older than the window while its child is inside it; the files are small and cached — found a better route
- **ud-5** — Usage lines with zero tokens (e.g. Claude <synthetic> replies) are dropped — they add nothing and would show a fake model — found a better route
- **ud-5** — Run active_ms for a project sums main-run durations only — subagent runs overlap their parent — something else had to be fixed first
- **ud-6** — Filter chip names come from a first unfiltered summarize_runs and a second summarize_runs runs for the filtered project — the chips must list every active project even while one is selected — found a better route
- **ud-6** — Existing api_usage test now asserts totals from fixture agent logs instead of the bee usage file — D6 moved totals to agent logs and the tab shows the empty state without agent activity — the plan was wrong about a fact
- **ud-8** — Edited crates/waggledance-core/src/usage.rs (summarize_runs and summarize_with take (Option<String>, Arc<AgentRun>), test helper returns Arc) — the Arc runs requirement reaches that signature; reserved under ud-8-worker per the leader note — something else had to be fixed first
- **ud-8** — Rewrite detection compares the first up-to-1024 consumed bytes rather than only the first line — a first-line-only check let the existing pi same-head rewrite test feed misaligned bytes; 1 KB still covers the first line in practice — found a better route
- **ud-8** — A trailing line without a newline is consumed when it already parses as JSON, otherwise it waits — keeps parity with the old whole-file parse (the server fixture writes a Claude log with no trailing newline) — hit an unforeseen obstacle
- **ud-8** — Eviction judges each file current stat mtime against since minus one day, Paseo records only by existence — the stored entry mtime would keep a touched-back file — the plan was wrong about a fact
- **ud-9** — Rendered the runs list as CSS-grid div rows instead of a table — details/summary cannot sit inside a table body — hit an unforeseen obstacle
- **ud-9** — Drew the disclosure arrow with .usage__task::before — a grid summary drops the native marker in Chromium — hit an unforeseen obstacle
- **ud-10** — import flag lives on LogState.imported so the incremental cache and parse_codex share one path, and model choice is one ranked Option (turn_context, then session_meta, then any payload model) instead of three fields — smallest shape covering both paths — found a better route

## Provenance

Proposed by `bee knowledge promote --work usage-dashboard` from 10 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/usage-dashboard/CONTEXT.md`, `docs/history/usage-dashboard/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "usage-dashboard" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-10-23T04:58:03.135Z), the work item declares no bee.areas.

area work/homepage-tabs:
  - [ud-1] waggledance_core::usage reads .bee/usage files and summarizes token spend per project, model (main vs subagent), local day and closed feature; main() sets the local offset before cli::run — feature-wide sync per the scribing stamp, 5 file(s) changed (trace .bee/cells/ud-1.json)
  - [ud-2] Usage tab and /api/usage share one usage read; window chips, project filter, KPIs, per-day bars, project/model/feature tables — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/ud-2.json)
  - [ud-4] Usage windows start at local midnight of the oldest per-day date with an inclusive lower bound; per-day bars now sum to totals — feature-wide sync per the scribing stamp, 2 file(s) changed (trace .bee/cells/ud-4.json)
  - [ud-5] agent_usage.rs parses Claude Code/pi/Codex logs and Paseo records behind a per-file cache; usage::summarize_runs builds totals, projects with Other, models, days and a runs list from run events — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/ud-5.json)
  - [ud-6] Usage tab and /api/usage read cached agent logs for every registered project plus Other, and list agent runs with subagents under their parent — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/ud-6.json)
  - [ud-8] Agent logs are read incrementally per file with the cache lock released; runs shared as Arc<AgentRun>; stale and deleted entries evicted; route test no longer wall-clock dependent — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/ud-8.json)
  - [ud-9] Agent runs list grouped per project with no Project column; subagents folded into closed details under their parent, parent totals include descendants — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/ud-9.json)
  - [ud-10] Codex parser drops imported sessions and counts spend from last_token_usage with the cache split — feature-wide sync per the scribing stamp, 2 file(s) changed (trace .bee/cells/ud-10.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell ud-1 — save as docs/knowledge/patterns/usage-dashboard-ud-1-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-1 — pitfall candidate
description: "Pitfall candidate mined from cell ud-1's capped trace: Committed Cargo.lock beside the named files — enabling the local-offset feature on time pulls in num_threads, so the lockfile changes — hit an unforeseen obsta…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-1-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-1.json]
  polarity: pitfall
---

# usage-dashboard cell ud-1 — pitfall candidate

## What the cell did

waggledance_core::usage reads .bee/usage files and summarizes token spend per project, model (main vs subagent), local day and closed feature; main() sets the local offset before cli::run

## Recorded evidence (verbatim from .bee/cells/ud-1.json)

- **deviation** — Committed Cargo.lock beside the named files — enabling the local-offset feature on time pulls in num_threads, so the lockfile changes — hit an unforeseen obstacle
- **deviation** — Wrote the module and its tests together, no separate red run first — the spec named every signature, so a stub-only red run would have proven nothing extra — found a better route
- **deviation** — A session on the partial oldest local date of the window counts in totals but in no per_day bucket, since per_day has exactly days entries ending today — the plan did not say how to treat the partial day — the plan was wrong about a fact
- **deviation** — A file whose schema is not bee-usage/v1 is counted as skipped, same as malformed JSON — the plan did not say whether it counts — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-2 — save as docs/knowledge/patterns/usage-dashboard-ud-2-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-2 — pitfall candidate
description: "Pitfall candidate mined from cell ud-2's capped trace: Added a Usage anchor beside Orchestrator in the home topbar (views::home_page) besides the tab bar link — the handset tab bar is display:none above 900px, so o…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-2-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-2.json]
  polarity: pitfall
---

# usage-dashboard cell ud-2 — pitfall candidate

## What the cell did

Usage tab and /api/usage share one usage read; window chips, project filter, KPIs, per-day bars, project/model/feature tables

## Recorded evidence (verbatim from .bee/cells/ud-2.json)

- **deviation** — Added a Usage anchor beside Orchestrator in the home topbar (views::home_page) besides the tab bar link — the handset tab bar is display:none above 900px, so on desktop the tab would only be reachable by typing the URL — the plan was wrong about a fact
- **deviation** — Widened the handset tab bar from five to six columns (app.css grid repeat(6)) and renamed the views test that pinned five to the_handset_tab_bar_keeps_the_six_destinations_it_was_widened_to — that test guarded the dcfbda20 five-column decision and D2 asks for the new tab-bar entry — something else had to be fixed first
- **deviation** — api_usage reuses the RegisterFlag query extractor instead of a separate query struct — one defensive parser for days/project serves both routes and repeated keys never 400 — found a better route
- **deviation** — Removed the stale five-destinations CSS comment above the tab bar grid rather than rewriting it — no-code-comments rule — something else had to be fixed first
- **deviation** — Red-first not shown as a separate red run: the new tests could only fail by not compiling before the types existed — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-3 — save as docs/knowledge/patterns/usage-dashboard-ud-3-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-3 — pitfall candidate
description: "Pitfall candidate mined from cell ud-3's capped trace: wrote the section as a new concept usage-tab.md and regenerated index.md instead of hand-editing index.md — index.md is generated by bee knowledge index; leade…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-3-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-3.json]
  polarity: pitfall
---

# usage-dashboard cell ud-3 — pitfall candidate

## What the cell did

Usage tab concept written at docs/knowledge/work/homepage-tabs/usage-tab.md with pointers; knowledge indexes regenerated

## Recorded evidence (verbatim from .bee/cells/ud-3.json)

- **deviation** — wrote the section as a new concept usage-tab.md and regenerated index.md instead of hand-editing index.md — index.md is generated by bee knowledge index; leader accepted via decision 8aca8532 — the plan was wrong about a fact
- **deviation** — the regen also rewrote concept counts in docs/knowledge/index.md and docs/knowledge/work/index.md, reserved and committed with the cell — the index generator rewrites parent indexes when a folder gains a concept — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-5 — save as docs/knowledge/patterns/usage-dashboard-ud-5-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-5 — pitfall candidate
description: "Pitfall candidate mined from cell ud-5's capped trace: Added summarize_runs beside summarize instead of changing summarize signature — server.rs (out of scope) calls summarize with four args and must keep compiling…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-5-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-5.json]
  polarity: pitfall
---

# usage-dashboard cell ud-5 — pitfall candidate

## What the cell did

agent_usage.rs parses Claude Code/pi/Codex logs and Paseo records behind a per-file cache; usage::summarize_runs builds totals, projects with Other, models, days and a runs list from run events

## Recorded evidence (verbatim from .bee/cells/ud-5.json)

- **deviation** — Added summarize_runs beside summarize instead of changing summarize signature — server.rs (out of scope) calls summarize with four args and must keep compiling; ud-6 calls summarize_runs — found a better route
- **deviation** — Paseo records are read regardless of the since_ms mtime horizon — a parent agent record can be older than the window while its child is inside it; the files are small and cached — found a better route
- **deviation** — Usage lines with zero tokens (e.g. Claude <synthetic> replies) are dropped — they add nothing and would show a fake model — found a better route
- **deviation** — Run active_ms for a project sums main-run durations only — subagent runs overlap their parent — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-6 — save as docs/knowledge/patterns/usage-dashboard-ud-6-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-6 — pitfall candidate
description: "Pitfall candidate mined from cell ud-6's capped trace: Filter chip names come from a first unfiltered summarize_runs and a second summarize_runs runs for the filtered project — the chips must list every active proj…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-6-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-6.json]
  polarity: pitfall
---

# usage-dashboard cell ud-6 — pitfall candidate

## What the cell did

Usage tab and /api/usage read cached agent logs for every registered project plus Other, and list agent runs with subagents under their parent

## Recorded evidence (verbatim from .bee/cells/ud-6.json)

- **deviation** — Filter chip names come from a first unfiltered summarize_runs and a second summarize_runs runs for the filtered project — the chips must list every active project even while one is selected — found a better route
- **deviation** — Existing api_usage test now asserts totals from fixture agent logs instead of the bee usage file — D6 moved totals to agent logs and the tab shows the empty state without agent activity — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-8 — save as docs/knowledge/patterns/usage-dashboard-ud-8-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-8 — pitfall candidate
description: "Pitfall candidate mined from cell ud-8's capped trace: Edited crates/waggledance-core/src/usage.rs (summarize_runs and summarize_with take (Option<String>, Arc<AgentRun>), test helper returns Arc) — the Arc runs re…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-8-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-8.json]
  polarity: pitfall
---

# usage-dashboard cell ud-8 — pitfall candidate

## What the cell did

Agent logs are read incrementally per file with the cache lock released; runs shared as Arc<AgentRun>; stale and deleted entries evicted; route test no longer wall-clock dependent

## Recorded evidence (verbatim from .bee/cells/ud-8.json)

- **deviation** — Edited crates/waggledance-core/src/usage.rs (summarize_runs and summarize_with take (Option<String>, Arc<AgentRun>), test helper returns Arc) — the Arc runs requirement reaches that signature; reserved under ud-8-worker per the leader note — something else had to be fixed first
- **deviation** — Rewrite detection compares the first up-to-1024 consumed bytes rather than only the first line — a first-line-only check let the existing pi same-head rewrite test feed misaligned bytes; 1 KB still covers the first line in practice — found a better route
- **deviation** — A trailing line without a newline is consumed when it already parses as JSON, otherwise it waits — keeps parity with the old whole-file parse (the server fixture writes a Claude log with no trailing newline) — hit an unforeseen obstacle
- **deviation** — Eviction judges each file current stat mtime against since minus one day, Paseo records only by existence — the stored entry mtime would keep a touched-back file — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-9 — save as docs/knowledge/patterns/usage-dashboard-ud-9-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-9 — pitfall candidate
description: "Pitfall candidate mined from cell ud-9's capped trace: Rendered the runs list as CSS-grid div rows instead of a table — details/summary cannot sit inside a table body — hit an unforeseen obstacle"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-9-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-9.json]
  polarity: pitfall
---

# usage-dashboard cell ud-9 — pitfall candidate

## What the cell did

Agent runs list grouped per project with no Project column; subagents folded into closed details under their parent, parent totals include descendants

## Recorded evidence (verbatim from .bee/cells/ud-9.json)

- **deviation** — Rendered the runs list as CSS-grid div rows instead of a table — details/summary cannot sit inside a table body — hit an unforeseen obstacle
- **deviation** — Drew the disclosure arrow with .usage__task::before — a grid summary drops the native marker in Chromium — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell ud-10 — save as docs/knowledge/patterns/usage-dashboard-ud-10-pitfall.md

---
type: bee.pattern
title: usage-dashboard cell ud-10 — pitfall candidate
description: "Pitfall candidate mined from cell ud-10's capped trace: import flag lives on LogState.imported so the incremental cache and parse_codex share one path, and model choice is one ranked Option (turn_context, then sessi…"
timestamp: 2026-10-23
bee:
  id: usage-dashboard-ud-10-pitfall
  lifecycle: draft
  areas: [work/homepage-tabs]
  sources: [.bee/cells/ud-10.json]
  polarity: pitfall
---

# usage-dashboard cell ud-10 — pitfall candidate

## What the cell did

Codex parser drops imported sessions and counts spend from last_token_usage with the cache split

## Recorded evidence (verbatim from .bee/cells/ud-10.json)

- **deviation** — import flag lives on LogState.imported so the incremental cache and parse_codex share one path, and model choice is one ranked Option (turn_context, then session_meta, then any payload model) instead of three fields — smallest shape covering both paths — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 10 capped cell(s) mined, 1 delivery draft, 8 area bullet(s), 8 pattern candidate(s), 0 file(s) written.