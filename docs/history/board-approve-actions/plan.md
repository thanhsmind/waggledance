---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Board Approve Actions

Mode: `standard` — 3 risk flags: multi-domain (daemon + bee reader + browser JS), covered-contract-change (board placement/current-stop tests), public-contracts (a new HTTP route).
Why this is the least workflow that protects the work: the change adds the board's first card-level write path, so it needs a plan and a walking skeleton, but every primitive it touches (bee CLI spawn, pane input, herdr snapshot, card render) already exists and is tested — no spike, no hard gate.

## Requirements (from CONTEXT.md)

- D1 (608e7903): a feature card shows one Approve + Reject pair for its current stop — UAT gate, merged shape+execution gate, or the agent's permission prompt (`activity.state == blocked`). Reject unapproves the gate / sends the refusal into the pane.
- D2 (9b6c3c25): gate approvals open a short confirm dialog naming the feature; the permission-prompt pair is one click.
- D3 (3da23985): buttons appear only on projects with `orchestration.enabled`; otherwise only the waiting-on badge.
- D4 (7b728f87): a gate action runs the project's own `bee gate …`, then sends one line into the pane bound to the feature (`activity.pane`) when one is live; with no live pane only the gate is written.
- D5 (442611f6): README's "the board never writes" sentence is replaced by the relay wording.

## Discovery

- `POST /api/projects/:id/pbi` (`crates/waggledance/src/server.rs:1440-1482`) already spawns `project_bee_binary(root)` with `current_dir(root)`, null stdin, and maps the four refusals (400/404/409/502 with bee's stderr tail) — the gate action reuses this shape verbatim.
- `POST /p/:id/_terminal/:pane_id/input` (`server.rs:2930-2950`) is the pane write; the terminal Approve button posts `{text:"Approve", submit:true}` and is enabled only on `BeeActivityState::Blocked` (`views.rs:2319-2324`, `assets/app.js:2243-2252`).
- The card renderer `bee_hub_card` (`views.rs:5456`) already receives `project_id`, `feature`, `panes` (this feature's bound panes with `bee_state`), and `agent` (the session entry) — every fact a button needs is on the card args; `BeeHubMergeData.uat_approved` (`views.rs:3894`) is the uat stop on Ready-to-merge rows; `bee_gate_current_stop` (`views.rs:5005`) gives shape/execution on In Progress rows and deliberately excludes uat.
- `Project::orchestration_enabled` is already on the project row the board pages load (`server.rs:1264-1280`, `views.rs:8099-8112`).
- `bee gate --name uat | --merge --approved true|false --lane <feature>` (bee 2.21.0 `--help`): `--actor auto` is refused for uat; the daemon passes `--lane <feature>` when `.bee/lanes/<feature>.json` exists, `--no-lane` otherwise; merged approvals may refuse on the advisor / plan-conflict preconditions — those refusals are bee's stderr and must reach the card.
- bee control-plane verbs refuse inside a granted worktree; the daemon always runs them with `current_dir` = the project root (main checkout), which is the same cwd the PBI route uses.

## Approach

Recommended (D1–D5): one new route `POST /p/:id/_bee/actions` with body `{kind, feature}` where `kind ∈ uat-approve | uat-reject | gate-approve | gate-reject | permission-approve | permission-reject`. The handler: (1) refuses unless `orchestration_enabled` (D3) — 403 with the settings-page remedy; (2) gate kinds spawn `bee gate` per D4 and, on success, look up the feature's live bound pane and `send_input` one line (`"<uat|gate> <approved|unapproved> from the board"`), reporting `woke: true|false` — a pane failure never undoes the gate; (3) permission kinds resolve the feature's bound pane whose `bee_state == Blocked` and send `Approve` (text+submit) or the refusal (`/keys` `escape`, falling back to text `No` if herdr reports the key unsupported — the writer verifies against a live pane); (4) every response is `{ok, woke?, error?}` and the board relies on the existing `/ws` change event for the re-render.

Card: `bee_hub_card` renders `<div class="bee-hub__actions" data-action-feature data-action-kind data-action-pane>` with the pair when the project is opted in and exactly one stop is current: uat on a Ready-to-merge row with `uat_approved == false`; shape/execution when `gate_stop` names them on an In Progress row; permission when `agent.bee_state == Blocked` (that one wins over a gate stop — it is the live human call). `app.js` wires the pair: gate kinds `confirm()`-free custom dialog (D2) → POST → buttons disabled + "…" until the page reloads; permission kinds post at once. An idempotency guard: a button fires once per page load.

Rejected: a generic `/api/bee` proxy that runs arbitrary bee verbs (the board would stop being a relay and the attack surface is unbounded); approving by writing `.bee/lanes/*.json` directly (violates CLI-only state); polling the action result instead of the `/ws` event (a second refresh channel).

Risk map: action route auth posture / MEDIUM / test that the route is behind the Host allowlist and opt-in; bee refusal surfacing / LOW / stderr tail test like the PBI route; wake line lands in a shell pane / LOW / only agent panes are targeted, never `kind == shell`; double fire across reload / LOW / one-shot guard test in JS is out of reach — covered by the disabled state plus bee's idempotent gate write.

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 Skeleton | `POST /p/:id/_bee/actions` (all six kinds), opt-in refusal, bee spawn, pane wake, tests | The write path is the whole risk; prove it end-to-end first | `curl` approves uat on a fixture project; lane file flips; a bound pane receives the line | Phase 2 |
| 2 Card | Pair on the card for the three stops, CSS, `app.js` wiring with confirm dialog + in-flight state | Makes the route reachable by a human | Click Approve on a Ready-to-merge card → confirm → card re-renders "uat approved" | Phase 3 |
| 3 Wording | README D5 sentence, knowledge concept, backlog row 13 closed | The invariant must be replaced the moment the write path ships | `README.md` reads the relay rule | board-run-actions |

Current slice: all three phases (one feature, ~6 product files).

## Test matrix

- Happy: uat-approve on an opted-in project spawns `bee gate --name uat --approved true --lane <f>` against a temp project whose `.bee/bin/bee` is a stub script recording argv; response `{ok:true, woke:true}` when a fake herdr holds a bound agent pane; the card for a pending-uat Ready-to-merge row carries the pair with `data-action-kind="uat"`.
- Edge: no live pane → `{ok:true, woke:false}`; project not opted in → 403 and the card carries no `.bee-hub__actions`; permission kind when no pane is `Blocked` → 409; a Blocked agent outranks a gate stop on the same card.
- Error: stub bee exits non-zero → 502 carrying its stderr tail; no bee binary → 409; unknown kind → 400; the Host allowlist still answers 421 for a foreign Host on this route.

## Out of scope

- Run review / Run compound / Start todo buttons (`board-run-actions`).
- Free-text replies or more canned replies on the card.
- Any authentication beyond the Host allowlist and the per-project opt-in.
