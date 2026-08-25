---
type: bee.delivery
title: projects-home — delivery
description: "Delivery record for work item projects-home: the projects list shows which projects have terminal sessions running and can register a new project itself, with the path validated before anything is written."
timestamp: 2026-08-08
bee:
  id: projects-home-delivery
  lifecycle: active
  areas: [web-interface, agent-terminal, system-overview]
  required_context: [docs/specs/web-interface.md, docs/specs/agent-terminal.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# projects-home — Delivery

## What shipped

The projects list became the place work starts rather than a static index.

Each row now carries the terminal sessions running in that project's own folder,
matched by the same boundary rule the rest of the interface uses, so a row never
claims a session belonging to a sibling folder. The badges appear only while the
terminal family is switched on, and reading them cannot hold the page hostage:
the snapshot they come from is taken under a time budget, and a slow or
unavailable source leaves the row rendering without badges instead of leaving the
page unrendered.

The page also registers a new project itself, from a form on the page, instead of
sending the operator to the command line. The path is validated before anything
is recorded, in a fixed order: a bounded pre-flight look at the path, then a
deny-list check that a candidate is not somewhere it must never be — including
anywhere *contained under* a denied location, not merely equal to it — and last a
duplicate check made on the canonical path, so two spellings of the same folder
cannot both be registered. Validation runs off the thread serving the page.

## Verify

`cargo test --workspace` green at each of the three caps. The edge cases were
split into tests of their own rather than folded into the happy path: a failing
store, the time budget expiring, a duplicate reached by a different spelling, and
the badges disappearing when the terminal switch is off.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from three capped cell traces.
