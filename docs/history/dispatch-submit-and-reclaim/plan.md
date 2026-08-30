# Dispatch: Submit and Reclaim — Plan

**Feature:** dispatch-submit-and-reclaim · **Lane:** standard · **Class:** feature
**Flags:** data-model, external-systems, multi-domain · **Product files:** 7
**Decisions:** `3ba233ae` (D1 — close the pane on completion), D2 (completion is an explicit
declaration, never an inferred state)
**Backlog:** `p-6cd25610` (unsubmitted send), `p-31d113c2` (pane never reclaimed)
**Worktree:** `waggledance--wt--dispatch-submit-and-reclaim` (branch `wt/dispatch-submit-and-reclaim`)
**Revision:** rev 3 — rev 2 rewrote rev 1's close guard after the review wave found it wrong
in three blocking ways (P1-1, P1-2, P1-3 below). Rev 3 promotes the marker-only guard from
the agent's inference to D2, the user's stated instruction, and widens it: the close path
reads **no** pane-derived signal at all.

## The two defects, as verified against source

Both were found live on 2026-08-30 during supervisor-seat spec drops into beehive, and both
are confirmed in the tree — not a stale-binary story. The running daemon is
`~/.cargo/bin/waggledance` (2026-08-30 07:13), newer than every source file below.

### Defect A — the submit Enter is fired blind and never checked

`orchestrate::send_task` (`orchestrate.rs:264-272`) hands the task to
`Herdr::send_input(pane_id, text, submit = true)`. The real implementation
(`herdr/socket.rs:644-666`) writes the text, waits for the pane's screen to stop changing,
then issues `pane.send_input {keys:["enter"]}`. The wait is capped at
`SETTLE_MAX_WAIT = 1500ms` (`socket.rs:219`) and returns on the deadline with **no signal to
the caller**; the Enter then goes out unconditionally. Claude Code's cold start runs past
1.5 s, the Enter is swallowed, and nothing reads the pane afterwards. The run row is
inserted, a `run_id` is returned, and `waggledance_await` reports `working` forever against
a byte-identical delta. Recovered by hand both times with `herdr pane send-keys <pane> enter`.

A settle-race fix already lives here (`terminal-attach-submit-race`). It narrowed the
window; it cannot close it, because a timing heuristic cannot know whether the keystroke was
accepted. **Only the agent's own observed state can answer that.**

### Defect B — nothing closes a dispatched pane

`orchestrate::finish` (`orchestrate.rs:652-675`) writes the run's terminal status and
optionally enqueues a notification — its entire side effect. It does not even take a herdr
handle. The `Herdr` trait (`herdr/mod.rs:164-247`) exposes `snapshot`, `ping`, `read_pane`,
`send_input`, `send_text`, `send_keys`, `tab_create`, `agent_start` — spawn verbs with **no
teardown counterpart**. The `runs` table (`repository.rs:625-637`) has no column describing a
pane's fate. Every spawn-dispatch leaks one live agent process.

## The decisive finding: herdr already solves defect A

Probing the live daemon's method table returned both halves of this feature, and
`herdr api schema --json` confirms the wire shapes:

- **`agent.prompt`** — `AgentPromptParams { target, text, wait: { until: [AgentStatus],
  timeout_ms } }`. Its contract, verbatim from `herdr agent prompt --help`:

  > If the agent is already blocked, submission is rejected with agent_blocked before any
  > input is sent. When an accepted submission starts from another non-working state, --wait
  > first requires an observed state change within 5000ms; otherwise it returns
  > agent_prompt_stalled. A shorter --timeout returns timeout instead. […] It does not track
  > turns: if the agent is already working, that active turn's completion may match.

- **`pane.close`** — takes `PaneTarget`; probed with a bogus pane id and answered
  `pane_not_found`, not `unknown variant`.

`agent.prompt` is the missing guarantee: it observes the agent leave its idle state and
names the failure when it does not. Defect A stops being "detect a wedged run later" and
becomes "the send either takes or refuses at dispatch time".

Two review findings sharpen how we call it:

- **The spawn race is already closed upstream.** `herdr agent start --help`: *"Success means
  the expected agent was detected in the same terminal and is ready for input"*, with a
  30 s readiness timeout. So by the time `start_declared_agent` returns (`herdr/mod.rs:459`)
  the agent is registered and promptable. No extra wait needed.
- **`until: [working]` alone is too narrow.** A short turn that goes working→idle between
  observer samples never matches. We pass `until: [working, idle, done]` with
  `timeout_ms: 8000` — deliberately above the daemon's own 5000 ms change-detection window,
  so a genuine no-change reports as `agent_prompt_stalled` rather than being masked as a
  plain `timeout`. **Only `agent_prompt_stalled` and `agent_blocked` are treated as a failed
  send**; a `timeout` after an observed state change means the text went in, which is all
  dispatch needs to know.

## What rev 1 got wrong (review findings, folded in)

