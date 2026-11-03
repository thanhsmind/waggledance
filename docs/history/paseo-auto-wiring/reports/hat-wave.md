# Hat wave synthesis — paseo-auto-wiring

Five seats on 2026-10-14/15 (four resumed after a rate-limit pause). The
claims audit matched all `read` anchors. Accepted findings:

| Finding | Seats | Landed |
|---|---|---|
| Discovery would register bee and Paseo worktrees as projects, forever, with a synchronous full index | value, facts-gaps, risks, alternatives, user-impact | D6 filters (local, `.git` dir, not archived / Paseo-owned / `$HOME` / `/tmp`, nested both ways), blocking-thread indexing |
| The file opt-in also reaches triggers; read every tick | facts-gaps | D5: same scope as the settings-page flag, mtime cache |
| A repo file grants consent to anyone who can change the checkout | risks | D5 accepted risk, documented (owner chose in-repo consent) |
| The owner's leftover AppImage `binary` line would keep the failing wrapper | user-impact | D7: wrapper demoted below the npm CLI; `setup paseo` reports leftovers |
| Refusal names only the settings page; broken file silent | user-impact, value | paw-2 rewrites the refusal with the file path and parse warnings |
| Pinned-first not in D3 | value, alternatives | dropped (D7) |
| Separate 60 s discovery loop | alternatives, risks | runs inside the delivery task, start + every 5 min (D6) |
| `enabled: Option<bool>` breaks test sites in delivery.rs | facts-gaps | delivery.rs added to paw-1 |
| mise glob duplicates and is host-specific | facts-gaps, value, alternatives | dedupe by canonical path, highest version; recorded as host fallback |
| Failed detection cached | user-impact, risks | only successes cached |
| Chosen workspace and reason invisible | risks, user-impact | reason in the log and run transcript header |

Not taken: a local trust step before a repo file counts (contradicts the
owner's no-manual rule; recorded as accepted risk); `waggledance project
prune --source paseo` (the filters keep junk out; unregistering stays out of
scope); narrowing the file's `[paseo]` table to `workspace` + model (the owner
asked which agent a leader runs as, so per-project provider/mode/thinking is
kept).
