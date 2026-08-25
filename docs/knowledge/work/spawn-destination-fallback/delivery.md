---
type: bee.delivery
title: spawn-destination-fallback — delivery
description: "Delivery record for work item spawn-destination-fallback: the spawn destination no longer depends on which pane a human has focused — a project owning any in-boundary pane is a valid place to start an agent."
timestamp: 2026-08-25
bee:
  id: spawn-destination-fallback-delivery
  lifecycle: active
  areas: [agent-terminal, orchestration]
  required_context: [docs/history/spawn-destination-fallback/CONTEXT.md]
  sources: [docs/history/spawn-destination-fallback/CONTEXT.md, docs/knowledge/work/dispatch-project-presets/delivery.md, docs/knowledge/work/ask-state-fleet-read/delivery.md]
---

# spawn-destination-fallback — Delivery

## What shipped

`resolve_spawn_destination` asked each workspace whether its **anchor** folder was
inside the project — and that anchor is workspace → active tab → that tab's layout →
its **currently focused pane**. So the question it really asked was *"is the cursor
inside this project right now?"*, and a project was dispatchable or not depending on
where someone happened to be looking.

Live case: beehive has two agent panes whose folders resolve under its own root, listed
for that project by `ask_state`, while the workspace *labelled* `beehive` holds
waggledance's panes. No anchor landed inside beehive, so a fully resolved preset label
still refused with `destination unresolved`.

A second pass now runs when the first finds nothing: the first workspace holding **any**
pane whose folder validates against the boundary, using that pane's own folder. The
anchor pass stays first and untouched.

## Locked decisions

| ID | Decision |
|----|----------|
| spawn-destination-fallback D1 | The existing anchor rule is tried first and unmodified; a project that resolves today keeps that exact destination. Additive by construction. |
| spawn-destination-fallback D2 | The fallback picks the first workspace holding any in-boundary pane, and that pane's resolved folder. Snapshot order is the order — no ranking invented. |
| spawn-destination-fallback D3 | Both passes validate through the same `Boundary`; the refusal and its fail-closed meaning are unchanged for a project that owns no in-boundary pane. |
| spawn-destination-fallback D4 | A pane's folder resolves as `cwd` then `foreground_cwd` — the same two steps `project_panes` takes, so a pane this resolver spawns beside is exactly a pane `ask_state` lists. |

## Verification

`cargo test -p waggledance` (889 after merge) and `-p waggledance-core` (438), green;
`cargo fmt --check` clean; clippy unchanged. The two pre-existing destination cases pass
**unedited**, which is what proves the anchor path did not move.

**This record's proof is not on a cell.** The merge recorded `verify: unchecked (no
capped cells)`, and the user accepted that knowingly rather than wait. Cell `sdf-1` was
dropped with its reason: a cross-worktree hold on `orchestrate.rs`, created by my own
mis-sequenced reservation from the main checkout, refreshed its own one-hour expiry on
ordinary bee calls and so never lapsed — the cell could not be claimed, therefore not
capped. The test output above is real; it simply lives here instead of on a cell trace.

## Learned

**A watcher can keep alive the thing it waits for.** The background loop I started to
wait out that hold called `bee reservations sweep` every minute, and something on that
path re-mirrored the hold — pushing its expiry forward faster than it could lapse. The
loop would have run forever. Waiting on a lock is only safe when the wait itself touches
nothing that can refresh it.

**A generated view can be the only copy.** Adding a PBI re-renders `docs/backlog.md`,
and that render deleted five hand-written `proposed` rows plus the whole
`## Done / Declined` section — rows that existed **only** in that file and never in
`.bee/backlog.jsonl`. One of the deleted rows, `p-732028ad`, was the item warning that
this exact render is lossy. Recovered from `HEAD` by the user's own hand; both agent
sessions were correctly guard-blocked from doing it (CLI-owned file, and a tree-wide
`git checkout` with siblings live).

## Open gaps

- **The spawn path is still dead against the installed herdr.** With the destination
  resolved, `agent.start` now fails: `missing field 'kind'`. waggledance pins
  `HERDR_PROTOCOL = 16`; the installed herdr speaks **protocol 20**, where
  `agent.start` requires `{name, kind, pane_id}` and `tab.create`'s response no longer
  carries `root_pane`. Read paths are unaffected, which is why nothing looked broken —
  but this also means the board's **Start / Run review / Run compound** cannot spawn a
  fresh pane either. Successor feature: `herdr-protocol-20`.
- Filing that as a PBI is itself blocked until the `docs/backlog.md` render is fixed —
  adding one would delete the just-restored rows again.
