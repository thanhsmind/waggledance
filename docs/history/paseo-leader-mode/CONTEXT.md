# Paseo Leader Mode — Context

**Feature slug:** paseo-leader-mode
**Date:** 2026-10-23
**Shaping session:** complete (spec-drop sup-20261023-2e06 from waggledance@7a0a46a)
**Scope:** Standard
**Domain types:** CALL | RUN

## Feature Boundary

A leader that the waggledance daemon opens through Paseo runs in the permission mode
waggledance asked for, and waggledance proves it from the agent's runtime mode, not
from Paseo's configured mode. The feature ends at the leader's mode; what the leader
does in that mode is unchanged.

## Evidence

- The beehive leader `ff24c29b` (labels `waggledance.role=leader`,
  `waggledance.via=wd`) was created 2026-10-22 15:59 with no `--mode`. Its snapshot
  showed `currentModeId=bypassPermissions` and `runtimeInfo.modeId=default`, so it
  asked the user for every Bash call for about 9 hours.
- Read again on 2026-10-23 01:18Z through Paseo's MCP `get_agent_status`: the same
  agent still reports `currentModeId=bypassPermissions`, `runtimeInfo.modeId=default`.
- `paseo inspect <id> --json` prints `Mode` from `currentModeId`
  (`@getpaseo/cli/dist/commands/agent/inspect.js:92`), so the CLI cannot see the gap.
- Paseo writes each agent record to `~/.paseo/agents/<cwd-slug>/<id>.json`; its
  `runtimeInfo.modeId` matched the live MCP value for both `ff24c29b` (`default`) and
  `9fb45d05` (`bypassPermissions`) on 2026-10-23.
- `paseo agent mode <id> <mode>` calls the daemon's `setAgentMode`, the same call as the
  manual `set_agent_mode` that fixed the leader.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | A new leader is always started with an explicit `--mode`. | Spec ask 1. A run without `--mode` produced the runtime/config split. |
| D2 | The mode comes from `.waggledance.toml` `[paseo] mode`, else the global `[paseo] mode`, else `bypassPermissions`. Setting `mode = ""` in the global block leaves the mode to Paseo and skips the check in D3. | Spec ask 2. Narrows paseo-auto-wiring D8 ("mode passed only when set") for `mode` only; `thinking` stays as D8 says. |
| D3 | After the leader is created, waggledance reads the agent's runtime mode (`runtimeInfo.modeId` from Paseo's agent record). If it differs from the asked mode, waggledance runs `paseo agent mode <id> <mode>` once and reads again. If it still differs, waggledance reports it: a warning in the daemon log and a line in the delivery reply naming the agent, the asked mode and the runtime mode. The task is still delivered. | Spec ask 3. The CLI's `Mode` field is the configured value that lied, so it is not the check. |
| D4 | The same check runs before a task is sent to an existing leader that waggledance opened. | The 9-hour leader was an existing one; a check only at create would not have repaired it. |
| D5 | The delivery leader path is the only place waggledance creates a Paseo agent today (`run_args` has one caller, `delivery.rs`); the retired supervisor no longer exists. Spec ask 4 needs no other change. | Checked with `grep -rn run_args crates` on 2026-10-23. |

### Agent's Discretion

- How long to wait for the runtime mode to settle after create, and how often to read.
- Trait method names on `Paseo`.

## Existing Code Context

- `crates/waggledance/src/paseo.rs` — `run_args`, `resolve_leader`, `PaseoCli`, `store_path` (precedent: waggledance already reads Paseo's `workspaces.json` read-only).
- `crates/waggledance/src/delivery.rs` — the leader start/send flow.
- `crates/waggledance-core/src/config.rs` — `PaseoConfig::default`.
- `docs/knowledge/patterns/paseo-leader-dispatch.md` — the concept that documents `mode`.
