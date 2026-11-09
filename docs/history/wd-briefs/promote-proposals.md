promote proposal for work item "wd-briefs" (docs/history/wd-briefs/CONTEXT.md + docs/history/wd-briefs/plan.md) — 8 capped cell(s): brf-1, brf-2, brf-3, brf-4, brf-5, brf-6, brf-7, brf-8
anchor: history — docs/history/wd-briefs/CONTEXT.md, docs/history/wd-briefs/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/wd-briefs/delivery.md

---
type: bee.delivery
title: wd-briefs — delivery
description: "Delivery record proposed by bee knowledge promote for work item wd-briefs: 8 capped cell(s), 6 recorded deviation(s)."
timestamp: 2026-10-28
bee:
  id: wd-briefs-delivery
  lifecycle: active
  required_context: [docs/history/wd-briefs/CONTEXT.md, docs/history/wd-briefs/plan.md]
  sources: [docs/history/wd-briefs/CONTEXT.md, docs/history/wd-briefs/plan.md, .bee/cells/brf-1.json, .bee/cells/brf-2.json, .bee/cells/brf-3.json, .bee/cells/brf-4.json, .bee/cells/brf-5.json, .bee/cells/brf-6.json, .bee/cells/brf-7.json, .bee/cells/brf-8.json]
---

# wd-briefs — Delivery

## What shipped

- **brf-1** — Fourteen owner briefs written under docs/briefs, checker green (14 file(s) changed)
- **brf-2** — Fourteen owner briefs written for the second half of open PBIs, checker green (14 file(s) changed)
- **brf-3** — Sixteen owner briefs for the October features (16 file(s) changed)
- **brf-4** — Fifteen owner briefs for late-August features written under docs/briefs, checker green (15 file(s) changed)
- **brf-5** — Eleven owner briefs for the 2026-08-25 features, each built from history, cells and the main log (11 file(s) changed)
- **brf-6** — Twelve owner briefs for the 2026-08-25 agent-mark and Todo-fold features: 9 test, 3 drop as superseded (12 file(s) changed)
- **brf-7** — Sixteen owner briefs written for the late-August features, checker green (16 file(s) changed)
- **brf-8** — Three lint briefs reconciled with clippy red on main (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **brf-1** — `python3 .bee/spikes/wd-briefs/check_briefs.py p-0f60abae p-179a0d12 p-19c4995c p-1f6d0ebf p-2f69f60c p-30ca3505 p-31d113c2 p-498dd298 p-6be4d51f p-6cd25610 p-6fab30aa p-7329a3c8 p-75c7b411 p-87c083c8` — format, word-count and banned-token check over the cell's fourteen briefs only
- **brf-2** — `python3 .bee/spikes/wd-briefs/check_briefs.py p-9212cae8 p-950905ce p-a541ffc2 p-c67e8eb5 p-cd0eadfc p-da42b8f5 p-e19137ac p-e5b770fe p-e62c967c p-ebd4ce69 p-f1b14b9b p-f738af40 p-fee218b5 sup-20260830-9c4e` — docs-only cell, the cell verify checks frontmatter, six labels, 120-220 words and banned ids/paths over exactly the 14 listed briefs
- **brf-3** — `python3 .bee/spikes/wd-briefs/check_briefs.py usage-paseo-projects push-and-prune-1023 paseo-dispatch-default-on usage-dashboard paseo-leader-mode wd-direct-leader supervisor-active-loop leader-mail-envelope openclaw-agent-target paseo-native-supervisor piggery-lessons paseo-auto-wiring leader-delivery paseo-leader-dispatch pi-paseo-plugin sheets-task-sync` — the cell verify over exactly the 16 briefs this cell owns, 0 failing on format, labels, word count and banned ids; content accuracy was checked by hand against the sources, which the checker does not…
- **brf-4** — `python3 .bee/spikes/wd-briefs/check_briefs.py cargo-build-slim ipad-dashboard-mobile dispatched-pane-agent-visibility waggledance-supervisor-mode-gate diff-file-collapse term-workspace-unify trigger-settings-toggle home-terminal-new-shell observer-tick-trigger lint-gates-green paseo-removal term-keys-nav-tint pane-kick-refresh slash-builtin-commands term-keys-shift-tab` — the cell verify over exactly the fifteen briefs this cell owns; format, word count and banned-token checks
- **brf-5** — `python3 .bee/spikes/wd-briefs/check_briefs.py board-entry-conditions herding-entry-conditions board-action-dupes herdr-protocol-20 spawn-destination-fallback board-live-morph dispatch-project-presets session-work-line card-glyph-is-status ask-state-fleet-read dots-off-marks-on` — the cell verify over exactly its own eleven briefs; other cells briefs not checked
- **brf-6** — `python3 .bee/spikes/wd-briefs/check_briefs.py rail-row-one-line badge-mark-is-status mark-state-tone drawer-row-stack drawer-mark-sprite drawer-agent-mark mark-before-status terminal-mark-only claude-code-mark agent-marks-everywhere agent-logo-marks todo-column-collapse` — cell verify checker over exactly the twelve listed briefs; format, word count and banned-token checks only, content judged by hand against commits and decisions
- **brf-7** — `python3 .bee/spikes/wd-briefs/check_briefs.py composer-slash-suggest board-run-reaper mailbox-inbox commit-label-condense push-main-sync home-terminal-panel changes-diff-screen board-visibility dispatch-submit-and-reclaim wd-supervisor-seat paseo-control term-reply-composer term-keys-grid paseo-support guide-wayfinding guide-vi` — docs cell; the format checker over exactly the listed briefs is the whole proof
- **brf-8** — `python3 .bee/spikes/wd-briefs/check_briefs.py p-ebd4ce69 p-950905ce lint-gates-green` — format checker over the three rewritten briefs; the content fact rests on cargo clippy -p waggledance --all-targets -- -D warnings failing at views.rs:2094 on this tree

## Deviations

- **brf-3** — Classed the reverted Sheets sync and the four supervisor-era items as drop instead of test — Sheets sync was reverted the day it merged and the Paseo supervisor was retired on 22/10, so nothing is left to test — the plan was wrong about a fact
- **brf-3** — Committed through git add -N and a path-scoped commit — the concurrent-worker guard refused git add on the shared index — hit an unforeseen obstacle
- **brf-3** — Passed the report through a report file — the worktree guard refused the JSON as a shell argument — hit an unforeseen obstacle
- **brf-4** — paseo-removal classified drop rather than test although merged — Paseo was reintroduced as the core dispatch path in October, so testing the removal is meaningless; D5 superseded-item rule applied — the plan was wrong about a fact
- **brf-5** — card-glyph-is-status and dots-off-marks-on briefed as on main though the inventory says unmerged — their cell commits are on main directly — the plan was wrong about a fact
- **brf-7** — Landed the briefs with a path-scoped commit after intent-to-add, not a shared-index stage — the concurrent-worker guard refuses staging into the shared index — hit an unforeseen obstacle

## Provenance

Proposed by `bee knowledge promote --work wd-briefs` from 8 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/wd-briefs/CONTEXT.md`, `docs/history/wd-briefs/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell brf-3 — save as docs/knowledge/patterns/wd-briefs-brf-3-pitfall.md

---
type: bee.pattern
title: wd-briefs cell brf-3 — pitfall candidate
description: "Pitfall candidate mined from cell brf-3's capped trace: Classed the reverted Sheets sync and the four supervisor-era items as drop instead of test — Sheets sync was reverted the day it merged and the Paseo superviso…"
timestamp: 2026-10-28
bee:
  id: wd-briefs-brf-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/brf-3.json]
  polarity: pitfall
