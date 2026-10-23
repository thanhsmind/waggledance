---
type: bee.area
title: homepage-tabs — the Usage tab
description: "The home page's third tab: token spend per project, per model (main vs subagents), per agent run and per closed feature over a 1, 7, 14 or 30-day window, read from the Claude Code, pi and Codex session logs joined with Paseo agent records; .bee/usage files feed only the finished-features list."
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
(`HomeTab::Usage`). It totals token spend over a window across every project
an agent worked in, splits it per model into main-session and subagent spend,
draws a per-day strip, lists every agent run as a task, and lists the features
closed inside the window. It counts tokens only: there are no prices, and it
never estimates money.

## Query parameters

- `days` — the window, one of `1`, `7`, `14`, `30` (`views::USAGE_WINDOWS`).
  Any other value, or none, falls back to `7`.
- `project` — a project id narrows every figure to that project; `other`
  narrows to the **Other** bucket. Empty means everything. An unknown id
  matches nothing, so all totals read zero. The picker lists the registered
  projects with spend in the window, plus Other.

`/api/usage` takes the same two parameters and returns the `UsageSummary` the
tab renders, as JSON: `days`, `since_ms`, `now_ms`, `totals`, `projects`,
`models`, `per_day`, `features`, `runs`.

## Where the numbers come from (D6)

Token spend comes from the agent runtimes' own session logs, one usage event
per assistant message. The tab only reads these files and never writes to
them or to a project.

| Source | Files read | New | Cache-read |
|---|---|---|---|
| Claude Code | `~/.claude/projects/<dir>/<session>.jsonl`, plus native subagent transcripts at `<dir>/<session>/subagents/agent-*.jsonl` with their `agent-*.meta.json` | `input_tokens` + `output_tokens` + `cache_creation_input_tokens` | `cache_read_input_tokens` |
| pi | `~/.pi/agent/sessions/<dir>/*.jsonl` | `input` + `output` + `cacheWrite` | `cacheRead` |
| Codex | `~/.codex/sessions/**/*.jsonl` | the rest of the delta | the delta of `cached_input_tokens` |
| Paseo | `$PASEO_HOME/agents/<dir>/*.json` (default `~/.paseo`) | names runs only, no tokens | — |

- **Claude Code** repeats one message across several log lines; the parser
  keeps the last line per message id and request id. A subagent's events
  take the model from its meta file when the line carries none.
- **pi** names the model `<provider>/<model>`.
- **Codex** reports only running totals (`total_token_usage.total_tokens`
  and `cached_input_tokens`), never the input/output/cache-write split. Each
  `token_count` event that raises the total becomes one event: cache-read is
  the rise in cached input, and new is the rise in total minus that. The
  model is the first one named by `session_meta` or `turn_context`, else
  `codex`.
- A line that does not parse, or a message without a timestamp, is skipped.
- **`.bee/usage/<feature>.json`** (schema `bee-usage/v1`, written by
  `bee close`) now feeds only the finished-features list. Its sessions add
  nothing to the totals, models, strip or project rows, so a closed feature
  is never counted twice.

## Agent runs (D7)

A run is one log file: a Claude Code session, a Claude Code subagent
transcript, a pi session or a Codex session. The **Agent runs** table shows
one row per run with spend in the window, with task, project, provider,
model, start, duration, new and cache-read tokens. Subagents nest under their
parent, and the table shows at most 300 rows with a count of the rest.

- **Task name**: a Paseo agent whose provider and session id match the run
  supplies the name. That name is its `bee_cell` label when present, else its
  title. Without a Paseo match, a Claude Code session uses its `aiTitle` (or
  summary) and a subagent its meta `description`. A run with no name shows
  the first 8 characters of its session id.
- **Parent**: a run is a subagent when it is a Claude Code subagent
  transcript (parent: its session), or when its Paseo record carries a
  `paseo.parent-agent-id` label that names another Paseo agent (parent: that
  agent's run).
- **Model** is the model with the most tokens in the run. **Duration** runs
  from the run's first to its last usage event.

## How spend is counted

- **Window**: whole calendar days in the daemon's local time. A `days`-day
  window is today plus the `days - 1` local dates before it, and covers
  `[local midnight starting the oldest of those dates, now]`. So `1` means
  today since local midnight. `since_ms` in the JSON is that midnight.
- **Event-time bucketing (D6, supersedes D1)**: each usage event counts on
  the local day of its own timestamp. A run that spans midnight splits across
  both days, and events before the window drop out even when the run is
  counted.
- **Project mapping (D6)**: a run belongs to the registered project whose
  canonical root contains the run's canonical cwd, the deepest root winning.
  A `<root>--wt--<feature>` worktree folds into `<root>` first. A cwd under
  no registered project lands in **Other**. Bee and non-bee projects count
  alike, so today's work in an unclosed feature shows.
- **New vs cache-read (D3)**: `new` is input + output + cache write, and
  `cached` is cache-read, per the source table above.
- **Main vs subagent (D5, D7)**: a subagent run's events count as subagent
  spend; every other run's count as main-session spend, kept apart per model.
- **Per-day strip (D4)**: exactly `days` entries, one per calendar day ending
  today. The local offset is read once at startup
  (`usage::set_local_offset_secs`). The window and the strip cover the same
  local days, so the bars add up to the totals (decision 48a185e4).
- **Project rows**: spend per project. **Active** is the sum of its main
  runs' durations; **Features** counts the features closed in the window.
- **Feature rows**: one row per feature whose `closed_at` is inside the
  window, from its `.bee/usage` file. Each row shows the project, feature,
  close time, cells capped (counted from the archived cells), active time
  (the sum of its session durations) and the wall span from the first
  session start to close. The row carries the feature's whole spend,
  including sessions that ended before the window opened.

## Cache and warm-up

`AgentLogCache` keeps each parsed file keyed by path and re-parses a file
only when its mtime or length changes. A request skips session logs last
modified before `days + 1` days ago; Paseo records are always read. At
startup the daemon warms the cache in the background over the last 30 days,
so the first open of the tab does not parse every log.

## Pointers

- `crates/waggledance-core/src/agent_usage.rs` — `parse_claude`,
  `parse_claude_subagent`, `parse_pi`, `parse_codex`, `parse_paseo`,
  `apply_paseo` (titles and parents), `project_for` (canonical roots,
  worktree folding), `AgentLogCache` and its `runs`, `AgentHomes` (the four
  home directories, `PASEO_HOME`)
- `crates/waggledance-core/src/usage.rs` — `read_usage` (parses
  `.bee/usage/`), `summarize_runs` (window, event buckets, Other, run rows,
  feature rows), `set_local_offset_secs`
- `crates/waggledance/src/server.rs` — the `/api/usage` route, `api_usage`,
  `usage_summary` (project mapping, `other` filter, `spawn_blocking` read),
  the startup warm-up, and the `days` / `project` parsing in `RegisterFlag`
- `crates/waggledance/src/views.rs` — `usage_tab`, `usage_runs_table`,
  `USAGE_WINDOWS`, `HomeTab::Usage`
- `docs/history/usage-dashboard/CONTEXT.md` — decisions D1 to D7
