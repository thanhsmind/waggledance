# Usage Dashboard — Context

**Feature slug:** usage-dashboard
**Date:** 2026-10-23
**Shaping session:** complete
**Scope:** Standard
**Domain types:** SEE | READ

## Feature Boundary

A new **Usage** tab on the waggledance home page that shows, across every
registered bee project, how many tokens each project spent, which models
(main session and subagents) spent them, and which features were completed and
how long they took — over a selectable 1-day, 7-day, 14-day or 30-day window.
It reads data bee already writes; it never changes bee, never estimates money,
and never writes to any project.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Token spend is bucketed by each session's end time (`ended_ms` in `.bee/usage/<feature>.json`) into the window. A feature that spans several days spreads across the days its sessions ended. A feature not yet closed has no usage file and contributes no spend. (decision 892cf93d) | A session whose `ended_ms` is inside the window counts in full; sessions are not split mid-session. |
| D2 | The dashboard is a new home tab named **Usage**, beside Board and Agents, aggregating every registered bee project, with a per-project filter. (decision 5239a98b) | New `HomeTab` variant and tab-bar entry. |
| D3 | Tokens only — no USD, no price table. Tokens split into **new** (input + output + cache write) and **cache-read**, per model. (decision be26628f) | Use the `new` / `cached` fields the usage file already carries. |
| D4 | Windows offered: 1 day, 7 days, 14 days, 30 days, measured back from now in the daemon's local time. | From the request. |
| D5 | Main-session tokens and subagent tokens are shown separately per model (`models` vs `subagent_models`). | From the request: "mỗi model subagent dùng hết bao nhiêu token". |
| D6 | Supersedes D1 (uat feedback, decision fc8c944a). Token spend comes from the agent runtimes' own session logs, per message: Claude Code (`~/.claude/projects`, native subagent transcripts included), pi (`~/.pi/agent/sessions`) and Codex (`~/.codex/sessions`). Each message counts on the local day of its timestamp and on the registered project whose canonical root contains its canonical cwd; a `<root>--wt--<feature>` worktree folds into `<root>`; anything else is **Other**. Bee usage files only feed the completed-features list. | Today's work in unclosed features and non-bee projects must show. |
| D7 | The tab lists agent runs as tasks: one row per Paseo agent or Claude Code subagent, named by the Paseo title (the `bee_cell` label when present) or the subagent description, with provider, model, project, parent, start, duration and tokens. A run is a subagent when Paseo records `paseo.parent-agent-id` or it is a Claude Code subagent transcript. (decision eac8b0f9) | User wants Paseo-level per-task, per-subagent model detail. |

### Agent's Discretion

- What a "completed task" row is: one closed feature (a usage file), showing
  project, feature, close time, cells capped, active agent time (sum of session
  durations), wall span (first session start to close), and its tokens by model.
- Layout, charts, ordering and empty-state wording, within the existing atelier
  design system and with no new JS dependency.
- Whether the data also comes out as JSON at an `/api/...` route (useful for
  tests and other consumers).

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| new tokens | `input + output + cache_write` — the usage file's `new` field |
| cache-read tokens | the usage file's `cached` field |
| active time | sum over sessions of `ended_ms - started_ms` |
| agent run | one provider session: a Paseo agent, a Claude Code session or subagent transcript, a pi session, or a Codex session |
| completed feature | a feature with a `.bee/usage/<feature>.json` file (written at `bee close`) |

## Existing Code Context

### Reusable Assets

- `crates/waggledance-core/src/bee.rs` — `read_snapshot` (~:1291), `read_rollup`
  (~:1543), `read_archived_cells` (~:2596), `BeeVelocity` (~:514) as the one
  existing time-window aggregate. Nothing reads `.bee/usage/` today.
- `crates/waggledance/src/server.rs` — `BeeSnapshotCache` (~:1188),
  `compute_bee_fingerprint` (~:1159), `cross_project_rollup` (~:1254),
  `is_bee_project` (~:3520).

### Established Patterns

- Server-rendered HTML strings in `views.rs`, embedded `assets/app.css` /
  `assets/app.js`, no frontend build step.
- `waggledance-core` is synchronous and framework-free; the server fans reads
  out over `spawn_blocking`.
- Route tests inline in `server.rs` `mod bee_route_tests` with the `oneshot`
  harness (`fresh_root`, `write`, `build_state`, `get`).

### Integration Points

- `views.rs` `enum HomeTab` (~:173) and `home_tabbar` (~:540).
- `server.rs` `fn router` (~:424) and `index_page` (~:882).

## Canonical References

- `.bee/usage/<feature>.json` — schema `bee-usage/v1`: `feature`, `closed_at`,
  `sessions[] {session_id, models, subagent_models, subagent_count, started_ms,
  ended_ms, totals}`, `skipped`, `totals`.
- `docs/knowledge/work/homepage-tabs/`, `docs/knowledge/work/agent-board/index.md`.

## Outstanding Questions

### Resolved In Planning

- [x] Cache: the bee-file path rides the existing `.bee/` fingerprint walk; the
  agent-log path got its own per-file `AgentLogCache` (plan.md Revisions 2-3).
- [x] Live updates: the Usage tab reloads with the existing `/ws` reload; agent
  logs are re-read incrementally on each view.

## Rejected And Superseded Ideas

- Estimated USD cost per model: rejected by D3; it would need a price table.
- Spend of features still in flight: covered by D6, which reads the agent logs
  directly instead of waiting for `bee close`.
