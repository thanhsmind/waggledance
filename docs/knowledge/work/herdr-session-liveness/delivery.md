---
type: bee.delivery
title: herdr-session-liveness — delivery
description: "Delivery record for work item herdr-session-liveness: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: herdr-session-liveness-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: []
  sources: [.bee/cells/hsl-1.json, docs/specs/bee-cockpit.md]
---

# herdr-session-liveness — Delivery

## What shipped

- **hsl-1** — A bee session record stays joined to its pane while herdr still hosts its session id (`agents[].agent_session.value`, parsed into `Agent.session_id`), even after its own heartbeat has aged past the thirty-minute window. A session blocked on a gate question fires no hook event, so heartbeat alone dropped it inside the hour and the pane fell back to herdr's *idle* reading of that dialog; now *needs approval* stays on the pane until the answer lands (3 file(s) changed).

## Verify

`cargo test -p waggledance --release` — 839 passed; `cargo clippy -p waggledance --release --all-targets -- -D warnings` — clean. Commits e34e400, 6e3268f.

## Learning

A liveness witness that only ticks on activity cannot see a wait. The hook writes on events; a dialog has none. Pair it with a witness that sees the process (herdr's pane list), and let either one keep the record alive.
