---
type: bee.pattern
title: A promote-proposal backlog triages itself before anything is read closely
description: "Practice: two mechanical signals — the proposal's own mining summary and each area spec's sources list — sort a large proposal backlog into the few that carry real candidates and the many that carry none."
timestamp: 2026-08-25
bee:
  id: the-proposal-backlog-triages-itself
  lifecycle: active
  areas: [workflow-state]
  sources: [docs/knowledge/index.md]
  polarity: practice
  signature: "area bullet(s), 0 pattern candidate(s)"
---

# A promote-proposal backlog triages itself before anything is read closely

## The situation

Proposals accumulate one per closed feature and go unapplied for months. Read at
full depth, dozens of them are a multi-day job, and reading them shallowly means
stamping reviews that never happened.

## The practice

Two signals sort the pile before any prose is read closely.

**The proposal's own last line** reports what the mining found — capped cells,
area bullets, pattern candidates. A proposal reporting no bullets and no
candidates has nothing to apply by construction; the review is confirming that
the work item's delivery record exists in the bundle.

**Each area spec's `sources:` list** names the features already merged into it.
This is a positive-only signal: a feature listed there is synced for certain, but
absence proves nothing, because specs maintain that list unevenly — one spec may
name twenty-seven features while another names seven and carries far more.
Absence means read the spec; it never means merge blindly.

## What the reading then shows

Mined area bullets mostly restate the cell trace at implementation level —
function, struct and middleware names — or say only "shipped". Specs describe
behavior and never code, so those are not spec material as written. Pattern
candidates mined from a single trace are usually one-off incident notes and fail
the multi-feature and generalizable bars.

Expect a low yield and do not manufacture one: across seventy-eight features in
this repo, three carried a behavior no spec stated, and one candidate earned
promotion. Every other feature still owed its `bee state scribing-run` stamp —
"reviewed, nothing worth keeping" is a legitimate result, and the stamp, not the
merge, is the receipt the reminder reads.
