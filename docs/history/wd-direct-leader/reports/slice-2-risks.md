# Slice 2 risks consult — wd-direct-leader (2026-10-22)

Seat: hat-risks (fable), over `slice-2.md` and `slice-2-cells.json` before plan rev 1.

| Finding | Landed in |
|---|---|
| Several open tasks on one project share one via=wd leader, so a permission row's `agentId` does not name a corr | `slice-2.md` Permission relay: the open task on that leader with the newest `last_delivered_at` |
| A leader blocked on a permission looks idle to the heartbeat | Heartbeat rule 1 requires `pending_permissions` 0 |
| `accept` archives a leader another open task or a queued `corr:n` run still uses | `accept` archives only when no queued or working run belongs to the leader and no other open task shares it (wdl-7) |
| Two replies in one second compute the same `corr:n`; the duplicate looks like success | a duplicate return retries with n + 1 (wdl-7) |
| A nudge can race delivery's own send | nudge under the project `LeaderLock`, idle re-checked inside (wdl-8) |
| A leader that mailed a question gets nudged | any mail stamps `last_mail_at` (wdl-6) |
| The board frame reads as a one-task report | board gets its own instruction line (wdl-8) |
| Closed tasks and permission requests are never pruned | the hourly prune covers them (wdl-6 method, wdl-8 call) |

Not taken: dropping the bare "ok means accept the newest corr" rule (carried from
paseo-native-supervisor D12 into D10); the archive guard above removes the irreversible
case it worried about.

Seat confidence 85.
