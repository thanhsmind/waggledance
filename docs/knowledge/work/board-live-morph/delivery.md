---
type: bee.delivery
title: board-live-morph — delivery
description: "Delivery record for work item board-live-morph: the two board surfaces stop reloading the whole page on a change signal — they refetch their own HTML, reconcile cards by key, keep what the reader opened open, and slide the survivors to their new boxes."
timestamp: 2026-08-25
bee:
  id: board-live-morph-delivery
  lifecycle: active
  required_context: [docs/history/board-live-morph/CONTEXT.md]
  sources: [docs/history/board-live-morph/CONTEXT.md, .bee/cells/archive/board-live-morph/blm-1.json, .bee/cells/archive/board-live-morph/blm-2.json]
---

# board-live-morph — Delivery

## What shipped

- **blm-1** — Every board card shell and finished row carries a stable key of its
  own — the thing's own detail link — taken from one hoisted value so the key and
  the link can never disagree (1 file changed).
- **blm-2** — A change signal no longer reloads the page. The board refetches its
  own HTML, reconciles cards against the live DOM by that key, keeps the live nodes
  it already had, keeps every card and column the reader opened open, and slides the
  survivors to their new boxes; every failure path still falls back to a full reload
  (3 files changed).

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **blm-1** — `cargo test -p waggledance`
- **blm-2** — `cargo test -p waggledance`

Verified end to end in a real browser against a running daemon: the change signal
arrived at 260ms, the section patched at 327ms with no navigation, a new card
entered, and eight surviving cards each took a transform before it was cleared to
let the transition carry them.

## Deviations

- **blm-1** — The finished row places its key after the link rather than beside the
  group attribute: two dozen pre-existing assertions pin those two attributes
  literally adjacent, and inserting between them would break all of them for no
  behavioural gain, since the row link *is* the key value.
- **blm-1** — Three assertions outside the cell's known-anchors list had to be
  fixed; the full test run surfaced them, the cell's own scoped run did not.
- **blm-2** — The new card and row transition rules were placed after the two real
  rules of the same name rather than beside the card container, because two existing
  tests locate those rules by first- and second-literal-substring match and an
  earlier occurrence of the same substring broke both.

## Provenance

Reviewed and applied from `bee knowledge promote --work board-live-morph`
(docs/history/board-live-morph/promote-proposals.md) at compound. Duplicate
restatements of the same deviation were merged; nothing else was dropped.
