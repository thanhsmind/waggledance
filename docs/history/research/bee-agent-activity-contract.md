---
artifact_contract: bee-research/v1
topic: bee-agent-activity-contract
depth: quick
date: 2026-08-22
---

# bee 2.20.0 agent-activity record — the contract waggledance reads

Source: beehive session, 2026-08-22; doc of record in the beehive repo
`docs/knowledge/areas/hook-runtime/agent-activity-record.md`. Verified
locally: this repo's `.bee/sessions/<id>.json` carries `activity` and
`<id>.activity.jsonl` after onboarding to 2.20.0 (`Local`).

## Per session — `<project>/.bee/sessions/<session_id>.json`

```json
"activity": {
  "state": "blocked",                 // working | waiting_input | blocked | idle | exited
  "event": "PermissionRequest",       // hook that produced the state
  "tool_name": "Bash", "tool_use_id": "toolu_01…",
  "at": "2026-08-22T13:31:49Z",       // last hook time — age it
  "pane": "w4:p4",                    // HERDR_PANE_ID when in a herdr pane
  "cwd": "/…/beehive--wt--agent-activity-hook",
  "feature": "agent-activity-hook",   // bound lane, else state.json feature
  "cell": "aah-4",                    // active claim's cell, absent when none
  "waiting_on_set_by_hook": true
}
```

History: `<session_id>.activity.jsonl`, last 50 transitions, same object +
`session_id` per row — diff the tail for notifications.

## Joins

- `activity.feature` → `.bee/lanes/<feature>.json` (phase, gates,
  waiting_on, route.lane, next_action) or `.bee/state.json`.
- `activity.cell` → `.bee/cells/<cell>.json` (title, status, files).
- `activity.cwd` → which checkout; `bee worktree list --json` maps grants.
- `activity.pane` → herdr pane id — the bridge to the terminal view.
- Shortcut: `bee status --json` → `workers[]` = `{session_id, lane, cell,
  last_heartbeat, activity, signal}` per live session.

## Read-time rules (never stored)

- `signal`: `live` when `activity.at` within 90 s, else `no_signal`; `null`
  for dead/closed sessions.
- Need-you = `state ∈ {blocked, waiting_input}`. **Approve only on
  `blocked`** (permission prompt); `waiting_input` is a question to answer
  by typing.
- Sticky: `blocked`/`waiting_input` never expire. `blocked` clears only on
  PostToolUse/PostToolUseFailure with the same `tool_use_id` (same
  `tool_name` when no id) or a turn boundary (UserPromptSubmit/Stop).
  Background subagent tool events never clear it. PermissionRequest may
  replace `waiting_input`.
- `waiting_on` on the lane/state record mirrors it: `gate` = blocked,
  `question` = waiting_input, `turn-end` = idle with control back to the
  human.

## State map (legend)

UserPromptSubmit / PreToolUse / PostToolUse / PostToolUseFailure → working ·
PermissionRequest, Notification:permission_prompt → blocked ·
Notification:agent_needs_input → waiting_input ·
Notification:idle_prompt | agent_completed, Stop → idle ·
SessionEnd (reason ≠ clear/resume) → exited. SubagentStop never wired.

## Per-project rollup (phone tiles)

Count sessions by state; need-you = blocked + waiting_input; group by
`activity.feature` ("feature X: 2 working, 1 blocked"); a tile is stale when
every session is `no_signal` or `exited`.

## Reader-side work queued

Backlog row "Read bee agent activity … render it in the cockpit"
(2026-08-22); decision `eecb8505`.
