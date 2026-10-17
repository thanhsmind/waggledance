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

---

## Orchestrator Rules & Safety

1. **Check Live Leads Before Dispatch**:
   Always run `wd-bridge.py state --project <target>` before spawning. If a lead agent is already actively working or swarming, reuse or inform the user rather than spawning a competing run.
2. **Provenance Tracking**:
   Every spec drop should begin with the provenance header:
   ```text
   spec-drop <corr-id> from waggledance@<sha>
   ```
3. **Respect Workspace Boundaries**:
   This skill operates as the orchestrator seat. It monitors, queries state, and dispatches to project leads. It does not directly modify source code in target repositories or merge branches without user consent.
