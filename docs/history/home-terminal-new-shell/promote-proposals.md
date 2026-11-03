promote proposal for work item "home-terminal-new-shell" (docs/history/home-terminal-new-shell/plan.md) — 1 capped cell(s): htns-1
anchor: history — docs/history/home-terminal-new-shell/plan.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/home-terminal-new-shell/delivery.md

---
type: bee.delivery
title: home-terminal-new-shell — delivery
description: "Delivery record proposed by bee knowledge promote for work item home-terminal-new-shell: 1 capped cell(s), 4 recorded deviation(s)."
timestamp: 2026-08-31
bee:
  id: home-terminal-new-shell-delivery
  lifecycle: active
  areas: [agent-terminal]
  required_context: [docs/history/home-terminal-new-shell/plan.md]
  sources: [docs/history/home-terminal-new-shell/plan.md, .bee/cells/htns-1.json]
---

# home-terminal-new-shell — Delivery

## What shipped

- **htns-1** — Homepage Terminals tab offers New shell, matching the project terminal picker; plain_shell flag retired (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **htns-1** — `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast`

## Deviations

- **htns-1** — The plan named views.rs alone, but three tests pinning the old shape live in server.rs and views.rs test mod, one of them homepage-terminal-full D5's data-project-id pin that only passed because its fixture configured no presets — narrowed that assertion instead of deleting it, and reserved server.rs on discovery — the plan was wrong about a fact
- **htns-1** — Cell declared files: [views.rs] but the tests pinning the old shape live in server.rs too; server.rs was reserved on discovery and both files are named in the report.
- **htns-1** — No route record: bee state route --set refuses for a lane-unbound session, and this session cannot be bound — the harness worktree guard refuses any command string containing the word b-i-n-d. Writing --no-lane would have clobbered todo-column-collapse own triage in the default record. Route facts (class=feature, lane=tiny, flags=covered-contract-change, files=1) are recorded in docs/history/home-terminal-new-shell/plan.md and the cell instead.
- **htns-1** — Live-daemon check not run: waggledance serve refuses a second instance and the users daemon is running on 7700; I did not displace it. Proof rests on the router-level HTTP tests plus the binary content check.

## Provenance

Proposed by `bee knowledge promote --work home-terminal-new-shell` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/home-terminal-new-shell/plan.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "home-terminal-new-shell" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-31T12:15:06.730Z), the work item declares no bee.areas.

area agent-terminal:
  - [htns-1] Homepage Terminals tab offers New shell, matching the project terminal picker; plain_shell flag retired — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/htns-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell htns-1 — save as docs/knowledge/patterns/home-terminal-new-shell-htns-1-pitfall.md

---
type: bee.pattern
title: home-terminal-new-shell cell htns-1 — pitfall candidate
description: "Pitfall candidate mined from cell htns-1's capped trace: The plan named views.rs alone, but three tests pinning the old shape live in server.rs and views.rs test mod, one of them homepage-terminal-full D5's data-proj…"
timestamp: 2026-08-31
bee:
  id: home-terminal-new-shell-htns-1-pitfall
  lifecycle: draft
  areas: [agent-terminal]
  sources: [.bee/cells/htns-1.json]
  polarity: pitfall
---

# home-terminal-new-shell cell htns-1 — pitfall candidate

## What the cell did

Homepage Terminals tab offers New shell, matching the project terminal picker; plain_shell flag retired

## Recorded evidence (verbatim from .bee/cells/htns-1.json)

- **deviation** — The plan named views.rs alone, but three tests pinning the old shape live in server.rs and views.rs test mod, one of them homepage-terminal-full D5's data-project-id pin that only passed because its fixture configured no presets — narrowed that assertion instead of deleting it, and reserved server.rs on discovery — the plan was wrong about a fact
- **deviation** — Cell declared files: [views.rs] but the tests pinning the old shape live in server.rs too; server.rs was reserved on discovery and both files are named in the report.
- **deviation** — No route record: bee state route --set refuses for a lane-unbound session, and this session cannot be bound — the harness worktree guard refuses any command string containing the word b-i-n-d. Writing --no-lane would have clobbered todo-column-collapse own triage in the default record. Route facts (class=feature, lane=tiny, flags=covered-contract-change, files=1) are recorded in docs/history/home-terminal-new-shell/plan.md and the cell instead.
- **deviation** — Live-daemon check not run: waggledance serve refuses a second instance and the users daemon is running on 7700; I did not displace it. Proof rests on the router-level HTTP tests plus the binary content check.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 1 area bullet(s), 1 pattern candidate(s), 0 file(s) written.