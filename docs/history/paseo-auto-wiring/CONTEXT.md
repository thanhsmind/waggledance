# Paseo Auto Wiring — Context

**Feature slug:** paseo-auto-wiring
**Date:** 2026-10-14
**Shaping session:** complete (four questions answered by the owner)
**Scope:** Standard
**Domain types:** CALL | RUN | ORGANIZE

## Feature Boundary

Waggledance wires itself to Paseo without hand-written per-workspace config:
it finds a working `paseo` CLI on its own, registers every Paseo workspace's
root as a waggledance project, picks a leader workspace automatically when a
project has several, and reads per-project settings (dispatch opt-in,
workspace override, leader model) from a `.waggledance.toml` committed in each
repo. It also fixes `waggledance setup paseo`, which fails on Paseo's stdout
log lines. It ends there: no change to how a request is queued or delivered
(`leader-delivery`), and bee's own worktree registration is bee's work
(handed to beehive as `sup-20261014-6234`).

Original ask (verbatim): «Tôi thấy rất sai nếu mỗi workspace lại phải thêm
manual giống hiện tại. Như paseo hiện tại có rất nhiều workspace nên chủ động
thêm vào chứ. Config nếu cần thì ở ngay trong project dễ edit»

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Every Paseo workspace's root directory is registered as a waggledance project automatically; nothing is written by hand per workspace. | Paseo already knows many workspaces; anphabe-goglbe and happyfarm are missing today. |
| D2 | Per-project settings live in `.waggledance.toml` at the repo root. An auto-registered project receives dispatched work only when that file turns orchestration on (default off); the settings-page opt-in keeps working. | In-repo, easy to edit, committed with the code; consent stays explicit. |
| D3 | Several local workspaces at one root: the leader's existing workspace wins, else the oldest local workspace at the root; `.waggledance.toml` may name one. The two-workspace refusal (`paseo-leader-dispatch` D8) is removed. | Owner chose automatic choice over refusal. |
| D4 | The global `[paseo]` block is optional: Paseo support turns on when a working CLI is found; the binary is detected (npm `@getpaseo/cli`, then `$PASEO_CLI`, then the AppImage wrapper), each candidate accepted only after a JSON `ls` parses; the global block only overrides. | The configured AppImage wrapper's `run` printed nothing from systemd while the npm CLI worked (2026-10-14, run-a07b58ecc3051860 lost). |

### Agent's Discretion

How often discovery runs, how "oldest" is read, the exact `.waggledance.toml`
keys beyond `orchestration`, and whether `[[paseo.leaders]]` stays as a
deprecated fallback.

## Findings (2026-10-14)

- `@getpaseo/cli` 0.10.3 at `~/.local/share/mise/installs/node/24.19.0/bin/paseo` works from a systemd user scope: blocking `run --json` printed `Using workspace wks_…` then a JSON document with `agentId` and `status`; `inspect`, `wait`, `workspace ls` returned JSON.
- `paseo workspace ls --json` carries `workspaceId, project, name, isolation, cwd` — no creation time. Paseo's store `~/.paseo/projects/workspaces.json` carries `createdAt`, `pinnedAt`, `archivedAt`, `kind`.
- `waggledance setup paseo` failed: `reading \`paseo plugin ls --json\`: invalid type: integer \`23\`` — it parses the wrapper's `23:43:… ›` log line as JSON (`crates/waggledance/src/setup.rs` `paseo_cli` / `paseo_listed`).

## Existing Code Context

- `crates/waggledance/src/paseo.rs` — `resolve_binary`, `resolve_leader`, parsing.
- `crates/waggledance-core/src/config.rs` — `PaseoConfig` (`enabled`, `binary`, `leaders`).
- `crates/waggledance-core/src/engine.rs` — `register`, `orchestration_allowed`.
- `crates/waggledance/src/delivery.rs` — daemon tick; `main.rs` gates delivery on `config.paseo.enabled`; `mcp.rs` gates leader dispatch on it.
- `crates/waggledance/src/setup.rs` — `paseo_cli`, `paseo_listed`.
