---
name: waggledance-orchestrator
description: "Orchestrate multi-project workflows across repositories managed by Waggledance and Beehive. Supports monitoring real-time project states, querying registered workspaces, full-text searching indexed markdown docs, and dispatching tasks (relay/execute mode) to worker agent panes via Waggledance."
---

# Waggledance Orchestrator

This skill enables OpenClaw to communicate with the local **Waggledance** instance (daemon at `http://127.0.0.1:7700` and MCP bridge) to orchestrate, monitor, search, and dispatch tasks across all managed projects (`beehive`, `waggledance`, `anphabe-bi-dashboard`, `a-blog`, `memorypad`, `jarvis`, etc.).

## Available Capabilities

1. **Project Inspection & Rollup**:
   - Query all registered repositories, root paths, and indexing status.
   - Read parsed Beehive project state (active feature, phase, cell counts, recent decisions, open P1 review findings, agent panes).
2. **Global Doc & Knowledge Search**:
   - Full-text search across documentation, decisions, and knowledge bases in one or all projects.
3. **Task & Spec Orchestration (Dispatch Door)**:
   - **Relay mode**: File specs/PBIs into a target project's backlog with correlation tracking (`spec-drop <corr-id> from waggledance@<sha>`).
   - **Execute mode**: Dispatch a lead agent in the target repo to run its full working chain to completion.
4. **Markdown Preview**:
   - Open and generate shareable local browser URLs for markdown documents.

---

## Tooling Interface: `wd-bridge.py`

The skill provides an executable CLI wrapper at:
```bash
/home/thanhsmind/aicoworker/openclaw/workspace-main/skills/waggledance-orchestrator/bin/wd-bridge.py
```

### Commands

### 1. `projects`
List all registered projects:
```bash
wd-bridge.py projects
```

### 2. `state`
Get parsed state, active feature, cell counts, and live pane inventory:
```bash
# Rollup of all projects
wd-bridge.py state

# Narrow to a specific project (e.g. beehive)
wd-bridge.py state --project beehive
```

### 3. `search`
Search across indexed project documentation:
```bash
wd-bridge.py search "herding transport" --limit 5
wd-bridge.py search "pipeline status" --project anphabe-bi-dashboard
```

### 4. `runs`
Inspect dispatched agent runs:
```bash
wd-bridge.py runs [--project <project_id>]
```

### 5. `dispatch`
Dispatch a task or spec to an agent pane:
```bash
# Dispatch via preset (e.g., claude-sonnet, pi-gpt-5.6-luna, agy-flash)
wd-bridge.py dispatch --project beehive --preset agy-flash --task "spec-drop ..."

# Or target an existing idle pane directly
wd-bridge.py dispatch --project beehive --pane-id w1:p3A --task "..."
```

### 6. `await`
Wait for a run to finish (up to 60s per call):
```bash
wd-bridge.py await <run_id> --timeout 60
```

### 7. `open`
Preview markdown file in browser via Waggledance:
```bash
wd-bridge.py open docs/decisions/0012-model-tiers-config.md --project-root /home/thanhsmind/Projects/goglbe/beehive
```

### 8. `handoff` — give a task to a project's leader

This is the default way to get work done in a project. You pick the project, the daemon
opens or reuses that project's own leader, and the leader's reports come back into this
session through waggledance's inbox.

Pick the project before you hand off: read the user's words and the project list
(`wd-bridge.py projects`), and name the project you chose when you tell the user what you
are doing. When the words could mean more than one project, ask the user which one and
wait — never guess a project silently.

```bash
wd-bridge.py handoff --project beehive --task "Thêm nút xuất CSV cho trang báo cáo"
wd-bridge.py handoff --project beehive --task-file /tmp/spec.md
wd-bridge.py handoff --project beehive --task "..." --corr t-1a2b3c
wd-bridge.py handoff --project beehive --task probe --dry-run
```

It mints a correlation id `t-<6 hex>` when `--corr` is absent, runs `waggledance mcp`
with `WAGGLEDANCE_CALLER=wd`, and calls `waggledance_dispatch` with
`{project, task, leader: true, request_id: <corr>}`. It prints
`{"corr": ..., "run_id": ..., "project": ..., "status": "queued", ...}`. Keep the
`corr`: every later message about this task carries it. `--dry-run` prints the command and
the MCP request and runs nothing.

Hand-off does not wait for the work: the daemon queues the task for that project's leader
and delivers it when the leader is idle, so a `queued` run id is a sent task, not a
finished one. Say what you handed off and to which project; do not report the work done.

