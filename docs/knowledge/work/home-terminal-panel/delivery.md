---
type: bee.delivery
title: home-terminal-panel — delivery
description: "Delivery record for work item home-terminal-panel: 4 capped cell(s), 4 recorded deviation(s)."
timestamp: 2026-08-30
bee:
  id: home-terminal-panel-delivery
  lifecycle: active
  areas: [web-interface]
  required_context: [docs/history/home-terminal-panel/CONTEXT.md, docs/history/home-terminal-panel/plan.md]
  sources: [docs/history/home-terminal-panel/CONTEXT.md, docs/history/home-terminal-panel/plan.md, .bee/cells/htp-1.json, .bee/cells/htp-2.json, .bee/cells/htp-3.json, .bee/cells/htp-4.json]
---

# home-terminal-panel — Delivery

## What shipped

- **htp-1** — `nav=1` renders the Code tree and the changed-file list as base-targeted nav-only frames (3 file(s) changed)
- **htp-2** — Terminals tab gains a Files|Diff sidebar and a named panel frame above the terminal, every URL server-emitted and every frame lazy (3 file(s) changed)
- **htp-3** — `panel=1`: embedded pages render without their in-page sidebar inside the homepage panel; header, picker, reviewed counter and sections stay, every emitted URL threads the flag (4 file(s) changed)
- **htp-4** — Open-in-Code-view link carries the page's own chrome query (full/embed/panel) (1 file(s) changed)

## Verify

`PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast` (all 4 cells).

## Spec sync

Already merged into `docs/specs/web-interface.md` ("Homepage terminals
sidebar") at the 2026-08-30T13:46 scribing run.

## Deviations (notable)

Four recorded deviations are single-cell implementation trade-offs (form vs.
scripted navigation, a widget-scoped id to preserve an existing invariant,
scope discipline against an adjacent cell's suggestion). Reviewed against the
promotion bar — none recur elsewhere; not promoted to a pattern.

## Provenance

Proposed by `bee knowledge promote --work home-terminal-panel` from 4 capped
cell trace(s), reviewed and applied by bee-capturing on 2026-08-31.
