---
type: bee.delivery
title: commit-label-condense — delivery
description: "Delivery record for work item commit-label-condense: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-30
bee:
  id: commit-label-condense-delivery
  lifecycle: active
  required_context: [.bee/lanes/commit-label-condense.json]
  sources: [.bee/lanes/commit-label-condense.json, .bee/cells/clc-1.json]
---

# commit-label-condense — Delivery

## What shipped

- **clc-1** — The Comparing picker and header condense bee machine-commit
  subjects to one canonical name form — `<slug> · <action>` — since one
  feature's bee commits otherwise render under three different spellings
  (decision `4d620c33`).

## Verify

`PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance views::`
— picker tests green including new condense cases.

## Spec sync

Merged into `docs/specs/web-interface.md` ("Changes screen (git diff)").

## Deviations

None recorded in the capped cell trace.

## Provenance

Proposed by `bee knowledge promote --work commit-label-condense` from 1
capped cell trace, reviewed and applied by bee-capturing on 2026-08-31.
