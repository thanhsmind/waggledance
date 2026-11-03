---
type: bee.pattern
title: "CI has never run in this repository, so the local test run is the only net"
description: "Pitfall: AGENTS.md promises CI runs the declared command on every push, but this repo is a GitHub fork with Actions dormant — the workflow is active, permissions say enabled, and the run count across the repo's whole history is zero."
timestamp: 2026-08-31
bee:
  id: ci-has-never-run-in-this-fork
  lifecycle: active
  areas: [workflow-state]
  sources: [.github/workflows/ci.yml]
  polarity: pitfall
  signature: relying on CI as the net
---

# CI has never run in this repository, so the local test run is the only net

## The fact

Measured 2026-08-31 while pushing: `github.com/thanhsmind/waggledance` is a
**fork** (of `vantt/mdview`), and GitHub keeps Actions dormant on a fork until a
human clicks *Enable* in the Actions tab.

Every cheap check says otherwise. `.github/workflows/ci.yml` is `active`, it
triggers on push to main, and `actions/permissions` reports `enabled: true`.
Only the run list tells the truth: `/actions/runs` reports `total_count` 0
across the repository's entire history.

This is [[existence-is-not-evidence]] in its most expensive form — the workflow
file exists, the switch reads on, and nothing has ever executed.

## What it changes

AGENTS.md states that "CI runs the full declared command on every push, the one
deterministic net". **That does not hold here.** The local run of
`commands.test` is the only proof this project ever gets.

The consequence is a reordering, not a new rule: a cap's scoped-green proof is
the **last** line of defence, not the first. Scope a proof to what a reviewer
would need if nothing else ever ran it — because nothing else will. The
"CI catches the rest" reasoning that justifies a narrow scope has no backing in
this repo.

## The fix, if wanted

Open the repository's Actions tab once and enable workflows. Until someone
does, treat every "CI will catch it" as an unbacked claim, and re-measure with
the run count rather than the permissions flag.
