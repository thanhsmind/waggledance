---
type: bee.delivery
title: bundle-mono-font — delivery
description: "Delivery record for work item bundle-mono-font: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-12
bee:
  id: bundle-mono-font-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: []
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/bundle-mono-font.json, .bee/cells/bundle-mono-font-1.json]
---

# bundle-mono-font — Delivery

## What shipped

- **bundle-mono-font-1** — Bundled JetBrains Mono offline (regular+bold woff2 data URIs) as --font-mono's leading face; added a server.rs test pinning the @font-face and token order; OFL licence copied to docs/licenses/JetBrainsMono-OFL.txt; README credits edit skipped as out of file-scope. (5 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **bundle-mono-font-1** — `cargo test --workspace`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work bundle-mono-font` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/bundle-mono-font.json`. Every line above is copied from a trace or from the work item; accepted at the 2026-08-23 compound run; the area specs were re-synced against the shipped source in the same run.
