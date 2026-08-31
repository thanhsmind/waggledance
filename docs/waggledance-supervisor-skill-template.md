---
name: waggledance-supervisor
description: Hand a spec into a target project's own team via a lead agent opened through waggledance's dispatch door — relay mode files it as a proposed PBI and stops; execute mode has the lead run its own full chain to a finished result. Use whenever a request routes work into another repo, whether to file it (hand off, drop, route) or to get it fully built/done there (làm hoàn chỉnh, do it completely, build it, finish it) — the skill states which mode first, always. Never drives that repo's work directly from this session in either mode.
---

# waggledance-supervisor

The cockpit-supervisor seat: a human hands this skill a spec, the mode gate below picks
relay or execute, and the seat opens one lead agent in the target repo carrying that spec
through waggledance's existing dispatch door — the run is visible afterwards. In relay
mode the lead files the spec and stops; in execute mode the lead runs its own full
working chain through to a finished result. Either way the lead runs its own working
flow — the seat never routes for it, never triages on its behalf, and never merges.
Everything mechanical this skill uses already exists in waggledance:
`waggledance_ask_state`, `waggledance_dispatch`, `waggledance_await`, `waggledance_runs`.
This skill adds no new tool and no control loop.

## Mode gate — decide before anything else

State explicitly, before dispatching anything, which mode this run is in and why.

- **Relay mode** — file the spec as a proposed PBI in the target repo's own backlog,
  then stop. The target repo's own triage decides what happens to it next, on its own
  schedule. This is the default: cheap, safe, and reversible.

- **Execute mode** — dispatch a lead into the target repo and have it run its own full
  working chain (explore → gate → plan → execute → scribe) through to a finished,
  proven result, in its own feature worktree, merging per that repo's own `uat_stop`
  configuration. This seat only dispatches and monitors that run — it never merges on
  the target repo's behalf and never approves that repo's gates itself, exactly as in
  relay mode.

**Phrases that push toward execute mode** — read intent, this list is not exhaustive:
"làm hoàn chỉnh", "thực hiện hoàn chỉnh", "làm tới nơi", "làm cho xong", "do it
completely", "build it", "implement it", "finish it", "get it done", "ship it".

**The governing principle:** when a single request mixes a completion verb ("làm hoàn
chỉnh", "build", "implement", "finish") with a routing verb ("gửi sang", "route to",
"hand off to", "drop into"), the completion verb decides the SCOPE — finished, proven
work, not just a filed spec — and the routing verb only decides WHO does the work: the
target repo's own lead, dispatched through this door, rather than this session. A
routing phrase's keyword match is not by itself grounds for relay mode when a completion
verb is also present in the same request — that silent match is exactly the trap this
gate exists to catch. A routing phrase with no completion verb anywhere in the request
means relay mode.

If the request is ambiguous — no completion verb, and no clear "just file this" language
either — ask the human which mode, rather than guessing.

## Input

- The **target project** — the repo the spec is meant to land in.
- The **spec** — the request text to hand off, in the human's own words or a pasted
  document. If either is missing, ask.

## Relay mode procedure

1. **Mint the correlation id.** Before dispatching anything, generate one short id that
   will name both this run and the PBI it creates in the target repo (e.g.
   `sup-<yyyymmdd>-<4-char hex>`). This id is what makes a re-send after a timeout safe
   — see the duplicate-PBI refusal below.

2. **Check for a live lead first.** Call `waggledance_ask_state` for the target project
   before dispatching. Reuse-before-spawn is *this skill's* policy, not something
   waggledance enforces — if the fleet read shows an agent already actively working that
   repo, do not spawn a second one. Tell the human a lead is already live there and wait,
   rather than dispatching a competing run.

