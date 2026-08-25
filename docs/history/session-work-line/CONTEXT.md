# Session Work Line — Context

**Feature slug:** session-work-line
**Date:** 2026-08-25
**Shaping session:** derived — no new product decisions
**Scope:** Quick
**Domain types:** READ | SEE

## Feature Boundary

waggledance reads the `work` object bee's activity hook now writes beside
`activity` on `.bee/sessions/<id>.json`, so a session can say what it was
asked to do and not only that it is busy. It ends at the reader and the
snapshot it feeds; drawing the value on the board is the deferred half below.

## Locked Decisions

Every product decision here was locked in the bee repo and is cited, not
re-decided: `docs/history/prompt-work-record/CONTEXT.md` D1–D6 in
`goglbe/beehive` (decisions `bd78f64a`, `856789db`, `5944ebbb`, `34944b7a`,
`f9bf6456`). D6 is the one that puts this work in this repo.

The reader's own calls, inside those decisions:

| ID | Decision | Rationale |
|----|----------|-----------|
| W1 | `BeeSession` gains `work: Option<BeeWork>`, parsed from the record's top-level `work` key. Absent, malformed, or titleless reads as `None`, never as a failed session parse. | The same fail-open rule `activity` already has (`bee.rs:2481`): a bad sub-object must never cost the whole session row. |
| W2 | `BeeWork` carries `title`, `status`, `turns`, `acceptance`, `updated_at` — and NOT the record's `text`. | `text` is the whole conversation, capped at 8000 characters per session, re-read on every snapshot. The board shows a title; the acceptance is the detail. Carrying the transcript would put every user's prose through every read for nothing. |
| W3 | `status` is carried verbatim as a `String`, an unrecognised value included. | A newer bee that adds a fifth status must not blank the row on an older viewer. Same posture as `BeeActivityState::Unknown`. |
| W4 | `title` and `acceptance` are path-scrubbed on the way in with the existing `scrub_paths`. | The codebase already scrubs free text (`BeeLane::next_action`) while carrying known path fields raw. Title and acceptance are free text, and bee's write-side scrub is belt to this reader's braces. |

## Existing Code Context

### Reusable Assets

- `crates/waggledance-core/src/bee.rs:2532` `parse_activity` — the fail-open
  sub-object parser this one mirrors, down to the "malformed is `None`" rule.
- `crates/waggledance-core/src/bee.rs:3298` `scrub_paths` — already in this
  crate, already used by `parse_lane`.

### Integration Points

- `crates/waggledance-core/src/bee.rs:2484` `parse_session` — gains the `root`
  parameter `parse_lane` already takes, so the scrub has a root to work against.
- `crates/waggledance/src/server.rs:29600` — the one test helper that builds a
  `BeeSession` by struct literal, so it needs the new field.

## Deferred

Rendering the value on the board is deferred, with a reason: `views.rs` is
reserved right now by the in-flight cell `cgs-1` (card-glyph-is-status), whose
132 uncommitted lines sit in that file. Taking the disjoint half first and
re-triaging the render after that cell merges is the standing rule for an
overlap, not a new judgement.

## Handoff Note

Decision IDs W1–W4 are this repo's; the product decisions they serve are
beehive's D1–D6 and are cited, never reinterpreted.
