---
type: bee.delivery
title: composer-slash-suggest — delivery
description: "Delivery record proposed by bee knowledge promote for work item composer-slash-suggest: 3 capped cell(s), 7 recorded deviation(s)."
timestamp: 2026-08-31
bee:
  id: composer-slash-suggest-delivery
  lifecycle: active
  required_context: [docs/history/composer-slash-suggest/CONTEXT.md, docs/history/composer-slash-suggest/plan.md]
  sources: [docs/history/composer-slash-suggest/CONTEXT.md, docs/history/composer-slash-suggest/plan.md, .bee/cells/csl-1.json, .bee/cells/csl-2.json, .bee/cells/csl-3.json]
---

# composer-slash-suggest — Delivery

## What shipped

- **csl-1** — GET /p/:id/_slash serves the projects slash commands and skills merged over the user-level set; /_slash serves the user level alone (3 file(s) changed)
- **csl-2** — Slash suggestion menu wired into all three reply composers with styling and handshake tests (3 file(s) changed)
- **csl-3** — Pane composers derive /p/:id/_slash from their own data-term-base, so home-tab panes load their project's commands and skills (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **csl-1** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance slash`
- **csl-2** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance views`
- **csl-3** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance views`

## Deviations

- **csl-1** — Wrote the fixture tests and the implementation in one file write instead of a failing-stub round trip first — the test build for this crate is a minutes-long compile and the stub pass bought no signal the fixtures did not already pin — found a better route
- **csl-1** — Added two route-level tests in server.rs bee_route_tests on top of the module fixture tests — the must-haves are stated about GET /p/:id/_slash and /_slash, which a scanner-only test cannot prove wired — found a better route
- **csl-1** — Formatted only the new slash.rs with rustfmt instead of running cargo fmt over the package — cargo fmt --check already reports 41 pre-existing diffs across this crate, so a package format would have swept unrelated files into this commit — hit an unforeseen obstacle
- **csl-1** — Committed through a private temp index (read-tree, update-index, write-tree, commit-tree) rather than git add plus git commit — the concurrent-worker git guard refuses the shared index while a sibling worker is live, and named this as the fix — hit an unforeseen obstacle
- **csl-2** — Project-pane composers fall back to /_slash when the page has no data-project-id — the cell named /p/<id>/_slash unconditionally, but the home page Terminals tab reaches these same forms with projectId null and would have fetched /p/null/_slash — the plan was wrong about a fact
- **csl-2** — Proved the JS behavior with a throwaway node DOM stub in the scratchpad (not in the repo) — views.rs string assertions cannot show that Enter is actually swallowed or that the insert replaces the leading token — found a better route
- **csl-3** — ran inline on the orchestrator session instead of a dispatched worker — tiny UAT fix (one URL expression plus one assertion), the tiny-cell inline exception — found a better route

## Provenance

Proposed by `bee knowledge promote --work composer-slash-suggest` from 3 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/composer-slash-suggest/CONTEXT.md`, `docs/history/composer-slash-suggest/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.
