# leader-cluster-steer — context

Source: spec-drop `sup-20261106-a7c3` from waggledance@aadece2.

Before a leader dispatch, the rule had two tiers: a request that extends a live
leader's work goes in with `follow_up_of`; everything else opens a new leader.
A request in the same cluster as a live leader (same area or module, nearby
files, different feature) fell into the second tier, so two leaders worked side
by side without knowing about each other. Leaders dispatching leaders hit the
same gap.

## Locked decisions

- **D1**: Before every leader dispatch, by any caller including a leader, the
  caller reads `waggledance_ask_state` → `leaders[].runs` and picks one of three
  tiers:
  1. Same work (same feature, same files, same output) → `follow_up_of` into a
     run of that family's `current` leader. Unchanged.
  2. Same cluster (same area or module, or overlapping paths, but a different
     feature) → open a new leader as normal, AND send a steer note to the
     running leader (D2).
  3. Not related → new leader, no steer.
- **D2**: A steer note is a `waggledance_dispatch` call with `leader: true`,
  `follow_up_of: <a run of the running leader>` and `steer: true`. `steer` is
  refused without `leader` or `follow_up_of`, and refused with
  `handoff: true`. The daemon puts the fixed marker line
  `[waggledance steer] Context only, not new work: do not build it, claim cells for it, or reply with a plan.`
  above the caller's text, so the marker can never be misspelled. The run is
  delivered like any follow-up: it waits while the leader is mid-turn, and
  lands when the leader is idle.
- **D3**: A steer run whose family has no live leader is never delivered and
  never opens a leader. It ends `lost` with the reason, and the caller's
  mailbox hears about it the way it hears about any lost run.
- **D4**: The steer note's text names what the new request is, the area it
  touches, the new leader's `run_id`, and what to watch for. The caller
  dispatches the new leader first, so the note can carry the new `run_id`.
- **D5**: The new leader's task carries a line naming the running leader's
  `run_id` and `task_summary`, so both sides know about each other.
- **D6**: The receiving leader treats a message whose first line is the
  marker as context only. It does not build it, claim cells for it, or reply
  with a plan. It keeps the note in mind, checks the named area before
  touching shared files, and ends the turn. The supervisor template states
  this, because every leader has that skill installed.
- **D7**: `waggledance_ask_state` → `leaders[].runs[]` gains `steer: bool`.
  `task_summary` skips the marker line, so a steer row shows its note.
  Callers judge clusters from real runs, so they skip `steer` rows.
- **D8**: No new store field for the cluster judgment. `task_summary` is the
  first filter. When it is not enough, the caller reads the run's full `task`
  from `waggledance_runs`. waggledance does not know a run's feature slug or
  paths. A caller-declared field would be an unverified claim that needs a
  migration, and it would still be judged against the task text.
- **D9**: A leader that opens a new leader for tier 2 or tier 3 passes
  `handoff: false`. Without it, leader-lifecycle D4 turns a leader's dispatch
  without `follow_up_of` into a handoff of the leader's own run.
- **D10**: The template stays the single source. `setup.rs` and `doctor.rs`
  already `include_str!` it. The tool descriptions and the pattern pages
  (`paseo-leader-dispatch`, `wd-direct-leader`, `paseo-native-supervisor`)
  say the same thing. Release 0.5.4.

## Out of scope

Mid-turn injection into a running leader (Paseo delivery stays turn-based),
wd's `reply`/`redo`/`accept`, and the herdr pane path.
