promote proposal for work item "term-keys-one-row" (.bee/logs/scribing-runs.jsonl) — 1 capped cell(s): tko-1
anchor: ledger — .bee/logs/scribing-runs.jsonl
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/term-keys-one-row/delivery.md

---
type: bee.delivery
title: term-keys-one-row — delivery
description: "Delivery record proposed by bee knowledge promote for work item term-keys-one-row: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: term-keys-one-row-delivery
  lifecycle: active
  areas: [terminal-pane]
  required_context: [.bee/logs/scribing-runs.jsonl]
  sources: [.bee/logs/scribing-runs.jsonl, .bee/cells/tko-1.json]
---

# term-keys-one-row — Delivery

## What shipped

- **tko-1** — Terminal keys share one row on a handset (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **tko-1** — `cargo test -p waggledance -- term_key term_controls`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work term-keys-one-row` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "term-keys-one-row" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-22T11:30:17.911Z), the work item declares no bee.areas.

area terminal-pane:
  - [tko-1] Terminal keys share one row on a handset — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/tko-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

None: no capped cell trace carries a deviation or a failure signature.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 1 area bullet(s), 0 pattern candidate(s), 0 file(s) written.