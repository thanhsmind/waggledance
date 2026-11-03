---
type: bee.pattern
title: "Resolve a pane's project from its own form, never from the page"
description: "Pitfall: the homepage Terminals tab hosts panes from many projects and carries no page-level project id, so any per-pane feature that reads the project from the page works everywhere it was tested and silently loses context there."
timestamp: 2026-08-31
bee:
  id: resolve-a-panes-project-from-the-form-not-the-page
  lifecycle: active
  areas: [agent-terminal]
  sources: [.bee/cells/csl-3.json]
  polarity: pitfall
  signature: per-pane feature reading data-project-id off the page
---

# Resolve a pane's project from its own form, never from the page

## The trap

There are two pages that render pane composers, and they disagree about what a
page *is*.

The project terminal page has one project, so a page-level `data-project-id` is
a correct shortcut. The **homepage Terminals tab** hosts panes belonging to many
projects and carries no page-level project id at all. A per-pane feature written
against the page therefore works on every screen it was likely tested on and
loses the pane's project on the one screen that has more than one.

`composer-slash-suggest` shipped this way and it took a UAT round trip: the
slash-suggest fetch resolved `/p/<id>/_slash` from the page, and on the home tab
that resolved to `/p/null/_slash`. `csl-3` fixed it by deriving the URL
per-form from **`data-term-base`** (`/p/<id>/_terminal/<pane>`), with the page
id kept only as a fallback.

## The rule

The pane's own form already carries the answer. Read it from there. A page-level
id is a fallback, never the source.

## The same mistake from the other direction

The Agents drawer showed the sibling shape: `agentRow` rewrote **every** row's
href to `/?tab=terminals&pane=<pane_id>`, on the page-level assumption that
every agent row names a herdr pane. A paseo row's id is `paseo:<uuid>`, which is
not a pane, so the Terminals tab opened on nothing — while the row's own
`agent.url` served a perfectly good page. A discriminator written into a comment
above that branch had also stopped being true a feature earlier, and the stale
comment is what let the rewrite look correct.

Both are the same error: a truth that holds for one page's rows applied to a
page whose rows are heterogeneous. When a container can hold items from more
than one owner, the owner is a property of the **item**.
