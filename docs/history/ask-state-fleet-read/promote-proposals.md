promote proposal for work item "ask-state-fleet-read" (docs/history/ask-state-fleet-read/CONTEXT.md) — 5 capped cell(s): asfr-1, asfr-2, asfr-3, asfr-4, asfr-5
anchor: history — docs/history/ask-state-fleet-read/CONTEXT.md
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/ask-state-fleet-read/delivery.md

---
type: bee.delivery
title: ask-state-fleet-read — delivery
description: "Delivery record proposed by bee knowledge promote for work item ask-state-fleet-read: 5 capped cell(s), 17 recorded deviation(s)."
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-delivery
  lifecycle: active
  required_context: [docs/history/ask-state-fleet-read/CONTEXT.md]
  sources: [docs/history/ask-state-fleet-read/CONTEXT.md, .bee/cells/asfr-1.json, .bee/cells/asfr-2.json, .bee/cells/asfr-3.json, .bee/cells/asfr-4.json, .bee/cells/asfr-5.json]
---

# ask-state-fleet-read — Delivery

## What shipped

- **asfr-1** — ask_state now names every agent kind a project offers, as labels only (2 file(s) changed)
- **asfr-2** — ask_state now shows an orchestrator the panes a project already has, so reuse is reachable instead of spawn-only (2 file(s) changed)
- **asfr-3** — An agent reading the tool list can now discover the two new fields and their three panes answers (1 file(s) changed)
- **asfr-4** — ask_state now actually returns panes against a live herdr instead of degrading to null on every call (1 file(s) changed)
- **asfr-5** — A published pane now carries bee's own state and feature, which is the half that answers whether it is really free to reuse (2 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **asfr-1** — `cargo test -p waggledance-core bee:: && cargo test -p waggledance mcp::`
- **asfr-2** — `cargo test -p waggledance mcp:: && cargo test -p waggledance server::`
- **asfr-3** — `cargo test -p waggledance --bin waggledance mcp::`
- **asfr-4** — `cargo test -p waggledance --bin waggledance mcp::`
- **asfr-5** — `cargo test -p waggledance --bin waggledance mcp::`

## Deviations

- **asfr-1** — Ran inline rather than through a dispatched execution worker: the user's standing no-subagents instruction for this session outranks the small-lane worker rule.
- **asfr-1** — The cell declared verify as cargo test -p waggledance mcp::, but the waggledance package has no lib target; the working command is cargo test -p waggledance --bin waggledance mcp::, and the cap was proved with the full package run instead.
- **asfr-1** — BeeHerdingRegistry.default is None for the inline-argv form of agent_command. That form names no label, and its tokens are exactly what D2 forbids publishing, so there is nothing publishable to put there; documented on the field rather than inventing a third state.
- **asfr-2** — Ran inline rather than through a dispatched execution worker: the user's standing no-subagents instruction for this session outranks the small-lane worker rule.
- **asfr-2** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **asfr-2** — The pane projection had to become a pure function taking a snapshot and a root, because Orchestration holds a concrete SocketHerdr rather than a trait object and so cannot be faked at the handler. D3 containment and the exact field set are proved there; the handler tests cover only the terminal-off and transport-unreachable branches.
- **asfr-2** — Commit also carries a formatting-only reflow of bee.rs left by the same rustfmt run over cell asfr-1 code.
- **asfr-3** — Cell added after the merged gate approved a two-cell shape: the gate's own D1 locks that the fields are published on this tool, and a field the tool never describes is not published, so this closes that shape rather than widening it.
- **asfr-3** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **asfr-3** — Cell added after the merged gate approved a two-cell shape. D1 locks that the fields are published on this tool, and a field the tool never describes is not discoverable, so this closes the approved shape rather than widening it.
- **asfr-4** — Fix-first cell opened against capped work: running the built binary against the real registry proved the whole path and showed asfr-2 delivered nothing in practice, which unit tests could not see because they hand the fleet read the orchestration slot they want.
- **asfr-4** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **asfr-4** — Fix-first cell against already-capped work: the defect was only visible by running the built binary end to end, never from the unit tests, which supply the orchestration slot themselves.
- **asfr-5** — Widened four server.rs items to pub(crate) (herdr_session_ids, project_bee_activity, apply_bee_activity, ProjectBeeActivity and its pane map) so the MCP path runs the board's join instead of a second copy of it.
- **asfr-5** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **asfr-5** — Widened four server.rs items to pub(crate) so the MCP path reuses the board own join rather than duplicating it.
- **asfr-5** — The pure pane test needed activity.cwd in its session fixture: the join drops any activity record with no cwd or one outside the boundary, which the first version of the fixture did not carry.

