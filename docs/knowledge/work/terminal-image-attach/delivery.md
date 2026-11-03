---
type: bee.delivery
title: terminal-image-attach — delivery
description: "Delivery record for work item terminal-image-attach: images can be attached to a reply on a project's terminal page — by picker, drag or paste — and are sent as one message, with what may be stored bounded on both size and count."
timestamp: 2026-08-08
bee:
  id: terminal-image-attach-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/specs/agent-terminal.md, docs/history/terminal-image-attach/CONTEXT.md]
  sources: [docs/history/terminal-image-attach/CONTEXT.md, docs/history/terminal-image-attach/plan.md]
---

# terminal-image-attach — Delivery

## What shipped

An agent can be shown a picture. On a project's terminal page the reply
composer accepts images three ways — a file picker, a drag onto the composer,
and a paste — several at a time. Each one appears as a chip the operator can
remove before sending, and the whole set leaves as a single message rather than
one message per file, so the agent reads the text and the images together.

What may be stored is bounded on every axis that could otherwise grow without
limit. A file is accepted only if it is genuinely an image: the declared type
must be on a short allowlist *and* the bytes themselves must begin the way that
type begins, so a renamed file is refused rather than stored. There is a ceiling
on the size of one file, checked in the page before the upload starts as well as
on the machine that stores it, and a ceiling on how many files one pane may hold
at once — reached only after stale files from earlier sessions are pruned, so
ordinary use never meets the cap. Stored names are generated rather than taken
from the upload, and a path containing spaces is quoted where it is handed on.

The whole surface is gated: it renders only on a project's own terminal pages,
behind the same switch as the rest of the terminal family. The Unassigned view
renders none of it.

## Verify

`cargo test --workspace` green at every cap, plus targeted runs over the attach
routes and the terminal page render. The route tests cover the guards, the type
allowlist and byte sniffing, both ceilings and the pruning that precedes the
count cap; the render test proves the Unassigned page carries none of the
composer's attach controls.

## Deviations

None recorded.

## Provenance

Written at bundle cleanup from three capped cell traces and the feature's
CONTEXT.md. The behavior is already stated in `docs/specs/agent-terminal.md`,
including per-pane storage and both ceilings. The verify commands in the traces
name the crate under its former name (`mdview`), retired by `waggledance-rename`.
