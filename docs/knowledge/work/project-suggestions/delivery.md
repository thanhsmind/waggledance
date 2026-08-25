---
type: bee.delivery
title: project-suggestions — delivery
description: "Delivery record for work item project-suggestions: the projects page offers unregistered folders that already have agent sessions running in them, and refuses to name any folder the operator has no business seeing."
timestamp: 2026-08-08
bee:
  id: project-suggestions-delivery
  lifecycle: active
  areas: [web-interface]
  required_context: [docs/specs/web-interface.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# project-suggestions — Delivery

## What shipped

An agent is often already working in a folder the interface does not know about.
The projects page now notices: it lists folders that have a live agent session
running in them but are not registered, each offered with its path and how many
sessions it holds, and registering one from the suggestion removes it from the
list and adds it as a project.

The list is a complement to what is registered, never a directory listing. A
candidate is dropped when it is contained under a project that is already
registered — compared component by component on the raw path, so a folder cannot
slip through by sharing a prefix with a registered root without actually being
under it. A working folder that is expressed with traversal segments is dropped
outright rather than resolved and shown, which closes two ways the page could
otherwise have named a folder outside anything the operator registered: a
sibling reached by climbing out of a known root, and a row whose button led
somewhere the page would then refuse to register.

Trailing-slash spellings collapse to one row, rows sort in a fixed byte order so
the list does not reshuffle between visits, and the block appears only while the
terminal family is switched on.

## Verify

`cargo test --workspace` green at each cap. The disclosure cases carry their own
tests — a sibling reached by traversal, a candidate under a registered root, a
duplicate differing only by trailing slash — and the register-from-suggestion
flow asserts the row is gone afterwards, alongside a duplicate banner case and a
switched-on-but-no-path case.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from five capped cell traces.
