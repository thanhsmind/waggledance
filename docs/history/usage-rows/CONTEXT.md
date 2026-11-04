# Usage Rows — Context

**Feature slug:** usage-rows
**Date:** 2026-11-04
**Shaping session:** complete (spec-drop sup-20261104-w8d2 from agent-works, owner-approved plan (a))
**Scope:** Standard
**Domain types:** READ | CONTRACT

## Feature Boundary

A new JSON endpoint `GET /api/usage/rows` on the daemon that hands agent-works
one flat row per (day, project, agent, provider, model, session), so its Usage
page can split waggledance spend per project. To fill the rows, the transcript
reader keeps each token kind and each transcript-recorded cost apart, counts
messages, tool calls and errors, and learns to read opencode's sqlite store.
Dispatched runs remember their agent's transcript session id so a row can name
the preset that started it. The existing Usage tab and `/api/usage` keep their
behaviour.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | `UsageEvent` keeps `input`, `output`, `cache_read` and `cache_write` separately, plus the transcript's own cost when it records one (pi today, opencode when present), split into input/output/cacheRead/cacheWrite parts and a total. The old `new` / `cached` values become derived (`new = input + output + cache_write`, `cached = cache_read`). | Owner spec item 1; the Usage tab must not change. |
| D2 | `GET /api/usage/rows?from=YYYY-MM-DD&to=YYYY-MM-DD` returns `{from, to, generatedAt, projects:[{id,name}], rows:[…]}` with exactly the field names of the spec: `day, team, project, agent, provider, model, session, preset, runTitle, parent, subagent, input, output, cacheRead, cacheWrite, tokens, cost, costInput, costOutput, costCacheRead, costCacheWrite, costSource, messages, toolCalls, errors`. `team` is always `"waggledance"`. | Owner spec item 2; agent-works builds against this shape in parallel. |
| D3 | Days are Asia/Saigon calendar days (fixed UTC+7, no DST), `from`/`to` inclusive. Neither given = the 30 days ending today. Only `from` = through today; only `to` = the 30 days ending `to`. A malformed date, `from > to`, or a span over 366 days answers `400 {"error": "<why>"}`. | Spec fixes the zone; the one-sided and span-cap cases are agent discretion. |
| D4 | `project` is the registered waggledance project whose root contains the message's cwd (a `--wt--` worktree folds into its root), else `"other"`. All registered projects count — not only Paseo-managed ones. `projects` lists every registered project plus `{id:"other",name:"Other"}`. | Spec says "registered project id"; the Paseo filter of e24c8d4b belongs to the Usage tab only. |
| D5 | `agent` is the preset label of the waggledance run whose stored session id matches the transcript session, else the agent kind (`claude`, `pi`, `codex`, `opencode`). `provider` is the model's provider (`anthropic` for Claude Code, `openai` for Codex, the transcript's own provider for pi and opencode). `model` is the bare model id (no `provider/` prefix). | Spec item 2. |
| D6 | `session` is the transcript identity: the session id, and `<session>/<agent-file>` for a Claude Code native subagent. `parent` is the parent's session in the same form, or null. `preset` is the matched run's preset label or null; `runTitle` is the transcript/Paseo title, else the first line of the matched run's task, else null. | Keeps a subagent's row apart from its parent's under the spec's grouping key. |
| D7 | `cost` and its four parts are null and `costSource` null when no message in the row recorded a cost; otherwise they are the sums of the recorded costs and `costSource` is `"transcript"`. No price table, no estimate (be26628f stands). | Spec item 2; agent-works prices null-cost rows. |
| D8 | `messages` counts assistant messages that carried usage or an error; `toolCalls` counts tool-use items in them (Claude `tool_use`, pi `toolCall`, Codex `function_call`/`custom_tool_call` lines, opencode `tool` parts); `errors` counts messages that ended in an error (pi `stopReason: "error"`, Claude `isApiErrorMessage`, opencode `error`, Codex `error` events). `tokens = input + output + cacheRead + cacheWrite`. | Agent discretion on what counts, recorded so agent-works reads it the same way. |
| D9 | The `runs` table gains a nullable `session_id` column. A spawned (herdr) dispatch writes it from herdr's `agent_session` for the new pane as soon as herdr reports one — at dispatch, and on any later await poll that still finds it empty. Paseo-delivered runs carry no preset label, so they are out of this column's scope. | Spec item 3. |
| D10 | opencode is read from `<data>/opencode/opencode.db` (`$XDG_DATA_HOME`, default `~/.local/share`), read-only, from the `session`, `message` (JSON `data`) and `part` tables; agy-native protobuf is out of scope. | Spec item 4. |

### Agent's Discretion

- Where the row builder lives (core module beside `agent_usage`), test shape,
  and the exact opencode SQL.
- Row order: by day, then project, agent, provider, model, session.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| row | one (day, project, agent, provider, model, session) group with summed numbers |
| agent kind | the runtime that wrote the transcript: claude, pi, codex, opencode |
| preset label | the waggledance dispatch preset that spawned the pane (`runs.preset_label`) |

## Existing Code Context

- `crates/waggledance-core/src/agent_usage.rs` — per-runtime transcript reader, `AgentLogCache::runs`, `project_for`, `fold_worktree`.
- `crates/waggledance-core/src/usage.rs` — `summarize_runs` consumes `e.new` / `e.cached` (:412).
- `crates/waggledance/src/server.rs` — `/api/usage` route (:441), `usage_summary` (:1436).
- `crates/waggledance-core/src/repository.rs` — `runs` table, `ALTER TABLE runs ADD COLUMN …` migrations (:1079-1126).
- `crates/waggledance/src/orchestrate.rs` — `dispatch_run` (:341), await poll with herdr snapshot (:730).
- `crates/waggledance/src/herdr/wire.rs:70` — `Agent.session_id` from herdr `agent_session.value`.

## Outstanding Questions

None blocking. The opencode store on the dev host is empty (0 messages), so the
opencode reader is proven against a fixture database built in the test.
