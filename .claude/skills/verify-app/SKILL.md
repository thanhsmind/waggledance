---
name: verify-app
description: Drive the real waggledance supervisor — the `waggledance` CLI, its headless daemon (`serve`/`status`/`stop`/`restart`) and its stdio MCP server — inside a disposable HOME, and keep the evidence. Use when a change touches anything a user or an agent reaches through the waggledance binary and needs a live proof (`green:live`), when checking that a fresh build still launches, or when sweeping every mapped feature before a release.
---

# verify-app — waggledance

waggledance is one Rust binary with three user surfaces, all driven here:

- **CLI** — `waggledance <verb>`: registry, daemon lifecycle, the agent verbs
  (`projects`, `state`, `runs`, `schema`, `search`, `dispatch`, `await`, `task`), `mail`.
- **Daemon** — `waggledance serve`: background loops, no HTTP listener. Liveness is
  the pid in `$HOME/.waggledance/daemon.lock`, read through `waggledance status`.
- **MCP server** — `waggledance mcp`: JSON-RPC over stdio, seven `waggledance_*` tools.

There is no web UI. Every drive goes through one script:

```sh
bash .bee/verify/verify-app/control-waggledance --help
```

Always invoke it as `bash <path>`, never as a bare path. bee renders this skill into
`.claude/skills/`, `.agents/skills/` and `.opencode/skills/` without the executable
bit (source `0755`, rendered copies `0644`), so a bare path fails with
`Permission denied`. The `bash` prefix runs from either copy. Edit only the source
under `.bee/verify/verify-app/`, then run `bee onboard --apply`.

## Isolation — why this is safe to run beside the user's daemon

The user's own daemon, registry and Paseo leaders are live on this host. A drive never
touches them:

- **Scratch HOME.** `data_dir()` hangs off `HOME`, and there is no `--data-dir` flag. Each
  run gets `.bee/tmp/verify-app/<run>/home`, so its `registry.db`, `daemon.lock`,
  config and herdr socket path are all private. Two runs side by side are fine.
- **No Paseo.** Every waggledance call runs under `env -i` with `PATH=/usr/bin:/bin`. A
  scratch daemon that finds a Paseo CLI lists the user's real Paseo workspaces into its
  registry every 5 minutes and sweeps idle leaders, whatever `[paseo] enabled` says.
  Launch refuses when `paseo` is on that PATH, and doctor fails when `serve.log` says
  `Paseo CLI detected`.
- **Scratch config.** `[paseo] enabled = false`, `[worktree_sweep] enabled = false`.
  The terminal supervisor, Telegram notify and observer trigger default off.
- **Scratch must stay inside the repo.** bee's write guard refuses Bash writes outside
  the checkout, so scratch lives under `.bee/tmp/` (gitignored) and evidence under
  `.bee/verify/evidence/` (gitignored, never swept).

Never drive the user's daemon (`~/.waggledance`), and never run `waggledance stop`,
`restart` or `register` without the scratch HOME. `cleanup` kills only a pid whose
`/proc/<pid>/environ` holds this run's `HOME=`.

## Launch

`drive` launches for you. To hold an instance open by hand:

```sh
bash .bee/verify/verify-app/control-waggledance launch --run mytest
```

What launch does:

1. Builds with `cargo build --profile fast -p waggledance --message-format=json` and
   takes the binary path from `compiler-artifact.executable`. It never assumes
   `target/fast/`, because `CARGO_TARGET_DIR` is redirected on some hosts.
2. Creates the scratch HOME, its `config.toml` and a one-commit scratch git repo at
   `.bee/tmp/verify-app/<run>/repo`.
3. Opens the registry once with `waggledance list` before the daemon starts. See
   Gotchas: concurrent first opens race.
4. Starts `setsid waggledance serve` detached, and logs to `.bee/tmp/verify-app/<run>/serve.log`.

**Ready** means `waggledance status --json` returns `"running":true`. Launch polls for up
to 10 s and prints the run id. On failure it prints `serve.log`.

Tear down with `cleanup --run <id>` (see Cleanup).

## Doctor

```sh
bash .bee/verify/verify-app/control-waggledance doctor --run mytest
```

This check is read-only. It passes only when all of these hold:

- The binary exists.
- The `daemon.lock` pid is alive and carries this run's `HOME`.
- `status --json` reports running at the binary's own version.
- No Paseo CLI was detected.
- The scratch registry holds exactly the expected number of projects.

The last check catches a daemon that pulled in real projects. Run doctor first whenever
anything looks off. `drive` runs it before and after the features.

## Drive

Two scopes, one script:

```sh
bash .bee/verify/verify-app/control-waggledance drive --fast [FEATURE…] [--expect-literal TEXT]
bash .bee/verify/verify-app/control-waggledance drive --full [--expect-literal TEXT]
```

- **`--fast`** is the per-change loop. It runs build, launch, doctor, the named features,
  doctor again, then cleanup. With no feature named, it drives `registry` as the smoke
  path. It needs no network, no secret and no live agent. It takes about 10 s on a warm
  build.
