---
type: bee.delivery
title: shell-input-newline — delivery
description: "Delivery record for work item shell-input-newline: 1 capped cell joining newline-split reply text into one line for shell panes before send."
timestamp: 2026-08-16
bee:
  id: shell-input-newline-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/specs/agent-terminal.md]
  sources: [.bee/cells/shell-input-newline-1.json]
---

# shell-input-newline — Delivery

## What shipped

- **shell-input-newline-1** — when the target of a terminal input is a shell
  pane (no agent joined by pane id in the pane-host snapshot, the same rule
  the pane list uses), the sent text is split on newlines, each line trimmed,
  empties dropped and the rest joined with one space; agent panes keep text
  verbatim. Root cause reproduced live: the pane host sends each `\n` as
  Enter, so a shell ran each fragment as its own command. `server.rs`;
  commit `f1776cf`.

## Verify

`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — green at cap (2026-08-16).

## Deviations

- The unassigned-pane route was left untouched: it builds from agents only,
  so a shell pane is unreachable there.
- Baseline run skipped pre-claim (main clean at `011fea7` with green CI);
  full suite run at finish.

## Provenance

Written at the 2026-08-23 compound run from the capped cell trace; behaviour
merged into `docs/specs/agent-terminal.md` ("Replying to an agent").
