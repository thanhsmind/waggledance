# Daemon lifecycle

A user starts the headless supervisor with `waggledance serve`, checks it with
`waggledance status`, and replaces or ends it with `restart` and `stop`. Only one daemon
runs for each HOME. Liveness is the pid in `daemon.lock`, because the daemon has no port.

## Sub-features

- `lifecycle-status` — `status` and `status --json` report running, the pid, the version
  and the project count.
- `lifecycle-single-instance` — a second `serve` prints `already running`, exits 0 and
  leaves the first daemon alone.
- `lifecycle-restart` — `restart` replaces the daemon with a new pid and the old one exits.
- `lifecycle-stop` — `stop` ends the daemon, and `status` then says `running: no`.
- `lifecycle-serve-again` — `serve` works again after a stop.

## How to get to it (user POV)

- Run `waggledance serve` in a terminal, or let `waggledance restart` start it detached.
- Run `waggledance status` (`--json` for agents), `waggledance restart` or `waggledance stop`.
- `waggledance doctor` also reports liveness. It is not driven here, because `doctor --fix`
  edits the user's Claude Code config.

## Driving it with control-waggledance

Preconditions:

- `launch` has started the scratch daemon and `doctor --run <id>` passes.

- **Status.** Run `status --json`. Stdout contains `"running":true`.
- **Duplicate.** Run `serve` again. It exits 0, stdout holds `A waggledance daemon is
  already running.`, and the `daemon.lock` pid is unchanged and still alive.
- **Restart.** Run `restart`. It exits 0, `daemon.lock` names a new pid that carries this
  HOME, and the old pid is gone from `/proc`.
- **Stop.** Run `stop`, then `status`. Stdout says `running: no`, and the pid is gone.
- **Serve again.** Start `serve` detached. `status --json` shows `"running":true`.
- **Drive it.** `bash .bee/verify/verify-app/control-waggledance drive --fast daemon-lifecycle`.

## Gotchas

- There is no TCP port to probe. A health check by port reads a healthy daemon as dead.
- `restart` starts the new daemon detached from the binary that ran `restart`. Use the
  scratch binary, or the user's installed daemon gets swapped.
- The first registry open races with `serve` (see SKILL.md Gotchas). Launch opens the
  registry before `serve`.
