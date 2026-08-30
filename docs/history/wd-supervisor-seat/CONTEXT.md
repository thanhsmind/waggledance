# WD Supervisor Seat — Context

**Feature slug:** wd-supervisor-seat
**Date:** 2026-08-30
**Shaping session:** complete (three scope questions answered by the owner; no fog remained)
**Scope:** Standard
**Domain types:** RUN | DOC
**Origin:** foreign-origin spec drop — PBI `p-ba554933`, provenance `from beehive@296e66c3`
(beehive decisions `b59e50c8`, touches `8fea3561`), owner-approved 2026-08-30.
Full request text: `docs/discovery/slp-human-up/wd-cockpit-request.md` in the beehive repo.

## Feature Boundary

waggledance gains a **cockpit-supervisor seat**: a human hands it a spec, it opens a
lead agent in a target repo carrying that spec through the existing dispatch door, and
the run is visible afterwards. The lead routes its own working flow; the seat never
routes for it and never merges.

The seat is a **shipped agent skill**, not daemon code. The waggledance daemon gains no
new authority, no new MCP tool, and no control loop — every mechanical step the seat
performs already exists (`waggledance_dispatch`, `waggledance_await`, `waggledance_runs`,
`waggledance_ask_state`).

It ends there. The three sibling items beehive routed to this repo — the widened
`ask_state` digest, the cockpit repository, and weekly reports — are each filed as their
own proposed PBI with the same provenance, and are outside this boundary.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
IDs are bee decision-log ids (search with `bee decisions search`).

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| 9b23f2ca | **D1 — Seat only.** This feature builds exactly the four accept-when criteria of `p-ba554933`. The widened `ask_state` digest, the cockpit repository, and weekly reports each become their own proposed PBI carrying the same provenance, outside this boundary. | The other three have no locked decisions in either repo; two of them need their own discovery first. |
| 10ed8bf4 | **D2 — The seat is a shipped agent skill.** A second skill template beside `docs/waggledance-skill-template.md`, installed by `waggledance doctor` exactly as the viewer skill already is. No daemon authority, no new MCP tool, no control loop. | `orchestrator-dispatch` D1 ("waggledance never decides what to dispatch — it only executes dispatches safely") and D2 (dispatch tool family fixed at three) both stay untouched. A supervisor loop or a fourth dispatch tool would have to supersede them. |
| 4d173a07 | **D3 — Opt-in on for all three projects.** `orchestration_enabled` is switched on for `beehive`, `waggledance` and `jarvis`. | `waggledance_dispatch` gates on the **target** project's flag, so `beehive` is the one the seat requires. The other two are the owner's explicit grant, given when asked. `orchestrator-dispatch` D6 is applied, not changed — the flag IS the consent. |
| 73b30272 | **D4 — Human-origin provenance.** The seat mints the correlation id before dispatch and sends `spec-drop <corr-id> from waggledance@<sha>` as the task's first line, `<sha>` being waggledance's HEAD short sha at send time. The same `<corr-id>` is the PBI id in the receiving repo. | beehive's convention has no human-origin spelling. `bee backlog pbi add --id` is first-add-wins, so a sender-minted id makes a re-send after a timeout idempotent instead of duplicating the drop. |

### Inherited constraints — cited, never reinterpreted

| Source | Constraint |
|---|---|
| `orchestrator-dispatch` D1 | waggledance implements the mechanical protocol only; the orchestrator is an external LLM agent. **waggledance never decides what to dispatch.** |
| `orchestrator-dispatch` D2 | The dispatch MCP family is exactly three tools: `dispatch`, `await`, and a run-state read. |
| `orchestrator-dispatch` D3 / `dispatch-project-presets` D2 | A caller names a **preset label** only — never argv, env or cwd, from either source. |
| `orchestrator-dispatch` D6 / `dispatch-project-presets` D5 | `terminal.enabled` plus the per-project `orchestration.enabled` are the only gate, default off. A project declaring an agent kind is not consent; the opt-in is. |
| `ask-state-fleet-read` D7/D8 | Seeing a pane grants nothing, and reuse-before-spawn is the calling agent's policy, never waggledance's. |
| beehive `b59e50c8` | Cockpit ownership sits on the waggledance side; the lead evaluates the spec and routes its own flow. beehive's local cockpit stays unbootstrapped. |
| beehive `8fea3561` | The waggledance supervisor's read-only rollup is the one cross-project awareness layer. |