## Provenance

Proposed by `bee knowledge promote --work ask-state-fleet-read` from 5 capped cell trace(s) in `.bee/cells/` and the anchor `docs/history/ask-state-fleet-read/CONTEXT.md`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

None: the work item declares no bee.areas, so there is no area to sync (D19).

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell asfr-1 — save as docs/knowledge/patterns/ask-state-fleet-read-asfr-1-pitfall.md

---
type: bee.pattern
title: ask-state-fleet-read cell asfr-1 — pitfall candidate
description: "Pitfall candidate mined from cell asfr-1's capped trace: Ran inline rather than through a dispatched execution worker: the user's standing no-subagents instruction for this session outranks the small-lane worker rule."
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-asfr-1-pitfall
  lifecycle: draft
  sources: [.bee/cells/asfr-1.json]
  polarity: pitfall
---

# ask-state-fleet-read cell asfr-1 — pitfall candidate

## What the cell did

ask_state now names every agent kind a project offers, as labels only

## Recorded evidence (verbatim from .bee/cells/asfr-1.json)

- **deviation** — Ran inline rather than through a dispatched execution worker: the user's standing no-subagents instruction for this session outranks the small-lane worker rule.
- **deviation** — The cell declared verify as cargo test -p waggledance mcp::, but the waggledance package has no lib target; the working command is cargo test -p waggledance --bin waggledance mcp::, and the cap was proved with the full package run instead.
- **deviation** — BeeHerdingRegistry.default is None for the inline-argv form of agent_command. That form names no label, and its tokens are exactly what D2 forbids publishing, so there is nothing publishable to put there; documented on the field rather than inventing a third state.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell asfr-2 — save as docs/knowledge/patterns/ask-state-fleet-read-asfr-2-pitfall.md

---
type: bee.pattern
title: ask-state-fleet-read cell asfr-2 — pitfall candidate
description: "Pitfall candidate mined from cell asfr-2's capped trace: Ran inline rather than through a dispatched execution worker: the user's standing no-subagents instruction for this session outranks the small-lane worker rule."
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-asfr-2-pitfall
  lifecycle: draft
  sources: [.bee/cells/asfr-2.json]
  polarity: pitfall
---

# ask-state-fleet-read cell asfr-2 — pitfall candidate

## What the cell did

ask_state now shows an orchestrator the panes a project already has, so reuse is reachable instead of spawn-only

## Recorded evidence (verbatim from .bee/cells/asfr-2.json)

- **deviation** — Ran inline rather than through a dispatched execution worker: the user's standing no-subagents instruction for this session outranks the small-lane worker rule.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **deviation** — The pane projection had to become a pure function taking a snapshot and a root, because Orchestration holds a concrete SocketHerdr rather than a trait object and so cannot be faked at the handler. D3 containment and the exact field set are proved there; the handler tests cover only the terminal-off and transport-unreachable branches.
- **deviation** — Commit also carries a formatting-only reflow of bee.rs left by the same rustfmt run over cell asfr-1 code.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell asfr-3 — save as docs/knowledge/patterns/ask-state-fleet-read-asfr-3-pitfall.md

---
type: bee.pattern
title: ask-state-fleet-read cell asfr-3 — pitfall candidate
description: "Pitfall candidate mined from cell asfr-3's capped trace: Cell added after the merged gate approved a two-cell shape: the gate's own D1 locks that the fields are published on this tool, and a field the tool never desc…"
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-asfr-3-pitfall
  lifecycle: draft
  sources: [.bee/cells/asfr-3.json]
  polarity: pitfall
---

# ask-state-fleet-read cell asfr-3 — pitfall candidate

## What the cell did

An agent reading the tool list can now discover the two new fields and their three panes answers

## Recorded evidence (verbatim from .bee/cells/asfr-3.json)

- **deviation** — Cell added after the merged gate approved a two-cell shape: the gate's own D1 locks that the fields are published on this tool, and a field the tool never describes is not published, so this closes that shape rather than widening it.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **deviation** — Cell added after the merged gate approved a two-cell shape. D1 locks that the fields are published on this tool, and a field the tool never describes is not discoverable, so this closes the approved shape rather than widening it.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell asfr-4 — save as docs/knowledge/patterns/ask-state-fleet-read-asfr-4-pitfall.md

