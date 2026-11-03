# Hat wave — wd-direct-leader plan step (2026-10-22)

Five seats (high-risk): facts-gaps, alternatives, user-impact (opus); risks (fable); value (opus).
Each read CONTEXT.md and the first plan.md draft. The leader's synthesis below; each
accepted finding names where it landed in plan.md.

## Accepted

| Finding | Seat | Landed in |
|---|---|---|
| Claims 1, 3 drifted lines; claim 5 quoted a paraphrase, not `seat.rs` bytes | facts-gaps | Load-bearing claims rows 1, 3, 5 re-anchored and re-quoted |
| `tasks` table has no slice-1 reader; corr resolves through `runs.request_id` | alternatives, value | wdl-1 shrinks to the inbox table; `tasks` moves to slice 2 |
| Reusing the runs notice columns breaks D3, D6, D8 (one slot per run, 1 h expiry, no kind) | alternatives | Approach "Rejected" |
| `via` is a new dispatch argument D1 does not list | facts-gaps, alternatives, user-impact | wd marks itself with env `WAGGLEDANCE_CALLER=wd` on `waggledance mcp`; run `caller` = `openclaw:wd`; tool schema unchanged (decision logged) |
| Isolation was one-sided: a non-wd dispatch could take a `via=wd` leader | facts-gaps, risks | wdl-2: non-wd dispatch skips `via=wd` leaders; probe added |
| Batch wake contradicts D5 "for each message" | facts-gaps | one message per wake turn, D5 as written |
| Done-report grammar undefined; wdl-2 and wdl-3 could disagree | facts-gaps | "Slice-1 contract" section: grammar, kinds, frame, shared by both cells |
| `seat::report` builds its own runtime and `block_on`s; panics inside the daemon | facts-gaps, risks | wdl-4 lifts `cli_run` to async |
| Head-of-line poison: a message the gateway always rejects blocks every later one | risks | inbox `attempts`; `dead` after 5 failed wakes; dead rows skipped and logged |
| Duplicate Telegram post when a turn delivers then times out | risks, user-impact | frame carries `#<seq>`; SKILL rule: a seq already answered is a redelivery; probe added |
| Slice 1 silent on a leader blocked on a permission or a lost run | value, risks, user-impact | wdl-2: a wd run's `blocked` writes a `permission` message, `lost` writes `error` |
| No delete half for inbox rows | risks | wdl-1: `delete_project` deletes the project's inbox rows; wdl-4 prunes acked rows older than 30 days |
| wd must pick the project; ambiguous asks | user-impact | wdl-5 SKILL rule: ask when unsure, name the chosen project |
| Binary skew between the leader's `waggledance` and the daemon | risks | wdl-5 live walk confirms both binaries by content |
| Partial behavior between slices (reply/accept still on the supervisor) | user-impact | nothing merges to main before slice 3 caps; the user meets the whole feature at once |

## Not taken

- "Unknown --kind" probe called padding (value): kept, it is one assertion and pins the kinds list.
- Per-(project, via) hold lane (risks): a busy plugin leader can park a wd task on the same
  project. Kept as the project-wide hold; one project rarely has both callers live. Named in plan.md "Out of scope" as an accepted limit.

## Verdict

Approve after the edits above. Confidence of the seats: 72–85.
