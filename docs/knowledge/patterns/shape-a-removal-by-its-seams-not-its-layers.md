---
type: bee.pattern
title: "Shape a removal by its seams, not its layers"
description: "Pitfall: a deletion looks like it slices backend-then-frontend, but the feature's arguments and struct fields run through the shared files at once — every layer split leaves a non-compiling tree between cells, and the honest shape is one atomic cell."
timestamp: 2026-08-31
bee:
  id: shape-a-removal-by-its-seams-not-its-layers
  lifecycle: active
  areas: [agent-board]
  sources: [.bee/cells/prm-1.json]
  polarity: pitfall
  signature: "removal split by layer, tree will not compile between cells"
---

# Shape a removal by its seams, not its layers

## The trap

Removing a feature reads like the easiest thing to slice: take the backend out
in one cell, the frontend in the next. `paseo-removal` — 5339 deletions across
11 files, two whole modules, four routes, 89 tests — could not be sliced that
way at all.

The reason is what a removal actually is. A feature that is worth removing has
grown *into* the shared files: `home_page` and `project_sidebar` took a paseo
argument, `ProjectSuggestion` carried `paseo_count`, `build_state` named two
`AppState` fields. Every candidate seam ran through `server.rs` and `views.rs`
at the same time, so any split left a tree that did not compile between cells —
and a non-compiling intermediate state cannot be capped, cannot be proven, and
cannot be handed to a sibling worker.

One atomic cell was the honest shape.

## The test scope is the whole suite

A removal is the case the narrow-scope cap rule does not cover. The risk of
deleting a feature is not in the files you touched — it is in everything that
quietly depended on them. `paseo-removal` declared the **full** declared test
command as its verify on purpose, and that was the right call rather than a
cautious one.

## The grep gate catches what the compiler cannot

After green, `rg -i paseo crates/` returning nothing is a separate gate. It
caught leftover **wording** in surviving shared comments — text that describes
a feature nobody can use any more. A compile check has no opinion about prose,
and a stale comment is exactly how the next reader learns something false. Note
which trees you deliberately exclude: `docs/history/` stayed byte-identical,
because history is never rewritten.

## Related

- [[a-breaking-interface-change-has-no-separable-cells]] — the same shape, from
  the other direction: when the interface moves, the cells cannot be split
  either.
