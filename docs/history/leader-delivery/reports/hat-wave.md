# Hat wave synthesis — leader-delivery

Five seats critiqued the first draft on 2026-10-14; a second live probe
answered the facts they found missing. Accepted findings:

| Finding | Seats | Landed |
|---|---|---|
| Bootstrap + `wait` repeats the idle-lag the plan rejected | facts-gaps, alternatives | Probe: blocking `run` returns `completed`; bootstrap removed (D8) |
| Which turn a busy send's `completed` belongs to | facts-gaps, value, risks, alternatives | Probe: a send into a running turn interrupts it; D5 forbids sending to a running agent |
| Killing a timed-out send does not end the turn | facts-gaps, risks | Probe confirmed; D6 holds the project until idle |
| Daemon binary unprobed under systemd; `$PASEO_CLI` path changes per launch | facts-gaps, risks, user-impact | Probe: wrapper works from a systemd scope; binary resolved per call with existence check (ld-1) |
| Queued forever with the daemon down | user-impact, risks | Heartbeat + paused warning (D8, ld-3) |
| Lost-but-still-running after restart; never-sent runs marked lost | user-impact, risks | D6 restart rules (ld-4) |
| Pre-send failures made permanent | risks | Requeue with backoff, lost after 5 (D6) |
| ld-3 removed await code still used by the reaper | value | Await rewrite moved into ld-4 |
| Captures missing for permission, run, no-wait, non-completed | value, facts-gaps | All captured; ld-1 commits them as fixtures |
| `pane_id` set before the agent id exists | facts-gaps | Claim keeps `paseo:pending`; `set_run_pane` (ld-2) |
| Herdr idempotency races | facts-gaps, risks | D7: sequential on herdr, recorded |
| Caller notice could interrupt a busy caller; panel gets none | user-impact, alternatives | D5 applies to notices; panel polls status (ld-5) |
| Notice wording | user-impact | Informational vs blocked-with-action (ld-4) |
| `queued` hidden behind `working` | user-impact | Public `queued` status with age (D8) |
| 6 h timeout stalls a queue | value | Default 1 h (D8) |
| Two daemons double-send | risks | Per-project lock held for the whole delivery |
| `notifyOnFinish` not used — why | alternatives | Rejected line in plan |
| New pattern may duplicate existing ones | value | ld-6 extends an existing pattern when one covers it |

Not taken: a single-instance assertion for `waggledance serve` (the lock
file covers double delivery); a log-tail check after `Completed` (the turn
end is Paseo's own signal, D2).
