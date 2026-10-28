# wd-briefs — Context

**Feature slug:** wd-briefs
**Date:** 2026-10-28
**Scope:** Docs lane
**Spec:** spec drop sup-20261028-b5e1 from agent-works; format locked in
agent-works `docs/history/wd-briefs/CONTEXT.md` D2 (owner decisions f49ed91a,
e8bef507 in the agent-works store).

## Feature Boundary

Write one owner-facing brief per open work item of this repo under
`docs/briefs/<id>.md`: every PBI whose status is proposed, ready or in-flight,
and every active feature workflow. The agent-works mirror reads these files and
turns them into decision-ready tasks on work.gogl.be. The feature ends at the
files: no code, no change to any PBI or workflow status, no WaggleDance
endpoint.

## Locked Decisions

| ID | Decision |
|----|----------|
| D1 | One file per item, `docs/briefs/<id>.md`; `<id>` is the PBI id or the feature slug. |
| D2 | Flat frontmatter between `---` lines, `key: value` only: `title` (≤ 80 chars, Vietnamese, states the outcome), `audience` (`owner`\|`internal`), `size` (`S`\|`M`\|`L`), `ask` (`build`\|`park`\|`drop`\|`answer`\|`test`). |
| D3 | Body: six paragraphs in Vietnamese, each opening with its bold label in this order: `**Là gì:**`, `**Vì sao:**`, `**Nguồn gốc:**`, `**Chi phí và rủi ro:**`, `**Anh cần làm:**`, `**Chuẩn xong:**` (2–4 bullets). 120–220 words. No ids, paths, function or file names in the body. |
| D4 | Content is assembled from the linked sources — request file, the owning feature's or the originating feature's CONTEXT and plan, review findings, decisions, cell records, git history — never from the one-line `cos` alone. |
| D5 | Classification: tooling, bee-internal and tech debt → `audience: internal`; stale or superseded → `ask: drop` with the reason; work already merged into main whose workflow was never closed → `ask: test` (the owner checks it, or for internal items agrees to close it); built and awaiting acceptance → `ask: test`; a proposed item worth doing → `build` or `park` with a recommendation; an open question that blocks → `answer`. |
| D6 | A PBI already bound to an active feature still gets its own brief (the mirror decides folding); the brief says it belongs to that feature's work. |
| D7 | `wd-briefs` itself gets no brief — it is the tool writing them, closed by this run. |
| D8 | When a PBI id equals an active feature slug (`board-run-reaper`), one file covers both: the feature brief, which names the request it came from. 99 items → 98 files. |

## Sources

The item list with phase, merge state, cell counts and history files is
`docs/history/wd-briefs/inventory.json`. History dirs absent in this worktree
may exist untracked in the main checkout or in a feature's own worktree.
