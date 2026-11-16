# Agent CLI

Agents use the waggledance CLI in place of, or next to, MCP. `search` finds the verb for
a job, `schema` describes the verbs and flags as JSON, `state` rolls up bee state for
each project, and `runs` lists dispatched runs. Output is JSON on stdout, and
`--fields`/`--limit` trim it.

## Sub-features

- `agentcli-search` — `search <words>` ranks verbs as `{"matches":[{"verb":…}]}`.
- `agentcli-schema` — `schema [verb]` prints the flags and help as JSON.
- `agentcli-state` — `state` prints one summary row per project, and `state <project>`
  prints one project's digest.
- `agentcli-fields` — `--fields a,b` keeps only those keys on each row.
- `agentcli-runs` — `runs` lists runs, and `runs show <id>` prints one run in full.
- `agentcli-unknown` — a misspelt verb exits non-zero.

## How to get to it (user POV)

- Run `waggledance search list projects`, `waggledance schema state`, `waggledance state
  [--fields …]` or `waggledance runs [--project p]`.
- `waggledance --help` names these as the agent verbs.

## Driving it with control-waggledance

Preconditions:

- One scratch project `repo` is registered. It has no `.bee/` directory, so its state
  fields are null.

- **Search.** Run `search list projects`. The first match is `"verb":"projects"`.
- **Schema.** Run `schema state`. The JSON names the `fields` flag.
- **State.** Run `state`. Stdout holds `"project_id":"repo"`.
- **Fields.** Run `state --fields project_id,phase`. `project_id` is kept and
  `cell_counts` is dropped.
- **Runs.** Run `runs`. It exits 0 on a registry with no runs.
- **Typo.** Run `sttae`. It exits non-zero.
- **Drive it.** `bash .bee/verify/verify-app/control-waggledance drive --fast agent-cli`.

## Gotchas

- The scratch repo has no bee store, so this recipe does not cover a real bee digest
  (feature, phase, cells). To prove one, run `bee onboard` in the scratch repo first and
  add a step that does so.
- A tool error goes to stderr with exit 1, and stdout stays JSON only. Assert on stdout
  and stderr separately.
