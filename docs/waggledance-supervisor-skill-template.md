---
name: waggledance-supervisor
description: Relay a spec from the human into a target project's own backlog as a proposed PBI, via a lead agent opened through waggledance's dispatch door. Use when the user asks to hand off, drop, or route a spec/request into another repo for that repo's own team to triage — never to drive that repo's work directly.
---

# waggledance-supervisor

The cockpit-supervisor seat: a human hands this skill a spec, it opens one lead agent
in the target repo carrying that spec through waggledance's existing dispatch door, and
the run is visible afterwards. The lead runs its own working flow — the seat never
routes for it, never triages on its behalf, and never merges. Everything mechanical this
skill uses already exists in waggledance: `waggledance_ask_state`, `waggledance_dispatch`,
`waggledance_await`, `waggledance_runs`. This skill adds no new tool and no control loop.

## Input

- The **target project** — the repo the spec is meant to land in.
- The **spec** — the request text to hand off, in the human's own words or a pasted
  document. If either is missing, ask.

## Procedure

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

## Refusals

These are the shapes an operator or a lead actually meets running this skill. Recognize
each one for what it is — most are not failures.

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
  `waggledance_ask_state` check is for. Seeing a live lead means: do not dispatch a
  second one. Report the existing run to the human instead of spawning a competing one.

- **Duplicate PBI id.** When the lead runs `bee backlog pbi add --id <corr-id>`, a
  duplicate `--id` **refuses** rather than overwriting — `--id` is a migration-only
  override and first-add wins. **This means the drop already landed under that
  correlation id, not that anything failed.** On a re-send after a timeout, this is the
  expected and correct outcome: do not re-mint a new correlation id and try again: check
  `waggledance_runs` / the target's backlog for the existing PBI instead of treating the
  refusal as an error to route around.

## Reporting

Tell the human, in one short summary: the correlation id, the `run_id`, and once the
lead reports back — where it wrote the spec and the PBI id it registered (which is the
same as the correlation id). Then stop. There is no next step for this skill to take;
the target repo's own triage takes it from here.
