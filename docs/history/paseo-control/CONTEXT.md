# Paseo Control — Context

**Feature slug:** paseo-control
**Date:** 2026-08-29
**Shaping session:** complete
**Scope:** Standard
**Domain types:** READ · SEE · WRITE

## Feature Boundary

A paseo agent listed in waggledance opens its own page. On that page the user
reads the conversation the agent is having, sends it a message, and answers a
pending permission request. It ends there: no starting new agents, no stopping
or archiving running ones, no workspace or schedule management.

This supersedes `paseo-support`'s **D1** (display-only; interactive control out
of scope) and narrows its **D2** (never depend on the paseo daemon) — see D3
below. Every other paseo-support decision (D3 register, D4 liveness filter, D5
cwd mapping) still stands unchanged.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The paseo agent surface is INTERACTIVE: read what the agent is doing, send it a message, and allow or deny a pending permission request. Stopping or archiving an agent is NOT in this slice. | The blocked-on-permission moment is the one display-only can never resolve, and it is the moment the user is away from the machine. |
| D2 | The agent page reads as a CONVERSATION — the user's messages and the agent's replies, each tool call collapsed to one short human line ("read 3 files", "ran tests"). Not a technical event log. | The surface is used from a phone through waggle.gogl.be; a full event log scrolls past readability there. |
| D3 | Control actions reach paseo through its own CLI (`paseo logs`, `paseo send`, `paseo permit allow`, `paseo permit deny`), which requires the paseo daemon to be up. Detection — WHICH agents exist — keeps reading the on-disk store. | Splitting the two keeps the agent list alive when the daemon is down, while control reports the daemon as unreachable instead of failing silently. Speaking the daemon protocol directly would reimplement a contract paseo owns. |
| D4 | The paseo control surface rides the SAME switch that already gates remote terminal input for herdr panes (`terminal_family_enabled`). No new config key. | Sending input to an agent from an unauthenticated `/` is precisely the capability that switch already governs; a second key would let the two disagree. |
| D5 | A daemon that is down, an unreachable CLI, or a failed send is reported ON the page as a named, visible state. Never a silent no-op, and never a success shape. | The user is remote; a message that looks sent but was not is the worst outcome this feature can produce. |

### Agent's Discretion

- The agent page's route, layout, and markup, and whether it reuses the
  existing reply-composer component or renders its own — constrained to
  matching the established terminal/composer idiom rather than inventing a
  new one.
- How the conversation refreshes (poll interval, or streaming via
  `paseo logs --follow`) — planning decides against the existing board
  polling precedent.
- How a tool call is collapsed into its one-line human phrasing, and which
  event types are shown at all.
- Whether the permission control appears inline in the conversation at the
  point of the request, or as a banner on the page.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Paseo agent page | The per-agent page this feature adds, reached by clicking a paseo agent row. |
| Conversation | The rendered view of `paseo logs` output per D2 — messages and replies, tool calls collapsed. |
| Pending permission | A permission request the agent is blocked on, listed by `paseo permit ls` and answerable with `paseo permit allow|deny`. |
| Daemon unreachable | The state where the paseo CLI cannot reach its daemon; detection still works, control does not (D3, D5). |

## Specific Ideas And References

Verified on this machine (2026-08-29), paseo CLI at `~/.local/bin/paseo`:

- `paseo logs <id> [--follow] [--tail n] [--filter tools|text|errors|permissions] [--json]`
  — the agent's activity timeline; `--json` gives structured events.
- `paseo send <id> --prompt <text> [--image <path>] [--no-wait] [--json]`
  — sends a message to an existing agent.
- `paseo permit ls` · `paseo permit allow <agent> [req_id]` · `paseo permit deny <agent> [req_id]`
  — the pending-permission surface.
- `paseo ls --json` — id, shortId, name, provider (`claude/claude-opus-5`),
  thinking, status, cwd, created.
- `paseo inspect <id>`, `paseo attach <id>`, `paseo stop <id>` — available,
  not used by this slice.
- Daemon identity: `~/.paseo/paseo.pid` carries `listen: "127.0.0.1:6767"`.
  The HTTP root answers 404 for guessed paths; the CLI is the contract.

## Existing Code Context

From the quick scout only.

### Reusable Assets

- `crates/waggledance-core/src/paseo.rs` — `PaseoAgent`, `list_live_agents`,
  `default_store_root` (shipped by paseo-support). The detection half of D3
  already exists.
- `crates/waggledance/src/server.rs` — `api_agents` / `agent_pane_rows` /
  `paseo_agent_row` already emit paseo rows with an empty `url`; this feature
  fills that url in.
- The herdr terminal page and its reply composer — the established idiom for
  "read what an agent is doing, then type to it".
- `crates/waggledance-core/src/process.rs` — existing process-spawn helpers,
  the likely home for CLI invocation.

### Established Patterns

- `terminal_family_enabled` gates every remote-input surface (D4).
- `assets/app.js` `agentRow` renders a non-anchor row when `url` is empty —
  the exact seam that becomes a link here.

### Integration Points

- The paseo row's `url` in the agents feed.
- A new page route for a paseo agent.
- A POST path for sending a message and for answering a permission.

## Canonical References

- `~/.local/bin/paseo` — the control contract (D3).
- `~/.paseo/agents/<slug>/<uuid>.json` — the detection store (paseo-support D2).
- `docs/history/paseo-support/CONTEXT.md` — the superseded D1 and the standing
  D3/D4/D5.

## Outstanding Questions

### Resolve Before Planning

- None. D1–D5 are locked.

### Deferred To Planning

- [ ] Whether the conversation refreshes by polling or by `--follow` streaming,
      and at what cadence.
- [ ] Where the CLI invocation lives (core vs binary crate) given the core
      crate's sync-only, no-web-framework constraint, and how a slow or hung
      CLI call is bounded.
- [ ] The exact mapping from `paseo logs --json` event types to the collapsed
      one-line phrasings of D2.

## Deferred Ideas

- Stopping or archiving an agent from waggledance (`paseo stop`, `paseo archive`).
- Starting a NEW paseo agent from waggledance (`paseo run`).
- Attaching images to a sent message (`paseo send --image`).
- Per-line expand-to-technical-detail in the conversation.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs D1–D5 are stable and were
locked with the user on 2026-08-29; the three decision-log events carry the
same text. Planning reads the locked decisions, routes the lane, works in a
feature worktree, and presents the shape gate.
