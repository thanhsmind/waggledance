---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: bee-agent-activity

Mode: `standard` — 2 risk flags: covered-contract-change, multi-domain
Why this is the least workflow that protects the work: seven product files
across core reader, server joins, views, client script and notifier; each
surface has literal tests that must move with it.

## Requirements (from CONTEXT.md)
- A1 read-only session-file source, derived signal, dash for absent fields.
- A2 joins by pane id, cwd boundary, feature.
- A3 five-state vocabulary, bee wins over herdr, need-you = blocked ∪ waiting_input, word beside colour, no_signal muted.
- A4 Approve only on blocked via a data attribute.
- A5 second notifier cursor, transition-based, deduped.
- A6 card agent line; drawer/Pinned add state + feature.

## Discovery
Gather over bee.rs/server.rs/views.rs/app.js/notify/watcher: `parse_session`
(`bee.rs:2316`) already opens the file; `TerminalPaneView.status` is the one
entry point of herdr status into cards (`views.rs:1565`); the need-you
predicate is one line (`views.rs:3791-3795`) feeding both the waiting chip
and the phone tile; `AgentPaneRow` (`server.rs:3479`) feeds drawer + Pinned;
`pane_controls` (`views.rs:1895`) emits no status today; the notifier is
herdr-only (`watcher.rs`, `notify/mod.rs`). Pane id and cwd formats match.

## Approach
Plug in at the seams named above; no new directories, no new background
loop. Rejected: shelling out to `bee status --json` (bee.rs is pure file I/O
by test); a separate status store (forks the truth); rewriting the herdr
cursor to a second vocabulary (a second cursor is smaller).

Risk map: bee.rs parsing / LOW / unit tests on fixtures · server joins /
MEDIUM / route tests with a sessions fixture · card/tile counters / MEDIUM /
existing literal tests rewritten · Approve gating / LOW / selector-parity +
markup test · notifier cursor / MEDIUM / transition unit tests.

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 Core reader | `BeeSession.activity` + `signal`; parse tests | everything joins on it | fixture session parses with state/feature/cell/signal | 2,3,4 |
| 2 Board, rail, drawer | pane↔session and feature↔session maps in server; card agent line + dot precedence; need-you predicate widened; Pinned rows + `/api/agents` + drawer row gain state/feature | the visible payoff | card shows "agent: blocked · cell · quiet 12s"; tile counts it | 3 |
| 3 Approve gating | `data-agent-state` on pane card; app.js disables Approve unless blocked | safety | Approve greyed on a waiting_input pane | 4 |
| 4 Notifier | bee activity cursor on the 2 s tick → outbox | closes the loop | Telegram message on entry to blocked | 5 |
| 5 Spec sync | bee-cockpit.md + agent-terminal.md | state layer | specs describe the five states | close |

## Test matrix
- Happy: session with activity blocked + pane w1:p1 → card line, dot blocked,
  tile need-you 1, Pinned row word "needs approval", drawer row field,
  Approve enabled; waiting_input → Approve disabled, still counted.
- Edge: activity without feature/cell → "—"; activity.at 2 min old →
  no_signal marker, not counted; session dead → ignored; pane id with no
  session → herdr status as today.
- Error: malformed activity object → session parses without activity;
  read-never-writes guards stay green.
- Notifier: waiting_input→blocked fires once; blocked→idle fires nothing;
  →exited fires once; owned-by-run suppressed.

## Out of scope
- Writing anything into `.bee/`.
- A WebSocket/SSE push path (polling stays).
- Rendering worker nicknames.
