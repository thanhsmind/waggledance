---
type: bee.pattern
title: A private-index commit leaves the shared index lying
description: "Pitfall: the private-index route around the concurrent-worker git guard does not refresh the shared index, so the next worker's commit silently reverts the previous one — and the working tree, which every green is read from, still looks correct."
timestamp: 2026-08-31
bee:
  id: a-private-index-commit-leaves-the-shared-one-lying
  lifecycle: active
  areas: [orchestration, workflow-state]
  sources: [docs/knowledge/work/term-workspace-unify/delivery.md]
  polarity: pitfall
  critical: true
  signature: a private-index commit leaves the shared index lying
---

# A private-index commit leaves the shared index lying

## The trap

The concurrent-worker git guard refuses `git add` while siblings are live, and
offers a private index as the way to land one cell's work:

```
GIT_INDEX_FILE=<tmp> git read-tree HEAD
GIT_INDEX_FILE=<tmp> git update-index --add <paths>
GIT_INDEX_FILE=<tmp> git write-tree
git commit-tree <tree> -p HEAD -m "<msg>"
git update-ref HEAD <commit>
```

That lands the commit correctly. What it does not do is touch the **shared**
index, which still holds the pre-commit blob for that path. The next worker to
commit through its own `read-tree HEAD`-plus-`write-tree` is fine — but a worker
that commits through the *shared* index, or writes a tree built from it, ships
the stale blob and **reverts the earlier cell**, inside a commit whose subject
describes something else.

## Why every check says it is fine

The working tree is never wrong. Each worker edited its own file correctly, so:

- `git status` shows the path modified or clean depending on which index it
  compares against — it does not say "HEAD lost content".
- `git diff HEAD -- <path>` is **empty** for the worker that just committed, and
  empty again for the next one, because the tree matches whatever HEAD now says.
- Every test run is green, because tests read the working tree — which still has
  all three cells' work.

The revert is visible in exactly one place: `git log -- <path>`, where one commit
applies `+56/−74` and the next undoes it `+74/−56`.

## What it looked like here

Three cells edited `views.rs`, `app.css` and `app.js` concurrently. The CSS cell
committed through a private index. The `views.rs` cell then committed a tree
carrying the stale `app.css`, reverting the rename while leaving the new
`.term-work` markup in `views.rs`. HEAD held old CSS under new markup — a
guaranteed broken page — and nothing caught it: the tree was correct, so the
suite was green, and the daemon built from that tree served correctly too. The
fourth cell found it only because a fresh checkout of HEAD failed the CSS tests.

## The tell

- Any cell in the wave committed through `GIT_INDEX_FILE` / `commit-tree`.
- Two commits touching one file with mirror-image insert/delete counts.
- A green suite on a tree that was never checked out from HEAD.

## What to do instead

- **Read the greens off HEAD, not off the tree.** At the end of a concurrent
  wave, run the suite against a clean checkout of HEAD — `git stash`-free, via a
  temporary worktree at HEAD — before calling the wave done. A tree-only green
  certifies a state no one will ever check out.
- **`git log -- <path>` for every file the wave touched**, looking for mirrored
  churn. This is the only check that sees the revert.
- **Prefer path-scoped `git commit -- <paths>`**, which the guard permits and
  which leaves no stale entry, over the private-index route. Reach for the
  private index only when the guard refuses the path-scoped form (it cannot see a
  pathspec past a heredoc — pass `-F <file>` or `-m` instead of `<<EOF` and the
  scoped form usually goes through).
- **Refresh the shared index after a private-index commit**, path-scoped, so the
  next worker cannot inherit the stale blob.

## Related

- `deferring-a-commit-on-a-contended-file.md` — the same boundary crossed from
  the other side: there the change rides into a stranger's commit, here a
  stranger's commit throws the change away.
- `the-binary-you-ran-is-not-the-one-you-built.md` — the artifact-level sibling.
  Both are "the thing you verified is not the thing that ships".
