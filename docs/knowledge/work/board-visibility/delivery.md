---
type: bee.delivery
title: board-visibility — delivery
description: "Delivery record for work item board-visibility: 11 capped cell(s), 19 recorded deviation(s)."
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
- **bv-1** — waiting_on_is_live excludes the turn-end idle mark (gate/question/unknown stay live) (1 file(s) changed)
- **bv-2** — BeeState carries the wait's kind and subject as an Option beside the unchanged waiting_on_live flag; reader passes no quality judgment (1 file(s) changed)
- **bv-3** — The In Progress card's waiting sentence now renders state.json's recorded subject when it beats the derived wording, and keeps the derived wording otherwise (1 file(s) changed)
- **bv-4** — The rail's project rows now name each project's active feature, its phase and its own live wait; idle rows render nothing (2 file(s) changed)
- **bv-5** — The rail pill names a project's live wait, tested honestly against a synthetic losing-branch subject (1 file(s) changed)
- **bv-6** — The rail's wait pill now names the recorded subject when it beats the bare label, clipped to one line (1 file(s) changed)

## Verify

- **bi-1..bi-5, bv-1..bv-6** — `cargo test -p waggledance-core -p waggledance` (per-cell scope; see `.bee/cells/*.json` for exact invocations).

## Spec sync

- bi-1..bi-5 → `docs/specs/human-mailbox-inbox.md` (new area).
- bv-1..bv-6 → `docs/specs/bee-cockpit.md` (Projects rail row naming, In Progress waiting-on line wording).

## Deviations (notable)

- bi-5's live-daemon-vs-harness choice and bv-5's probe-binary verification are
  instances of the `the-binary-you-ran-is-not-the-one-you-built` pattern,
  already recorded there under this feature's name.
- The remaining recorded deviations (rollup field placement, route additions,
  clippy allow-list additions, per-cell test updates) are single-cell
  implementation choices — reviewed and not promoted; none recur elsewhere.

## Provenance

Proposed by `bee knowledge promote --work board-visibility` from 11 capped
cell trace(s) in `.bee/cells/` and the anchor `docs/history/board-visibility/plan.md`,
reviewed and applied by bee-capturing on 2026-08-31.
