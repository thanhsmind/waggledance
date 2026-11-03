---
type: bee.pattern
title: A breaking interface change has no separable cells
description: "Pitfall: splitting a change that breaks every caller into one cell per caller produces cells that cannot each be proved green, because nothing compiles until the whole set lands — the split is recorded as a deviation instead of being seen at plan time."
timestamp: 2026-08-25
bee:
  id: a-breaking-interface-change-has-no-separable-cells
  lifecycle: active
  areas: [workflow-state, orchestration]
  sources: [.bee/cells/archive/herdr-protocol-20/hp20-1.json, .bee/cells/archive/herdr-protocol-20/hp20-2.json, .bee/cells/archive/herdr-protocol-20/hp20-3.json, docs/knowledge/work/herdr-protocol-20/delivery.md]
  polarity: pitfall
  signature: capped against a slice-wide green run
---

# A breaking interface change has no separable cells

## The trap

A plan divides work the way the code reads: change the interface, then update the
first caller, then the second. Each step looks like a cell — one clear outcome, one
small set of files, one proof.

It is not one, and the reason is structural rather than a matter of care. Changing
the shape of something every caller depends on breaks all of them the instant it
lands. Until the last caller follows, nothing builds, so no intermediate cell has a
green run available to be capped against. The plan asked for a proof that cannot
exist yet.

## What it looked like here

A port to a new terminal protocol changed one trait's signature and its three
callers. Three cells were planned and all three had to be executed as a single
landing, capped in order against one shared green run and one shared commit. The
same fact was then recorded four separate times across the cells' traces — "could
not be capped separately", "the slice landed as one commit", "capped against the
slice-wide green run" — each an honest deviation, none of them news by the second
telling. The work was correct throughout; the shape it was asked to fit was not.

## The practice

- At plan time, ask of each cell: is there a green run available at the end of this
  cell alone? If a cell's change breaks callers the next cell repairs, the answer is
  no and the two are one cell.
- Size the cell to the compile boundary, not to the file boundary. One cell that
  changes an interface and every caller is honest; three that cannot each build are
  not, however small each looks.
- When it is discovered mid-flight rather than at plan time, record it once on the
  first cell and let the later caps cite it, rather than restating it on each.
- This is the mirror of [[prove-the-whole-path]]: there, slices each go green while
  the feature stays inert; here, the slices cannot go green at all. Both come from
  cutting cells where the code reads rather than where a proof can close.
