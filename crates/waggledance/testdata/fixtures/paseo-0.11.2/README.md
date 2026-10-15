# Paseo 0.11.2 CLI captures

Raw stdout (and one stderr) of the Paseo CLI, captured on 2026-10-14 against
Paseo 0.11.2 with throwaway agents titled `wd probe* (delete me)`, all deleted
afterwards. Each file pins one behaviour the waggledance Paseo client in
`crates/waggledance/src/paseo.rs` relies on. The tests in that file read every
file here with `include_str!` and parse it through the same functions
`PaseoCli` uses (`parse_turn_end`, `parse_inspect`, `parse_leaders`,
`parse_document`, `strip_noise`, `last_id`).

A new Paseo version that changes any of these outputs gets a new sibling
directory, `paseo-<version>/`, rather than an edit here.

## Environments

- **Inside a Paseo agent**: run from a shell of a live Paseo agent, so
  `PASEO_AGENT_ID`, `PASEO_AGENT_CWD` and `PASEO_CLI` are set.
  `$PASEO_CLI` is the bundled CLI inside the mounted AppImage.
- **Systemd user scope**: run through `systemd-run --user`, the environment
  the waggledance daemon runs in, with none of the `PASEO_AGENT_*` variables.
- **Systemd user scope, npm CLI**: the same scope running the npm package
  `@getpaseo/cli` 0.10.3 (`~/.local/share/mise/installs/node/24.19.0/bin/paseo`)
  against the Paseo 0.11.2 daemon, captured on 2026-10-14 with the throwaway
  agent `wd probe4 (delete me)`, deleted afterwards. The `npm-*` files come
  from here and `wrapper-plugin-ls.out` from the AppImage wrapper in the same
  session.

Most captures came from the probe script `probe.sh` (kept in the bee cache,
not committed), run inside a Paseo agent. It used `"$PASEO_CLI"` as `$CLI`,
workspace `wks_e75630b751320215`, and these prompt files:

- `long.txt`: ``Run the shell command `sleep 25` and then reply with exactly LONG-DONE.``
- `short.txt`: `Reply with exactly the word SHORT-DONE.`
- `kill.txt`: ``Run the shell command `sleep 20` and then reply with exactly KILL-SURVIVED.``
- `perm.txt`: `Create a file named wd-probe-permission.txt containing the word hi in the current directory.`

`<ID>` is the agent created by `run-blocking-cli.out`; `<ID3>` is a second
agent started in mode `default` (Always Ask) so that a write needs a
permission.

## Files

