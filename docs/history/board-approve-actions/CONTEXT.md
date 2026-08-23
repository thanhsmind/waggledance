# Board Approve Actions — Context

**Feature slug:** board-approve-actions
**Date:** 2026-08-23
**Shaping session:** complete
**Scope:** Standard
**Domain types:** SEE | CALL

## Feature Boundary

A feature card on the console board carries one Approve + Reject pair for
its current human stop — the UAT gate, the merged shape+execution gate, or
the agent's permission prompt — and pressing it writes the answer into bee
(through the project's own `bee` CLI) or into the bound herdr pane, then
wakes the waiting session. It ends before any button that starts an agent
run (review, compound, start-todo — those are `board-run-actions`).

Research of record: `docs/history/research/board-actions-from-orchestrators.md`.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 (608e7903) | A feature card shows one Approve + Reject pair for its current stop: UAT gate, merged shape+execution gate, or the agent's permission prompt (`activity.state == blocked`). Reject unapproves the gate / sends the refusal into the pane. | One pair per card; Reject makes a wrong click reversible on the board. |
| D2 (9b6c3c25) | Gate approvals (UAT, shape+execution) open a short confirm dialog naming the feature; the permission-prompt Approve/Reject is one click, matching the terminal button. | Gates are irreversible stops. |
| D3 (3da23985) | Board write actions are enabled per project by the existing `orchestration.enabled` opt-in; a project without it shows only the waiting-on badge, never a button. | No new config key. |
| D4 (7b728f87) | After a gate approval the daemon writes the gate through the project's own bee CLI, then sends one line into the pane bound to the feature (session `activity.pane`) so the waiting session wakes and its waiting-on mark clears; with no live pane only the gate is written. | A board click is not a user message; the pane line stands in for it. |
| D5 (442611f6) | README's "the board never writes" invariant is replaced by: the board writes only through the project's own bee CLI or a herdr pane, only on an explicit human click in an opted-in project, and never decides what to do — it relays. | README and knowledge docs carry the new wording. |

### Agent's Discretion

- Exact wording of the confirm dialog and the pane wake-up line.
- Whether the pair renders on the home board, the per-project board, or both — must be consistent with where the waiting-on badge already renders.
- In-flight affordance (button disabled + spinner until the `changed` WebSocket event lands) and the idempotency key that prevents a double fire across the page reload.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| current stop | The one thing the feature waits on a human for right now: `uat` pending with cells all capped, merged gate pending, or a bound session with `activity.state == blocked`. |
| bound pane | The herdr pane named by the feature's live session record `activity.pane`. |
| relay | The board's role: it carries a human click into bee or a pane and mirrors the result; it never originates a decision. |

## Existing Code Context

### Reusable Assets

- `crates/waggledance/src/server.rs:1455-1478` — `POST /api/projects/:id/pbi` spawns the project's `.bee/bin/bee …` and 409s without a vendored bee; the precedent for a gate write.
- `crates/waggledance/src/server.rs:2942` — `POST /p/:id/_terminal/:pane_id/input` (herdr `send_input`); the existing Approve button posts `{text:"Approve", submit:true}` here.
- `crates/waggledance/src/views.rs:1581-1584`, `:2290-2295` — terminal Approve button and its `blocked`-only enable rule (decision 110d9120).
- `crates/waggledance-core/src/bee.rs:203-216`, `:586-610` — live `waiting_on` rule and session `activity` (`state`, `pane`, `feature`).
- `docs/history/orchestrator-dispatch/CONTEXT.md` D6 — `orchestration.enabled` per-project opt-in and its reader.

### Established Patterns

- Host-header allowlist as the only request guard (`server.rs:552-587`); every pane route is cwd-contained to the project boundary.
- Board re-renders via the `/ws` `{"changed":[…]}` event → `location.reload()`.

### Integration Points

- `crates/waggledance/src/views.rs:3942-4017` — card membership / current-stop derivation (`bee_gate_current_stop` at `:2651` excludes uat today; the card needs uat as a stop).
- `README.md:128-131` — the sentence D5 replaces.
- `.bee/backlog.jsonl:13` — "Agent board V2: on-board Approve/Reject buttons" is this feature.

## Canonical References

- `bee gate --name uat|--merge --approved true|false --lane <feature>` — the gate write; `--actor auto` is refused for uat (human-only).
- `docs/history/research/bee-agent-activity-contract.md:52-59` — approve only on `blocked`; sticky-clear rules.

## Outstanding Questions

### Deferred To Planning

- [ ] Does `bee gate` accept a `--reason` for `--actor user`, so the record can say "board click"? — read `bee gate --help --json`; if not, the wake-up line is the only provenance.
- [ ] Which bee CLI refusals (advisor precondition, plan-conflict precondition, `WORKTREE_MERGE_UAT_PENDING`) must surface verbatim on the card instead of a generic error.

## Deferred Ideas

- More canned replies (Retry, free-text reply) on the card — `docs/history/terminal-approve-button/CONTEXT.md:71-73` already defers this.
