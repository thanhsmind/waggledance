---
type: bee.area
title: homepage-tabs — the Usage tab
description: "The home page's third tab: token spend per registered bee project, per model (main vs subagents) and per closed feature over a 1, 7, 14 or 30-day window, read from .bee/usage/<feature>.json."
timestamp: 2026-10-23
bee:
  id: homepage-tabs-usage-tab
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  sources: [docs/history/usage-dashboard/CONTEXT.md, docs/history/usage-dashboard/plan.md]
---

# homepage-tabs — the Usage tab

## What it is

The third home tab, after Kanban and Terminals, reached at `/?tab=usage`
(`HomeTab::Usage`). It totals token spend across every registered bee project
over a window, splits it per model into main-session and subagent spend, draws
a per-day strip, and lists the features closed inside the window. It counts
tokens only: there are no prices, and it never estimates money.

## Query parameters

- `days` — the window, one of `1`, `7`, `14`, `30` (`views::USAGE_WINDOWS`).
  Any other value, or none, falls back to `7`.
- `project` — a project id narrows every figure to that project. Empty means
  every bee project. An unknown id matches nothing, so all totals read zero.

`/api/usage` takes the same two parameters and returns the `UsageSummary` the
tab renders, as JSON: `days`, `since_ms`, `now_ms`, `totals`, `projects`,
`models`, `per_day`, `features`.

## Where the numbers come from

Each project's `.bee/usage/<feature>.json` (schema `bee-usage/v1`). `bee close`
writes that file, so a feature still in flight has none and adds nothing to the
tab. A file with another schema, or one that does not parse, is skipped.
The tab only reads these files and never writes to a project.

## How spend is counted

- **Window**: rolling, measured back from now. It covers
  `(now - days × 24h, now]`.
- **Session-end bucketing (D1)**: a session counts in full when its `ended_ms`
  falls inside the window. Sessions are never split. A session without
  `ended_ms` falls back to the feature's `closed_at`. A session with neither
  is not counted. A feature that ran across several days spreads its spend
  over the days its sessions ended.
- **New vs cache-read (D3)**: `new` is input + output + cache write, and
  `cached` is cache-read. Both come straight from the usage file.
- **Main vs subagent (D5)**: the file's `models` field counts as main-session
  spend and its `subagent_models` field as subagent spend, kept apart per
  model.
- **Per-day strip (D4)**: the strip has exactly `days` entries, one per
  calendar day ending today. Days use the daemon's local time: the offset is
  read once at startup (`usage::set_local_offset_secs`). The window is a
  rolling `days × 24h` and the strip has whole local days, so a session that
  ended on the partial oldest day counts in the totals but appears in no bar.
- **Feature rows**: one row per feature whose `closed_at` is inside the
  window. Each row shows the project, feature, close time, cells capped
  (counted from the archived cells) and active time (the sum of its session
  durations). It also shows the wall span, which runs from the first session
  start to close. The row carries the feature's whole spend, including
  sessions that ended before the window opened.

## Pointers

- `crates/waggledance-core/src/usage.rs` — `read_usage` (parses
  `.bee/usage/`), `summarize` (window, buckets, per-day strip, feature rows),
  `set_local_offset_secs`
- `crates/waggledance/src/server.rs` — the `/api/usage` route, `api_usage`,
  `usage_summary` (bee projects, project filter, `spawn_blocking` read), and
  the `days` / `project` parsing in `RegisterFlag`
- `crates/waggledance/src/views.rs` — `usage_tab`, `USAGE_WINDOWS`,
  `HomeTab::Usage`
- `docs/history/usage-dashboard/CONTEXT.md` — decisions D1 to D5
