# Goal-check judge — wd-direct-leader (2026-10-22)

Three read-only `review` dispatches (opus), one per slice, at frozen head `b45985d`.
Builder model also opus, so independence is `same-model`. Every cell PASS. The judges
returned numeric confidence and `check`/`result` keys; the leader transcribed them into
the store's `judge-verdict/1` shape below (`id`/`status`, confidence 85+ = high,
80–84 = medium) without changing any verdict.

Low notes the judges raised, none failing a check:

- wdl-2: the slice-1 test for a blocked wd run's permission message was replaced in
  slice 2 when the watcher became the one writer; its truth is proven at 17526eb.
- wdl-2: two existing `LeadersRead::Listed` test literals gained a compile-required field.
- wdl-8: the board's change check reads open tasks only, so closing a task alone sends no board.
- wdl-9: the report cross-references §4.5 for §4.6, quotes the board line differently
  from the code, and ran its tests with `--release`.

```json wdl-1
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"v6-opens-at-v7-runs-intact","status":"PASS","evidence":"test migrate_adds_the_inbox_table_to_a_v6_database_keeping_its_runs"},{"id":"next-pending-skips-acked-and-dead","status":"PASS","evidence":"tests next_pending_inbox_message_is_the_lowest_seq_and_ack_hides_it, five_failed_attempts_mark_an_inbox_message_dead_and_next_pending_skips_it"},{"id":"engine-exposes-inbox-methods","status":"PASS","evidence":"engine.rs pass-throughs used by delivery, mail, wake"},{"id":"no-change-to-runs-or-migrations-1-6","status":"PASS","evidence":"4ba5b2b hunks touch only new code"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-2
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"wd-dispatch-only-via-wd-leaders","status":"PASS","evidence":"paseo.rs resolve_leader wd branch; test a_wd_run_starts_a_via_wd_leader_beside_a_non_wd_one"},{"id":"non-wd-skips-via-wd-leaders","status":"PASS","evidence":"test a_non_wd_run_skips_a_via_wd_leader"},{"id":"blocked-wd-run-one-permission-message","status":"PASS","evidence":"17526eb test a_blocked_wd_run_mails_one_permission_and_sends_no_notice; live inbox seq 1"},{"id":"caller-reaches-resolve-leader","status":"PASS","evidence":"delivery.rs run_caller == WD_CALLER passed to resolve_leader"},{"id":"existing-tests-unedited","status":"PASS","evidence":"only compile-required field added to two literals, no assertion touched"},{"id":"dispatch-schema-unchanged","status":"PASS","evidence":"test a_wd_caller_comes_from_waggledance_caller_only_without_a_paseo_agent"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-3
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"missing-parts-exit-2-no-row","status":"PASS","evidence":"test a_result_missing_proof_and_where_names_both_and_writes_nothing"},{"id":"valid-mail-prints-seq","status":"PASS","evidence":"test a_valid_result_inserts_one_row_and_returns_its_seq"},{"id":"cli-reaches-mail","status":"PASS","evidence":"cli.rs Command::Mail dispatch"},{"id":"existing-cli-unchanged","status":"PASS","evidence":"6e6085f cli.rs additions only"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-4
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"ack-only-after-ok-turn","status":"PASS","evidence":"test wake_acks_a_message_after_an_ok_turn and the non-zero and not-ok tests"},{"id":"dead-after-five-failures","status":"PASS","evidence":"test wake_marks_a_message_dead_on_its_fifth_failure_and_wakes_the_next"},{"id":"reconciler-starts-and-stops-wake","status":"PASS","evidence":"test an_openclaw_agent_id_starts_the_wake_and_its_absence_stops_it"},{"id":"supervisor-report-unchanged","status":"PASS","evidence":"seat.rs reused the moved helpers with the same bodies"},{"id":"no-runtime-in-daemon-task","status":"PASS","evidence":"test wake_cli_run_awaits_a_child_inside_the_running_runtime"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-5
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"handoff-returns-corr-and-run-id","status":"PASS","evidence":"slice-1-live.md corr t-69838f, run_id run-195e88cfa5951fcc"},{"id":"result-reaches-wd-and-acked","status":"PASS","evidence":"wd transcript frame waggledance#t-69838f #2; inbox seq 2 acked 14:24:26"},{"id":"bridge-sets-wd-caller","status":"PASS","evidence":"wd-bridge.py WD_ENV; run row caller openclaw:wd"},{"id":"setup-parity-test-passes","status":"PASS","evidence":"setup::tests::embedded_openclaw_files_equal_the_repo_files green"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-6
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"hand-off-opens-one-task","status":"PASS","evidence":"test a_wd_hand_off_opens_one_task_and_follow_ups_and_other_callers_open_none"},{"id":"delivery-records-leader-clears-marks","status":"PASS","evidence":"test delivering_a_wd_run_records_its_leader_and_clears_the_nudge_marks"},{"id":"any-mail-stamps-last-mail","status":"PASS","evidence":"test every_accepted_mail_stamps_the_tasks_last_mail_and_a_refused_one_does_not"},{"id":"migrations-1-7-unchanged","status":"PASS","evidence":"7f45e8e adds migration 8 only"},{"id":"non-wd-writes-no-task","status":"PASS","evidence":"test delivering_a_non_wd_run_touches_no_task"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-7
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"reply-queues-corr-n","status":"PASS","evidence":"test reply_and_redo_queue_numbered_wd_runs_that_resolve_to_the_corr"},{"id":"accept-closes-and-archives","status":"PASS","evidence":"tests accept_closes_the_task_archives_an_unused_leader_and_repeats_quietly, accept_leaves_a_leader_another_task_or_a_live_run_still_uses"},{"id":"allow-deny-run-paseo-permit","status":"PASS","evidence":"test allow_and_deny_permit_the_leader_and_mark_the_request_answered"},{"id":"dispatch-schema-unchanged","status":"PASS","evidence":"no dispatch_schema hunk"},{"id":"bridge-never-chooses","status":"PASS","evidence":"wd-bridge.py permit answer is a required positional"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-8
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"permission-once-per-request","status":"PASS","evidence":"watcher tests; delivery mail_permissions removed"},{"id":"nudge-once-then-progress-once","status":"PASS","evidence":"test a_silent_idle_leader_is_nudged_once_then_wd_hears_progress_once"},{"id":"board-only-on-change","status":"PASS","evidence":"test a_board_goes_out_only_when_a_task_changed"},{"id":"reconciler-starts-both-loops","status":"PASS","evidence":"test an_agent_id_with_paseo_starts_the_heartbeat_and_its_absence_stops_it"},{"id":"daemon-never-answers","status":"PASS","evidence":"only waggledance_task calls permit"},{"id":"no-model-turn","status":"PASS","evidence":"heartbeat.rs uses paseo and engine calls only"}],"fixability":"automatic","confidence":"high"}
```

