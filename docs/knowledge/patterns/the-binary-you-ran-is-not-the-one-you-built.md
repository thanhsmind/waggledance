---
type: bee.pattern
title: The binary you ran is not the one you built
description: "Pitfall: every cheap way to identify a built artifact — its version string, its in-repo path, even cargo's own reported executable path — can name a stale binary, so a green proof run certifies code that was never executed."
timestamp: 2026-08-30
bee:
  id: the-binary-you-ran-is-not-the-one-you-built
  lifecycle: active
  areas: [orchestration, web-interface]
  sources: [docs/history/board-visibility/proof.md, docs/history/dispatch-submit-and-reclaim/proof.md, docs/history/board-visibility/promote-proposals.md]
  polarity: pitfall
  critical: true
  signature: the binary you ran is not the one you built
---

# The binary you ran is not the one you built

## The trap

A live proof run rebuilds, runs the thing, and reports green. The build succeeded,
the command answered, the output looks right — and the code under test never ran,
because the artifact that executed was not the artifact that was just compiled.

This trap is not one mistake. It is a family, and each member defeats the remedy
the previous one teaches. All three fired on the same host inside one day.

**Disguise 1 — the redirected target directory.** `CARGO_TARGET_DIR` points
somewhere other than the repo, so the familiar in-repo `target/<profile>/<bin>`
still exists, still runs, and still reports a plausible version. Four days stale
and byte-for-byte wrong:

```
$ ~/.cache/cargo-target/fast/waggledance --version   # fresh
waggledance 0.5.2
$ ./target/fast/waggledance --version                # four days old
waggledance 0.5.2
```

Same version string. The version is not evidence.

*The remedy it teaches:* resolve the path cargo actually wrote, from
`cargo build --message-format=json`'s `compiler-artifact.executable`.

**Disguise 2 — two source trees sharing one target directory.** A worktree and
its main checkout share `CARGO_TARGET_DIR`. Cargo uplifts both trees' binaries to
the same output path and reuses the same dependency slot, so building the
comparison tree reports `Finished in 0.11s` and leaves a binary byte-identical to
the patched one. `cmp` is silent; the shas match.

*This defeats disguise 1's remedy:* `--message-format=json` reported the **same
`executable` path for both trees**. Resolving the path correctly still handed back
the wrong artifact.

**Disguise 3 — two install paths with the same name.** The daemon runs
`waggledance` from `~/.local/bin`, which precedes `~/.cargo/bin` on `PATH`. A
`cargo install --path .` writes to `~/.cargo/bin`, reports *"Replacing …"*, and
changes nothing about what restarts. Worse, the shadowing copy had been installed
from the git remote at an old commit, so a full day of merged work was absent from
the running process while every install and restart reported success.

*This defeats both earlier remedies:* the freshly built artifact was correctly
resolved and correctly installed. It was simply not the file being executed.

**Disguise 4 — a concurrent session overwrites the shared target, and the
content check does not discriminate.** Disguise 2's shared `CARGO_TARGET_DIR` is
not only an A/B hazard. With several sessions live in the same repo, *any* of
them building from main between your build and your install replaces the
artifact at the path cargo just handed you — no comparison tree required, and
nothing in your own transcript looks wrong.

*This defeats "prove by content" too,* when the literal you grep for is not
unique to the new code. A change that MOVES an existing string rather than
adding one leaves the old binary matching your check:

```
# the change inlined this button into its parent format string
$ rg -a -c 'term-create__pane">New shell' new-build   # 1
$ rg -a -c 'term-create__pane">New shell' old-build   # 1  ← proves nothing
```

Both builds contain that literal; only the new one contains it *indented, joined
to the enclosing `<div>`*. The check that separates them has to be the one the
old artifact cannot satisfy:

```
$ rg -a -c '^  <button type="button" class="term-create__pane">New shell' old-build   # 0
```

The give-away that the check was hollow: the freshly restarted daemon still
served the old behaviour while every content check printed `1`.

## The tell

Any of these, and the artifact is unverified:

- A version string, a build timestamp, or a "Finished"/"Replacing" line offered as
  proof that new code is live.
- A comparison between two builds where nobody checked the two binaries differ.
- A restart reported from a command name rather than from the path that name
  resolves to.
- A proof run whose evidence never names an absolute path.

## What to do instead

- **Prove the artifact by its content, not its identity.** Grep the binary for a
  string only the new code contains — a new route, a new attribute, a new error
  message. `strings <path> | rg -c '<literal from this change>'` answers in one
  line.
- **Prove the check itself first: run it against the OLD artifact and require
  `0`.** A check that matches both builds is not a check, and a change that moves
  or re-indents an existing literal produces exactly that (disguise 4). If the
  change adds no new string, grep for the literal *in its new surroundings* —
  the enclosing tag, the leading indentation — and confirm the old build scores
  zero before you trust the new build scoring one.
- **Give every build its own `CARGO_TARGET_DIR`** — not only comparison trees.
  With sibling sessions live in the repo, the shared target is overwritten
  between your build and your install (disguise 4). Verify differing artifacts
  (`sha256sum`, `cmp`) before trusting any A/B result.
- **Resolve the running process, never the command name:** `readlink -f /proc/<pid>/exe`,
  or `which -a <name>` to see the whole shadowing order. A restart is proven by the
  new process pointing at the new file, not by the restart command's own output.
- **Close the loop on behaviour, not on the build.** After a restart, ask the
  running service for something only the new code can answer. A `404` where the
  new route should be is the fastest true answer available.

## Recurrence

- `dispatch-submit-and-reclaim` (2026-08-30) — disguise 1. The proof cell was
  written with an explicit warning about it and hit disguise 2 anyway.
- `board-visibility` (2026-08-30) — disguise 2, found live when a "rebuild HEAD"
  produced a binary identical to the patched one.
- Reloading the daemon for UAT (2026-08-30) — disguise 3. Caught only because the
  new route answered `404` when it should have answered `200`; every install and
  restart message had reported success.
- `home-terminal-new-shell` (2026-08-31) — disguise 4, both halves. A sibling
  session's build from main replaced the shared-target artifact between the build
  and the install, and the content check passed anyway because the change had
  moved an existing button literal instead of adding a new one. Caught only by
  closing the loop on behaviour: the restarted daemon's `/?tab=terminals` still
  omitted the create box that the new code always renders.

## Related

- `proof-run-in-the-wrong-checkout.md` is the sibling trap: that one is the wrong
  **source tree**, this one is the wrong **artifact**. A run can be wrong in
  either dimension independently.