- **P1-1 — `RunStatus::Failed` does not exist.** `orchestrate.rs:493-512` defines exactly
  `Working, Done, Blocked, Timeout`. Rev 1's "close on Done and Failed" would not compile.
  Worse, it never considered **`Working`**, which *does* reach `finish`: `timed_out_status`
  (`:630-635`) yields `Timeout` **or `Working`** when the await deadline expires against a
  pane still reading as working — pinned by `await_run_times_out_while_working`
  (`orchestrate.rs:805`). That is the most-alive state that reaches `finish`.
- **P1-2 — `Done` has two producers and only one is trustworthy.** `orchestrate.rs:608-610`
  returns `Done` on a fresh marker. `orchestrate.rs:612-622` returns `Done` after
  `STABILITY_READS` (3) unchanged reads at 500 ms — **~1.5 s of static screen** — for any
  pane whose agent status is `Unknown` *or absent from `snapshot.agents`*. That is live on
  this host right now: `herdr agent list` returns `{"agents":[]}` while `herdr pane list`
  returns 13 panes, all `agent_status: "unknown"`. An agent that pauses 1.5 s on a tool call
  would be declared done and, under rev 1, killed.
- **P1-3 — the close would reach panes waggledance never created.** `DispatchTarget::Pane`
  (`orchestrate.rs:116`) dispatches into a pre-existing agent pane the user owns; only
  `Spawn` creates one. D1's rationale is explicitly scoped to what waggledance made. `finish`
  cannot tell them apart — but `run.preset_label` is `None` exactly for the pane-target case
  (`orchestrate.rs:361`, `domain.rs:72-74`) and is the discriminator.
- **P2-1 — rev 1's "pane herdr does not track as an agent" exception was false.**
  `dispatch_run` preflights every `Pane` target (`:305`), and `preflight` (`:184-202`) looks
  the pane up in `snapshot.agents`, refusing with `NoSuchPane` when absent. Every dispatch
  target is agent-tracked, so `agent.prompt` covers both paths and the exception is dropped.
- **P2-3 — do not retry a stalled submit.** Input is withheld only on `agent_blocked`; a
  *stall* means the text was already delivered. Retrying would re-type the task into a
  composer that may already hold it. Refuse instead.
- **P2-4 — closing the pane breaks re-await.** `mcp.rs:1063-1090` calls `await_run`
  unconditionally with no terminal-status short-circuit, so a second await on a finished run
  re-reads the pane. Against a closed pane that is a propagated error. The stored transcript
  has to answer instead — which is why capture lands before close.
- **P2-5 — do not put a `final_transcript` field on `Run`.** `domain.rs:83-92` records the
  deliberate precedent: `feature` lives as a **column only**, reached through store methods,
  because a `Run` field touches every construction site (`orchestrate.rs:357`, `:689`;
  `server.rs:13756`, `:32943`, `:33051`; `views.rs:13303`; `mcp.rs:2232`; `main.rs:689`;
  `engine.rs:957`; `repository.rs:706`) plus positional `row_to_run` and three column lists.
  Follow the precedent.

## Close guard — the whole safety argument in one place

**D2 governs: completion is an explicit declaration, never an inferred state.** The user's
reason is that a pane's observed state cannot distinguish *finished* from *running in the
background*, so any status-derived completion signal can kill a working agent.

A pane is closed **only** when every one of these holds:

1. the run reached `Done` **via the marker branch** (`orchestrate.rs:609`) — the agent itself
   printed `HERDR_DONE_<nonce>`;
2. `run.preset_label.is_some()` — waggledance spawned this pane;
3. the final transcript write already succeeded.

Guard 1 is a whitelist of exactly one producer, not a blacklist of bad states. The close path
reads **no** `agent_status`, no pane liveness, no screen stability, and does not branch on
`RunStatus` beyond "was this the marker branch" — so a future status, or a future producer of
`Done`, cannot silently acquire the right to close a pane. Concretely this excludes the
stability heuristic at `:621`, and `Working`, `Timeout` and `Blocked` besides. `Blocked` would
be excluded on its own merit anyway: a blocked agent is waiting on a human, and the human
needs the pane to answer in.

The honest cost: a run that finishes without printing its marker never closes its pane, so the
leak is narrowed rather than eliminated. That is the correct trade — a leaked pane costs
machine performance, a killed working agent costs the work. Both live beehive drops completed
via the marker, so the reported case closes.

A close failure never changes the run's status: the work finished, the pane is bookkeeping.

### The adjacent defect D2 exposes, recorded not fixed

By D2's own standard, `orchestrate.rs:612-622` is already wrong on its own terms: it reports a
**run** `Done` after three unchanged 500 ms reads of a pane whose agent status is `Unknown` or
absent — an inferred completion, exactly what D2 forbids, and it can report done for an agent
that merely paused on a tool call. This feature does not act on that `Done` (guard 1), but it
also does not remove it: dropping the heuristic would make every non-marker run hang to the
await timeout, which is a contract change beyond the two filed defects. Filed as a follow-up.

