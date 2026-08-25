---
type: bee.delivery
title: topbar-mobile-menu — delivery
description: "Delivery record for work item topbar-mobile-menu: on a narrow screen everything in the top bar that navigates away collapses behind one menu, and the menu needs no scripting to open."
timestamp: 2026-08-08
bee:
  id: topbar-mobile-menu-delivery
  lifecycle: active
  areas: [web-interface]
  required_context: [docs/specs/web-interface.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# topbar-mobile-menu — Delivery

## What shipped

On a phone the top bar ran out of room and its navigation wrapped, taking a
second line away from the page underneath.

Below the shared narrow-screen threshold the parts of the bar that navigate
*away* from the current page now collapse behind a single menu control at the
bar's right edge, opening as a full-width panel under the bar with one
comfortably-sized row per destination and the current one marked. What stays on
the bar is what acts on the page in front of the reader rather than leaving it.
Above the threshold the bar is untouched.

The menu opens and closes without scripting, so it works on a page whose scripts
have not loaded or have failed.

## Verify

`cargo test --workspace` green.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from the capped trace of `topbar-mobile-menu-1`. The
behavior is already stated in `docs/specs/web-interface.md`, including the
no-scripting requirement.
