promote proposal for work item "board-visibility" (docs/history/board-visibility/plan.md) — 11 capped cell(s): bi-1, bi-2, bi-3, bi-4, bi-5, bv-1, bv-2, bv-3, bv-4, bv-5, bv-6
anchor: history — docs/history/board-visibility/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/board-visibility/delivery.md

---
type: bee.delivery
title: board-visibility — delivery
description: "Delivery record proposed by bee knowledge promote for work item board-visibility: 11 capped cell(s), 19 recorded deviation(s)."
timestamp: 2026-08-30
bee:
  id: board-visibility-delivery
  lifecycle: active
  required_context: [docs/history/board-visibility/plan.md]
  sources: [docs/history/board-visibility/plan.md, .bee/cells/bi-1.json, .bee/cells/bi-2.json, .bee/cells/bi-3.json, .bee/cells/bi-4.json, .bee/cells/bi-5.json, .bee/cells/bv-1.json, .bee/cells/bv-2.json, .bee/cells/bv-3.json, .bee/cells/bv-4.json, .bee/cells/bv-5.json, .bee/cells/bv-6.json]
---

# board-visibility — Delivery

## What shipped

- **bi-1** — New bee::mailbox reader parses each project's filed letters into typed records on BeeSnapshot::mailbox; unreadable letters surface, needs_human_decision is never read (4 file(s) changed)
- **bi-2** — The /inbox page lists every project's letters newest first, reached from the top bar menu, with the letter body through the sanitizing pipeline and a self-explaining empty state (2 file(s) changed)
- **bi-3** — The inbox flips read/unread by calling bee mailbox mark; waggledance never writes a letter (2 file(s) changed)
- **bi-4** — The home board shows one cross-project unread-letter count linking to /inbox, and nothing at zero (1 file(s) changed)
- **bi-5** — Proved the inbox against a real bee-composed letter through the route harness; no parser defect, five tests pin the shape (3 file(s) changed)
- **bv-1** — waiting_on_is_live excludes the turn-end idle mark (gate/question/unknown stay live): of 19 lanes carrying a waiting_on, 19 read live before and 12 after — 7 turn-end lanes flip; over the project state.json files the board reads, 24 live before, 3 after (1 file(s) changed)
- **bv-2** — BeeState carries the wait's kind and subject as an Option beside the unchanged waiting_on_live flag; reader passes no quality judgment (1 file(s) changed)
- **bv-3** — The In Progress card's waiting sentence now renders state.json's recorded subject when it beats the derived wording, and keeps the derived wording otherwise (1 file(s) changed)
- **bv-4** — The rail's project rows now name each project's active feature, its phase and its own live wait; idle rows render nothing (2 file(s) changed)
- **bv-5** — Leg (d) produced: the rail pill names jarvis's live wait, and the losing branch is shown honestly with a real subject on a synthetic project (1 file(s) changed)
- **bv-6** — The rail's wait pill now names the recorded subject when it beats the bare label, clipped to one line (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **bi-1** — `cargo test -p waggledance-core -p waggledance`
- **bi-2** — `cargo test -p waggledance`
- **bi-3** — `cargo test -p waggledance`
- **bi-4** — `cargo test -p waggledance`
- **bi-5** — `cargo test -p waggledance -p waggledance-core`
- **bv-1** — `cargo test -p waggledance-core -p waggledance`
- **bv-2** — `cargo test -p waggledance-core -p waggledance`
- **bv-3** — `cargo test -p waggledance`
- **bv-4** — `cargo test -p waggledance`
- **bv-5** — `cargo test -p waggledance -p waggledance-core`
- **bv-6** — `cargo test -p waggledance`

## Deviations