### 9. `reply`, `redo`, `accept`, `permit` — relay the user's answer
```bash
wd-bridge.py reply --corr t-1a2b3c --text "Dùng định dạng ngày dd/mm/yyyy"
wd-bridge.py redo --corr t-1a2b3c --text "Nút phải nằm bên phải"
wd-bridge.py accept --corr t-1a2b3c
wd-bridge.py permit allow t-1a2b3c
wd-bridge.py permit deny t-1a2b3c perm-42
```
Each runs `waggledance mcp` with `WAGGLEDANCE_CALLER=wd` and calls `waggledance_task` with
`{corr, action, text?, req_id?}`, and accepts `--dry-run` too.

- `reply` and `redo` queue the user's words for the task's leader; the leader gets them
  under the same `waggledance#<corr>` tag.
- `accept` closes the task and archives its leader when no other task or queued run still
  uses it; it prints `"archived": false` and why otherwise.
- `permit allow|deny <corr> [req_id]` answers the leader's permission request; without a
  `req_id` it answers the newest unanswered request of that task.

A command that fails exits non-zero with one line naming the cause, such as an unknown or
closed corr, or a task whose leader has not received it yet. Tell the user it did not go
through.

### Message shapes

What `handoff` sends: the dispatch request `{project, task, leader: true, request_id:
<corr>}` — the leader receives `Task waggledance#<corr> from wd.` followed by the mail rule,
the advisor rule, and the task text unchanged.

What comes back: a turn in this session per message, tagged
`[waggledance#<corr> #<seq>] <project> — <kind label>: <text>`. `<seq>` is the inbox row's
number and the kinds are a result (kết quả), a question (câu hỏi), progress (tiến độ), an
error (lỗi), a permission request (xin quyền), closed (đã đóng) or the task board (bảng
việc). Your reply to that turn goes to the user on Telegram.

A `#<seq>` you have already answered can arrive again: waggledance marks a message read
only after your turn ends, so a turn that failed redelivers the same row with the same
`#<seq>`. Treat a repeated `#<seq>` for the same corr as a redelivery — skip it, and do not
write the user a second time for it.

### Handling a report or a board

1. Check the report against the task you handed off under that `corr`: does it answer what the user asked? Say plainly when it does not, or when a result has no proof or leaves work undone.
2. Write the user one clear message in Vietnamese. Start it with the tag `[waggledance#<corr>] <project> — <kind>` unchanged. Quote numbers, paths, commands and the leader's words verbatim.
3. A question: put it to the user plainly, verbatim, and wait for their answer. Never answer a leader's question on the user's behalf, and never pick an option for them; relay their answer with `reply`.
4. A board (`bảng việc`): list each open task with its project, step, who it waits on and since when. Put the questions waiting on the user first, each with how long it has waited.
5. A permission request (`xin quyền`): name the project, the tool or command, and its `req_id`, and ask the user to allow or deny it. Never decide one yourself: run `permit` only with the answer the user gave.

### Reading the user's answer

- A bare `ok` or `được` from the user means `accept` of the newest open `corr` — the newest `[waggledance#<corr>]` result not yet accepted.
- The user's allow or deny of a permission request (`cho phép`, `từ chối`) means `permit allow` or `permit deny` with that request's `corr` and `req_id`. Never choose allow or deny for the user, and never read a bare `ok` as an allow.
- A sentence that answers a question means `reply` with that question's `corr` and the user's words as `--text`.
- A request to change or do again what was delivered means `redo` with its `corr`.
- When a sentence could answer either of two open questions, ask the user which one, naming both tags.

### When `handoff` fails

`handoff` exits non-zero with one line naming waggledance. Then tell the user that
waggledance is down and the task did not go through. Never say the task was handed off
unless the command printed a `run_id`.

### Known limit

A project whose leader is busy with another caller's task parks the wd task on the same
project until that leader is idle. Hand-off still returns at once with `queued`.

---

## Orchestrator Rules & Safety

1. **Prefer `handoff` for new work**:
   Hand tasks to a project's leader with `handoff`; the daemon checks live leaders itself. Use `dispatch` only when the user asks for a specific preset or pane.
2. **Check Live Leads Before Dispatch**:
   Always run `wd-bridge.py state --project <target>` before spawning. If a lead agent is already actively working or swarming, reuse or inform the user rather than spawning a competing run.
3. **Provenance Tracking**:
   Every spec drop should begin with the provenance header:
   ```text
   spec-drop <corr-id> from waggledance@<sha>
   ```
4. **Respect Workspace Boundaries**:
   This skill operates as the orchestrator seat. It monitors, queries state, and dispatches to project leads. It does not directly modify source code in target repositories or merge branches without user consent.
