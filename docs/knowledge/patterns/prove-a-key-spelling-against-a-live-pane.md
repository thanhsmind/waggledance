---
type: bee.pattern
title: Prove a key spelling against a live pane running cat -v before wiring a button for it
description: "Practice: whether herdr honours a given key spelling, and what bytes it delivers, is answerable in one POST to a live pane running cat -v — which is cheaper and more certain than reading a wire format and shipping a button that sends nothing."
timestamp: 2026-08-31
bee:
  id: prove-a-key-spelling-against-a-live-pane
  lifecycle: active
  areas: [agent-terminal]
  sources: [.bee/cells/tks-1.json]
  polarity: practice
  signature: key spelling assumed from a wire format
---

# Prove a key spelling against a live pane running cat -v before wiring a button for it

## The situation

The pane key grid needs a new key. Whether the transport honours the spelling
you have in mind — and what the terminal on the other end actually receives —
is guesswork from documentation, and the failure mode is a button that ships,
renders, passes its tests, and sends nothing.

## The practice

Run `cat -v` in a live pane, POST the key to it, and read what comes out.
`cat -v` prints control bytes visibly, so the answer is on screen rather than
inferred.

`term-keys-shifttab` settled the fact this way: **herdr honours the `shift+tab`
wire spelling and delivers it as `ESC[Z`**. That one measurement let the pane
key grid replace its Alt latch with a plain ⇧Tab key and drop the last
`data-mod` button — a simplification that would have been unsafe to attempt on
an assumption.

## Why it generalizes

It is the cheapest end-to-end proof in this repo: one live pane, one POST, one
look. It costs less than reading the wire format carefully, and unlike reading
it, it cannot be wrong. Reach for it before wiring **any** new key, and
especially before removing a mechanism (a latch, a modifier) on the belief that
a simpler spelling covers it.

## Related

- [[prove-the-whole-path]] — a user-visible outcome owes one proof that crosses
  every seam; for a key, this is that proof.
