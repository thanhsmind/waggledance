# anphabe intake — Context

**Feature slug:** anphabe-intake
**Date:** 2026-11-12
**Shaping session:** complete
**Scope:** Standard
**Domain types:** RUN | CALL

## Feature Boundary

The waggledance daemon pulls the code work that concerns the user out of the anphabe MCP (Visual Feedback comments and tasks), matches each item to a registered project, and hands the confident matches to that project's leader; it ends at the dispatch, and never writes anything back to anphabe.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Only code work is brought in: bugs, features, dashboards, websites, MCP functions. Digital and marketing work (ads, posts, campaigns, content) is never dispatched. A matched item goes to its project's leader to be done, not filed (decision ad9b778b). | The user: "chỉ có liên quan tới code thì mới mang về, còn những task digital thì không cần". |
| D2 | An item concerns the user when it is a Visual Feedback root comment the user wrote, that mentions the user, or that sits on a review page the user created, or an anphabe task the user owns or collaborates on (decision 389f77b0). The user's email is a config value, never a literal in code. | — |
| D3 | A daemon loop runs every 15 minutes. Each pass spawns one `claude -p` run on `claude-haiku-5-5` with only the mcp-gateway tools allowed; the run fetches and classifies and returns JSON (decision 905ef00e). | Spike 2026-11-12: 36 s, 4 turns, 0.012 USD per pass. |
| D4 | waggledance, not the model, owns the ledger, the threshold and the dispatch. An item with confidence of 0.8 or more and a registered project is dispatched with `leader: true` and `request_id` `intake-<source>-<id>`; a retry never sends twice (decision 905ef00e). | The model stays out of the path that can double-send or mis-route. |
| D5 | An item below 0.8, or with no project, is held: recorded in the ledger, listed by `waggledance intake list`, and announced once through the existing notify path. It is never dispatched by the loop (decision 905ef00e). | — |
| D6 | Nothing is written back to anphabe: no reply, no status change, no comment (decision 905ef00e). | Writes there are visible to colleagues and were not asked for. |
| D7 | The loop is opt-in through an `[intake]` config section, off by default; `waggledance intake run` makes one pass by hand. | Other hosts of waggledance have no anphabe access. |

### Agent's Discretion

D2-D7 are the agent's recommendations, offered to the user on 2026-11-12 and recorded under `gate_bypass = full`; the user can override any of them. Prompt wording, ledger schema, JSON shape, how the loop locates the `claude` binary, and the task text handed to the leader are planning's call. The task text carries the item's link, body, page URL, element selector and image links so the leader needs no anphabe lookup to start.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| item | One Visual Feedback root comment or one anphabe task, keyed by `<source>-<id>`. |
| held | An item the loop recorded but will not dispatch: below the threshold or with no project. |
| pass | One loop iteration: one Haiku run, then ledger update and dispatches. |

## Read set

### Knowledge

| Concept | Grade at read | Why it matters |
|---------|---------------|----------------|
| docs/knowledge/areas/supervisor-daemon.md | unverified | The daemon binds no socket and hosts the background loops; the agent CLI is a thin layer over `mcp::call_tool`. |
| docs/knowledge/patterns/paseo-leader-dispatch.md | pattern | How `waggledance_dispatch` with `leader: true` and `request_id` opens a leader and refuses a duplicate. |
| docs/knowledge/patterns/paseo-native-supervisor.md | retired | The earlier long-lived supervisor agent was retired by wd-direct-leader; this feature adds a pass-based loop, not a resident agent. |

### Code

| Path | What it decides |
|------|-----------------|
| `crates/waggledance/src/main.rs` | `TerminalBackground::reconcile*` starts and stops each background loop. |
| `crates/waggledance/src/worktree_sweep.rs` | The shape of a periodic, opt-in daemon loop with a first-tick delay. |
| `crates/waggledance-core/src/config.rs` | Config sections and their defaults (`WorktreeSweepConfig`). |
| `crates/waggledance/src/mcp.rs` | `call_tool` and the dispatch tool the loop reuses. |
| `crates/waggledance/src/agent_cli.rs` | Where the `intake` verbs join the CLI. |
| `crates/waggledance-core/src/repository.rs` | `SCHEMA` and `MIGRATIONS`, home of the ledger table. |

## Specific Ideas And References

- The spike prompt and its output are in this conversation's record (2026-11-12): Haiku paired the "+ câu con" task with the identical Visual Feedback comment, and gave `null` to items on `lms.anphanext.vn`.
- anphabe-goglbe already drops Webinar, Event, Resource (`document`) and Project rows from `list_my_tasks` (commit 1dbde9f, deployed version 9de59a68). `total_count` still counts the dropped rows; the loop counts the array.

## Canonical References

- docs/knowledge/index.md — Knowledge bundle index (see "Critical patterns")

## Outstanding Questions

- Duplicate pairs (a task and a comment for the same bug): dispatch once, keyed by the first item, and mark the twin as covered. Planning confirms how the JSON names the twin.