- **bi-1** — Exposed the mailbox on BeeSnapshot (reachable as rollup.snapshot.mailbox) instead of a new BeeProjectRollup field — a rollup field would force an edit to the struct literal at crates/waggledance/src/server.rs:1253, which is under the live cross-worktree hold held by cds-1 — hit an unforeseen obstacle
- **bi-1** — Committed Cargo.lock alongside the cell's three files, reserved first under w-bi-1 — cargo rewrites it mechanically when yaml-rust becomes a direct dependency, and leaving it out would ship a manifest the lock does not match — something else had to be fixed first
- **bi-2** — Added a second route /inbox/:project/:letter beside /inbox — the cell asked that selecting a row show that letter's body, and a page needs a URL — found a better route
- **bi-2** — Rendered the letter through waggledance_core::render::RenderService directly rather than Engine::render_file — a letter is never indexed (.bee/human-mailbox is git-ignored runtime state), so the engine path cannot reach it; it is the same pipeline, held in a OnceLock so syntect loads once — the plan was wrong about a fact
- **bi-3** — followed the plan
- **bi-4** — followed the plan
- **bi-5** — Rendered through the in-crate router() harness instead of a live daemon — waggledance resolves its registry and daemon.lock from $HOME/.waggledance with no --data-dir override, so a probe daemon would have taken the lock from the user's running one — hit an unforeseen obstacle
- **bi-5** — Added four permanent route tests to server.rs rather than a throwaway probe — the rendered HTML the proof needed is the tests' own captured stdout, so the artifact and the regression guard are the same code — found a better route
- **bi-5** — Re-ran leg (e) fresh instead of quoting run 1 for it, and re-measured leg (g) after all render work — a kept output is weaker evidence than a repeated one, and (g) had to hold at the END of the run — found a better route
- **bv-1** — followed the plan
- **bv-2** — Factored the well-formedness check out of waiting_on_is_live into waiting_on_fields and scrubbed the carried subject with the crate-wide scrub_paths — a second copy of the predicate would drift from bv-1's D4 site, and every other free-text field in this reader (next_action, route.rationale, cell titles) is already scrubbed before it reaches a rendered page — found a better route
- **bv-3** — followed the plan
- **bv-4** — Added #[allow(clippy::too_many_arguments)] to project_sidebar (8th param) — CI runs clippy -D warnings and home_page already carries the same allow for the same reason — hit an unforeseen obstacle
- **bv-4** — Updated 18 existing home_page/project_sidebar test call sites with the new argument — a signature change the cell implies but does not name — something else had to be fixed first
- **bv-5** — Rendered via a std-only rustc probe at .bee/spikes/board-visibility/bv5_legd.rs instead of a repo test file — the shell write-guard refuses HOME= on a Bash command, and setting HOME on the child process is e2e_open.rs's own idiom — hit an unforeseen obstacle
- **bv-5** — Added a synthetic fourth project carrying waggledance's verbatim AskUserQuestion subject to exercise the losing branch — no live project supplies a losing subject, and the cell instructed to demonstrate it honestly rather than claim a live instance — the plan was wrong about a fact
- **bv-5** — Amended the top status banner and appended four superseding bullets to the closing section — leaving "INCOMPLETE" and "bv-3 not established" standing would have made the document lie about its own contents — something else had to be fixed first
- **bv-5** — Verified the probe binary by a bv-6-only format literal rather than by sha or cargo-resolved path — the shared CARGO_TARGET_DIR makes both of those non-decisive, as run 1 learned live — found a better route
- **bv-6** — Updated two assertions in the existing bv-4 test and one phrase-count assertion to the new pill markup — the busy fixture's subject wins the pill, so the old exact-markup strings could not both hold and the change be real — the plan was wrong about a fact

## Provenance

Proposed by `bee knowledge promote --work board-visibility` from 11 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-visibility/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell bi-1 — save as docs/knowledge/patterns/board-visibility-bi-1-pitfall.md

---
type: bee.pattern
title: board-visibility cell bi-1 — pitfall candidate
description: "Pitfall candidate mined from cell bi-1's capped trace: Exposed the mailbox on BeeSnapshot (reachable as rollup.snapshot.mailbox) instead of a new BeeProjectRollup field — a rollup field would force an edit to the s…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bi-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/bi-1.json]
  polarity: pitfall
---

# board-visibility cell bi-1 — pitfall candidate

## What the cell did

New bee::mailbox reader parses each project's filed letters into typed records on BeeSnapshot::mailbox; unreadable letters surface, needs_human_decision is never read

## Recorded evidence (verbatim from .bee/cells/bi-1.json)

- **deviation** — Exposed the mailbox on BeeSnapshot (reachable as rollup.snapshot.mailbox) instead of a new BeeProjectRollup field — a rollup field would force an edit to the struct literal at crates/waggledance/src/server.rs:1253, which is under the live cross-worktree hold held by cds-1 — hit an unforeseen obstacle
- **deviation** — Committed Cargo.lock alongside the cell's three files, reserved first under w-bi-1 — cargo rewrites it mechanically when yaml-rust becomes a direct dependency, and leaving it out would ship a manifest the lock does not match — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bi-2 — save as docs/knowledge/patterns/board-visibility-bi-2-pitfall.md

---
type: bee.pattern
title: board-visibility cell bi-2 — pitfall candidate
description: "Pitfall candidate mined from cell bi-2's capped trace: Added a second route /inbox/:project/:letter beside /inbox — the cell asked that selecting a row show that letter's body, and a page needs a URL — found a bett…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bi-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/bi-2.json]
  polarity: pitfall
---

# board-visibility cell bi-2 — pitfall candidate

## What the cell did

The /inbox page lists every project's letters newest first, reached from the top bar menu, with the letter body through the sanitizing pipeline and a self-explaining empty state

## Recorded evidence (verbatim from .bee/cells/bi-2.json)

