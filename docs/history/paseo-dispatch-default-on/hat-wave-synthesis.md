# Hat-wave synthesis — paseo-dispatch-default-on

Plan-step wave, five seats (decision 4deae727). The risks seat failed twice on a
fable HTTP 429 (out of credits); the leader ran the risk pass (decision 3ec23f00).
Synthesis recorded as decision e42120e6.

## Accepted into plan.md

- facts-gaps: claims rows 1, 3 and 6 rewritten to exact bytes; new row 7 — Paseo
  leader dispatch has no `terminal.enabled` gate; the risk map now says so.
- facts-gaps + alternatives: every Engine fn returning `Project` fills the effective
  consent (`list_projects`, `get_project`, `register`, `ensure_project`,
  `list_projects_with_queued_paseo_runs`).
- alternatives: a `.waggledance.toml` that exists but cannot be read or parsed means
  off (decision 4b745240), so the only off switch never fails open.
- value: keep `set_orchestration_enabled` and the store setter (backlog row filed);
  rewrite only tests whose subject is the opt-in rule; pd-2 proof states zero hits.
- user-impact: Settings hint says edits apply without restart, that the observer
  trigger acts on every project that is on, and shows parse warnings per row; the
  owner is told 13 of 17 registered projects become dispatchable (PKM, piggery and a
  worktree entry among them).

## Leader risk pass (in place of the risks seat)

- Widening: any MCP caller can now hand work to any managed project through a Paseo
  leader unless its file says `orchestration = false`. Owner-requested (D1); bounded by
  the per-project file and by Paseo being installed.
- Trigger: `terminal.trigger_enabled` is false on this host; when turned on it acts on
  every project that is on. Stated in the Settings hint.
- No data-loss path: nothing writes into any repo; the DB column stays.

Verdict: proceed with the revised plan.
