---
type: bee.pattern
title: "Read the live theme's token definition, never the base contract's"
description: "Pitfall: a design token borrowed by reading contract.css can mean something entirely different in the theme actually serving the page, and the sibling defect in the same report shows why green contains() assertions are no evidence that a stylesheet still parses."
timestamp: 2026-08-31
bee:
  id: read-the-live-themes-token-not-the-base-contracts
  lifecycle: active
  areas: [agent-board]
  sources: [crates/waggledance/assets/atelier/console.css]
  polarity: pitfall
  signature: token borrowed from the base contract
---

# Read the live theme's token definition, never the base contract's

## The trap

`--color-accent-alt-2` reads as a neutral accent in `contract.css`. In
`console.css` — the theme actually live under `data-theme=console` — the whole
`accent-alt` set is redefined as **board lane identity**, and `alt-2` is the
orange that means *needs you*.

Borrowing the token from the base contract therefore paints a control in the
colour of an alarm. The name is stable across themes; the meaning is not. A
token's definition is theme-local, so the file to read is the one the page
actually loads, never the base the theme overrides.

## The sibling defect in the same report

Both of these arrived behind one user sentence — "I do not see the CSS reload".
The second is worth the same shelf space.

A CSS comment inside `views.rs`'s inline `<style>` **closed one line early**.
Eight lines of prose became CSS, the parser choked there, and it swallowed the
rule immediately below it. Every `contains()` assertion stayed green, because
the literal was still present in the served bytes — just no longer inside a
comment.

**String assertions cannot see structure.** The guard that can is
`inline_style_blocks_have_balanced_comments_and_braces`, which counts `/*`
against `*/` and braces per served `<style>` block, and was proven red against
the reintroduced defect before being kept.

## The diagnostic order that found both

Serve the page and inspect what actually reaches the browser — rather than
trusting green tests plus a byte-identical binary. Both defects were invisible
to the suite and obvious in the response body. See
[[a-second-daemon-proves-it-without-taking-the-users]] for doing that without
displacing the user's daemon.

## Pointers

- `crates/waggledance/assets/atelier/console.css` — the live theme's accent set
- `crates/waggledance/assets/atelier/contract.css` — the base, and the trap
