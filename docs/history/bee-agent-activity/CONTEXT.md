---
feature: bee-agent-activity
started: 2026-08-22
status: locked
decisions: [77320e44, 3d631a7a, 8105fd2f, 110d9120, 893039ae, 8cce43b3]
---

# Read bee's agent activity and show it across the cockpit

## What the user asked for

bee 2.20.0 now records, per session, what the agent is doing from Claude
Code hooks (`docs/history/research/bee-agent-activity-contract.md`). Show it
on the board, the rail's Pinned rows, the Agents drawer, the terminal pane
card (Approve only when it is safe), the phone tiles, and the notifier.

## Locked decisions

**A1 — Source is the session file, read-only.** (decision `77320e44`)
`activity` is parsed from `.bee/sessions/<id>.json` in the existing
file-read path; `<id>.activity.jsonl` is tailed read-only by the notifier.
`signal` (live / no_signal at 90 s on `activity.at`) is derived at read.
Absent `feature`/`cell` render as "—".

**A2 — Joins.** (decision `3d631a7a`)
Session ↔ pane: `activity.pane == pane_id`. Session ↔ project:
`activity.cwd` through the same Boundary rule panes use (worktree siblings
included, first project wins). Session ↔ card: `activity.feature`.
*Reconciled 2026-08-23:* the feature join also decides a board card's
terminal markers — a main-checkout pane whose session names a feature
appears on that card only (decision `3daa1ea7`, board-pane-lane-pin); and
the feature itself is resolved cell > worktree > lane > record
(agent-feature-resolution).

**A3 — Vocabulary and precedence.** (decision `8105fd2f`)
Five states: working, waiting_input ("needs an answer"), blocked ("needs
approval"), idle, exited. bee's state wins over herdr's screen-derived status
wherever both exist for a pane. need-you = blocked ∪ waiting_input in every
count (card line, In Progress waiting chip, phone need-you tile, Pinned,
drawer). Every state reads as a word beside its colour. `no_signal` is a
muted marker, never need-you.

**A4 — Approve gating.** (decision `110d9120`)
The pane card carries the bee state as a data attribute; Approve is enabled
only for `blocked`, disabled with an explanatory title otherwise; a pane
with no bee record keeps today's behaviour.

**A5 — Notifier.** (decision `893039ae`)
A second cursor over bee session activity on the existing 2 s tick, same
outbox and run-ownership suppression; fires once on entry into
{blocked, waiting_input} (escalation does not re-fire) and once on exited.

**A6 — Card agent line.** (decision `8cce43b3`)
"agent: <state> · <cell title or id | —> · quiet <age>"; drawer and Pinned
rows add the state word and feature; worker nicknames stay unrendered.

## Boundaries this feature inherits

- bee.rs never writes and never shells out (read-never-writes tests).
- The cockpit is read-only except the existing terminal write routes.
- Cards render only data-backed elements (console-theme-kanban D2).
- Status never speaks by colour alone (rail landmark test).
- Existing literal tests on badges, dots, waiting chip, stat tiles, Approve
  button, `/api/agents`, notify obligations move in lockstep, never deleted.

## Added during UAT

**R1 — Ready to merge widens.** (decision `63bffb34`, supersedes `420cec71`)
A feature enters the column when its worktree grant is open, its execution
gate is approved, it has ≥1 cell and every non-dropped cell is capped —
uat approved or not; the card says `uat approved` / `uat pending` and how
long it has been ready (latest cap). uat-approved first. Zero-cell grants
are not ready. A stored `merge_ready` fact from bee, once written, is
preferred; this derivation is the fallback.

**R2 — Compact footer and branch on Ready to merge.** (decision `8b057354`)
The progress bar, the "n/m cells done" label and the separate "Last
activity" row collapse into one footer line — a ring glyph + `n/m cells`
left, last-activity time right — on every card; the Ready to merge card
shows its `wt/<feature>` branch line under the title.
