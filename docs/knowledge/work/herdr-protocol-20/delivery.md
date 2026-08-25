---
type: bee.delivery
title: herdr-protocol-20 — delivery
description: "Delivery record for work item herdr-protocol-20: waggledance's herdr write calls ported from protocol 16 to the 20 the installed server speaks, which is what finally made a real agent spawn work."
timestamp: 2026-08-25
bee:
  id: herdr-protocol-20-delivery
  lifecycle: active
  areas: [agent-terminal, orchestration]
  required_context: [docs/history/herdr-protocol-20/CONTEXT.md, docs/history/herdr-protocol-20/plan.md]
  sources: [docs/history/herdr-protocol-20/CONTEXT.md, docs/history/herdr-protocol-20/plan.md, docs/knowledge/work/spawn-destination-fallback/delivery.md, docs/knowledge/patterns/the-test-builds-the-collaborator-production-does-not.md]
---

# herdr-protocol-20 — Delivery

## What shipped

waggledance pinned `HERDR_PROTOCOL = 16`; the installed herdr (0.8.0) speaks **20**.
Five of the seven methods waggledance calls still validate as they stand — which is why
every read path looked healthy: the snapshot, the pane list, `ask_state`, the board, the
terminal view. The two that drifted are both on the write side, and both were **dead**:

| Call | was | protocol 20 |
|---|---|---|
| `agent.start` | `{name, argv, workspace_id, cwd, focus}` | `{name, kind, pane_id}` + `args` |
| `tab.create` response | `{tab, root_pane:{pane_id}}` | `{type, tab}` — `TabInfo` carries no pane id |

`agent.start` no longer creates anything; it attaches to a pane that already exists. So a
spawn is three steps: create the tab, find its pane by `tab_id` in a fresh snapshot, start
there. One shared helper (`herdr::start_agent_in_new_tab`) owns that hop for all three
callers — the board's spawn, the board's shell-create, and MCP dispatch.

`kind` is `argv[0]` and `args` the rest — the same split `bee herding wave` performs,
which is why every `herding.agents` entry leads with `claude` / `pi` / `agy`.

**This is what made the whole four-feature chain real.** Before it, every spawn path in
the product was dead against the installed herdr, the board's own **Start** button
included.

## Locked decisions

| ID | Decision |
|----|----------|
| herdr-protocol-20 D1 | `agent.start` sends `{name, kind, pane_id}` + `args`; `kind` = `argv[0]`, `args` = `argv[1..]`. |
| herdr-protocol-20 D2 | Spawning is `tab.create` → snapshot lookup by `tab_id` → `agent.start` into that pane. |
| herdr-protocol-20 D3 | A tab that yields no pane is a **typed failure** naming the orphaned tab; it starts nothing and never reaches for another pane. |
| herdr-protocol-20 D4 | `HERDR_PROTOCOL` → 20, justified by the seven-method audit and nothing else. |
| herdr-protocol-20 D5 | Read calls and the three send calls untouched. |
| herdr-protocol-20 D6 | No check relaxed: preflight, containment, opt-in, presets-only all stand. |
| herdr-protocol-20 D7 | `FakeHerdr` moves with the real client. |

## Behaviour that changed

- **A spawned agent lands in its own new tab**, rather than splitting into the
  workspace's active tab — `agent.start` cannot place itself any more. The terminal card
  now reads `workspace · Shell` instead of `workspace · main`.
- **A freshly created pane is offered again while its shell comes up.** herdr answers
  `agent_pane_busy: … is not an available shell` for a fraction of a second after the
  pane exists. It is re-offered six times, 200ms apart, and **only** on that code — a
  name collision, a dead socket or a refusal from the agent itself still surfaces on the
  first attempt. Giving up returns herdr's own last words.

## Verification

`cargo test -p waggledance` (892) and `-p waggledance-core` (438) on main after the
merge, green.

**And the live proof, which is the point:** a dispatch into beehive with
`pi-opencode-free` — a label only that project declares — started `pi` on
`opencode/x-preview-f-free:high` in `~/Projects/goglbe/beehive` and returned
`run-57f2ccff17effcb3`, status `working`. beehive's opt-in was turned on for that one
dispatch and off again immediately, with the refusal re-verified.

## Learned

**A drifted double is worse than an empty one** — see
`patterns/the-test-builds-the-collaborator-production-does-not`. `FakeHerdr` had been
speaking protocol 16 for four versions, so every green test about spawning was evidence
about the double, not about herdr. An empty slot at least returns null; a stale double
returns confident green. This is the second instance today: the first was
`ask-state-fleet-read`, where a lazily-built handle made `panes` null on every real call
while 875 unit tests passed.

**Read paths hide write drift.** Nothing looked broken because everything anyone looked
*at* still worked. A protocol audit should be per-method, and the cheapest one is
first-party: `herdr api schema --json` from the installed binary.

**Two tests were removed, not contorted.** Both pinned behaviour of a `cwd` parameter
protocol 20 deleted; their reasons are recorded where they stood, and the concern one of
them protected (never invent a placement) now lives in `HerdrError::TabPaneUnresolved`.

## Open gaps

- The four cells landed as **one commit** — a trait signature change leaves nothing in
  the crate compiling until every caller follows, so per-cell green was not achievable.
  Recorded on each cap rather than papered over.
- bee's object form of a `herding.agents` entry still does not resolve (PBI
  `p-9212cae8`), so `agy-flash` remains declared-but-unstartable.
- `waggledance`, `jarvis`, `jarvis-mcp` and `memorypad` declare no `herding` block.
- No project has `orchestration.enabled` on: the capability works, and nothing dispatches
  unattended today. That is by design and worth stating so the two are not confused.
