---
type: bee.delivery
title: board-run-actions — delivery
description: "Delivery record for work item board-run-actions: the board's run path — Start, Run review and Run compound on a feature card start an agent, in the feature's live pane or in a fresh pane in its worktree, one live run per feature."
timestamp: 2026-08-23
bee:
  id: board-run-actions-delivery
  lifecycle: active
  areas: [bee-cockpit, agent-terminal, orchestration]
  required_context: [docs/history/board-run-actions/CONTEXT.md, docs/history/board-run-actions/plan.md]
  sources: [docs/history/board-run-actions/CONTEXT.md, docs/history/board-run-actions/plan.md, docs/history/research/board-actions-from-orchestrators.md, docs/knowledge/work/board-approve-actions/delivery.md]
---

# board-run-actions — Delivery

## What shipped

`board-approve-actions` gave the card an answer; this feature gives it a start. A
feature card now carries buttons that begin agent work — **Start** on a Todo row,
**Run review** on a Review row, **Run compound** on a Compound row — and the daemon
either sends one line into the feature's own live pane or spawns a fresh agent in the
feature's worktree with the project's default bee preset. The feature ends at the run
being started and mirrored on the card; it never judges or merges what the run produces.

The relay rule is unchanged and now covers these buttons too: the board writes only
through the project's own `bee` CLI or as one line into a herdr pane, only on an explicit
human click in an opted-in project (`board-approve-actions` D3), and it originates no
decision of its own. README's relay paragraph names the three run actions alongside the
three approvals, per that feature's D5.

## Locked decisions

| ID | Decision |
|----|----------|
| D1 (3971c768, spawn clause superseded) | Run review / Run compound send one slash-command line into the feature's live bound pane when one exists; with no live pane the daemon spawns a fresh agent in the feature's worktree. As locked, D1 named `bee herding run` as the spawner; planning rejected it because its brief forbids the bee workflow D2 needs, and the spawn is waggledance's own `agent_start` through `orchestrate::dispatch_run` — the supersession is logged in the decision store (2026-08-23). Reuse the session that already holds the context; spawn only when nothing is live. |
| D2 (d864ae40) | Start on a Todo card is one full dispatch: new feature worktree, new pane, the project's default agent preset started with a brief to take the PBI through bee-hive; gates still stop per `gate_bypass` as in any session. The card moves to In Progress on the session's own activity signal, not on the click. |
| D3 (4c366c35) | One board-triggered run per feature at a time: while one is live the card's action buttons lock and the card reads "running: <action>" with a link to the pane; truth is bee's dispatch ledger plus session activity. Never a queue. |
| D4 (6b7f34aa) | Board-spawned agents always use the project's default preset (`herding.agent_command` resolved through `herding.agents` in `.bee/config.json`); the board offers no agent picker. |
| Inherits | `board-approve-actions` D3 (per-project opt-in) and D5 (relay wording). |

## Contract

**Route** — the same one door as the approvals, `POST /p/:id/_bee/actions` with body
`{kind, feature}`, gaining three kinds:

```
kind ∈ run-review | run-compound | start-todo
```

- `403` when the project is not opted in — the inherited `orchestration.enabled` switch
  (`board-approve-actions` D3), carrying the settings-page remedy.
- **Lock first (D3).** A live board run for this feature answers `409` naming the pane;
  the board never queues a second run behind the first.
- **`run-review` / `run-compound` (D1).** With a pane bound to the feature holding a live
  bee session, the daemon posts one line into it and answers `mode: "pane"`. With none,
  it spawns in the feature's worktree (the project root when the feature has no grant)
  and answers `mode: "spawned"`.
- **`start-todo` (D2).** `bee worktree new --feature <f>` unless a grant already exists,
  then a spawn in that worktree, then `bee herding record-worker` so bee's own occupancy
  view stays honest about the pane the board just opened.
- **Preset (D4).** Spawn argv comes from `.bee/config.json` `herding` —
  `agent_command` resolved through `herding.agents`, in both its string and array forms.
  A project with no `herding` config answers `409` naming the file.
- Response `{ok, run_id, pane_id, mode}`; `start-todo` adds `recorded` (whether
  `bee herding record-worker` succeeded — a failure there never undoes the spawn).
  With the terminal-family switch off every run kind answers the pane routes' `404`,
  while the gate halves of the approve kinds still write. The board re-renders on the
  existing `/ws` `{"changed":[…]}` event, never on a second refresh channel.
- Review / compound with no live pane spawn in the feature's granted worktree, or in
  the project root when the feature has no worktree; Start always spawns (D2).

**Slash lines sent** — the exact text the daemon writes into the pane or hands the
spawned agent:

| Kind | Line |
|------|------|
| `run-review` | `/bee-reviewing review feature <f>` |
| `run-compound` | `/bee-capturing flush the capture queue and compound feature <f>` |
| `start-todo` | `Run .bee/bin/bee orient, then take feature <f> through bee-hive to done — gates stop per this repo's gate_bypass.` |

The `start-todo` brief is a bee-hive brief on purpose: `bee herding run`'s own brief
tells the agent to ignore the bee workflow, which contradicts D2, so Start uses the
daemon's spawn path rather than `herding run`.

**`runs.feature` column** — the `runs` table gains `feature TEXT NULL`, added by one more
`PRAGMA user_version` migration step. Board-started runs record the feature they belong
to, which is what makes the lock and the card's "running" state readable — and what keeps
the Runs view honest. The feature is a column, never a substring of the task text.

**The lock rule (D3)** — one *working* Run per feature *whose pane still exists*. A run
whose pane is gone never locks the card, so a pane left open as forensics is the only
thing that can hold a lock open, and a crashed pane releases it. Truth is the dispatch
ledger plus the herdr snapshot, checked together on every action.

**Card** — Todo rows carry `data-action-kind="start"`, Review rows `review`, Compound
rows `compound`, rendered only for an opted-in project. A locked row drops its buttons
and reads `running: <action>` with a link to `/p/<id>/_terminal/<pane>`. `app.js` reuses
the approve feature's confirm dialog and one-shot in-flight guard: these clicks cost an
agent, so they confirm like a gate.

## Boundary

Judging or merging what the spawned run produces is the session's own bee chain, not the
board's. A preset picker per click and queueing a second run behind the first are both
refused by decision (D4, D3).

Backlog row **"Board run actions: Run review / Run compound / Start todo buttons on the
board card"** (`.bee/backlog.jsonl`) is delivered by this feature.

## Provenance

Written from the locked decisions in
`docs/history/board-run-actions/CONTEXT.md` and the approved shape in
`docs/history/board-run-actions/plan.md`, and from `board-approve-actions` D5, which puts
the relay rule in both README and the knowledge layer.