### Agent's Discretion

Everything the decisions leave open is planning's choice: the skill's file name and
frontmatter wording, the exact shape of the correlation id, how the skill words its
refusals, which `doctor` assertions pin the new template, and whether the seat checks
`waggledance_ask_state` before dispatching. Constraint: reuse the existing doctor
install path and the existing three dispatch tools before adding anything.

## Acceptance Criteria

Carried verbatim from `p-ba554933`:

1. A human hands a spec to waggledance and a lead agent opens in the target repo
   (beehive) carrying it, via the dispatch door.
2. The spec lands in the target repo's backlog per its spec-drop convention — a
   proposed PBI, a provenance line, not dispatchable until triage locks `CONTEXT.md`.
3. The run is visible in `waggledance_runs`.
4. Merge to main stays human-only.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Seat | The cockpit-supervisor role: the procedure a human-facing agent follows to relay a spec into a target repo. A skill, not a process. |
| Lead | The agent the seat opens in the target repo. It runs that repo's own bee flow and owns every routing choice. |
| Spec drop | A foreign-origin spec entering a repo's backlog as a proposed PBI whose id is the sender's correlation id, with a `from <repo>@<commit>` provenance line. |
| Correlation id | The one string tying the waggledance run row to the receiving repo's PBI id. Minted by the seat before dispatch. |

## Existing Code Context

The mechanical path is already built and needs no change:

- `crates/waggledance/src/mcp.rs:886` — `handle_dispatch`: validates, checks
  `terminal.enabled`, checks `orchestration_allowed`, resolves the preset, dispatches.
- `crates/waggledance/src/mcp.rs:822` — `resolve_preset`: global
  `terminal.agent_presets` first, then the target project's own `.bee/config.json`
  `herding.agents`; refuses before any herdr call.
- `crates/waggledance/src/orchestrate.rs:293` — `dispatch_run`: preflight, baseline
  capture, marker mint, send, then inserts the `Run` row.
- `crates/waggledance/src/mcp.rs:1108` — `handle_runs`: read-only run list, 100/project.
- `crates/waggledance-core/src/engine.rs:381` — `orchestration_allowed`.
- `crates/waggledance/src/doctor.rs:652` — `SKILL_TEMPLATE`, the
  `include_str!` + install path a second template follows.
- `crates/waggledance/src/views.rs:9844` — the per-project Orchestration checkbox;
  `crates/waggledance/src/server.rs:1528` — `POST /api/projects/:id/orchestration`.

Target-side facts, verified 2026-08-30: beehive's four preset labels
(`claude-sonnet` default, `pi-opencode-free`, `pi-agy-flash-3.7`, `agy-flash`) all
resolve; its workspace has live panes; `gate_bypass` is `full`; its local herding
cockpit is not bootstrapped.

## Canonical References

- `docs/discovery/slp-human-up/wd-cockpit-request.md` (beehive) — the request text
- `docs/history/orchestrator-dispatch/CONTEXT.md` — D1–D8, the inherited constraints
- `docs/history/dispatch-project-presets/CONTEXT.md` — D1–D7, preset resolution
- `docs/knowledge/work/ask-state-fleet-read/delivery.md` — the fleet-read contract
- `skills/bee-shaping/SKILL.md:112` (beehive) — the spec-drop convention, verbatim
- `docs/history/slp-advisor-nudge/CONTEXT.md` (beehive) — where the sibling items came from

## Outstanding Questions

### Resolve Before Planning

None. The three scope questions were answered by the owner before this lock.
