# wd Direct Leader — slice 3 contract

Slice 3 of `docs/history/wd-direct-leader/plan.md` ("Shape"): retire the Paseo supervisor
(D11) and sync the knowledge layer. This file is the one home for what slice-3 cells
remove and keep.

## Remove

- The `waggledance supervisor` command and both subcommands (`ensure`, `report`) in
  `crates/waggledance/src/cli.rs`, and `crates/waggledance/src/seat.rs` with its tests.
  Helpers `wake.rs` already owns stay in `wake.rs`.
- `SupervisorConfig` and the `supervisor` field of `Config` in
  `crates/waggledance-core/src/config.rs`. An existing `[supervisor]` table in a user's
  config file must still load (serde ignores it).
- `docs/waggledance-supervisor-agent.md` (the supervisor agent's instructions).
- `VIA_LABEL` (`waggledance.via=supervisor`), `via_leaders_args` and the supervisor arm of
  `ask_state.leaders[].via`: `via` reads `wd` or `daemon`. Existing leaders still labelled
  `waggledance.via=supervisor` read `daemon` from then on; accepted.
- The tests that pin the removed items: the `c.supervisor.*` config tests, the
  `via_leaders_args` test in `paseo.rs`, the `via: supervisor` test in `mcp.rs`.

## Keep

- The herdr watchdog (`crates/waggledance/src/supervisor.rs`,
  `terminal.supervisor_enabled`) and `bee supervisor`: different features that share the word.
- The `waggledance-supervisor` Claude skill (relay and execute modes). Its template
  `docs/waggledance-supervisor-skill-template.md` changes only in leader mode: from
  aicoworker, `wd-bridge.py handoff --project <p> --task <t>`; from any other session,
  `waggledance_dispatch {project, task, leader: true, request_id}`. No `supervisor
  ensure`, no `paseo send` to a supervisor.
- `[openclaw]` config, the hook token and `setup openclaw` as they are.

## Knowledge

- New pattern `docs/knowledge/patterns/wd-direct-leader.md`: wd hands off through the
  daemon, the inbox and wake, the follow-up tool, the heartbeat and the permission
  watcher, with Pointers to the files.
- `docs/knowledge/patterns/paseo-native-supervisor.md`: lifecycle `retired`, one line
  pointing to the new pattern.
- `docs/knowledge/patterns/paseo-leader-dispatch.md`: the Frozen section lifted; wd is a
  caller, its runs carry `openclaw:wd` and reach the inbox, not caller notices.
- `docs/knowledge/patterns/index.md`, `README.md` and any spec line (including the
  doctor row in `docs/specs/doctor.md`) that promises `supervisor ensure`
  or `supervisor report`.

## Host step

After merge-ready code is installed, the leader, once:

1. lists the supervisor agent (`paseo ls -g --label waggledance.role=supervisor`) and
   deletes any Paseo heartbeat or schedule that targets it;
2. archives it (`paseo archive <id>`), which Paseo keeps, so a revert can unarchive it;
3. keeps `~/.waggledance/supervisor/` on disk as the old ledger (user data, never deleted
   by the feature);
4. runs `waggledance doctor --fix` so the installed `waggledance-supervisor` skill picks
   up the new leader mode.
