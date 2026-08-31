---
type: bee.delivery
title: changes-diff-screen — delivery
description: "Delivery record for work item changes-diff-screen: 8 capped cell(s), 15 recorded deviation(s)."
timestamp: 2026-08-30
bee:
  id: changes-diff-screen-delivery
  lifecycle: active
  areas: [web-interface]
  required_context: [docs/history/changes-diff-screen/CONTEXT.md, docs/history/changes-diff-screen/plan.md]
  sources: [docs/history/changes-diff-screen/CONTEXT.md, docs/history/changes-diff-screen/plan.md, .bee/cells/cds-1.json, .bee/cells/cds-2.json, .bee/cells/cds-3.json, .bee/cells/cds-4.json, .bee/cells/cds-5.json, .bee/cells/cds-6.json, .bee/cells/cds-7.json, .bee/cells/cds-8.json]
---

# changes-diff-screen — Delivery

## What shipped

- **cds-1** — Working-tree diff end to end: git diff rendered as side-by-side sections, with an empty state and an aggregate hidden-files filter (5 file(s) changed)
- **cds-2** — Changes screen: dir-grouped changed-file sidebar with M/A/D/R badges and per-file counts, sticky per-file headers, syntax-highlighted panes, Docs|Code|Changes in the topbar everywhere (4 file(s) changed)
- **cds-3** — Reviewed marks: header checkbox per file, mirrored sidebar tick, live N/M counter, localStorage marks keyed by a server-emitted content hash so an edit drops the stale mark (4 file(s) changed)
- **cds-4** — Sec-Fetch guard allows cross-site top-level navigations, unblocking the post-Access-login redirect; cross-site POSTs and non-navigation GETs still refused (1 file(s) changed)
- **cds-5** — `?commit=<sha>` shows that commit against its parent end to end; invalid values fall back to the working tree without echoing (4 file(s) changed)
- **cds-6** — The Changes header picks its base: Working tree plus recent commits, reviewed marks keyed per base (3 file(s) changed)
- **cds-7** — `embed=1` renders the Changes and Code pages without the topbar; sidebars, picker and reviewed marks stay, in-page links carry the flag (4 file(s) changed)
- **cds-8** — Terminal page gains FILES | DIFF toggles that split an embedded frame over the terminal, default closed (3 file(s) changed)

## Verify

`PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast` (all 8 cells).

## Spec sync

Already merged into `docs/specs/web-interface.md` ("Changes screen (git diff)")
at the 2026-08-30T12:36 scribing run.

## Deviations (notable)

Fifteen recorded deviations are single-cell implementation trade-offs (git-call
sequencing, data-model shape, rustfmt scope discipline, out-of-declared-file
edits reserved before writing). Reviewed against the promotion bar
(multi-feature relevance, meaningful waste prevented, generalizable) — none
recur elsewhere; not promoted to a pattern.

## Provenance

Proposed by `bee knowledge promote --work changes-diff-screen` from 8 capped
cell trace(s), reviewed and applied by bee-capturing on 2026-08-31.
