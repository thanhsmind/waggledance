# Piggery Lessons — Context

**Feature slug:** piggery-lessons
**Date:** 2026-10-16
**Shaping session:** complete (spec drop sup-20261016-19a3 states the ask in full; no gray area left for the owner)
**Scope:** Small (docs and backlog only)
**Domain types:** SEE | RUN

## Feature Boundary

An evidence-based evaluation of what piggery v0.10.2 can still teach the
waggledance dispatch, delivery and board paths, after `leader-delivery`
D1–D4 already took four lessons from it. The output is one research brief
under `docs/history/research/` and one `proposed` PBI per improvement worth
doing. It ends there: no product code changes, no piggery integration, no
edits to the piggery repository.

Origin: spec drop `sup-20261016-19a3` from `waggledance@e8b3751`.

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Only Claude Code inside Paseo is in scope. piggery's multi-harness layer (drivers, adapter socket, per-harness setup) is not evaluated. | The owner said bee is already the harness (spec drop). |
| D2 | Each of the six hypotheses closes as one of: worth doing / already equivalent / not worth it / wrong premise, with evidence from this repo (file and line, or command output) or from a recorded capture. | Spec drop "Giả thuyết cần kiểm chứng". |
| D3 | Every improvement judged worth doing becomes its own `proposed` PBI; nothing is built in this feature. | Spec drop "Phạm vi". |
| D4 | Locked decisions of earlier features are cited, never reopened. A finding that conflicts with one is reported with its evidence and left for the owner. | AGENTS.md; leader-delivery D2/D6 are in play. |

### Agent's Discretion

The brief's file name, its section order inside the research template, and
how findings rank by value over cost.

## Existing Code Context

- `crates/waggledance/src/delivery.rs` — leader queue, delivery, done/lost, caller notices.
- `crates/waggledance/src/paseo.rs` — `resolve_leader`, `parse_turn_end`, fixture tests.
- `crates/waggledance/testdata/fixtures/paseo-0.11.2/` — recorded Paseo CLI captures.
- `crates/waggledance/src/server.rs`, `crates/waggledance/src/views.rs` — board, `/api/board`, "Waiting on you".
- `crates/waggledance/src/trigger.rs`, `docs/history/observer-tick-trigger/CONTEXT.md` — observer tick.
- `docs/waggledance-supervisor-skill-template.md` — the supervisor skill's leader path.
- Prior piggery-derived features: `docs/history/leader-delivery/CONTEXT.md`, `docs/history/pi-paseo-plugin/CONTEXT.md`.
