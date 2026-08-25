---
type: bee.delivery
title: terminal-open-access — delivery
description: "Delivery record for work item terminal-open-access: the terminal family carries no credential of its own — a single switch decides whether it exists, and a second one decides whether the Unassigned view does."
timestamp: 2026-08-07
bee:
  id: terminal-open-access-delivery
  lifecycle: active
  areas: [agent-terminal, settings, web-interface, bee-cockpit]
  required_context: [docs/specs/agent-terminal.md, docs/specs/settings.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# terminal-open-access — Delivery

## What shipped

The terminal family had a login of its own — a token, a sign-in page, a rotation
route and controls in Settings. It protected pages served only to the local
machine, and it was the only part of the interface that asked for a credential,
so it read as a second front door on a house with one.

It is gone, not disabled. The sign-in and rotation routes, the token controls in
Settings and the module behind them were deleted, and every terminal route now
answers on the strength of the switch alone.

What replaced it is a single question asked consistently. When the family is
switched off, a page route answers as an ordinary not-found — the same answer a
route that never existed gives, so the switch does not advertise what it is
hiding — while a route the page's own scripts call answers with a reason those
scripts can act on. The Unassigned view, which shows sessions belonging to no
registered project, is gated by a second switch of its own, and it is shown only
when both are on: the family switch and its own. Off, it answers in the same
not-found shape as the rest of the family, including the marker that would
otherwise reveal its presence on the home page.

One route tightened rather than loosened: the endpoint that changes the switch
accepts only a structured request body, so an ordinary form post cannot flip it.

## Verify

`cargo test --workspace` green at each cap, with route tests covering all
fifteen terminal routes in both switch states, both switches at the Unassigned
routes, and the refusal of a form-encoded switch change — that last one also
checked by hand against a scratch daemon. The four affected specs were rewritten
in the same feature to describe what the code now does rather than the retired
login.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from five capped cell traces.