3. **Dispatch through the door.** Call `waggledance_dispatch` with `{project, task,
   preset}` — name a **preset label** only, never raw argv, an environment variable, or a
   working directory; those are not accepted inputs and never appear in a `task` string
   either. The `task` has exactly three parts, in this order:

   - **Line 1 — the provenance line, verbatim format:**

     ```
     spec-drop <corr-id> from waggledance@<sha>
     ```

     `<corr-id>` is the id minted in step 1. `<sha>` is waggledance's own HEAD short sha
     at send time (not the target's). This line is what lets the target repo record
     where the drop came from and lets a duplicate re-send be recognized as the same
     drop rather than a new one.

   - **The spec body** — the request text itself, unmodified.

   - **The file-and-stop contract**, spelled out explicitly in the task so the lead
     cannot read it as an invitation to keep going:

     > Write this spec into this repo's own docs tree, in this repo's own spec-drop
     > format. Register it with `bee backlog pbi add --id <corr-id> --status proposed`.
     > Report what you wrote and where. Then **stop** — do not triage it, do not lock a
     > CONTEXT.md, do not start routing or planning work from it, and do not merge
     > anything. Filing the spec and stopping *is* the task; the receiving repo's own
     > triage decides what happens to it next, on its own schedule.

   `waggledance_dispatch` returns `{run_id, warnings}` — keep the `run_id`.

4. **Track the run.** Report the `run_id` to the human immediately. Check progress with
   `waggledance_runs` (the run list for the project) or `waggledance_await` (poll one run,
   up to 60s per call) — poll `waggledance_await` in a loop if a longer wait is needed,
   rather than blocking on a single call.

5. **Never merge, and never ask for a merge.** Merging to the target repo's main branch
   is the human's gesture, made in that repo, on that repo's own terms. This skill has no
   merge tool and reports the drop as complete once the lead has filed the spec and
   registered the PBI — it does not wait for or request any merge, ever.

## Execute mode procedure

1. **Mint the correlation id.** Same as relay mode, step 1 — one short id (e.g.
   `sup-<yyyymmdd>-<4-char hex>`) naming this run, minted before dispatching anything.

2. **Check for a live lead first.** Same as relay mode, step 2 — reuse-before-spawn
   applies to execute mode exactly as it does to relay mode. A live lead already working
   that repo means: report it and wait, never dispatch a competing one.

3. **Dispatch through the door.** Call `waggledance_dispatch` with `{project, task,
   preset}` — a **preset label** only, never raw argv, an environment variable, or a
   working directory. The `task` again has three parts, in this order:

   - **Line 1 — the same provenance line format** as relay mode
     (`spec-drop <corr-id> from waggledance@<sha>`), so the drop is traceable and a
     re-send after a timeout is recognizable the same way.

   - **The spec body** — the request text itself, unmodified.

   - **The execute-to-completion contract**, spelled out explicitly so the lead runs its
     own full chain rather than stopping at a filed spec:

     > Treat this spec as your own feature work in this repo. Run this repo's full
     > working chain end to end — explore, gate, plan, execute, scribe — starting in
     > this repo's own feature worktree from the outset. Prove each step the way this
     > repo's own proof rules require; a claim of "done" needs this repo's own fresh
     > command output beside it. Merge only through this repo's own `uat_stop`
     > configuration and this repo's own gate approvals — being dispatched from
     > elsewhere is never grounds to merge or self-approve a gate here. Report progress
     > as the chain moves, and report the finished result — or the blocker — when done.

   `waggledance_dispatch` returns `{run_id, warnings}` — keep the `run_id`.

4. **Track the run.** Report the `run_id` to the human immediately. This is a
   longer-running chain than relay mode's file-and-stop — expect multiple checkpoints,
   not one. Check progress with `waggledance_runs` or `waggledance_await` (poll one run,
   up to 60s per call), polling in a loop rather than blocking on a single call, and
   relay the lead's own progress reports as they arrive.

5. **Never merge, and never approve that repo's gates.** Even though execute mode asks
   the lead to carry the work all the way to a finished, merged result, the merge itself
   and every gate approval along the way remain that repo's own gesture, on that repo's
   own terms — dispatching the lead from here is not standing authorization to make
   either call on its behalf. This seat dispatches and monitors; it does not merge, and
   it does not answer that repo's gate questions.

## Refusals

These are the shapes an operator or a lead actually meets running this skill, in either
mode unless noted. Recognize each one for what it is — most are not failures.

- **Project not opted in.** `waggledance_dispatch` refuses with:

  > project `<id>` has not opted into orchestrator dispatch — enable it from the
  > project's settings page

  This is not a bug and not retryable from here — the target project's owner must flip
  its orchestration opt-in first. Surface it to the human; do not work around it.

- **Preset label does not resolve.** The dispatch call refuses, naming the label that
  was searched for and the project whose preset registry (global, then that project's
  own `.bee/config.json`) came up empty. Fix by asking the human for a preset label that
  is actually registered for that target — never by substituting raw argv, an env var,
  or a pane path in its place.

- **Destination unresolved.** The dispatch call refuses with `destination unresolved`
  when the target project has no in-boundary pane to carry the run. This means the
  target repo's workspace needs a focused pane before a lead can open there — it is not
  something this skill can route around by picking a different project.

- **A lead is already working that repo.** Not an MCP error — this is what step 2's
  `waggledance_ask_state` check is for, in both modes. Seeing a live lead means: do not
  dispatch a second one. Report the existing run to the human instead of spawning a
  competing one.

- **Duplicate PBI id (relay mode).** When the lead runs `bee backlog pbi add --id
  <corr-id>`, a duplicate `--id` **refuses** rather than overwriting — `--id` is a
  migration-only override and first-add wins. **This means the drop already landed under
  that correlation id, not that anything failed.** On a re-send after a timeout, this is
  the expected and correct outcome: do not re-mint a new correlation id and try again:
  check `waggledance_runs` / the target's backlog for the existing PBI instead of
  treating the refusal as an error to route around. Execute mode's lead is not required
  to register a PBI at all — its own chain may create backlog/decision state as a normal
  part of shaping and planning, on its own repo's terms.

## Reporting

**Relay mode:** tell the human, in one short summary, the correlation id, the `run_id`,
and once the lead reports back — where it wrote the spec and the PBI id it registered
(which is the same as the correlation id). Then stop. There is no next step for this
skill to take; the target repo's own triage takes it from here.

**Execute mode:** tell the human the correlation id and `run_id` immediately, then relay
the lead's own checkpoints as they arrive, and its finished result (or blocker) when the
chain ends. Then stop. There is no next step for this skill to take beyond monitoring and
reporting; the target repo's own gates and merge decide what happens next.
