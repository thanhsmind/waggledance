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

### 8. `handoff` — give a task to the waggledance supervisor

This is the default way to get work done in a project. The supervisor (one Paseo agent for every project) picks the project, opens or reuses its leader, and reports back into this session.
```bash
wd-bridge.py handoff --task "Thêm nút xuất CSV cho trang báo cáo"
wd-bridge.py handoff --task-file /tmp/spec.md
wd-bridge.py handoff --task "..." --corr t-1a2b3c
wd-bridge.py handoff --task probe --dry-run
```
It runs `waggledance supervisor ensure --json` (finds the supervisor, or starts it), mints a correlation id `t-<6 hex>` when `--corr` is absent, and sends `[handoff <corr>] <task>` with `paseo send <supervisor_id> --no-wait`. It prints `{"supervisor_id": ..., "corr": ..., "sent": true}`. Keep the `corr`: every later message about this task carries it. `--dry-run` prints the two commands and runs nothing.

### 9. `reply`, `redo`, `accept` — relay the user's answer
```bash
wd-bridge.py reply --corr t-1a2b3c --text "Dùng định dạng ngày dd/mm/yyyy"
wd-bridge.py redo --corr t-1a2b3c --text "Nút phải nằm bên phải"
wd-bridge.py accept --corr t-1a2b3c
```
Each sends one message to the supervisor the same way `handoff` does, and accepts `--dry-run` too.

### Message shapes

What this skill sends to the supervisor, exactly:

| Command | Message |
|---|---|
| `handoff` | `[handoff <corr>] <task>` |
| `reply` | `[reply <corr>] <text>` |
| `redo` | `[redo <corr>] <text>` |
| `accept` | `[accept <corr>]` |

What comes back: the supervisor's report arrives in this session as a turn, tagged `[waggledance#<corr>] <project> — <kind>: <text>`, where the kind is a result (kết quả), a question (câu hỏi), waiting on a permission (đang chờ quyền), an error (lỗi), closed (đã đóng), progress (tiến độ) or the task board (bảng việc). Your reply to that turn goes to the user on Telegram.

### Handling a report or a board

1. Check the report against the task you handed off under that `corr`: does it answer what the user asked? Say plainly when it does not, or when a result has no proof or leaves work undone.
2. Write the user one clear message in Vietnamese. Start it with the tag `[waggledance#<corr>] <project> — <kind>` unchanged. Quote numbers, paths, commands and the leader's words verbatim.
3. A question: put it to the user plainly, verbatim, and wait for their answer. Never answer a leader's question on the user's behalf, and never pick an option for them; relay their answer with `reply`.
4. A board (`bảng việc`): list each open task with its project, step, who it waits on and since when. Put the questions waiting on the user first, each with how long it has waited.

### Reading the user's answer

- A bare `ok` or `được` from the user means `accept` of the newest open `corr` — the newest `[waggledance#<corr>]` result not yet accepted.
- A sentence that answers a question means `reply` with that question's `corr` and the user's words as `--text`.
- A request to change or do again what was delivered means `redo` with its `corr`.
- When a sentence could answer either of two open questions, ask the user which one, naming both tags.

### When `handoff` fails

`handoff`, `reply`, `redo` and `accept` exit non-zero with one line naming Paseo or waggledance. Then tell the user that Paseo or waggledance is down and the task did not go through. Never say the task was handed off unless the command printed `"sent": true`.

### Known limit

Two `handoff` calls racing while no supervisor exists may start two supervisors: `ensure` takes no lock. After the race the next `ensure` returns the supervisor `paseo ls` lists first, which is the newest, so later messages all reach one consistent supervisor.

---

## Orchestrator Rules & Safety

1. **Prefer `handoff` for new work**:
   Hand tasks to the supervisor with `handoff`; it checks live leaders itself. Use `dispatch` only when the user asks for a specific preset or pane.
2. **Check Live Leads Before Dispatch**:
   Always run `wd-bridge.py state --project <target>` before spawning. If a lead agent is already actively working or swarming, reuse or inform the user rather than spawning a competing run.
3. **Provenance Tracking**:
   Every spec drop should begin with the provenance header:
   ```text
   spec-drop <corr-id> from waggledance@<sha>
   ```
4. **Respect Workspace Boundaries**:
   This skill operates as the orchestrator seat. It monitors, queries state, and dispatches to project leads. It does not directly modify source code in target repositories or merge branches without user consent.