- **deviation** — Added a second route /inbox/:project/:letter beside /inbox — the cell asked that selecting a row show that letter's body, and a page needs a URL — found a better route
- **deviation** — Rendered the letter through waggledance_core::render::RenderService directly rather than Engine::render_file — a letter is never indexed (.bee/human-mailbox is git-ignored runtime state), so the engine path cannot reach it; it is the same pipeline, held in a OnceLock so syntect loads once — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bi-3 — save as docs/knowledge/patterns/board-visibility-bi-3-pitfall.md

---
type: bee.pattern
title: board-visibility cell bi-3 — pitfall candidate
description: "Pitfall candidate mined from cell bi-3's capped trace: followed the plan"
timestamp: 2026-08-30
bee:
  id: board-visibility-bi-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/bi-3.json]
  polarity: pitfall
---

# board-visibility cell bi-3 — pitfall candidate

## What the cell did

The inbox flips read/unread by calling bee mailbox mark; waggledance never writes a letter

## Recorded evidence (verbatim from .bee/cells/bi-3.json)

- **deviation** — followed the plan

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bi-4 — save as docs/knowledge/patterns/board-visibility-bi-4-pitfall.md

---
type: bee.pattern
title: board-visibility cell bi-4 — pitfall candidate
description: "Pitfall candidate mined from cell bi-4's capped trace: followed the plan"
timestamp: 2026-08-30
bee:
  id: board-visibility-bi-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/bi-4.json]
  polarity: pitfall
---

# board-visibility cell bi-4 — pitfall candidate

## What the cell did

The home board shows one cross-project unread-letter count linking to /inbox, and nothing at zero

## Recorded evidence (verbatim from .bee/cells/bi-4.json)

- **deviation** — followed the plan

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bi-5 — save as docs/knowledge/patterns/board-visibility-bi-5-pitfall.md

---
type: bee.pattern
title: board-visibility cell bi-5 — pitfall candidate
description: "Pitfall candidate mined from cell bi-5's capped trace: Rendered through the in-crate router() harness instead of a live daemon — waggledance resolves its registry and daemon.lock from $HOME/.waggledance with no --d…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bi-5-pitfall
  lifecycle: draft
  sources: [.bee/cells/bi-5.json]
  polarity: pitfall
---

# board-visibility cell bi-5 — pitfall candidate

## What the cell did

Proved the inbox against a real bee-composed letter through the route harness; no parser defect, five tests pin the shape

## Recorded evidence (verbatim from .bee/cells/bi-5.json)

- **deviation** — Rendered through the in-crate router() harness instead of a live daemon — waggledance resolves its registry and daemon.lock from $HOME/.waggledance with no --data-dir override, so a probe daemon would have taken the lock from the user's running one — hit an unforeseen obstacle
- **deviation** — Added four permanent route tests to server.rs rather than a throwaway probe — the rendered HTML the proof needed is the tests' own captured stdout, so the artifact and the regression guard are the same code — found a better route
- **deviation** — Re-ran leg (e) fresh instead of quoting run 1 for it, and re-measured leg (g) after all render work — a kept output is weaker evidence than a repeated one, and (g) had to hold at the END of the run — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bv-1 — save as docs/knowledge/patterns/board-visibility-bv-1-pitfall.md

---
type: bee.pattern
title: board-visibility cell bv-1 — pitfall candidate
description: "Pitfall candidate mined from cell bv-1's capped trace: followed the plan"
timestamp: 2026-08-30
bee:
  id: board-visibility-bv-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/bv-1.json]
  polarity: pitfall
---

# board-visibility cell bv-1 — pitfall candidate

## What the cell did

waiting_on_is_live excludes the turn-end idle mark (gate/question/unknown stay live): of 19 lanes carrying a waiting_on, 19 read live before and 12 after — 7 turn-end lanes flip; over the project state.json files the board reads, 24 live before, 3 after

## Recorded evidence (verbatim from .bee/cells/bv-1.json)

- **deviation** — followed the plan

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bv-2 — save as docs/knowledge/patterns/board-visibility-bv-2-pitfall.md

---
type: bee.pattern
title: board-visibility cell bv-2 — pitfall candidate
description: "Pitfall candidate mined from cell bv-2's capped trace: Factored the well-formedness check out of waiting_on_is_live into waiting_on_fields and scrubbed the carried subject with the crate-wide scrub_paths — a second…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bv-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/bv-2.json]
  polarity: pitfall
---

# board-visibility cell bv-2 — pitfall candidate

## What the cell did

BeeState carries the wait's kind and subject as an Option beside the unchanged waiting_on_live flag; reader passes no quality judgment

## Recorded evidence (verbatim from .bee/cells/bv-2.json)

