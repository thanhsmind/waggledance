---
type: bee.delivery
title: diff-file-collapse — delivery
description: "Delivery record proposed by bee knowledge promote for work item diff-file-collapse: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-31
bee:
  id: diff-file-collapse-delivery
  lifecycle: active
  required_context: [docs/history/diff-file-collapse/CONTEXT.md]
  sources: [docs/history/diff-file-collapse/CONTEXT.md, .bee/cells/dfc-1.json]
---

# diff-file-collapse — Delivery

## What shipped

- **dfc-1** — Changes screen gains a per-file fold button and one Collapse all / Expand all, both scripting-only and unpersisted (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **dfc-1** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work diff-file-collapse` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/diff-file-collapse/CONTEXT.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.
