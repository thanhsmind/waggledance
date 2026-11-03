---
type: bee.delivery
title: board-entry-conditions — delivery
description: "Delivery record for work item board-entry-conditions: the board resolves a project's default agent through the whole entry shape, so one herding.agents declaration no longer behaves two ways depending on which button started it."
timestamp: 2026-08-25
bee:
  id: board-entry-conditions-delivery
  lifecycle: active
  areas: [agent-terminal, bee-cockpit, orchestration]
  required_context: [docs/history/board-entry-conditions/CONTEXT.md]
  sources: [docs/history/board-entry-conditions/CONTEXT.md, docs/knowledge/work/herding-entry-conditions/delivery.md, docs/knowledge/work/board-run-actions/delivery.md]
---

# board-entry-conditions — Delivery

## What shipped

`herding-entry-conditions` taught the dispatch path to honour a project's whole
`herding.agents` entry. The board did not follow: **Start / Run review / Run compound**
resolved through `herding_agent_argv` and dropped everything around the command. One
declaration, two spawners, two behaviours — an agent started from a card ran with its
`env` unset and then waited at a trust prompt nobody had asked it to wait at.

`herding_default_entry` is the entry-shaped sibling of `herding_agent_argv`: an inline
argv `agent_command` names no label and so declares nothing around itself; a named one
resolves through the reader that already existed and keeps whatever it declares. The
board's two call sites pass it into the shared spawn path, and the placeholder that
promised this change is gone.

## Locked decisions

| ID | Decision |
|----|----------|
| board-entry-conditions D1 | The board resolves through the full entry shape, applying conditions with the same code the dispatch path uses — not a second implementation. |
| board-entry-conditions D2 | An inline-argv `agent_command` declares no conditions and behaves exactly as before. |
| board-entry-conditions D3 | Nothing about the trust write is re-decided; `herding-entry-conditions` D3–D5 are inherited whole. |
| board-entry-conditions D4 | A board spawn trusts the **feature's own granted worktree** — a sibling outside the project root the MCP path validates against. Stated rather than assumed: it is the directory the agent actually runs in, and one bee created for this project. |

## Verification

`cargo test -p waggledance` (899) and `-p waggledance-core` (443) on main after the merge,
green. The existing `herding_agent_argv` cases pass **unedited**, which is what proves its
contract did not move — it has other callers and this change was not theirs.

**No live run, deliberately.** Exercising this means a board **Start**, which opens a fresh
worktree and starts a real agent for a card the user did not ask to start. The shared
spawn path it now reaches was proved live in `herding-entry-conditions`
(`run-48e951cf2a67257a`); what this adds on top is the resolver, and the unit cases cover
it. Recorded as a gap in the evidence rather than described as proven.

## Open gaps

- **A failed trust seeding has no route to a board reader.** `herding-entry-conditions` D9
  gave the dispatch answer a home for that warning; the board surfaces run outcomes on the
  card and has no equivalent field. So on the board, fail-open is currently also
  fail-quiet — the exact shape D9 exists to prevent, on the one surface it does not yet
  reach.
- bee's built-in entries are still not mirrored; `waggledance`, `jarvis`, `jarvis-mcp` and
  `memorypad` declare no `herding` block.
- No project has `orchestration.enabled` on. The capability works; nothing dispatches
  unattended.
