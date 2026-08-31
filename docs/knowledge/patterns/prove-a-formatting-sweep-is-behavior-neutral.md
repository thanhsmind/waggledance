---
type: bee.pattern
title: Prove a formatting sweep is behavior-neutral by diffing it whitespace-stripped
description: "Practice: a 34-hunk fmt sweep is usually waved through on the assertion that fmt is safe — stripping all whitespace from before and after and diffing character by character turns that assertion into a short, checkable list of real differences."
timestamp: 2026-08-31
bee:
  id: prove-a-formatting-sweep-is-behavior-neutral
  lifecycle: active
  areas: [workflow-state]
  sources: [.bee/cells/lgg-1.json]
  polarity: practice
  signature: fmt sweep assumed safe
---

# Prove a formatting sweep is behavior-neutral by diffing it whitespace-stripped

## The situation

`lint-gates-green` landed 34 `cargo fmt` hunks across 6 files, mixed in with
four real fixes. A reviewer reading that diff has no way to tell the mechanical
changes from the meaningful ones, and the usual defence — "fmt does not change
behavior" — is an assertion, not evidence.

## The practice

Strip all whitespace from the before and after trees and diff them character by
character. What survives is exactly the set of non-whitespace changes.

On this sweep that left **ten** differences: nine trailing commas, and one match
arm losing its block braces. Ten reviewable items instead of 34 hunks of noise —
and, unlike the assertion, a list that would have grown if the sweep had
actually touched something.

## The sibling finding: not every clippy finding is a defect

The same run met five `result_large_err` warnings, all on helpers returning
`Result<T, axum::Response>` — where the `Err` arm **is** the ready-made HTTP
refusal the caller returns verbatim. Boxing it would have added an allocation
and an unwrap at every call site to satisfy a lint that misread the idiom.

One crate-level `allow` carrying that rationale beat both alternatives: five
scattered suppressions, and bending the design to the lint. Record why a lint is
wrong where the next reader will meet it; a bare `allow` is indistinguishable
from giving up.

## Verify on the branch you will merge into

The gates were confirmed green **on main itself** at `b3ab7b5`, not only on the
branch — see [[proof-run-in-the-wrong-checkout]] for why that distinction is
worth the extra run.
