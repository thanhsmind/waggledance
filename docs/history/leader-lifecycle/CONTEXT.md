# leader-lifecycle — context

Source: spec-drop `sup-20261106-7c1e` from waggledance@0adfb55, chosen by the
owner on 2026-11-06 after an audit of four days of runs.

A leader that reaches its context mark hands off through
`waggledance_dispatch {leader: true}` without `follow_up_of`, so the new
leader opens a new family. The old leader stays idle for 24 hours, and the
old family still resolves to it. In agent-works, follow-ups landed on both
leaders; one reported done with no cells and no commits. Paseo also kept 13
idle leader tabs open.

## Locked decisions

- **D1** (`06ae7954`): `waggledance_dispatch` takes `handoff: true` beside
  `leader: true` and `follow_up_of`. It is refused without `follow_up_of` or
  without `leader`. A handoff run is stored in the referenced run's family
  (`runs.follow_up_of`) and flagged in a new `runs.handoff` column
  (migration 11). `handoff: false` is accepted and opts out of D4.
- **D2** (`a154388b`): delivery of a handoff run never waits on open family
  runs and never sends to an existing leader. It starts a fresh leader whose
  prompt is the handoff task text unchanged; the transcript header names the
  leader it takes over from. Once the successor's agent id is known, the
  daemon retires every older leader of the family: it polls until that leader
  is neither `running` nor `initializing` and carries no `queued`, `working`
  or `blocked` run, then archives it. When that does not happen within the
  turn timeout, the idle sweep takes it.
- **D3** (`2b8efeba`): a `follow_up_of` run resolves its leader over the
  family runs created before it plus every handoff run in the family, in
  family order. The leader is the paseo pane of the newest such run that has
  one, so a successor wins even over runs queued before the handoff. The run
  stays `queued` while any of those runs is `queued`, `working` or `blocked`.
- **D4** (`f40d887e`): a `leader: true` dispatch with no `follow_up_of` and
  no `handoff: false`, whose caller (`PASEO_AGENT_ID`) is the pane of a run in
  the same project, is a handoff of that caller's newest run in the project.
  The result carries a warning telling the caller to pass `follow_up_of` with
  `handoff: true`. The `bee_origin` label is not read: the dispatch tool never
  calls Paseo, and the agent id already proves the caller leads the project.
- **D5** (`344e1f58`, supersedes leader-idle-close D1 `af012b0b`): the idle
  limit is `[paseo] leader_idle_minutes`, default 30, in the global config,
  overridable per project in `.waggledance.toml` `[paseo]` for leaders whose
  carried runs belong to that project. The sweep archives a leader only when
  it is not archived, not `running`/`initializing`, has no pending permission,
  has been idle strictly longer than the limit, carries no `queued`,
  `working` or `blocked` run, and no run in the family of a run it carries is
  `queued`, `working` or `blocked`. A leader carrying no run is judged by
  idle time alone. `waggledance doctor` shows the limit in force.
- **D6** (`e0f7806a`, supersedes `1ab9a58b`): no archive when a run ends
  `done`; the idle sweep (D5), handoff retirement (D2) and wd `accept` are the
  three archive paths.
- **D7** (`0598e140`): `waggledance_ask_state` → `leaders[]` entries gain
  `family_root` and `current`. The supervisor template and
  `docs/knowledge/patterns/paseo-leader-dispatch.md` say: a small follow-up
  on a live lane always goes through `follow_up_of`; a new leader opens only
  for new, unrelated work; a leader at its context mark hands off with
  `follow_up_of` and `handoff: true`.

## Out of scope

wd `reply`/`redo`/`accept`, the herdr pane path and Paseo itself are
unchanged. No change to how a non-handoff, non-follow-up dispatch opens a
leader.
