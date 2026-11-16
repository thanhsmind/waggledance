# waggledance verification map

This directory is the maintained source for verifying what a user or an agent reaches
through the `waggledance` binary. Read this index before driving, then use the matching
feature file as the recipe.

## Baseline preconditions

- Build and launch only through `bash .bee/verify/verify-app/control-waggledance`. It
  resolves the binary cargo really wrote and gives each run a scratch HOME at
  `.bee/tmp/verify-app/<run>/home`.
- The scratch registry starts empty. Launch creates one scratch git repo at
  `.bee/tmp/verify-app/<run>/repo`, and that repo is registered as id `repo`.
- `paseo` must not be on `/usr/bin:/bin`. Every waggledance call runs with that PATH under `env -i`.
- `doctor --run <id>` must pass before any recipe.
- Never drive the user's `~/.waggledance` daemon.

## Driving conventions

- `drive --fast <feature-id>` runs one recipe end to end. `drive --full` runs every recipe.
- A recipe step is a `step <name> wd <args>` call in the script, recorded as
  `NN-<name>.cmd|.out|.err|.exit`.
- Treat command text as literal. Assert on stable strings in stdout and stderr, on JSON
  keys and on exit codes, never on timing or table spacing.
- Restore the baseline after a mutation. `registry` re-registers the scratch repo after
  it unregisters it.

## Proof and skip reporting

- CLI proof is the command, stdout, stderr and exit code, all in `transcript.md`.
- Mutation proof adds a read-only second view: `registry.db` through `sqlite3 -readonly`,
  `status --json`, or `/proc/<pid>`.
- Record the feature id and entry point (`@cli`, `@mcp`, `@daemon`) with the evidence path.
- A path that needs a live Paseo or herdr agent is written as `SKIPPED` in
  `verdict.txt`, never as passed.

## Feature entry contract

Each feature file has an H1, one paragraph, and four H2s in this order:
`Sub-features`, `How to get to it (user POV)`, `Driving it with control-waggledance`,
`Gotchas`.

## Features

- [Daemon lifecycle](./daemon-lifecycle.md) covers starting, refusing a duplicate,
  restarting, stopping and status.
- [Project registry](./registry.md) covers registering, listing, the JSON view and
  unregistering.
- [Agent CLI](./agent-cli.md) covers verb search, schema, the state rollup with field
  selection, and runs.
- [MCP server](./mcp-server.md) covers the stdio handshake, the seven tools and the error
  for an unknown tool.
- [Leader mail and dispatch](./leader-mail-and-dispatch.md) covers the refusals of
  dispatch and mail. The live leader path is a named skip.
