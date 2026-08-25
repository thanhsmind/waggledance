# A Spawn Destination That Does Not Depend On Where The Cursor Is — Context

**Feature slug:** spawn-destination-fallback
**Date:** 2026-08-25
**Shaping session:** complete
**Scope:** Quick
**Domain types:** RUN

## What was asked

PBI `p-a953e7c0`, raised from a live run: with beehive's orchestration opt-in on and a
label that resolved, dispatch still refused — *"no herdr workspace has a resolved
working directory under this project's own root; refusing to start an agent in an
arbitrary directory."* beehive has agent panes whose folders resolve under its root, and
`ask_state` lists them for that project, so the project plainly owns somewhere to start.

## What was found

`resolve_spawn_destination` (`orchestrate.rs:389`) asks each workspace for
`anchor_cwd_for_workspace`, and that anchor is: workspace → its **active tab** → that
tab's layout → its **currently focused pane** → that pane's folder
(`herdr/wire.rs:294-317`). So the question it actually asks is *"is the pane the user
happens to have focused right now inside this project?"* — and a project is dispatchable
or not depending on where the cursor sits. On this machine the workspace labelled
`beehive` holds panes whose folders resolve under **waggledance's** root (visible in
`/api/agents`: `pane_id w1:p3` and `w1:p5`, `workspace: "beehive"`,
`project_id: "waggledance"`), so no workspace anchor lands inside beehive at all.

The refusal itself is right and stays: it will not start a process in an arbitrary
directory. The fix is a better destination, never a looser check.

## What will be done

`resolve_spawn_destination` gains a second pass, tried only when the first finds nothing:

1. **Unchanged, first** — a workspace whose anchor folder resolves inside the boundary
   wins, exactly as today. Nothing that spawns today changes where it spawns.
2. **New, on a miss** — the first workspace holding **any** pane whose folder resolves
   inside the boundary, using that pane's resolved folder as the destination.

Additive by construction, the same discipline `dispatch-project-presets` D1 used: only
projects that refuse today can begin resolving. Fail-closed is untouched — every
candidate still passes `Boundary::validate_existing`, so the destination is always a
directory the project itself owns.

## Locked decisions

| ID | Decision |
|----|----------|
| D1 | The existing anchor rule is tried first and is not modified. A project that resolves a destination today keeps that exact destination. |
| D2 | The fallback picks the first workspace holding any in-boundary pane, and uses **that pane's** resolved folder — not the project root, and not an unvalidated path. Pane order is the snapshot's own; no ranking is invented. |
| D3 | Both passes validate through the same `Boundary`. The refusal, its wording, and its fail-closed meaning are unchanged for a project that genuinely owns no in-boundary pane. |
| D4 | A pane's folder resolves the same way `project_panes` already resolves one: `cwd`, falling back to `foreground_cwd`. Two readers of "where is this pane" would drift. |

## Out of scope

- Choosing a *better* pane than the first (least-busy, most-recent, matching a feature) —
  that is ranking, and nothing here needs it yet.
- The workspace-label-versus-project drift itself (`workspace: "beehive"` holding
  waggledance panes). That is herdr's naming, not waggledance's to correct.
- Anything after the destination resolves: preflight, marker, baseline and wait
  semantics are untouched.
