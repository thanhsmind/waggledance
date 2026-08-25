---
type: bee.pattern
title: Assertions that pin literal adjacency decide where new code is allowed to go
description: "Pitfall: tests that locate markup or style by literal substring, whole-string equality, or first-versus-second match turn the ordering of a rendered file into a contract — so the natural place to insert a new attribute or rule is the one place that breaks a dozen tests for no behavioural reason."
timestamp: 2026-08-25
bee:
  id: assertions-that-pin-literal-adjacency
  lifecycle: active
  areas: [web-interface, bee-cockpit]
  sources: [.bee/cells/archive/board-live-morph/blm-1.json, .bee/cells/archive/board-live-morph/blm-2.json, docs/knowledge/work/board-live-morph/delivery.md]
  polarity: pitfall
  signature: literal-substring match
---

# Assertions that pin literal adjacency decide where new code is allowed to go

## The trap

A renderer emits markup or style as strings, and its tests check the output the
cheapest way available: a whole-string equality on a small fragment, a substring that
happens to span two attributes, or "the first occurrence of this selector". None of
those state a behaviour. All of them state an *ordering*, and once a few dozen of
them exist, the ordering is a contract nobody wrote down.

The cost lands on the next person who adds a field. The obvious insertion point —
beside the related attribute, next to the related rule — is precisely the point that
splits an asserted adjacency or shifts a first-match, and the change goes red in
places that have nothing to do with it.

## What it looked like here

Adding one stable key attribute to a board card and a finished row, and adding
transition rules for two existing class names:

- Two dozen assertions pinned a group attribute and a link attribute literally
  adjacent on a row, so the key had to go *after* the link instead of beside its
  sibling.
- Three whole-shell equality assertions outside the cell's listed anchors broke; only
  the full suite surfaced them, the cell's own scoped run did not.
- Two tests located style rules by first- and second-literal-substring match, so new
  rules for the same class names had to be placed after the real ones rather than
  beside the container they belong to.

Three deviations in one two-cell feature, none of them about behaviour.

## The tell

Grep the test file for the class name or attribute you are about to touch. If the
hits are `assert_eq!` on a whole fragment, or a substring spanning two attributes, or
anything selecting by *n*-th occurrence, the file's ordering is load-bearing before
you edit it.

## What to do instead

- Run the *package* suite, not the cell's named anchors, before capping a renderer
  change. Scoped-green is exactly what misses these.
- Place the new thing where the tests allow and record a one-line deviation saying
  why it is not where it belongs — the reason is the artifact, not the position.
- When writing a new assertion, assert the presence of what the behaviour promises,
  never the neighbours it happens to sit between.
