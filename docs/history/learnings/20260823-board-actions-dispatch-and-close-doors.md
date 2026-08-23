# Learnings — board-approve-actions (2026-08-23)

## Dispatch must register the worker and reserve its files before the Agent call

`bee dispatch prepare --kind cell` renders the prompt but does not add the
worker to `state.json workers[]` nor reserve the cell's files. Three of five
workers hit a `cells finish` refusal and either self-registered or capped with
`--inline-reason`. From the main checkout, before each dispatch:
`bee state worker add --nickname <w> --cell <id>` and
`bee reservations reserve --agent <w> --cell <id> --path <p>` per file.
Backlog row logged as friction (P2, layer bee).

## Control-plane verbs refuse inside a granted worktree

`state start-feature / route / gate / worker add / scribing-run` all refuse
from the feature worktree and must run from main; execution workers must be
dispatched from a session whose cwd is the worktree. The orchestrator therefore
hops: main (claim, register, reserve, prepare) → worktree (dispatch, commit
docs) → main (judge-record, capture, close). Budget one hop per wave.

## The write guard refuses `$VAR` paths and `cd` prefixes

Every Bash write must name a literal path; shell variables and a leading `cd`
are refused. Build commands in a long call as literal absolute paths.

## The close "impact" door keys on decisions, not on annotation prose

Annotating a citing doc with a "Reconciled …" note did not clear the door, and
logging a `touches:` decision widened it (every doc citing the new decision
joined the list). The door never refused `bee close` — it reports — but what it
actually accepts as reconciliation is still unknown; the learning is to check
`bee close --dry-run` right after the first cap, not at the end, so the
remedy can be found while the feature is still cheap to touch.

## A per-cell review pass pays for itself on a write path

The independent review of the four behavior cells found a real D4 deviation
(pane chosen by directory, not by the session's feature) that tests had not
pinned, plus a reload-churn hazard from broadcasting `state.json`. Both were
one fix-first cell. For any cell that adds a write path, dispatch the reviewer
before UAT, not after.
