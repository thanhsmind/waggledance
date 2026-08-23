---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Board Run Actions

Mode: `standard` — 3 risk flags: multi-domain (daemon + herdr + bee CLI + browser), data-model (the `runs` table gains a `feature` column), covered-contract-change (the board action route and card tests).
Why this is the least workflow that protects the work: every primitive exists — `POST /p/:id/_bee/actions` (board-approve-actions), the MCP dispatch path that spawns a preset agent and records a `Run` (`mcp.rs::run_dispatch`, `orchestrate.rs`), the pane-input path, and bee's `worktree new` / `herding record-worker` CLIs. The new work is three kinds on the existing route, one column, three buttons, and a lock.

## Requirements (from CONTEXT.md)

- D1 (3971c768): Run review / Run compound send one slash-command line into the feature's live bound pane when one exists; with no live pane the daemon spawns a fresh agent in the feature's worktree.
- D2 (d864ae40): Start on a Todo card is one full herding dispatch — new feature worktree, new pane, the project's default preset with a brief to take the feature through bee-hive; gates still stop per `gate_bypass`.
- D3 (4c366c35): one board-triggered run per feature at a time; while one is live the card's buttons lock and the card reads "running: <action>" with a link to the pane; truth is the dispatch ledger plus session activity.
- D4 (6b7f34aa): board-spawned agents use the project's default preset (`herding.agent_command` resolved through `herding.agents` in `.bee/config.json`); no picker.
- Inherited: board-approve-actions D3 (per-project opt-in) and D5 (relay wording).

## Discovery

- `crates/waggledance/src/mcp.rs` `run_dispatch` (rg -n 'async fn run_dispatch') already does spawn-or-target: `orchestrate::resolve_spawn_destination` → `herdr.agent_start(&workspace_id, Some(&cwd), &preset.argv)` → `capture_baseline` → `mint_marker` → `send_task` → `Run { status: "working" }` via `engine.insert_run`. The board path reuses it with argv from bee's config instead of a waggledance preset label, and a chosen `cwd`.
- `Run` (`waggledance-core/src/domain.rs:68`) carries `project_id, pane_id, preset_label, task, baseline, marker, status` — no feature. `repository.rs::migrate` uses `PRAGMA user_version` steps, so one more step adds `feature TEXT NULL`.
- `orchestrate::await_run` / `RunStatus` (`orchestrate.rs:282-338`) derive working/done/blocked from the pane transcript; the card lock reads `status == "working"` and checks the pane still exists in the herdr snapshot so a dead pane never locks a card forever.
- The approve feature's `bee_action` (`server.rs` rg -n 'async fn bee_action\(') parses `BeeActionKind`; `pane_is_bound_to_feature` picks the feature's own pane; `bee_hub_action_kind` / `bee_hub_action_pair` (`views.rs`) render the pair; `app.js` `CONFIRM` + `fire()` wire it. Todo/Review/Compound rows render through `bee_hub_finished_row_with_extra` with `BeeHubFinishedData {feature, docs}` (`views.rs` rg -n 'BeeHubPlacement::Todo').
- `.bee/config.json` here: `herding.agent_command = "claude-sonnet"` (a string naming `herding.agents["claude-sonnet"]`); bee's own resolution also accepts an array form. `bee herding run --dry-run` confirmed its brief tells the agent to ignore bee workflow — wrong for D2, so Start uses the waggledance spawn path with a bee-hive brief, not `herding run`.
- `bee worktree new --feature <f>` must run with cwd = main checkout; the daemon already spawns bee with `current_dir(root)`.

## Approach

Recommended: extend `POST /p/:id/_bee/actions` with kinds `run-review`, `run-compound`, `start-todo` (D1/D2). Resolution per kind: (a) lock — if a live board run exists for the feature, 409 (D3); (b) `run-review`/`run-compound`: if a pane is bound to the feature with a live bee session, dispatch to that pane (Run recorded with `feature`), sending `/bee-reviewing review feature <f>` or `/bee-capturing flush the capture queue and compound feature <f>`; else spawn in the feature's granted worktree (or the project root when none) with argv from `.bee/config.json` `herding` (D4); (c) `start-todo`: run `bee worktree new --feature <f>` unless a grant exists, spawn in it with the brief "Run `bee orient`, then take feature <f> through bee-hive to done", then `bee herding record-worker --name board-<f> --pane-id … --path … --task <f>` so bee's occupancy stays honest. Response `{ok, run_id, pane_id, mode: "pane"|"spawned"}`.

Card: Todo rows get **Start**, Review rows **Run review**, Compound rows **Run compound** (opt-in only); when a live run exists the row reads `running: <action>` linking to `/p/<id>/_terminal/<pane>` and no button. `app.js` reuses the confirm dialog (agent's discretion: these cost an agent) and `fire()`.

Rejected: `bee herding run` for spawning (its brief forbids bee workflow — contradicts D2); a separate `/runs` POST route (the board should keep one action door); encoding the feature in the task text instead of a column (invisible to the Runs view and brittle).

Risk map: runaway spawn / MEDIUM / lock test + 409; wrong cwd for spawn / LOW / boundary check already in `resolve_spawn_destination`; bee config parse / LOW / unit test on both string and array `agent_command`.

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 Daemon | `Run.feature` column + migration; three kinds on the action route with lock, pane-or-spawn, worktree-new, record-worker; herding argv reader; tests | The run path is the risk | `curl run-review` on a feature with a live pane sends the slash line; on one without, a pane appears in the worktree; second call → 409 | 2 |
| 2 Card | Buttons on the three rows, running state, `app.js` wiring | Reachable by a human | Click Start on a Todo card → confirm → pane opens, card reads running | 3 |
| 3 Wording | README relay sentence names the run actions; knowledge delivery concept | Invariant text stays true | README | done |

Current slice: all three phases.

## Test matrix

- Happy: `run-review` with a bound live pane → Run recorded with feature, task is the slash line, mode `pane`; `start-todo` without a grant → stub bee sees `worktree new --feature <f>` then `herding record-worker`, fake herdr sees `agent_start` with the config argv and the worktree cwd, mode `spawned`.
- Edge: second action while a Run is `working` and its pane exists → 409 naming the pane; a `working` Run whose pane is gone → not a lock; no `herding` config → 409 naming `.bee/config.json`; not opted in → 403.
- Error: stub bee fails `worktree new` → 502 with stderr tail and no spawn; herdr down → 502 and no Run inserted.
- Card: opted-in Todo row carries `data-action-kind="start"`; Review row `review`; Compound row `compound`; locked row shows `running: review` with the pane link and no buttons.

## Out of scope

- Judging or merging the spawned run's result (the session does its own bee chain).
- A preset picker (D4) or queueing (D3).