```json wdl-9
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"permission-through-wd-unblocks","status":"PASS","evidence":"slice-2-live.md section 4"},{"id":"accept-archives","status":"PASS","evidence":"slice-2-live.md archived true"},{"id":"heartbeat-nudge-then-progress","status":"PASS","evidence":"slice-2-live.md section 5"},{"id":"heartbeat-secs-restored","status":"PASS","evidence":"daemon reports 600"},{"id":"no-unrecorded-permit","status":"PASS","evidence":"both answers have a recorded wd-bridge permit allow"}],"fixability":"automatic","confidence":"medium"}
```

```json wdl-10
{"schema":"judge-verdict/1","verdict":"PASS","checks":[{"id":"supervisor-unknown-command","status":"PASS","evidence":"fresh build: unrecognized subcommand 'supervisor', exit 2"},{"id":"supervisor-table-still-loads","status":"PASS","evidence":"test a_retired_supervisor_table_should_still_load"},{"id":"via-wd-or-daemon","status":"PASS","evidence":"mcp.rs attach_leaders"},{"id":"leader-mode-two-doors","status":"PASS","evidence":"skill template leader-mode section"},{"id":"watchdog-untouched","status":"PASS","evidence":"no change to supervisor.rs or supervisor_enabled code"},{"id":"no-behaviour-change-elsewhere","status":"PASS","evidence":"waggledance 1269 passed, core 537 passed"}],"fixability":"automatic","confidence":"high"}
```
