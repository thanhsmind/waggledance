---
type: bee.delivery
title: agents-drawer-feature — delivery
description: "Delivery record for work item agents-drawer-feature: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: agents-drawer-feature-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/cells/archive/agents-drawer-feature/adf-2.json]
---

# agents-drawer-feature — Delivery

## What shipped

Every row of the Agents drawer carries a second line naming the feature the
agent is working (its resolved feature — see agent-feature-resolution), in a
monospace caption under the agent's name; the line is absent when the session
is bound to nothing. Before this the feature lived only in the row's hover
title, which a touch screen can never reach, so a phone user saw three rows
named "Claude Code" with no way to tell them apart. Decision logged
2026-08-23 (Agents drawer row second line).

## Verify

`cargo test -p waggledance --quiet` green, plus a manual check of the drawer
on a handset width.

## Deviations

None recorded in the capped cell trace.