---
type: bee.pattern
title: ask-state-fleet-read cell asfr-4 — pitfall candidate
description: "Pitfall candidate mined from cell asfr-4's capped trace: Fix-first cell opened against capped work: running the built binary against the real registry proved the whole path and showed asfr-2 delivered nothing in prac…"
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-asfr-4-pitfall
  lifecycle: draft
  sources: [.bee/cells/asfr-4.json]
  polarity: pitfall
---

# ask-state-fleet-read cell asfr-4 — pitfall candidate

## What the cell did

ask_state now actually returns panes against a live herdr instead of degrading to null on every call

## Recorded evidence (verbatim from .bee/cells/asfr-4.json)

- **deviation** — Fix-first cell opened against capped work: running the built binary against the real registry proved the whole path and showed asfr-2 delivered nothing in practice, which unit tests could not see because they hand the fleet read the orchestration slot they want.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **deviation** — Fix-first cell against already-capped work: the defect was only visible by running the built binary end to end, never from the unit tests, which supply the orchestration slot themselves.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

from cell asfr-5 — save as docs/knowledge/patterns/ask-state-fleet-read-asfr-5-pitfall.md

---
type: bee.pattern
title: ask-state-fleet-read cell asfr-5 — pitfall candidate
description: "Pitfall candidate mined from cell asfr-5's capped trace: Widened four server.rs items to pub(crate) (herdr_session_ids, project_bee_activity, apply_bee_activity, ProjectBeeActivity and its pane map) so the MCP path r…"
timestamp: 2026-08-25
bee:
  id: ask-state-fleet-read-asfr-5-pitfall
  lifecycle: draft
  sources: [.bee/cells/asfr-5.json]
  polarity: pitfall
---

# ask-state-fleet-read cell asfr-5 — pitfall candidate

## What the cell did

A published pane now carries bee's own state and feature, which is the half that answers whether it is really free to reuse

## Recorded evidence (verbatim from .bee/cells/asfr-5.json)

- **deviation** — Widened four server.rs items to pub(crate) (herdr_session_ids, project_bee_activity, apply_bee_activity, ProjectBeeActivity and its pane map) so the MCP path runs the board's join instead of a second copy of it.
- **deviation** — Ran inline rather than through a dispatched execution worker: the user standing no-subagents instruction for this session outranks the small-lane worker rule.
- **deviation** — Widened four server.rs items to pub(crate) so the MCP path reuses the board own join rather than duplicating it.
- **deviation** — The pure pane test needed activity.cwd in its session fixture: the join drops any activity record with no cwd or one outside the boundary, which the first version of the fixture did not carry.

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 5 capped cell(s) mined, 1 delivery draft, 0 area bullet(s), 5 pattern candidate(s), 0 file(s) written.

---

<!-- bee:not-a-deferral: this section records a completed review with no outstanding action. -->

## Resolution — 2026-08-25, reviewed

Reviewed in a sweep of the unapplied-proposal backlog.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/ask-state-fleet-read/delivery.md`
  already exists as a curated record; the generated draft would replace it with a
  list of cell ids and raw deviations.
- **(b) Area updates** — nothing proposed by the generator.
- **(c) Pattern candidates** — none promoted, from five candidates:
  - *Checked and rejected on evidence.* One deviation says the declared verify
    `cargo test -p waggledance mcp::` could not work because the package has no lib
    target. Re-run on 2026-08-25: that command passes 42 tests. The claim does not
    hold today, so nothing generalizable was promoted from it.
  - *Already active.* The fix-first cell that found asfr-2 delivered nothing in
    practice — visible only by running the built binary, because the unit tests
    supply the orchestration slot themselves — is exactly
    `docs/knowledge/patterns/the-test-builds-the-collaborator-production-does-not.md`,
    and it also demonstrates `prove-the-whole-path`. Recorded here as a recurrence
    of both rather than promoted again.
  - *Not a pitfall.* "Ran inline rather than through a dispatched execution worker",
    on four cells, is a standing user instruction being honoured.
  - *Not a pitfall.* Making the pane projection a pure function, and widening four
    items to `pub(crate)` so the MCP path reuses the board's join instead of copying
    it, are design choices the delivery record already carries.

<!-- /bee:not-a-deferral -->