- **`--full`** drives every feature in the map. Run it for a release or an audit.
- **`--expect-literal TEXT`** greps the built binary for a string only your change
  contains. A version string is not evidence that your code ran
  (`docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md`).

Feature ids (`features` prints them). Each one has a file under `features/`:

| id | surface | file |
|---|---|---|
| `daemon-lifecycle` | `serve`, `status`, `restart`, `stop` | `features/daemon-lifecycle.md` |
| `registry` | `register`, `list`, `projects`, `unregister` | `features/registry.md` |
| `agent-cli` | `search`, `schema`, `state`, `runs` | `features/agent-cli.md` |
| `mcp-server` | `waggledance mcp` over stdio | `features/mcp-server.md` |
| `leader-mail-and-dispatch` | `dispatch`, `mail send` (refusal paths) | `features/leader-mail-and-dispatch.md` |

For a cap proof on a user-facing change, drive the feature or features the change
touches and record:

```
bash .bee/verify/verify-app/control-waggledance drive --fast registry --expect-literal '<literal>' — green:live registry@cli — .bee/verify/evidence/<run>/
```

Name each entry point you drove (`@cli`, `@mcp`, `@daemon`). An entry point you did not
drive is reported as skipped, never as covered by another. When a change adds a feature
or a new entry point, add its checks as a `feature_<id>` function in the script and add a
file to `features/`. Use a step and check pair for each user action, and a read-only
second view (`registry.db`, `status`, files) for each mutation.

Neither scope is part of `commands.test`. `cargo test` asks whether the code is correct;
a drive asks whether the shipped binary works for a user.

## Evidence

Each drive writes `.bee/verify/evidence/<run>/`, which survives cleanup:

- `transcript.md` — the header gives the checkout HEAD, a flag for uncommitted crate
  changes, the binary path, its sha256, mtime and version. After it come every command
  (scratch HOME shown as `$SCRATCH_HOME`), its exit code, stdout and stderr.
- `NN-<step>.cmd|.out|.err|.exit` — the raw record of each step.
- `verdict.txt` — one `PASS`/`FAIL` line per check, plus `SKIPPED` lines for paths the drive
  cannot reach.
- `summary.txt` — `result: green|red`, the scope, the features, the counts and the evidence path.
- `serve.log` (plus `serve.before-stop.log`, `serve.attempt1.log`, `launch-retries` when they
  apply) — daemon logs with ANSI codes stripped.
- `mcp-request.jsonl` — the exact JSON-RPC sent, for `mcp-server`.

The proof standards:

- Drive the shipped binary through the same verbs a user or agent types. Never use an
  internal setter or a test-only path.
- Capture the action and the state that results from it.
- Check side effects next to output. Registry mutations are read back from `registry.db`
  with `sqlite3 -readonly`, and daemon lifecycle is read back from `/proc` and `daemon.lock`.
- Mock nothing. The one stubbed boundary is Paseo, which is kept off, so dispatch is
  proven only up to its refusal. Live leader dispatch is a named skip, never a claim.

## Cleanup

```sh
bash .bee/verify/verify-app/control-waggledance cleanup --run mytest
```

Cleanup runs `waggledance stop` under the scratch HOME and waits for the lock pid to
exit. It then kills that pid, and the pid launch started, only when its environ carries
this run's `HOME`. It copies the logs into the evidence directory and deletes
`.bee/tmp/verify-app/<run>/` (it refuses any other path). It never kills by process
name and never deletes evidence. `drive` always cleans up, on red too. After a broken
hand-run, list `.bee/tmp/verify-app/` and clean up each leftover run id.

Prune old evidence by hand (`.bee/verify/evidence/<run>/`). Nothing sweeps it.

## Gotchas

- **The first open of a fresh `registry.db` races.** When `serve` and other waggledance
  processes open a brand-new registry at the same instant, the slower one can die with
  `duplicate column name: session_id|follow_up_of|handoff`. In a reproduction, 2 of 30
  fresh HOMEs failed. This is a real product finding, filed in the bee backlog
  (`registry-migrations`). Launch avoids it by opening the registry once before `serve`.
  Do not "fix" that ordering away.
- **SQLite `xShmMap` I/O error on ntfs3.** This checkout sits on an ntfs3 mount, and a
  launch there once failed with `disk I/O error … xShmMap`. Launch retries once on that
  exact error and records the retry in `launch-retries`. Repeated failures point at the
  filesystem, not the change.
- **A second `serve` exits 0** and prints `A waggledance daemon is already running.` By
  design, it is not an error.
- **`waggledance state` always lists the scratch project as id `repo`** (taken from its
  directory name). The project is registered as `--name verify-scratch`, but `id` and
  `name` differ.
- **Bash guard.** Commands that write through `$VAR` paths, or after a `cd`, are refused.
  Run writes from the script, or with literal in-repo paths.