---

# wd-briefs cell brf-3 — pitfall candidate

## What the cell did

Sixteen owner briefs for the October features

## Recorded evidence (verbatim from .bee/cells/brf-3.json)

- **deviation** — Classed the reverted Sheets sync and the four supervisor-era items as drop instead of test — Sheets sync was reverted the day it merged and the Paseo supervisor was retired on 22/10, so nothing is left to test — the plan was wrong about a fact
- **deviation** — Committed through git add -N and a path-scoped commit — the concurrent-worker guard refused git add on the shared index — hit an unforeseen obstacle
- **deviation** — Passed the report through a report file — the worktree guard refused the JSON as a shell argument — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell brf-4 — save as docs/knowledge/patterns/wd-briefs-brf-4-pitfall.md

---
type: bee.pattern
title: wd-briefs cell brf-4 — pitfall candidate
description: "Pitfall candidate mined from cell brf-4's capped trace: paseo-removal classified drop rather than test although merged — Paseo was reintroduced as the core dispatch path in October, so testing the removal is meaning…"
timestamp: 2026-10-28
bee:
  id: wd-briefs-brf-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/brf-4.json]
  polarity: pitfall
---

# wd-briefs cell brf-4 — pitfall candidate

## What the cell did

Fifteen owner briefs for late-August features written under docs/briefs, checker green

## Recorded evidence (verbatim from .bee/cells/brf-4.json)

- **deviation** — paseo-removal classified drop rather than test although merged — Paseo was reintroduced as the core dispatch path in October, so testing the removal is meaningless; D5 superseded-item rule applied — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell brf-5 — save as docs/knowledge/patterns/wd-briefs-brf-5-pitfall.md

---
type: bee.pattern
title: wd-briefs cell brf-5 — pitfall candidate
description: "Pitfall candidate mined from cell brf-5's capped trace: card-glyph-is-status and dots-off-marks-on briefed as on main though the inventory says unmerged — their cell commits are on main directly — the plan was wrong…"
timestamp: 2026-10-28
bee:
  id: wd-briefs-brf-5-pitfall
  lifecycle: draft
  sources: [.bee/cells/brf-5.json]
  polarity: pitfall
---

# wd-briefs cell brf-5 — pitfall candidate

## What the cell did

Eleven owner briefs for the 2026-08-25 features, each built from history, cells and the main log

## Recorded evidence (verbatim from .bee/cells/brf-5.json)

- **deviation** — card-glyph-is-status and dots-off-marks-on briefed as on main though the inventory says unmerged — their cell commits are on main directly — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell brf-7 — save as docs/knowledge/patterns/wd-briefs-brf-7-pitfall.md

---
type: bee.pattern
title: wd-briefs cell brf-7 — pitfall candidate
description: "Pitfall candidate mined from cell brf-7's capped trace: Landed the briefs with a path-scoped commit after intent-to-add, not a shared-index stage — the concurrent-worker guard refuses staging into the shared index —…"
timestamp: 2026-10-28
bee:
  id: wd-briefs-brf-7-pitfall
  lifecycle: draft
  sources: [.bee/cells/brf-7.json]
  polarity: pitfall
---

# wd-briefs cell brf-7 — pitfall candidate

## What the cell did

Sixteen owner briefs written for the late-August features, checker green

## Recorded evidence (verbatim from .bee/cells/brf-7.json)

- **deviation** — Landed the briefs with a path-scoped commit after intent-to-add, not a shared-index stage — the concurrent-worker guard refuses staging into the shared index — hit an unforeseen obstacle

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 8 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 4 pattern candidate(s), 0 file(s) written.