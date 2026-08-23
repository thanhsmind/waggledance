---
type: bee.delivery
title: tabbar-collapse — delivery
description: "Delivery record for work item tabbar-collapse: 1 capped cell(s), 5 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: tabbar-collapse-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [docs/specs/bee-cockpit.md]
  sources: [.bee/cells/archive/tabbar-collapse/tbc-1.json]
---

# tabbar-collapse — Delivery

## What shipped

The handset bottom tab bar (Board · Agents · Projects · Settings, from
console-phone-layout) folds away. It is hidden on first visit; a small centred
pill handle on the bottom edge shows or hides it, and the choice is remembered
per browser across reloads. With scripting off the bar stays visible and the
handle stays hidden — the same shape the rail collapse already uses, so the
cockpit has one hide-and-remember idiom, not two. Decision 75a5b463 (touches
41015896, the tab bar itself).

The bar stole a fixed strip of a phone-width terminal on every reload; the
terminal is the screen a phone user most often keeps open.

## Verify

`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D
warnings && cargo test -p waggledance home_page_renders_the_handset_tab_bar`
green; the existing tab-bar test was extended in place to find the bar by its
new id rather than a sibling test being added.

## Deviations

- The bar is a sibling of the page shell, not a descendant, so the handle's
  script and styling key off the bar itself rather than the shell; the shell
  still drives the content and scroll-stack offsets that are its descendants.
- No CONTEXT.md existed for this feature; the worker read decision 75a5b463
  from the store instead. The worker also registered its own worker record —
  dispatch had not created one and the cap refused without it. Neither
  generalizes beyond this run; the pattern candidate the close mined from them
  was not promoted for that reason.