## Shape — one slice, five cells

### `dsr-1` — `Herdr::agent_prompt`, a submit that confirms

Trait method on `herdr/mod.rs`, `SocketHerdr` impl over `agent.prompt` (`herdr/socket.rs`),
`FakeHerdr` impl (`herdr/fake.rs`). Map `agent_prompt_stalled` and `agent_blocked` to typed
variants, distinct from `timeout` — the caller must branch on stalled.

`send_input` is **not** touched: `sendinput_with_submit_issues_two_distinct_socket_requests`
and `sendinput_still_sends_enter_when_screen_never_settles` (`socket.rs:1398`, `:1464`) keep
asserting today's behavior for other callers. Reviewer ran
`cargo test -p waggledance --bin waggledance sendinput` → 6 passed.

**Proof:** mock-socket-server unit tests (the seam at `socket.rs:1297+`) — accepted, stalled,
blocked, and timeout-after-change.

### `dsr-2` — Route the dispatch send through it

`send_task` / `dispatch_run` call `agent_prompt` with `until: [working, idle, done]`,
`timeout_ms: 8000`, for **both** dispatch targets. Stalled or blocked → fail through the
existing `DispatchRefusal::SendFailed` channel (`orchestrate.rs:99`) naming the stall. No
retry. No new MCP status, no widened tool contract.

`dispatch_run` already sends at `:352-354` and inserts at `:369-371`, so "a refused send
inserts no run row" is preserved, not built.

**Proof:** `orchestrate` tests over `FakeHerdr` — a normal dispatch returns a run; a stalled
dispatch refuses and leaves no run row.

### `dsr-3` — Capture the transcript, and serve a finished run from the store

Additive nullable `final_transcript` column on `runs` (`repository.rs` schema + `ALTER TABLE`
migration, following the `feature` precedent at `repository.rs:546`), reached through store
methods — **no `Run` struct field** (P2-5). `finish` writes the delta it already holds.

Same cell, because the close in `dsr-4` depends on it: `await_run` short-circuits a run
already in a terminal status, answering from the stored status + transcript instead of
reading the pane (P2-4).

**Proof:** repository test for migration idempotence (precedent `repository.rs:1133`);
`orchestrate` test that a second await on a finished run answers without a `read_pane` call.

### `dsr-4` — Close the pane, under the guard

`Herdr::close_pane` (trait + socket over `pane.close` + fake), called from `finish` under all
three guard conditions above. `finish` needs a herdr handle threaded through
`await_run` → `await_run_with_poll_interval` → `finish` (mechanical, `orchestrate.rs:549/573/652`).

**Proof:** `orchestrate` tests over a `FakeHerdr` recording close calls — marker-`Done` with a
preset closes; stability-`Done` does not; `Working`, `Timeout`, `Blocked` do not; a
`preset_label: None` run does not; a close error still reports `Done`. Plus one test pinning
D2 structurally: a run reaching `finish` with `RunStatus::Done` but not through the marker
branch closes nothing, so the guard cannot later be loosened to "status == Done".

### `dsr-5` — Prove the whole path live

Rebuild (`cargo build --profile fast -p waggledance`), install, restart the daemon, run one
real supervisor-seat spec drop and record: the task submitted with **no** hand-sent Enter;
`waggledance_runs` shows `done` with a non-empty `final_transcript`; the pane is gone from
`herdr pane list`; a second `waggledance_await` on that run still answers `done`.

Per `docs/knowledge/patterns/prove-the-whole-path.md`: unit tests over `FakeHerdr` cannot
prove the real daemon accepts our parameter shapes.

## Smaller-path check

**Is there a cheaper shape that still honors D1?** Rejected: keep `send_input` and add a
post-Enter composer-text comparison with retries. More code than `agent.prompt`, guesses at a
composer's rendering, and needs its own new run status to report a wedge. Rejected: drop the
transcript capture — it removes the data-model flag but silently pays D1's named cost instead
of covering it, and P2-4 shows the close needs a stored answer anyway.

## What this plan does not do

- No reuse-before-spawn, no idle-pane TTL sweep: D1 chose close-on-done over both.
- **The spawn path's own blind Enter stays.** `herdr/mod.rs:451-453` sends the env-export line
  with the same `send_input(.., true)` mechanism. `agent.prompt` cannot apply — at that moment
  the pane is a shell, not a registered agent. A swallowed export would silently drop the
  agent's env. Named here as a real follow-up, not fixed in this feature.
- No terminal-status short-circuit beyond what `dsr-3` needs for the close.
- No cleanup of the six merged-but-uncleaned worktrees `bee worktree list` reports — same
  theme, different work.
- **No pane cleanup by hand.** Rev 1 proposed closing beehive's `w1:pD` and `w1:pE`; `w1:pD`
  is already gone and `w1:pE` is the user's currently-focused pane with no agent on it.
