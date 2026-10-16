# Piggery Lessons — Context

**Feature slug:** piggery-lessons
**Date:** 2026-10-16
**Shaping session:** complete (spec drop sup-20261016-19a3 states the ask in full; no gray area left for the owner)
**Scope:** Standard (evaluation, then the three fixes the owner asked for)
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

### Added when the owner asked to fix the three findings (2026-10-16, «Vậy fix 3 lỗi đó»)

The feature widens from evaluation to building its three PBIs. D3's "nothing
is built" holds for the evaluation pass; this section supersedes it for the
build.

| ID | Decision | Rationale |
|----|----------|-----------|
| D5 | `p-929bf879`: the after-permission path keeps its logic. Two new Paseo 0.11.2 captures pin it: after `permit allow`, the agent stays `running` and `wait` blocks until the turn ends, then reports `idle`; when the turn asks for a second permission, `wait` reports `permission`. Fixture tests assert both. | Probe 2026-10-16 (`.bee/tmp/pg/probe2`, `probe3`): `wait` returned 24 s after the allow, right after `PERM-DONE`. The brief's suspicion that `idle` arrives early is refuted, and luật 9's missing capture is the real gap. |
| D6 | `p-4cae7502`: a Paseo leader run in status `blocked`, or `lost` with `updated_at` in the last hour, is a leader wait. `/api/board` gains `leader_waiting: [{project_id, project_name, run_id, status, since, url}]`, and the project's home rail row shows one pill naming the wait (permission or lost). Read from the `runs` table only. | The board scans only `working` runs (`server.rs:3496`); the data already exists. One hour matches the caller notice window. |
| D7 | `p-f6975647`: a caller notice lives on its run row. Migration 6 adds `notice`, `notice_state` (`pending` / `sent` / `dropped`) and `notice_until`. `end()` writes status, final transcript and notice in one statement. The flush reads pending notices from the store, marks `sent` after a send, and marks `dropped` on expiry. A newer notice for the same run replaces one still pending. The 30 s spacing per caller stays in memory. | Luật 4. Spacing is a rate, not a fact a restart must keep. |
| D8 | No plan-step hat wave for this build. | Three small fixes, each pinned by evidence already in the brief and the probe captures; gate_bypass is `full`. Recorded deviation from the hat-wave form rule. |

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
