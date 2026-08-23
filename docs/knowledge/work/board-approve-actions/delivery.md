---
type: bee.delivery
title: board-approve-actions — delivery
description: "Delivery record for work item board-approve-actions: the board's first card-level write path — one Approve + Reject pair per feature card that relays a human click into the project's own bee CLI or its bound herdr pane."
timestamp: 2026-08-23
bee:
  id: board-approve-actions-delivery
  lifecycle: active
  areas: [bee-cockpit, agent-terminal, web-interface]
  required_context: [docs/history/board-approve-actions/CONTEXT.md, docs/history/board-approve-actions/plan.md]
  sources: [docs/history/board-approve-actions/CONTEXT.md, docs/history/board-approve-actions/plan.md, docs/history/research/board-actions-from-orchestrators.md]
---

# board-approve-actions — Delivery

## What shipped

The console board stopped being read-only. A feature card carries one Approve +
Reject pair for the single thing that feature waits on a human for right now,
and pressing it writes the answer into bee — or into the agent's pane — and
wakes the waiting session.

The rule that replaces the old "the board never writes" invariant, in
`README.md` and here: **the board relays; it never decides.** It writes in
exactly two ways — through the project's own `bee` CLI, or as one line into a
herdr pane — only on an explicit human click, and only in a project that opted
in on the settings page. It originates no decision of its own; it carries the
human's and mirrors the result back onto the card. The old paragraph's claim
that the daemon compared each project's store byte-for-byte around every request
is gone with it: no code ever implemented that check.

## Locked decisions

| ID | Decision |
|----|----------|
| D1 (608e7903) | A feature card shows one Approve + Reject pair for its current stop: the UAT gate, the merged shape+execution gate, or the agent's permission prompt (`activity.state == blocked`). Reject unapproves the gate or sends the refusal into the pane — a wrong click stays reversible on the board. |
| D2 (9b6c3c25) | Gate approvals (UAT, shape+execution) open a short confirm dialog naming the feature; the permission-prompt pair is one click, matching the terminal's own Approve button. Gates are irreversible stops; a permission prompt is not. |
| D3 (3da23985) | Board write actions are enabled per project by the existing `orchestration.enabled` opt-in. A project without it shows only the waiting-on badge, never a button. No new config key. |
| D4 (7b728f87) | After a gate approval the daemon writes the gate through the project's own bee CLI, then sends one line into the pane bound to the feature (session `activity.pane`) so the waiting session wakes and its waiting-on mark clears. With no live pane, only the gate is written — a pane failure never undoes the gate. A board click is not a user message; the pane line stands in for it. |
| D5 (442611f6) | README's "the board never writes" invariant is replaced by the relay rule: the board writes only through the project's own bee CLI or a herdr pane, only on an explicit human click in an opted-in project, and never decides what to do. README and the knowledge layer carry the new wording. |

## Contract

**Route** — `POST /p/:id/_bee/actions` with body `{kind, feature}`, where

```
kind ∈ uat-approve | uat-reject | gate-approve | gate-reject
     | permission-approve | permission-reject
```

- `403` when the project is not opted in (D3), carrying the settings-page remedy.
- Gate kinds spawn the project's `.bee/bin/bee gate …` with `current_dir` at the
  project root (the `POST /api/projects/:id/pbi` shape), then send one wake line
  into the feature's bound pane; the response reports `{ok, woke, error?}` so the
  card can say whether a session was woken.
- Permission kinds resolve the feature's bound pane whose bee state is `blocked`
  and post into the existing pane-input route; `409` when no pane is blocked.
- Bee's own refusals (missing vendored bee, advisor or plan-conflict precondition,
  `WORKTREE_MERGE_UAT_PENDING`) reach the card as bee's stderr tail, not a generic
  error.
- The board re-renders on the existing `/ws` `{"changed":[…]}` event; the action
  response is never a second refresh channel.

**Card attributes** — the pair renders as `.bee-hub__actions` carrying
`data-action-kind` (the stop: `uat`, `gate`, `permission`), plus the feature and
pane it acts on. `app.js` keys its confirm dialog (D2) and its one-shot in-flight
guard off `data-action-kind`, so the markup contract is what the browser layer
reads — never the button label.

## Settled in execution

Rules that settled while the cells ran, each a capture stub merged here:

- **Pair placement.** The action pair renders as a *sibling* after the card's
  `</details>` (and after the Ready-to-merge row's `</a>`), never nested: a
  button inside `<summary>` toggles the disclosure and a button inside `<a>`
  is invalid markup. One shared rule, `bee_hub_action_kind` (opt-in →
  permission → uat → gate `shape`/`execution`; `context` earns no pair),
  decides the stop for card and row alike so the two surfaces cannot drift.
- **Terminal-family switch.** The pane half of the action route honors the
  terminal-family switch exactly as `/input` and `/keys` do: switch off → the
  gate is still written, the response reads `woke:false`, and no herdr call is
  made; permission kinds answer the `/input` route's 404.
- **Watcher broadcast.** The file watcher broadcasts exactly two non-markdown
  shapes — `.bee/state.json` and direct children of `.bee/lanes/*.json` —
  reported but never indexed; sessions, cells, logs and reservations stay
  silent. The browser steers those two shapes to board surfaces only, behind
  one 1.5 s trailing reload timer that re-arms instead of firing while a
  dialog is open, so a burst of lane writes is one reload and a confirm never
  vanishes under the user.
- **Pane join follows the session, not the directory.** A wake line or a
  permission keystroke reaches only a pane whose session `activity.feature`
  equals the requested feature (`TerminalPaneView.bee_feature`) — the same
  join that pins a main-checkout pane to one card (decision `3daa1ea7`,
  board-pane-lane-pin). A pane with no live bee session is never a target,
  so two lanes sharing the main checkout can never receive each other's
  Approve. `state.json` is broadcast only when its `{feature, phase,
  approved_gates}` projection changes, and the 1.5 s debounce applies only
  to those bee-signal bursts on board surfaces; a reload blocked by an open
  modal flushes once on the next click, key-up or close.
- **Permission reject (unverified).** Rejecting a permission prompt sends the
  herdr key `escape`; no live blocked Claude pane was available to verify that
  it dismisses the prompt. Text `No` + submit is the named fallback — a
  one-line swap in `bee_action_permission` if a live check disagrees.

## Boundary

Buttons that *start an agent run* — Run review, Run compound, Start todo — are
not part of this feature; they are `board-run-actions`.
<!-- bee:not-a-deferral: records an earlier feature's boundary (terminal-approve-button), not a promise this feature makes -->
Free-text and further canned replies on the card remain outside scope, as
`docs/history/terminal-approve-button/CONTEXT.md` already records.
<!-- /bee:not-a-deferral -->

Backlog row **"Agent board V2: on-board Approve/Reject buttons"**
(`.bee/backlog.jsonl`) is delivered by this feature.

## Provenance

Written from the locked decisions in
`docs/history/board-approve-actions/CONTEXT.md` and the approved shape in
`docs/history/board-approve-actions/plan.md`, per D5, which puts the relay rule
in both README and the knowledge layer.
