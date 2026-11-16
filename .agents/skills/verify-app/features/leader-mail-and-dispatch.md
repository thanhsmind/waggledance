# Leader mail and dispatch

An orchestrator hands work to a project with `waggledance dispatch` (to the project's Paseo
leader, a preset or a running pane) and gets a `run_id`. Leaders report back with
`waggledance mail send`, and `task` answers them. Without a live agent, only the
refusals can be driven, and those refusals are what keep bad input out.

## Sub-features

- `dispatch-refuse-paseo-off` — `dispatch --leader` with Paseo off exits non-zero, names
  Paseo and writes no run row.
- `mail-refuse-unknown-corr` — `mail send` for an unknown corr exits 2 with
  `no run found for corr`.
- `mail-refuse-bad-envelope` — a wrong `--to` and a wrong `--kind` are both named in one refusal.
- `dispatch-live` (not driven) — a real dispatch to a Paseo leader or herdr pane, then
  `await`, `task`, and accepted mail.

## How to get to it (user POV)

- Run `waggledance dispatch --project <id> --leader --task "<text>" [--request-id r]`, or
  use the MCP tool `waggledance_dispatch`.
- Run `echo "<body>" | waggledance mail send --to wd --corr <corr> --kind result|question|progress|error`.
- Run `waggledance task <corr> reply|redo|accept|allow|deny`.

## Driving it with control-waggledance

Preconditions:

- One scratch project `repo` is registered, Paseo is off, and no runs exist.

- **Dispatch refused.** Run `dispatch --project repo --leader --task "verify-app probe"
  --request-id verify-app-1`. It exits non-zero, the output mentions Paseo, and
  `registry.db` holds no run row.
- **Mail, unknown corr.** Pipe a body into `mail send --to wd --corr no-such-corr --kind
  result`. It exits 2, and stderr holds `no run found for corr`.
- **Mail, bad envelope.** Run `mail send --to someone … --kind gossip`. Stderr names both
  `recipient` and `kind`.
- **Skip line.** `verdict.txt` records `SKIPPED live leader dispatch, await, task and
  accepted mail`.
- **Drive it.** `bash .bee/verify/verify-app/control-waggledance drive --fast leader-mail-and-dispatch`.

## Gotchas

- Never let a scratch daemon see the user's Paseo to drive the live path. Its discovery
  and idle-leader sweep act on the user's real workspaces. A live proof needs a
  disposable Paseo of its own, which this skill does not provide yet.
- A `result` mail must carry the parts `Done`, `Proof`, `Left` and `Where`. A test of
  accepted mail needs a real corr from a dispatched run.
