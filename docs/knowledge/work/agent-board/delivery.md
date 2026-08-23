---
type: bee.delivery
title: agent-board — delivery
description: "Delivery record for work item agent-board: 2 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-11
bee:
  id: agent-board-delivery
  lifecycle: active
  areas: [bee-cockpit-board]
  required_context: []
  sources: [docs/history/agent-board/CONTEXT.md, docs/history/agent-board/plan.md, .bee/cells/agent-board-1.json, .bee/cells/agent-board-2.json]
---

# agent-board — Delivery

## What shipped

- **agent-board-1** — Replaced the phase board with the agent Kanban board skeleton (Todo/In Progress/Done from D7 buckets, agent badges, blocked markers, honest Backlog/Review placeholders); retired bee_phase_board_section/bee_phase_card/LIFECYCLE_ORDER and their tests, replaced with agent-board equivalents; cargo test --workspace green (742 passed). (2 file(s) changed)
- **agent-board-2** — Backlog and Review columns filled, Done overflow collapses into a details summary (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **agent-board-1** — `cargo test --workspace`
- **agent-board-2** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work agent-board` from 2 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/agent-board/CONTEXT.md`, `docs/history/agent-board/plan.md`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
