# Paseo Dispatch Default On — Context

**Feature slug:** paseo-dispatch-default-on
**Date:** 2026-10-23
**Shaping session:** complete
**Scope:** Standard
**Domain types:** CALL | SEE

## Feature Boundary

Every project waggledance manages accepts dispatched work (MCP dispatch, Paseo
leader dispatch, board actions) without a `.waggledance.toml`; a project turns
dispatch off only with `orchestration = false` in its own `.waggledance.toml`.
The global `terminal.enabled` switch and every other gate stay as they are.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Supersedes paseo-auto-wiring D2 (decision 9fe4dfca). A registered project — auto-registered from Paseo or registered by hand — accepts dispatched work by default with no file. (decision fb1b49ab) | Owner 2026-10-23: a dispatch to a-blog was refused only because the file was missing. |
| D2 | The only per-project off switch is `orchestration = false` in the project's own `.waggledance.toml`. Absent file, absent key, or `true` means on. | The owner's in-repo switch stays; nothing is written into any repo. |
| D3 | The per-project consent is one computed value used everywhere it is read today: MCP dispatch, leader dispatch, the observer trigger, the supervisor-mailbox scan, board Approve/Reject and run buttons, and the Settings page. | The board and Settings today read the DB flag directly and would disagree with dispatch. |
| D4 | The Settings page per-project checkbox and its POST route go away; the Orchestration section lists each project's effective state and says how to turn one off. The DB column stays and is no longer read. | A checkbox that no longer decides anything would mislead. |
| D5 | The refusal for an opted-out project names the project, the file path and the `orchestration = false` line to remove. | — |

### Agent's Discretion

- How the computed consent reaches the board views (fill `Project.orchestration_enabled` on read, or pass the predicate).
- What a `.waggledance.toml` that fails to parse means for consent; default: a parse failure does not turn dispatch off, and the warning is surfaced where it is today.

## Existing Code Context

From gather job-1792736906600-4133640-1.

- `crates/waggledance-core/src/engine.rs:391` `orchestration_allowed` — today `db flag || file == Some(true)`.
- `crates/waggledance-core/src/engine.rs:86` — `register` writes `orchestration_enabled: false`.
- Readers: `mcp.rs:1193`, `mcp.rs:1302`, `trigger.rs:388`, `trigger.rs:770` via the predicate; `server.rs:3091`, `views.rs:6011/6425/7172/7900/7954/11326` read the DB field directly.
- Settings: `views.rs:11285-11454`, route `server.rs:522`, handler `server.rs:2025`.
- Refusal text: `mcp.rs:1247` `not_opted_in`; board 403 `server.rs:3091`; setup copy `setup.rs:567`.

## Canonical References

- `docs/knowledge/patterns/paseo-leader-dispatch.md` — leader dispatch and the `.waggledance.toml` contract.
- `docs/specs/mcp-surface.md` R1 — the two switches.
- `docs/knowledge/patterns/an-opt-in-that-blocks-the-live-proof.md` — premise changes.
- `README.md` "Per-project settings: .waggledance.toml".
