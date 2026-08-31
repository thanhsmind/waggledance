---
type: bee.pattern
title: "A second daemon proves the live path without taking the user's"
description: "Practice: waggledance refuses a second instance and the user's daemon owns 7700, so a live proof looks impossible — an isolated HOME and a free port give you a whole disposable installation to prove against."
timestamp: 2026-08-31
bee:
  id: a-second-daemon-proves-it-without-taking-the-users
  lifecycle: active
  areas: [agent-terminal, changes-diff-screen]
  sources: [.bee/cells/htns-1.json, .bee/cells/dfc-1.json]
  polarity: practice
  signature: "live proof skipped, daemon already running"
---

# A second daemon proves the live path without taking the user's

## The situation

A change is user-visible, so it owes one proof of the whole path
([[prove-the-whole-path]]). But `waggledance serve` refuses a second instance,
the user's own daemon is running on 7700 and must not be displaced, and the
installed binary is the old one. Two features in a row settled for a weaker
proof and said so on the cap: `home-terminal-new-shell` recorded "live-daemon
check not run … proof rests on the router-level HTTP tests plus the binary
content check".

## The practice

The refusal is about the port and the data directory, not about the binary.
Both are relocatable, so a whole second installation costs four commands:

1. Build and resolve the artifact cargo actually wrote, then confirm it by
   content, never by version string ([[the-binary-you-ran-is-not-the-one-you-built]]):
   `strings <path> | rg '<a literal only the change contains>'`.
2. Make a scratch repo with the state the screen needs — for a diff screen that
   means a real git repo with real working-tree changes, not an empty one.
3. `HOME=<scratch> <artifact> register <scratch-repo>`, then
   `HOME=<scratch> <artifact> serve --port <free> --host 127.0.0.1` in the
   background. `data_dir()` hangs off `HOME`, so the scratch daemon shares no
   registry, no index, no `daemon.lock` and no port with the user's — all four
   live under `$HOME/.waggledance`, and there is no `--data-dir` override, which
   is exactly why relocating `HOME` is the whole trick.
4. `curl` the actual route and assert on what the browser would receive.

Tear down by killing that one process; nothing the user owns was touched.

## The guard you will meet at step 3

**A worktree-isolated session refuses `HOME=` on a Bash command.** The write
guard cannot verify what a command does once its environment is rewritten, so
it declines rather than risk a write outside the worktree — and a feature
worktree is precisely where you will be standing when the proof is owed.

Two ways past it, both already used in this repo. Leave the worktree for the
proof (`ExitWorktree` with `keep`, run it from main, go back) — the commits are
already made, so nothing is at risk. Or set the variable **on the child
process** rather than on the shell command, which is `e2e_open.rs`'s own idiom
and what `board-visibility` fell back to when the guard refused.

## The cheaper alternative when only the HTML matters

If what you need is the rendered response and not a live process, the in-crate
`router()` harness gives you the same bytes with no daemon at all — and
`board-visibility` turned that into the better artifact: **four permanent route
tests instead of a throwaway probe**, so the proof and the regression guard are
the same code. Reach for the second daemon when the thing under test is the
daemon; reach for the router harness when it is the response.

## What it does and does not prove

It proves the whole server path: routing, rendering, and the exact markup
shipped to the browser. It does not prove behavior that only exists once
scripts run — for that, see [[a-js-only-behavior-can-still-be-proven]].

`diff-file-collapse` ran both halves and reported the gap plainly: the served
page carried the new controls with their ids and aria wiring, and the click was
still unproven because no browser was reachable. Naming the half you did not
prove is the point; a proof that quietly covers less than it claims is worse
than a small one that says so.

## Pointers

- Data directory resolution: `crates/waggledance-core/src/config.rs` (`data_dir`)
- Worked example, `diff-file-collapse`: isolated HOME under the session
  scratchpad, port 7793, a three-file scratch repo with a dirty tree