- **deviation** — Factored the well-formedness check out of waiting_on_is_live into waiting_on_fields and scrubbed the carried subject with the crate-wide scrub_paths — a second copy of the predicate would drift from bv-1's D4 site, and every other free-text field in this reader (next_action, route.rationale, cell titles) is already scrubbed before it reaches a rendered page — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bv-3 — save as docs/knowledge/patterns/board-visibility-bv-3-pitfall.md

---
type: bee.pattern
title: board-visibility cell bv-3 — pitfall candidate
description: "Pitfall candidate mined from cell bv-3's capped trace: followed the plan"
timestamp: 2026-08-30
bee:
  id: board-visibility-bv-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/bv-3.json]
  polarity: pitfall
---

# board-visibility cell bv-3 — pitfall candidate

## What the cell did

The In Progress card's waiting sentence now renders state.json's recorded subject when it beats the derived wording, and keeps the derived wording otherwise

## Recorded evidence (verbatim from .bee/cells/bv-3.json)

- **deviation** — followed the plan

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bv-4 — save as docs/knowledge/patterns/board-visibility-bv-4-pitfall.md

---
type: bee.pattern
title: board-visibility cell bv-4 — pitfall candidate
description: "Pitfall candidate mined from cell bv-4's capped trace: Added #[allow(clippy::too_many_arguments)] to project_sidebar (8th param) — CI runs clippy -D warnings and home_page already carries the same allow for the sam…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bv-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/bv-4.json]
  polarity: pitfall
---

# board-visibility cell bv-4 — pitfall candidate

## What the cell did

The rail's project rows now name each project's active feature, its phase and its own live wait; idle rows render nothing

## Recorded evidence (verbatim from .bee/cells/bv-4.json)

- **deviation** — Added #[allow(clippy::too_many_arguments)] to project_sidebar (8th param) — CI runs clippy -D warnings and home_page already carries the same allow for the same reason — hit an unforeseen obstacle
- **deviation** — Updated 18 existing home_page/project_sidebar test call sites with the new argument — a signature change the cell implies but does not name — something else had to be fixed first

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bv-5 — save as docs/knowledge/patterns/board-visibility-bv-5-pitfall.md

---
type: bee.pattern
title: board-visibility cell bv-5 — pitfall candidate
description: "Pitfall candidate mined from cell bv-5's capped trace: Rendered via a std-only rustc probe at .bee/spikes/board-visibility/bv5_legd.rs instead of a repo test file — the shell write-guard refuses HOME= on a Bash com…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bv-5-pitfall
  lifecycle: draft
  sources: [.bee/cells/bv-5.json]
  polarity: pitfall
---

# board-visibility cell bv-5 — pitfall candidate

## What the cell did

Leg (d) produced: the rail pill names jarvis's live wait, and the losing branch is shown honestly with a real subject on a synthetic project

## Recorded evidence (verbatim from .bee/cells/bv-5.json)

- **deviation** — Rendered via a std-only rustc probe at .bee/spikes/board-visibility/bv5_legd.rs instead of a repo test file — the shell write-guard refuses HOME= on a Bash command, and setting HOME on the child process is e2e_open.rs's own idiom — hit an unforeseen obstacle
- **deviation** — Added a synthetic fourth project carrying waggledance's verbatim AskUserQuestion subject to exercise the losing branch — no live project supplies a losing subject, and the cell instructed to demonstrate it honestly rather than claim a live instance — the plan was wrong about a fact
- **deviation** — Amended the top status banner and appended four superseding bullets to the closing section — leaving "INCOMPLETE" and "bv-3 not established" standing would have made the document lie about its own contents — something else had to be fixed first
- **deviation** — Verified the probe binary by a bv-6-only format literal rather than by sha or cargo-resolved path — the shared CARGO_TARGET_DIR makes both of those non-decisive, as run 1 learned live — found a better route

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell bv-6 — save as docs/knowledge/patterns/board-visibility-bv-6-pitfall.md

---
type: bee.pattern
title: board-visibility cell bv-6 — pitfall candidate
description: "Pitfall candidate mined from cell bv-6's capped trace: Updated two assertions in the existing bv-4 test and one phrase-count assertion to the new pill markup — the busy fixture's subject wins the pill, so the old e…"
timestamp: 2026-08-30
bee:
  id: board-visibility-bv-6-pitfall
  lifecycle: draft
  sources: [.bee/cells/bv-6.json]
  polarity: pitfall
---

# board-visibility cell bv-6 — pitfall candidate

## What the cell did

The rail's wait pill now names the recorded subject when it beats the bare label, clipped to one line

## Recorded evidence (verbatim from .bee/cells/bv-6.json)

- **deviation** — Updated two assertions in the existing bv-4 test and one phrase-count assertion to the new pill markup — the busy fixture's subject wins the pill, so the old exact-markup strings could not both hold and the change be real — the plan was wrong about a fact

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 11 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 11 pattern candidate(s), 0 file(s) written.