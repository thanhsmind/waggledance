# leader-follow-up — context

Source: spec-drop `sup-20261105-d3a9` from waggledance@14187c4.

A session that dispatches with `leader: true` cannot steer a related request
into the leader already working the task: only wd's `reply`/`redo` reach an
existing leader. In `anphabe-goglbe`, a follow-up written in prose opened a
second leader beside the first. The owner wants waggledance to know which
leader owns which work, and to route a related request into that leader.

## Locked decisions

- **D1** (`0f51193e`): `waggledance_dispatch` takes an optional
  `follow_up_of` — a `run_id` or a `request_id` of a run in the same project.
  It is valid only with `leader: true`; it is refused beside `preset` or
  `pane_id`, or without `leader`, and refused when no run in that project
  matches. The resolved run id is stored on the new run
  (`runs.follow_up_of`). The `request_id` duplicate check runs first, as
  before. The task text is sent unchanged.
- **D2** (`b3c4d87a`, supersedes leader-per-task D1 `a29369df`): every leader
  dispatch starts a fresh leader, and delivery never looks up, reuses or waits
  on an existing leader, unless the run is a wd follow-up (leader-idle-close
  D2) or names `follow_up_of`. A `follow_up_of` run belongs to a family: the
  root of its `follow_up_of` chain plus every run that follows up any run in
  it. Its leader is the paseo pane of the newest earlier family run that has
  one, so a chain follows the newest leader.
  - An earlier family run is still `queued` or `working`: the run stays
    `queued`.
  - The leader is not archived and is `running` or `initializing`: the run
    stays `queued` and is retried on a later tick.
  - The leader is not archived and idle: the task is sent to it
    (`follow_on`), mode check first.
  - The leader is archived, cannot be inspected, or was never started: a new
    leader starts. Its prompt is the task texts of the chain from the root to
    the referenced run, each followed by a blank line, then this task. The
    run's transcript header names the gone leader; the run's pane records the
    new one.
- **D3** (`1ab9a58b`, supersedes leader-per-task D2 `0398ca94`): the daemon no
  longer archives a leader when its run ends `done`, wd or not. The 24-hour
  idle sweep (leader-idle-close D1) archives an idle leader; wd `accept`
  still archives a wd leader. A `lost` or `blocked` run's leader is not
  archived either. The cheaper shape the spec invited was weighed: no shape
  is cheaper than reusing the sweep that already exists, and a working day
  fits inside 24 hours.
- **D4** (`aa16f27d`): `waggledance_ask_state` → `leaders[]` entries gain
  `runs`: the runs whose pane is `paseo:<agent_id>`, newest first, at most 5,
  each `{run_id, request_id, status, follow_up_of, task_summary}`.
  `task_summary` is the first non-blank task line that is not provenance (a
  `spec-drop <corr> from waggledance@<sha>` line or the wd
  `Task waggledance#<corr> from wd.` preamble), trimmed to 200 characters.
  `waggledance_runs` rows carry `follow_up_of` too.
- **D5**: the callers are taught in the tool descriptions, the
  `waggledance-supervisor` skill (its source is
  `docs/waggledance-supervisor-skill-template.md`, installed by
  `waggledance setup`), and the two knowledge patterns
  `paseo-leader-dispatch.md` and `wd-direct-leader.md`: read `ask_state`
  leaders first; when the new spec extends work a live leader owns, dispatch
  with `follow_up_of: <run_id>` and no fresh correlation id; otherwise mint a
  new one.

## Out of scope

wd-bridge's `reply`/`redo`/`accept` keep working as they do. No change to the
herdr pane path or to Paseo.
