---
type: bee.area
title: Owner briefs for every open work item
description: "One Vietnamese decision-ready brief per open PBI and active feature under docs/briefs/, read by the work.gogl.be mirror"
tags: [briefs, mirror]
timestamp: 2026-10-28
bee:
  id: owner-briefs-owner-briefs-for-every-open-work-item
  lifecycle: active
  areas: [owner-briefs]
---

## What it is

Every open work item of this repo carries one owner brief at
`docs/briefs/<id>.md`: one per PBI whose status is proposed, ready or
in-flight, and one per active feature workflow. The agent-works mirror reads
these files and renders them as decision-ready tasks on work.gogl.be, so the
owner can approve, park, drop, answer or test without opening anything else
(wd-briefs D1).

## Shape

- Flat frontmatter, four keys only: `title` (Vietnamese, ≤ 80 chars, the
  outcome), `audience` (`owner`|`internal`), `size` (`S`|`M`|`L`), `ask`
  (`build`|`park`|`drop`|`answer`|`test`) (wd-briefs D2).
- Six Vietnamese paragraphs with bold labels in fixed order — Là gì, Vì sao,
  Nguồn gốc, Chi phí và rủi ro, Anh cần làm, Chuẩn xong — 120–220
  whitespace-split words, no ids, paths, slugs or function names in the body;
  the mirror adds the technical footer (wd-briefs D3). Vietnamese syllables
  count as words, so a draft aimed at about 180 syllables lands in range.
- Content is assembled from the item's linked sources — request file, owning
  or deferring feature's CONTEXT and plan, decisions, cell records, the main
  commit log — never from the one-line `cos` alone (wd-briefs D4).

## Classification

Tooling, bee mechanics, build, lint and debt are `internal`; anything the
owner sees is `owner`. Merged-but-unclosed and built-awaiting-acceptance work
asks `test`; stale or superseded work asks `drop` with its reason; a proposed
item worth doing asks `build` or `park` with a recommendation; a blocking open
question asks `answer` (wd-briefs D5). A PBI bound to an active feature keeps
its own brief (wd-briefs D6); a PBI whose id equals a feature slug shares that
feature's brief (wd-briefs D8); the brief-writing feature itself gets none
(wd-briefs D7).

## Keeping them true

A brief is a snapshot: when an item's state changes, its brief is rewritten,
and a merge flag or a one-line summary is never trusted over the commit log.
On the first pass the inventory's merge flag missed work landed directly on
main, and three lint briefs claimed green gates that clippy showed red; both
were caught only by checking the tree.

## Pointers

- `docs/briefs/` — the briefs.
- `docs/history/wd-briefs/CONTEXT.md` — the locked decisions D1–D8.
- `.bee/spikes/wd-briefs/check_briefs.py` (in the wd-briefs worktree) — the format checker.
