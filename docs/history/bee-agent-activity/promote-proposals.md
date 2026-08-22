promote proposal for work item "bee-agent-activity" (docs/history/bee-agent-activity/CONTEXT.md + docs/history/bee-agent-activity/plan.md) — 8 capped cell(s): baa-1, baa-2, baa-3, baa-4, baa-5, baa-6, baa-7, baa-8
anchor: history — docs/history/bee-agent-activity/CONTEXT.md, docs/history/bee-agent-activity/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/bee-agent-activity/delivery.md

---
type: bee.delivery
title: bee-agent-activity — delivery
description: "Delivery record proposed by bee knowledge promote for work item bee-agent-activity: 8 capped cell(s), 17 recorded deviation(s)."
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-delivery
  lifecycle: active
  areas: [bee-cockpit, agent-terminal]
  required_context: [docs/history/bee-agent-activity/CONTEXT.md, docs/history/bee-agent-activity/plan.md]
  sources: [docs/history/bee-agent-activity/CONTEXT.md, docs/history/bee-agent-activity/plan.md, .bee/cells/baa-1.json, .bee/cells/baa-2.json, .bee/cells/baa-3.json, .bee/cells/baa-4.json, .bee/cells/baa-5.json, .bee/cells/baa-6.json, .bee/cells/baa-7.json, .bee/cells/baa-8.json]
---

# bee-agent-activity — Delivery

## What shipped

- **baa-1** — BeeSession now carries a parsed activity record and a 90-second-rule signal, malformed activity dropped without failing the read (1 file(s) changed)
- **baa-2** — bee agent activity reaches board cards, badges, need-you counts, Pinned rows and the Agents drawer (4 file(s) changed)
- **baa-3** — Approve is offered only for a bee-blocked pane (or one with no bee record); every other state renders it disabled with the reason in its title, and both app.js handlers refuse a disabled click (3 file(s) changed)
- **baa-4** — Activity cursor notifies once on entry into need-you and once on exited, through the existing outbox (3 file(s) changed)
- **baa-5** — Both specs now state the five agent states, the precedence rule, the need-you count, no signal, the Approve gate and the notifier rule (2 file(s) changed)
- **baa-6** — Agent line is a full-width row under the card title (1 file(s) changed)
- **baa-7** — Ready to merge now holds every capped feature with an open worktree, with a uat approved/pending line and ready age, approved first (2 file(s) changed)
- **baa-8** — Card footer is one cells-and-time line; Ready to merge shows its worktree branch (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **baa-1** — `cargo test -p waggledance-core -- session activity`
- **baa-2** — `cargo test -p waggledance -- bee_hub agent_activity api_agents pinned`
- **baa-3** — `cargo test -p waggledance -- approve pane_controls`
- **baa-4** — `cargo test -p waggledance -- watcher notify activity`
- **baa-5** — `rg -n "needs approval|needs an answer|no signal" docs/specs/bee-cockpit.md docs/specs/agent-terminal.md`
- **baa-6** — `cargo test -p waggledance -- bee_hub_agent bee_hub_card`
- **baa-7** — `cargo test -p waggledance -- ready_to_merge bee_hub`
- **baa-8** — `cargo test -p waggledance -- bee_hub_card bee_hub_footer ready_to_merge`

## Deviations

- **baa-1** — Added ACTIVITY_LIVE_SECONDS const and two extra tests (unknown state carried verbatim, word() mapping) beyond the cell's list
- **baa-1** — Ran cargo fmt and clippy on the crate to match repo CI
- **baa-2** — TerminalPaneView carries a third field bee_no_signal beside the two the cell named: A3 says a no_signal record is never need-you, and that cannot be decided from the state alone at the pane
- **baa-2** — /api/agents bee_state carries the machine state (blocked/waiting_input) rather than the human word, because app.js ranks on it; app.js maps it to A3 word vocabulary for display
- **baa-2** — the .bee-hub__agent* rules went into views.rs bee_hub_style() beside every other .bee-hub__* rule rather than app.css, which holds no .bee-hub__ component rules at all
- **baa-2** — docs/specs/bee-cockpit.md untouched — spec sync is the plan phase 5 cell and the spec is not in this cell files
- **baa-3** — Known limit: the pane card is server-rendered and the screen poller only replaces .term-screen text, so the Approve gate is only as fresh as the last page load
- **baa-3** — Fixed a pre-existing rustfmt violation in crates/waggledance/src/server.rs:25032 (a long test name from baa-2) — my reserved file, and it would have failed CI fmt for the whole feature
- **baa-3** — No app.css change: the reply-button rules live in views.rs PROJECT_TAB_STYLE, so the new .term-reply__approve:disabled rule went there
- **baa-4** — Project roots come from the engine already passed to reconcile_notify (a watcher::BeeRoots port re-asked each tick), not from the server bee cache — that cache lives in AppState the watcher cannot reach without new plumbing, and server.rs was held by a sibling cell
- **baa-4** — run_async now yields a WatchEvent enum (Status | Activity) so one handler carries both cursors rather than a second handler parameter
- **baa-4** — First sight of a session (from == None) already in a need-you state counts as an entry and fires, per the dispatch note
- **baa-4** — The crate was transiently red from a sibling in-progress views.rs edit; waited and retried the build rather than touching that file
- **baa-7** — The Ready to merge card is a dense `.bee-hub__row` (display: block), not the In Progress card`s flex summary, so the merge line renders as a block <p> inside the row instead of taking `.bee-hub__agent`s `flex: 1 0 100%; order: 11` pair — same visible result, a full-width muted second line under the title
- **baa-7** — Rewrote the ctk-8 placement test into two: the widened membership rule plus uat line and ordering, and the not-ready cases (open cell, zero cells, execution gate unapproved). No test deleted.
- **baa-7** — docs/specs/bee-cockpit.md (affects_specs) left untouched — it is not in the cell`s files.
- **baa-8** — Two commits for the cell: the worker's views.rs commit (6b62075) landed while server.rs was held by another worktree; the orchestrator retargeted the two literals in a second commit once the lease was released

## Provenance

Proposed by `bee knowledge promote --work bee-agent-activity` from 8 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/bee-agent-activity/CONTEXT.md`, `docs/history/bee-agent-activity/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "bee-agent-activity" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-22T16:19:59.310Z), the work item declares no bee.areas.

