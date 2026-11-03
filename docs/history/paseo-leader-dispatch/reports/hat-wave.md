# Hat wave synthesis — paseo-leader-dispatch

Five seats (facts-gaps, risks, value, alternatives, user-impact) critiqued the
first plan draft on 2026-10-14. The claims audit matched all 13 original
anchors. Accepted findings and where they landed:

| Finding | Seats | Landed |
|---|---|---|
| "Idle with a newer UpdatedAt" can report `done` before the leader starts; old log tail leaks in | value, risks, facts-gaps, user-impact | pld-2: baseline is the dispatch time; done needs assistant text in `logs --since` on two polls |
| Busy leader behavior of `paseo send` unverified | facts-gaps, user-impact | D7: refuse a busy leader |
| Two callers can start two leaders | risks | D7: per-project lock through `paseo run` |
| `paseo run` without `--cwd` starts in the caller's directory | risks | pld-1: `--cwd <root>` always |
| `paseo run` takes the prompt positionally | facts-gaps | pld-1: `--` before the task |
| Two local workspaces at one root, no rule | facts-gaps, risks | D8: refuse naming the ids |
| `terminal.enabled` and the herdr handle gate a Paseo-only path | user-impact, risks, facts-gaps | D6 |
| Unawaited runs stay `working`; board liveness is herdr-only | facts-gaps, risks | pld-3 |
| CLI subcommand is a second door; `waggledance mcp` already serves | alternatives | D9 |
| `preset_label = None` keeps leader runs out of the herdr reaper and `finish` | alternatives | pld-2 |
| Default model id and permission bypass chosen without the owner | value, user-impact, risks | D10 |
| Second composer on the surface duplicates the panel | value | D9 |
| Composer feedback: started/reused, workspace, keep text, timeout wording | user-impact | pld-4 |
| `logs` prints text, not JSON; `ls` needs `-g`; ids via `-q` | facts-gaps, risks | pld-1, claims 6, 7, 11 |
| `lib.rs` does not exist; `trigger.rs` listed with no change | facts-gaps, value | removed from file lists |

Not taken: carrying `orchestration_enabled` into the board payload for a
project picker (the picker was cut by D9); polling run status in the composer
after send (the Runs board already shows it).
