---
type: bee.delivery
title: todo-column-collapse — delivery
description: "Delivery record for work item todo-column-collapse: the board's Todo column ships folded behind a native disclosure on both boards, keeping its dot, label and true count in the summary; the four columns beside it are unchanged."
timestamp: 2026-08-25
bee:
  id: todo-column-collapse-delivery
  lifecycle: active
  areas: [bee-cockpit, web-interface]
  required_context: [docs/specs/bee-cockpit.md, docs/specs/web-interface.md, .bee/lanes/todo-column-collapse.json]
  sources: [.bee/lanes/todo-column-collapse.json, .bee/cells/tcc-1.json]
---

# todo-column-collapse — Delivery

## What shipped

The board's Todo column arrives folded. It is the one column that grows without
bound while the four beside it stay short, so its length was costing the reader
the rest of the board — a person opening the board to see what is in flight had
to scroll past everything not yet started.

Todo is now a native disclosure: closed on arrival, opened by clicking its
header, with a chevron on the right that turns as it opens. The header keeps
everything it stated before — the lane dot, the column name, and the true count
of what is inside — so a folded column never understates what it holds. The
other four columns keep their previous shape and are not foldable; only Todo
grows the way that made folding worth it.

Nothing is remembered between visits. There is no script behind the fold and no
stored preference, so a refresh returns the column to closed, matching how the
archive bar and the In Progress cards already behave. That is the deliberate
trade: a reader who opens Todo is opening it for this reading, not changing the
board for every future one.

Both boards changed together — the per-project board and the cross-project one —
because the column means the same thing on each, and a fold that applied to only
one would read as a bug on the other.

## Locked decisions

| ID | Decision |
|---|---|
| 99d0b579 | The Todo column ships collapsed, and only Todo folds — native disclosure, nothing persisted, with the dot, label and true count kept in the summary. |

## Verify

`cargo test -p waggledance` green — 860 + 3 + 5 passed at cap. The tests that
pinned Todo's previous wrapper were updated rather than deleted, and one asserts
the pairing that carries the whole point of the feature: Todo ships closed while
still stating its true count. The four unfolded columns are asserted to keep
their previous shape.

Not yet confirmed against a running daemon — the recorded next step is a daemon
restart to see the folded column on the live board.

## Deviations

The cell capped without a commit. The file it had to touch already held
substantial unrelated in-flight work when the cell started, and one commit per
cell cannot be honoured on a file like that without sweeping a stranger's
changes into this feature's commit. The cell recorded the reason and left the
work staged-free for the owner to split.

That deferral did not hold, and the outcome is the part worth keeping: the fold
went on to land inside commit `30f4917`, alongside the unrelated agent-mark work
it was sharing the file with — swept in exactly as predicted, by whoever
committed the file next. On a hot shared file, deferring the commit does not
protect the boundary; it only moves who does the sweeping and removes the
deferring cell's say in how the result is described.

A formatting pass also normalised one pre-existing unformatted assertion
belonging to that same in-flight work.

## Pointers

`bee_hub_group` in `crates/waggledance/src/views.rs` carries a `collapsible`
flag; when set, the group renders as `<details class="bee-hub__group">` with no
`open` attribute and its `<h4 class="bee-hub__group-header">` inside a
`<summary>`. The summary's rules — native marker stripped, pointer cursor, focus
ring, chevron rotation on `[open]` — live in `bee_hub_style`. Shipped in
`30f4917`.

## Provenance

Written at feature close from the capped trace of `tcc-1` and decision
`99d0b579`. The commit that carried the change was verified in git history at
scribing time, not taken from the cell's own record, which still reads as
uncommitted.