area bee-cockpit:
  - [baa-2] bee agent activity reaches board cards, badges, need-you counts, Pinned rows and the Agents drawer — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/baa-2.json)
  - [baa-3] Approve is offered only for a bee-blocked pane (or one with no bee record); every other state renders it disabled with the reason in its title, and both app.js handlers refuse a disabled click — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/baa-3.json)
  - [baa-4] Activity cursor notifies once on entry into need-you and once on exited, through the existing outbox — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/baa-4.json)
  - [baa-6] Agent line is a full-width row under the card title — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/baa-6.json)
  - [baa-7] Ready to merge now holds every capped feature with an open worktree, with a uat approved/pending line and ready age, approved first — feature-wide sync per the scribing stamp, 2 file(s) changed (trace .bee/cells/baa-7.json)
  - [baa-8] Card footer is one cells-and-time line; Ready to merge shows its worktree branch — feature-wide sync per the scribing stamp, 2 file(s) changed (trace .bee/cells/baa-8.json)

area agent-terminal:
  - [baa-2] bee agent activity reaches board cards, badges, need-you counts, Pinned rows and the Agents drawer — feature-wide sync per the scribing stamp, 4 file(s) changed (trace .bee/cells/baa-2.json)
  - [baa-3] Approve is offered only for a bee-blocked pane (or one with no bee record); every other state renders it disabled with the reason in its title, and both app.js handlers refuse a disabled click — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/baa-3.json)
  - [baa-4] Activity cursor notifies once on entry into need-you and once on exited, through the existing outbox — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/baa-4.json)
  - [baa-6] Agent line is a full-width row under the card title — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/baa-6.json)
  - [baa-7] Ready to merge now holds every capped feature with an open worktree, with a uat approved/pending line and ready age, approved first — feature-wide sync per the scribing stamp, 2 file(s) changed (trace .bee/cells/baa-7.json)
  - [baa-8] Card footer is one cells-and-time line; Ready to merge shows its worktree branch — feature-wide sync per the scribing stamp, 2 file(s) changed (trace .bee/cells/baa-8.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell baa-1 — save as docs/knowledge/patterns/bee-agent-activity-baa-1-pitfall.md

---
type: bee.pattern
title: bee-agent-activity cell baa-1 — pitfall candidate
description: "Pitfall candidate mined from cell baa-1's capped trace: Added ACTIVITY_LIVE_SECONDS const and two extra tests (unknown state carried verbatim, word() mapping) beyond the cell's list"
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-baa-1-pitfall
  lifecycle: draft
  areas: [bee-cockpit, agent-terminal]
  sources: [.bee/cells/baa-1.json]
  polarity: pitfall
---

# bee-agent-activity cell baa-1 — pitfall candidate

## What the cell did

BeeSession now carries a parsed activity record and a 90-second-rule signal, malformed activity dropped without failing the read

## Recorded evidence (verbatim from .bee/cells/baa-1.json)

- **deviation** — Added ACTIVITY_LIVE_SECONDS const and two extra tests (unknown state carried verbatim, word() mapping) beyond the cell's list
- **deviation** — Ran cargo fmt and clippy on the crate to match repo CI

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell baa-2 — save as docs/knowledge/patterns/bee-agent-activity-baa-2-pitfall.md

---
type: bee.pattern
title: bee-agent-activity cell baa-2 — pitfall candidate
description: "Pitfall candidate mined from cell baa-2's capped trace: TerminalPaneView carries a third field bee_no_signal beside the two the cell named: A3 says a no_signal record is never need-you, and that cannot be decided fr…"
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-baa-2-pitfall
  lifecycle: draft
  areas: [bee-cockpit, agent-terminal]
  sources: [.bee/cells/baa-2.json]
  polarity: pitfall
---

# bee-agent-activity cell baa-2 — pitfall candidate

## What the cell did

bee agent activity reaches board cards, badges, need-you counts, Pinned rows and the Agents drawer

## Recorded evidence (verbatim from .bee/cells/baa-2.json)

- **deviation** — TerminalPaneView carries a third field bee_no_signal beside the two the cell named: A3 says a no_signal record is never need-you, and that cannot be decided from the state alone at the pane
- **deviation** — /api/agents bee_state carries the machine state (blocked/waiting_input) rather than the human word, because app.js ranks on it; app.js maps it to A3 word vocabulary for display
- **deviation** — the .bee-hub__agent* rules went into views.rs bee_hub_style() beside every other .bee-hub__* rule rather than app.css, which holds no .bee-hub__ component rules at all
- **deviation** — docs/specs/bee-cockpit.md untouched — spec sync is the plan phase 5 cell and the spec is not in this cell files

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell baa-3 — save as docs/knowledge/patterns/bee-agent-activity-baa-3-pitfall.md

---
type: bee.pattern
title: bee-agent-activity cell baa-3 — pitfall candidate
description: "Pitfall candidate mined from cell baa-3's capped trace: Known limit: the pane card is server-rendered and the screen poller only replaces .term-screen text, so the Approve gate is only as fresh as the last page load"
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-baa-3-pitfall
  lifecycle: draft
  areas: [bee-cockpit, agent-terminal]
  sources: [.bee/cells/baa-3.json]
  polarity: pitfall
---

# bee-agent-activity cell baa-3 — pitfall candidate

## What the cell did

Approve is offered only for a bee-blocked pane (or one with no bee record); every other state renders it disabled with the reason in its title, and both app.js handlers refuse a disabled click

## Recorded evidence (verbatim from .bee/cells/baa-3.json)

- **deviation** — Known limit: the pane card is server-rendered and the screen poller only replaces .term-screen text, so the Approve gate is only as fresh as the last page load
- **deviation** — Fixed a pre-existing rustfmt violation in crates/waggledance/src/server.rs:25032 (a long test name from baa-2) — my reserved file, and it would have failed CI fmt for the whole feature
- **deviation** — No app.css change: the reply-button rules live in views.rs PROJECT_TAB_STYLE, so the new .term-reply__approve:disabled rule went there

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell baa-4 — save as docs/knowledge/patterns/bee-agent-activity-baa-4-pitfall.md

---
type: bee.pattern
title: bee-agent-activity cell baa-4 — pitfall candidate
description: "Pitfall candidate mined from cell baa-4's capped trace: Project roots come from the engine already passed to reconcile_notify (a watcher::BeeRoots port re-asked each tick), not from the server bee cache — that cache…"
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-baa-4-pitfall
  lifecycle: draft
  areas: [bee-cockpit, agent-terminal]
  sources: [.bee/cells/baa-4.json]
  polarity: pitfall
---

# bee-agent-activity cell baa-4 — pitfall candidate

## What the cell did

Activity cursor notifies once on entry into need-you and once on exited, through the existing outbox

## Recorded evidence (verbatim from .bee/cells/baa-4.json)

- **deviation** — Project roots come from the engine already passed to reconcile_notify (a watcher::BeeRoots port re-asked each tick), not from the server bee cache — that cache lives in AppState the watcher cannot reach without new plumbing, and server.rs was held by a sibling cell
- **deviation** — run_async now yields a WatchEvent enum (Status | Activity) so one handler carries both cursors rather than a second handler parameter
- **deviation** — First sight of a session (from == None) already in a need-you state counts as an entry and fires, per the dispatch note
- **deviation** — The crate was transiently red from a sibling in-progress views.rs edit; waited and retried the build rather than touching that file

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell baa-7 — save as docs/knowledge/patterns/bee-agent-activity-baa-7-pitfall.md

---
type: bee.pattern
title: bee-agent-activity cell baa-7 — pitfall candidate
description: "Pitfall candidate mined from cell baa-7's capped trace: The Ready to merge card is a dense `.bee-hub__row` (display: block), not the In Progress card`s flex summary, so the merge line renders as a block <p> inside t…"
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-baa-7-pitfall
  lifecycle: draft
  areas: [bee-cockpit, agent-terminal]
  sources: [.bee/cells/baa-7.json]
  polarity: pitfall
---

# bee-agent-activity cell baa-7 — pitfall candidate

## What the cell did

Ready to merge now holds every capped feature with an open worktree, with a uat approved/pending line and ready age, approved first

## Recorded evidence (verbatim from .bee/cells/baa-7.json)

- **deviation** — The Ready to merge card is a dense `.bee-hub__row` (display: block), not the In Progress card`s flex summary, so the merge line renders as a block <p> inside the row instead of taking `.bee-hub__agent`s `flex: 1 0 100%; order: 11` pair — same visible result, a full-width muted second line under the title
- **deviation** — Rewrote the ctk-8 placement test into two: the widened membership rule plus uat line and ordering, and the not-ready cases (open cell, zero cells, execution gate unapproved). No test deleted.
- **deviation** — docs/specs/bee-cockpit.md (affects_specs) left untouched — it is not in the cell`s files.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell baa-8 — save as docs/knowledge/patterns/bee-agent-activity-baa-8-pitfall.md

---
type: bee.pattern
title: bee-agent-activity cell baa-8 — pitfall candidate
description: "Pitfall candidate mined from cell baa-8's capped trace: Two commits for the cell: the worker's views.rs commit (6b62075) landed while server.rs was held by another worktree; the orchestrator retargeted the two liter…"
timestamp: 2026-08-22
bee:
  id: bee-agent-activity-baa-8-pitfall
  lifecycle: draft
  areas: [bee-cockpit, agent-terminal]
  sources: [.bee/cells/baa-8.json]
  polarity: pitfall
---

# bee-agent-activity cell baa-8 — pitfall candidate

## What the cell did

Card footer is one cells-and-time line; Ready to merge shows its worktree branch

## Recorded evidence (verbatim from .bee/cells/baa-8.json)

- **deviation** — Two commits for the cell: the worker's views.rs commit (6b62075) landed while server.rs was held by another worktree; the orchestrator retargeted the two literals in a second commit once the lease was released

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 8 capped cell(s) mined, 1 delivery draft, 12 area bullet(s), 6 pattern candidate(s), 0 file(s) written.