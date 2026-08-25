---
type: bee.delivery
title: session-work-line — delivery
description: "Delivery record for work item session-work-line: a live session's line now names what that session was asked to do, not just which lane it is bound to."
timestamp: 2026-08-25
bee:
  id: session-work-line-delivery
  lifecycle: active
  required_context: [docs/history/session-work-line/CONTEXT.md]
  sources: [docs/history/session-work-line/CONTEXT.md, .bee/cells/swl-1.json, .bee/cells/swl-2.json]
---

# session-work-line — Delivery

## What shipped

- **swl-1** — The session snapshot carries the work record bee's own hook writes —
  title, status, turn count, acceptance — parsed fail-open, path-scrubbed, and
  deliberately without the conversation text (2 files changed).
- **swl-2** — The Live strip row names what each live session was asked to do, with
  its status both as a word and as an attribute, and the acceptance on the row's
  title (1 file changed).

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **swl-1** — `cargo test -p waggledance-core && cargo check --workspace`
- **swl-2** — `cargo test --workspace`

## Deviations

- **swl-1** — Rendering was deferred with a reason: the view file was reserved by
  the in-flight cell `cgs-1` (132 uncommitted lines), so the disjoint reader half was
  taken first and the render re-triaged after that cell merged.
- **swl-1** — This repo had no declared test command until now; `cargo test
  --workspace` was recorded at the user's explicit approval, by hand, because
  `bee config set` is declared in the registry but was never ported off Node and its
  own refusal names hand-editing the JSON as the remedy.
- **swl-1** — The session parser gained a root parameter so the path scrub has a
  root; both call sites already had one in hand.
- **swl-2** — The locked shape asks for a board card once a record carries an
  acceptance; not built. The board's card unit is the *feature*, not the session, so
  a session-shaped card is a new card type and a design move of its own — reported
  rather than improvised.
- **swl-2** — The user approved this scope in conversation after the brief had
  deferred it; the lane's shape and execution gates were already approved for the
  feature, so no second gate was presented.
- **swl-2** — One pre-existing warning stands in the suite (a duplicated attribute in
  the MCP surface), in a file this cell never touched.

## Provenance

Reviewed and applied from `bee knowledge promote --work session-work-line` at
compound. Duplicate restatements of the same deviation were merged; nothing else was
dropped.