| File | Command | Environment | Pins |
|---|---|---|---|
| `run-blocking-cli.out` | `"$CLI" run --json --provider claude/claude-sonnet-5-5 --mode plan --workspace <ws> --cwd <root> --title "wd probe2 (delete me)" --label waggledance.probe=2 "Reply with exactly the word FIRST-RUN and nothing else."` | inside a Paseo agent | A blocking `run` (no `-d`) returns at the end of the first turn with `status: completed` and the new `agentId`, so a new leader can start with its task. Parses to `TurnEnd::Completed`. |
| `run-blocking.out` | `paseo run ... --json` through the `~/.local/bin/paseo` PATH wrapper | inside a Paseo agent | The wrapper prints only startup logs, including multi-line `[login-shell-env] ... {` objects, and creates no agent. `strip_noise` reduces it to nothing; parsing a turn end fails instead of reading noise as data. |
| `send-blocking.out` | `"$PASEO_CLI" send <id> --prompt-file p2.txt --json` | inside a Paseo agent (earlier probe) | A blocking `send` (no `--no-wait`) returns when the turn ends, `status: completed`. Parses to `TurnEnd::Completed`. |
| `wait.out` | `"$PASEO_CLI" wait <id> --json` | inside a Paseo agent (earlier probe) | `wait` reports `status: idle` with the last activity items in `message`. Parses to `TurnEnd::Other { status: "idle", .. }`. |
| `send-nowait.out` | `"$CLI" send <ID> --prompt-file long.txt --no-wait --json` | inside a Paseo agent | A no-wait send returns at once with `status: sent`. |
| `inspect-running.out` | `"$CLI" inspect <ID> --json`, 3 s after `send-nowait` | inside a Paseo agent | `inspect` reports `Status: running` and an empty `PendingPermissions`; `ParentAgentId` shows the agent became a child of the calling agent, which is why child processes drop `PASEO_AGENT_ID` and `PASEO_AGENT_CWD`. |
| `send-while-busy.out` | `"$CLI" send <ID> --prompt-file short.txt --json`, while the long turn ran | inside a Paseo agent | A blocking send into a running agent still returns `status: completed`. Parses to `TurnEnd::Completed`. |
| `logs-after-busy.out` | `"$CLI" logs <ID> --tail 20` | inside a Paseo agent | The send above interrupted the running turn: `LONG-DONE` never appears, `SHORT-DONE` does. Logs text has no noise to strip. |
| `send-killed.out` | `"$CLI" send <ID> --prompt-file kill.txt --json`, killed after 6 s | inside a Paseo agent | A killed blocking send prints nothing; parsing a turn end fails. |
| `logs-after-kill.out` | `"$CLI" logs <ID> --tail 6`, 25 s after the kill | inside a Paseo agent | Killing the sender does not end the turn: the shell command kept running to completion. |
| `send-perm.out` | `"$CLI" send <ID3> --prompt-file perm.txt --json` | inside a Paseo agent | A blocking send returns early with `status: permission` when the agent waits on one. Parses to `TurnEnd::Permission`. |
| `inspect-pending.out` | `"$CLI" inspect <ID3> --json`, 25 s into that send | inside a Paseo agent | `PendingPermissions` lists one entry with `tool: Write`; `parse_inspect` yields it. |
| `permit-ls.out` | `"$CLI" permit ls --json` | inside a Paseo agent | The global permission list names the same permission (`name: Write`). |
| `wrapper-systemd-ls.out` | `systemd-run --user --wait --pipe --quiet -p WorkingDirectory=/tmp ~/.local/bin/paseo ls -g --json` (stdout) | systemd user scope | The PATH wrapper works from the daemon's environment: two single-line startup logs, then the JSON list. `parse_leaders` reads five agent ids. |
| `npm-run-blocking.out` | `systemd-run --user --wait --pipe ... ~/.local/share/mise/installs/node/24.19.0/bin/paseo run --json --title "wd probe4 (delete me)" ...` | systemd user scope, npm CLI | The npm CLI's blocking `run` prints a `Using workspace wks_…` line before its JSON document; `parse_turn_end` skips the line and reads the new `agentId` with `status: permission`. |
| `npm-workspace-ls.out` | `systemd-run --user --wait --pipe ... node/24.19.0/bin/paseo workspace ls --json` | systemd user scope, npm CLI | `workspace ls` is clean JSON carrying `workspaceId`, `project`, `name`, `isolation` and `cwd`, no creation time. |
| `npm-plugin-ls.out` | `systemd-run --user --wait --pipe ... node/24.19.0/bin/paseo plugin ls --json` | systemd user scope, npm CLI | `plugin ls` from the npm CLI is a clean JSON list naming the `waggledance` plugin. |
| `wrapper-plugin-ls.out` | `systemd-run --user --wait --pipe ... ~/.local/bin/paseo plugin ls --json` | systemd user scope | The AppImage wrapper's `plugin ls` starts with two `HH:MM:SS.mmm ›` log lines; `waggledance setup paseo` reads it through `parse_document` instead of failing on the clock digits. |
| `wrapper-systemd-ls.err` | the same command (stderr) | systemd user scope | The wrapper's stderr is one `[linux-sandbox] ...` line that starts with `[` yet is not JSON; it would fool the document rule, which is why the client discards stderr. Parsing it as a list fails. |
| `workspaces-store.json` | none: written by hand after the keys of `~/.paseo/projects/workspaces.json` (read with `python3` on 2026-10-14) | synthetic ids and paths | Paseo's own store, read-only: `createdAt` orders the leader pick (`wks_fixture_b` is the oldest of three local workspaces at one root); `archivedAt` and `isPaseoOwnedWorktree` keep a workspace out of discovery. |
