# Board Run Actions — Context

**Feature slug:** board-run-actions
**Date:** 2026-08-23
**Shaping session:** complete (builds after `board-approve-actions`)
**Scope:** Standard
**Domain types:** SEE | RUN

## Feature Boundary

A feature card on the console board carries buttons that start agent work —
Run review, Run compound, and Start on a Todo card — and the daemon either
sends the command into the feature's live pane or spawns a fresh agent
through bee herding. It depends on the action endpoint and opt-in that
`board-approve-actions` lands first and ends at the run being started and
mirrored; it never judges or merges the result.

Research of record: `docs/history/research/board-actions-from-orchestrators.md`.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 (3971c768) | Run review / Run compound send one slash-command line into the feature's live bound pane when one exists; with no live pane the daemon spawns a fresh agent in the feature's worktree through `bee herding run`. | Reuse the session holding context; spawn only when nothing is live. |
| D2 (d864ae40) | Start on a Todo card is one full herding dispatch: new feature worktree, new pane, the project's default agent preset started with a brief to take the PBI through bee-hive; gates still stop per `gate_bypass` as in any session. | The card moves to In Progress on the session's own activity signal, not on the click. |
| D3 (4c366c35) | One board-triggered run per feature at a time: while one is live the card's action buttons lock and the card reads "running: <action>" with a link to the pane; truth is bee's dispatch ledger plus session activity. | Never a queue. |
| D4 (6b7f34aa) | Board-spawned agents always use the project's default preset (`herding.agent_command`); the board offers no agent picker. | — |
| Inherits | `board-approve-actions` D3 (opt-in) and D5 (relay wording). | |

### Agent's Discretion

- The exact slash-command lines and the spawn brief text.
- Whether Run review / Run compound need a confirm dialog (cost-bearing) — default to the same confirm as gate approvals unless planning finds a reason not to.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| live run | A board-triggered dispatch whose ledger row has no outcome yet and whose pane/session still signals. |
| default preset | `herding.agent_command` resolved through `herding.agents` in `.bee/config.json`. |

## Existing Code Context

### Reusable Assets

- `bee herding run --task/--task-file --cwd --job-id --ceiling --idle-timeout --json` — spawn + file-mailbox result (`.bee/mailbox/<job>/result-N.json`).
- `bee herding occupancy --json` / `.bee/wave-ledger.jsonl` / `.bee/logs/dispatch.jsonl` — live-run truth.
- `.claude/skills/bee-herding/references/role-dispatch.md:312-460` — the dispatch sequence (`bee worktree new` → `herdr pane split` → `herdr agent start` → `bee herding record-worker`).
- `crates/waggledance/src/mcp.rs:227-276` — `waggledance_dispatch / await / runs` and the `/p/:id/_runs` view.

### Integration Points

- The action endpoint and card button slot from `board-approve-actions`.
- `crates/waggledance/src/views.rs:3604-3631` — Todo / Review / Compound column placement.

## Outstanding Questions

### Deferred To Planning

- [ ] Does `bee herding run` accept the project's string `herding.agent_command` ("claude-sonnet" registry name) without `--agent`? — `--dry-run` answers it.
- [ ] Which ledger field marks a run finished for the card lock release when the pane was left open as forensics.

## Deferred Ideas

- A preset picker per click (rejected in D4).
- Queueing a second run behind the first (rejected in D3).
