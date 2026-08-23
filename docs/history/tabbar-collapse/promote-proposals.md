promote proposal for work item "tabbar-collapse" (.bee/logs/scribing-runs.jsonl + .bee/lanes/tabbar-collapse.json + docs/history/tabbar-collapse/promote-proposals.md) — 1 capped cell(s): tbc-1
anchor: ledger — .bee/logs/scribing-runs.jsonl, .bee/lanes/tabbar-collapse.json, docs/history/tabbar-collapse/promote-proposals.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/tabbar-collapse/delivery.md

---
type: bee.delivery
title: tabbar-collapse — delivery
description: "Delivery record proposed by bee knowledge promote for work item tabbar-collapse: 1 capped cell(s), 5 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: tabbar-collapse-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [.bee/logs/scribing-runs.jsonl, .bee/lanes/tabbar-collapse.json, docs/history/tabbar-collapse/promote-proposals.md]
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/tabbar-collapse.json, docs/history/tabbar-collapse/promote-proposals.md, .bee/cells/archive/tabbar-collapse/tbc-1.json]
---

# tabbar-collapse — Delivery

## What shipped

- **tbc-1** — Phone tab bar folds away behind a remembered bottom-edge handle; hidden on first visit, visible with scripting off (4 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **tbc-1** — `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p waggledance home_page_renders_the_handset_tab_bar`

## Deviations

- **tbc-1** — JS resolves the shell with document.querySelector(".home-shell") instead of nav.closest(): the tab bar renders as a SIBLING of .home-shell in the body, not inside it, so closest() would have returned null
- **tbc-1** — Handle CSS keys off .home-tabbar--hidden + .home-tabbar__toggle rather than the shell class, same nesting reason; the shell class still drives > main and .term-scroll__stack, which ARE descendants
- **tbc-1** — Updated the existing test nav finder to carry the new id=home-tabbar and extended that test in place rather than adding a sibling test
- **tbc-1** — docs/history/tabbar-collapse/CONTEXT.md named as an input does not exist; read decision 75a5b463 from the store instead
- **tbc-1** — Registered the worker record myself (bee state worker add) — dispatch had not created one, and cells finish refused the cap without it

## Provenance

Proposed by `bee knowledge promote --work tabbar-collapse` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/tabbar-collapse.json`, `docs/history/tabbar-collapse/promote-proposals.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "tabbar-collapse" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-23T04:43:05.601Z), the work item declares no bee.areas.

area bee-cockpit:
  - [tbc-1] Phone tab bar folds away behind a remembered bottom-edge handle; hidden on first visit, visible with scripting off — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/archive/tabbar-collapse/tbc-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell tbc-1 — save as docs/knowledge/patterns/tabbar-collapse-tbc-1-pitfall.md

---
type: bee.pattern
title: tabbar-collapse cell tbc-1 — pitfall candidate
description: "Pitfall candidate mined from cell tbc-1's capped trace: JS resolves the shell with document.querySelector(\".home-shell\") instead of nav.closest(): the tab bar renders as a SIBLING of .home-shell in the body, not ins…"
timestamp: 2026-08-23
bee:
  id: tabbar-collapse-tbc-1-pitfall
  lifecycle: draft
  areas: [bee-cockpit]
  sources: [.bee/cells/archive/tabbar-collapse/tbc-1.json]
  polarity: pitfall
---

# tabbar-collapse cell tbc-1 — pitfall candidate

## What the cell did

Phone tab bar folds away behind a remembered bottom-edge handle; hidden on first visit, visible with scripting off

## Recorded evidence (verbatim from .bee/cells/archive/tabbar-collapse/tbc-1.json)

- **deviation** — JS resolves the shell with document.querySelector(".home-shell") instead of nav.closest(): the tab bar renders as a SIBLING of .home-shell in the body, not inside it, so closest() would have returned null
- **deviation** — Handle CSS keys off .home-tabbar--hidden + .home-tabbar__toggle rather than the shell class, same nesting reason; the shell class still drives > main and .term-scroll__stack, which ARE descendants
- **deviation** — Updated the existing test nav finder to carry the new id=home-tabbar and extended that test in place rather than adding a sibling test
- **deviation** — docs/history/tabbar-collapse/CONTEXT.md named as an input does not exist; read decision 75a5b463 from the store instead
- **deviation** — Registered the worker record myself (bee state worker add) — dispatch had not created one, and cells finish refused the cap without it

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 1 area bullet(s), 1 pattern candidate(s), 0 file(s) written.