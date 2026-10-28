# wd-briefs — Plan

**Lane:** small, docs only. **Gate:** merged shape+execution, auto-approved under
`gate_bypass=full`.

## Shape

98 brief files under `docs/briefs/`, written by seven parallel docs workers,
one cell each, disjoint file sets (the item ids per cell are in each cell's
`files`). Grouping keeps related items together so a worker reads a shared
source once: PBIs in two cells, the October features in one, the late-August
features in two, and the 2026-08-25 cluster of agent-mark and board-polish
features in two.

| Cell | Items |
|------|-------|
| brf-1 | 14 PBIs |
| brf-2 | 14 PBIs (includes the in-flight mailbox request) |
| brf-3 | 16 features, October |
| brf-4 | 15 features, late August |
| brf-7 | 16 features, late August |
| brf-5 | 11 features, 2026-08-25 |
| brf-6 | 12 features, 2026-08-25 |

## Cells (current slice — one slice)

1. **brf-1 — briefs for 14 open PBIs.** One file per PBI id; role docs.
2. **brf-2 — briefs for 14 open PBIs,** including the in-flight mailbox request.
3. **brf-3 — briefs for the 16 October features.**
4. **brf-4 — briefs for 15 late-August features.**
5. **brf-7 — briefs for 16 late-August features,** including board-run-reaper (D8).
6. **brf-5 — briefs for 11 features from 2026-08-25.**
7. **brf-6 — briefs for 12 features from 2026-08-25,** the agent-mark cluster.


8. **brf-8 — reconcile the lint briefs** with clippy red on main today, found by the leader completeness check.

```json
[
 {
  "id": "brf-1",
  "feature": "wd-briefs",
  "title": "Write owner briefs for the first half of open PBIs",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/p-0f60abae.md",
   "docs/briefs/p-179a0d12.md",
   "docs/briefs/p-19c4995c.md",
   "docs/briefs/p-1f6d0ebf.md",
   "docs/briefs/p-2f69f60c.md",
   "docs/briefs/p-30ca3505.md",
   "docs/briefs/p-31d113c2.md",
   "docs/briefs/p-498dd298.md",
   "docs/briefs/p-6be4d51f.md",
   "docs/briefs/p-6cd25610.md",
   "docs/briefs/p-6fab30aa.md",
   "docs/briefs/p-7329a3c8.md",
   "docs/briefs/p-75c7b411.md",
   "docs/briefs/p-87c083c8.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- p-0f60abae (PBI, status proposed, bound feature paseo-native-supervisor): Several leaders per project when a task is marked independent\n- p-179a0d12 (PBI, status proposed, bound feature None): observer-tick-trigger: D4b can silently lose a transition when an MCP-awaited run reaches blocked first\n- p-19c4995c (PBI, status proposed, bound feature None): observer-tick-trigger: D9 can't close the loop through the D4d escalation mailbox\n- p-1f6d0ebf (PBI, status proposed, bound feature None): await reports a run done from screen stability, not from the agent saying so\n- p-2f69f60c (PBI, status proposed, bound feature None): CLAUDE.md still names target/fast/waggledance, a path that is stale on this host\n- p-30ca3505 (PBI, status proposed, bound feature None): wd-supervisor: the cockpit repository\n- p-31d113c2 (PBI, status proposed, bound feature None): Dispatched panes are never reclaimed — every run leaves a live agent behind\n- p-498dd298 (PBI, status proposed, bound feature None): doctor reorders every key in the user's ~/.claude.json\n- p-6be4d51f (PBI, status proposed, bound feature None): Attach images to a request sent to a Paseo leader\n- p-6cd25610 (PBI, status proposed, bound feature None): dispatch_run reports a healthy run when the task was pasted but never submitted\n- p-6fab30aa (PBI, status proposed, bound feature lint-gates-green): A contributor running the repo's own lint gates on a clean main sees them pass, so a red gate means their change broke something\n- p-7329a3c8 (PBI, status proposed, bound feature None): Dispatch fails with 'database is locked' right after the daemon starts\n- p-75c7b411 (PBI, status proposed, bound feature None): Stop or archive a project's Paseo leader from waggledance\n- p-87c083c8 (PBI, status proposed, bound feature None): A blocked notice reached the caller hours late, after its run was already done",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py p-0f60abae p-179a0d12 p-19c4995c p-1f6d0ebf p-2f69f60c p-30ca3505 p-31d113c2 p-498dd298 p-6be4d51f p-6cd25610 p-6fab30aa p-7329a3c8 p-75c7b411 p-87c083c8",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/p-0f60abae.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-179a0d12.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-19c4995c.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-1f6d0ebf.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-2f69f60c.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-30ca3505.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-31d113c2.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-498dd298.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-6be4d51f.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-6cd25610.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-6fab30aa.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-7329a3c8.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-75c7b411.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-87c083c8.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-2",
  "feature": "wd-briefs",
  "title": "Write owner briefs for the second half of open PBIs",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/p-9212cae8.md",
   "docs/briefs/p-950905ce.md",
   "docs/briefs/p-a541ffc2.md",
   "docs/briefs/p-c67e8eb5.md",
   "docs/briefs/p-cd0eadfc.md",
   "docs/briefs/p-da42b8f5.md",
   "docs/briefs/p-e19137ac.md",
   "docs/briefs/p-e5b770fe.md",
   "docs/briefs/p-e62c967c.md",
   "docs/briefs/p-ebd4ce69.md",
   "docs/briefs/p-f1b14b9b.md",
   "docs/briefs/p-f738af40.md",
   "docs/briefs/p-fee218b5.md",
   "docs/briefs/sup-20260830-9c4e.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- p-9212cae8 (PBI, status proposed, bound feature None): Read the whole herding.agents entry shape bee declares, and honour its spawn conditions\n- p-950905ce (PBI, status proposed, bound feature None): CI's clippy and rustfmt gates are both red on main\n- p-a541ffc2 (PBI, status proposed, bound feature None): Optional swarm manifest declaring which registered projects form one coordinated group\n- p-c67e8eb5 (PBI, status proposed, bound feature None): commands.test in waggledance's .bee/config.json points at beehive's manifest\n- p-cd0eadfc (PBI, status proposed, bound feature None): bee backlog add refuses every argument shape\n- p-da42b8f5 (PBI, status proposed, bound feature None): wd-supervisor: widen the ask_state digest for the supervisor seat\n- p-e19137ac (PBI, status proposed, bound feature None): wd-supervisor: weekly reports across registered projects\n- p-e5b770fe (PBI, status proposed, bound feature None): Parse install.ps1 in CI — no pwsh on the dev machine\n- p-e62c967c (PBI, status proposed, bound feature None): Prove the desktop crate builds — it never has\n- p-ebd4ce69 (PBI, status proposed, bound feature lint-gates-green): A contributor running the repo's own lint gates on a clean main sees them pass, so a red gate means their change broke something\n- p-f1b14b9b (PBI, status proposed, bound feature None): observer-tick-trigger: a genuinely independent trigger_dry_run (preview without arming trigger_enabled)\n- p-f738af40 (PBI, status proposed, bound feature None): A leader run is marked done at the end of the leader's first turn, while a long bee chain keeps working across later turns\n- p-fee218b5 (PBI, status proposed, bound feature None): The work record drifts: one session's 48 turns all sat under the first request's stub\n- sup-20260830-9c4e (PBI, status in-flight, bound feature mailbox-inbox): As the human, I want digests and unfinished-run status surfaced in the same mailbox inbox section as letters, so I don't miss run summaries or silently-failed r",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py p-9212cae8 p-950905ce p-a541ffc2 p-c67e8eb5 p-cd0eadfc p-da42b8f5 p-e19137ac p-e5b770fe p-e62c967c p-ebd4ce69 p-f1b14b9b p-f738af40 p-fee218b5 sup-20260830-9c4e",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/p-9212cae8.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-950905ce.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-a541ffc2.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-c67e8eb5.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-cd0eadfc.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-da42b8f5.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-e19137ac.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-e5b770fe.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-e62c967c.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-ebd4ce69.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-f1b14b9b.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-f738af40.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/p-fee218b5.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/sup-20260830-9c4e.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-3",
  "feature": "wd-briefs",
  "title": "Write owner briefs for the October features",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/usage-paseo-projects.md",
   "docs/briefs/push-and-prune-1023.md",
   "docs/briefs/paseo-dispatch-default-on.md",
   "docs/briefs/usage-dashboard.md",
   "docs/briefs/paseo-leader-mode.md",
   "docs/briefs/wd-direct-leader.md",
   "docs/briefs/supervisor-active-loop.md",
   "docs/briefs/leader-mail-envelope.md",
   "docs/briefs/openclaw-agent-target.md",
   "docs/briefs/paseo-native-supervisor.md",
   "docs/briefs/piggery-lessons.md",
   "docs/briefs/paseo-auto-wiring.md",
   "docs/briefs/leader-delivery.md",
   "docs/briefs/paseo-leader-dispatch.md",
   "docs/briefs/pi-paseo-plugin.md",
   "docs/briefs/sheets-task-sync.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- usage-paseo-projects (feature, phase idle, created 2026-10-23, merged into main: True, cells {})\n- push-and-prune-1023 (feature, phase idle, created 2026-10-23, merged into main: False, cells {})\n- paseo-dispatch-default-on (feature, phase idle, created 2026-10-23, merged into main: True, cells {})\n- usage-dashboard (feature, phase idle, created 2026-10-23, merged into main: True, cells {})\n- paseo-leader-mode (feature, phase compounding, created 2026-10-23, merged into main: True, cells {'capped': 2})\n- wd-direct-leader (feature, phase compounding, created 2026-10-22, merged into main: True, cells {'capped': 11})\n- supervisor-active-loop (feature, phase idle, created 2026-10-18, merged into main: True, cells {})\n- leader-mail-envelope (feature, phase idle, created 2026-10-18, merged into main: True, cells {})\n- openclaw-agent-target (feature, phase idle, created 2026-10-17, merged into main: True, cells {})\n- paseo-native-supervisor (feature, phase compounding, created 2026-10-17, merged into main: True, cells {'capped': 6})\n- piggery-lessons (feature, phase idle, created 2026-10-16, merged into main: True, cells {})\n- paseo-auto-wiring (feature, phase idle, created 2026-10-14, merged into main: True, cells {})\n- leader-delivery (feature, phase idle, created 2026-10-14, merged into main: True, cells {})\n- paseo-leader-dispatch (feature, phase idle, created 2026-10-14, merged into main: True, cells {})\n- pi-paseo-plugin (feature, phase idle, created 2026-10-06, merged into main: True, cells {})\n- sheets-task-sync (feature, phase compounding, created 2026-09-24, merged into main: True, cells {'capped': 4})",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py usage-paseo-projects push-and-prune-1023 paseo-dispatch-default-on usage-dashboard paseo-leader-mode wd-direct-leader supervisor-active-loop leader-mail-envelope openclaw-agent-target paseo-native-supervisor piggery-lessons paseo-auto-wiring leader-delivery paseo-leader-dispatch pi-paseo-plugin sheets-task-sync",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/usage-paseo-projects.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/push-and-prune-1023.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-dispatch-default-on.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/usage-dashboard.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-leader-mode.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/wd-direct-leader.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/supervisor-active-loop.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/leader-mail-envelope.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/openclaw-agent-target.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-native-supervisor.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/piggery-lessons.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-auto-wiring.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/leader-delivery.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-leader-dispatch.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/pi-paseo-plugin.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/sheets-task-sync.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-4",
  "feature": "wd-briefs",
  "title": "Write owner briefs for late-August features, part one",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/cargo-build-slim.md",
   "docs/briefs/ipad-dashboard-mobile.md",
   "docs/briefs/dispatched-pane-agent-visibility.md",
   "docs/briefs/waggledance-supervisor-mode-gate.md",
   "docs/briefs/diff-file-collapse.md",
   "docs/briefs/term-workspace-unify.md",
   "docs/briefs/trigger-settings-toggle.md",
   "docs/briefs/home-terminal-new-shell.md",
   "docs/briefs/observer-tick-trigger.md",
   "docs/briefs/lint-gates-green.md",
   "docs/briefs/paseo-removal.md",
   "docs/briefs/term-keys-nav-tint.md",
   "docs/briefs/pane-kick-refresh.md",
   "docs/briefs/slash-builtin-commands.md",
   "docs/briefs/term-keys-shift-tab.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- cargo-build-slim (feature, phase idle, created 2026-09-23, merged into main: True, cells {})\n- ipad-dashboard-mobile (feature, phase idle, created 2026-09-11, merged into main: True, cells {})\n- dispatched-pane-agent-visibility (feature, phase exploring, created 2026-08-31, merged into main: False, cells {})\n- waggledance-supervisor-mode-gate (feature, phase idle, created 2026-08-31, merged into main: True, cells {})\n- diff-file-collapse (feature, phase idle, created 2026-08-31, merged into main: True, cells {})\n- term-workspace-unify (feature, phase planning, created 2026-08-31, merged into main: True, cells {'capped': 4})\n- trigger-settings-toggle (feature, phase idle, created 2026-08-31, merged into main: True, cells {})\n- home-terminal-new-shell (feature, phase idle, created 2026-08-31, merged into main: True, cells {})\n- observer-tick-trigger (feature, phase idle, created 2026-08-31, merged into main: True, cells {})\n- lint-gates-green (feature, phase exploring, created 2026-08-31, merged into main: True, cells {'capped': 1})\n- paseo-removal (feature, phase exploring, created 2026-08-31, merged into main: True, cells {'capped': 1})\n- term-keys-nav-tint (feature, phase exploring, created 2026-08-31, merged into main: True, cells {'capped': 2})\n- pane-kick-refresh (feature, phase exploring, created 2026-08-31, merged into main: True, cells {'capped': 1})\n- slash-builtin-commands (feature, phase exploring, created 2026-08-31, merged into main: True, cells {'capped': 1})\n- term-keys-shift-tab (feature, phase exploring, created 2026-08-31, merged into main: True, cells {'capped': 1})",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py cargo-build-slim ipad-dashboard-mobile dispatched-pane-agent-visibility waggledance-supervisor-mode-gate diff-file-collapse term-workspace-unify trigger-settings-toggle home-terminal-new-shell observer-tick-trigger lint-gates-green paseo-removal term-keys-nav-tint pane-kick-refresh slash-builtin-commands term-keys-shift-tab",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/cargo-build-slim.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/ipad-dashboard-mobile.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/dispatched-pane-agent-visibility.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/waggledance-supervisor-mode-gate.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/diff-file-collapse.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/term-workspace-unify.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/trigger-settings-toggle.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/home-terminal-new-shell.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/observer-tick-trigger.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/lint-gates-green.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-removal.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/term-keys-nav-tint.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/pane-kick-refresh.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/slash-builtin-commands.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/term-keys-shift-tab.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-7",
  "feature": "wd-briefs",
  "title": "Write owner briefs for late-August features, part two",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/composer-slash-suggest.md",
   "docs/briefs/board-run-reaper.md",
   "docs/briefs/mailbox-inbox.md",
   "docs/briefs/commit-label-condense.md",
   "docs/briefs/push-main-sync.md",
   "docs/briefs/home-terminal-panel.md",
   "docs/briefs/changes-diff-screen.md",
   "docs/briefs/board-visibility.md",
   "docs/briefs/dispatch-submit-and-reclaim.md",
   "docs/briefs/wd-supervisor-seat.md",
   "docs/briefs/paseo-control.md",
   "docs/briefs/term-reply-composer.md",
   "docs/briefs/term-keys-grid.md",
   "docs/briefs/paseo-support.md",
   "docs/briefs/guide-wayfinding.md",
   "docs/briefs/guide-vi.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- composer-slash-suggest (feature, phase swarming, created 2026-08-31, merged into main: True, cells {'capped': 3})\n- board-run-reaper (feature, phase idle, created 2026-08-31, merged into main: True, cells {})\n- mailbox-inbox (feature, phase compounding, created 2026-08-30, merged into main: True, cells {'capped': 2})\n- commit-label-condense (feature, phase idle, created 2026-08-30, merged into main: False, cells {})\n- push-main-sync (feature, phase exploring, created 2026-08-30, merged into main: False, cells {})\n- home-terminal-panel (feature, phase idle, created 2026-08-30, merged into main: True, cells {})\n- changes-diff-screen (feature, phase idle, created 2026-08-30, merged into main: True, cells {})\n- board-visibility (feature, phase idle, created 2026-08-30, merged into main: True, cells {})\n- dispatch-submit-and-reclaim (feature, phase swarming, created 2026-08-30, merged into main: True, cells {'capped': 7})\n- wd-supervisor-seat (feature, phase swarming, created 2026-08-30, merged into main: True, cells {'capped': 3})\n- paseo-control (feature, phase swarming, created 2026-08-29, merged into main: True, cells {'capped': 6})\n- term-reply-composer (feature, phase idle, created 2026-08-29, merged into main: True, cells {})\n- term-keys-grid (feature, phase idle, created 2026-08-28, merged into main: True, cells {})\n- paseo-support (feature, phase swarming, created 2026-08-27, merged into main: True, cells {'capped': 3, 'dropped': 1})\n- guide-wayfinding (feature, phase swarming, created 2026-08-26, merged into main: True, cells {'capped': 1})\n- guide-vi (feature, phase swarming, created 2026-08-26, merged into main: True, cells {'capped': 6})",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py composer-slash-suggest board-run-reaper mailbox-inbox commit-label-condense push-main-sync home-terminal-panel changes-diff-screen board-visibility dispatch-submit-and-reclaim wd-supervisor-seat paseo-control term-reply-composer term-keys-grid paseo-support guide-wayfinding guide-vi",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/composer-slash-suggest.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/board-run-reaper.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/mailbox-inbox.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/commit-label-condense.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/push-main-sync.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/home-terminal-panel.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/changes-diff-screen.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/board-visibility.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/dispatch-submit-and-reclaim.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/wd-supervisor-seat.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-control.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/term-reply-composer.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/term-keys-grid.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/paseo-support.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/guide-wayfinding.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/guide-vi.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-5",
  "feature": "wd-briefs",
  "title": "Write owner briefs for the 2026-08-25 features, part one",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/board-entry-conditions.md",
   "docs/briefs/herding-entry-conditions.md",
   "docs/briefs/board-action-dupes.md",
   "docs/briefs/herdr-protocol-20.md",
   "docs/briefs/spawn-destination-fallback.md",
   "docs/briefs/board-live-morph.md",
   "docs/briefs/dispatch-project-presets.md",
   "docs/briefs/session-work-line.md",
   "docs/briefs/card-glyph-is-status.md",
   "docs/briefs/ask-state-fleet-read.md",
   "docs/briefs/dots-off-marks-on.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- board-entry-conditions (feature, phase idle, created 2026-08-25, merged into main: True, cells {})\n- herding-entry-conditions (feature, phase idle, created 2026-08-25, merged into main: True, cells {})\n- board-action-dupes (feature, phase scribing, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- herdr-protocol-20 (feature, phase compounding, created 2026-08-25, merged into main: True, cells {})\n- spawn-destination-fallback (feature, phase compounding, created 2026-08-25, merged into main: True, cells {})\n- board-live-morph (feature, phase compounding, created 2026-08-25, merged into main: True, cells {})\n- dispatch-project-presets (feature, phase compounding, created 2026-08-25, merged into main: True, cells {})\n- session-work-line (feature, phase compounding, created 2026-08-25, merged into main: True, cells {'capped': 2})\n- card-glyph-is-status (feature, phase compounding, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- ask-state-fleet-read (feature, phase compounding, created 2026-08-25, merged into main: True, cells {})\n- dots-off-marks-on (feature, phase compounding, created 2026-08-25, merged into main: False, cells {'capped': 1})",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py board-entry-conditions herding-entry-conditions board-action-dupes herdr-protocol-20 spawn-destination-fallback board-live-morph dispatch-project-presets session-work-line card-glyph-is-status ask-state-fleet-read dots-off-marks-on",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/board-entry-conditions.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/herding-entry-conditions.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/board-action-dupes.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/herdr-protocol-20.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/spawn-destination-fallback.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/board-live-morph.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/dispatch-project-presets.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/session-work-line.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/card-glyph-is-status.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/ask-state-fleet-read.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/dots-off-marks-on.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-6",
  "feature": "wd-briefs",
  "title": "Write owner briefs for the 2026-08-25 features, part two",
  "lane": "small",
  "role": "docs",
  "files": [
   "docs/briefs/rail-row-one-line.md",
   "docs/briefs/badge-mark-is-status.md",
   "docs/briefs/mark-state-tone.md",
   "docs/briefs/drawer-row-stack.md",
   "docs/briefs/drawer-mark-sprite.md",
   "docs/briefs/drawer-agent-mark.md",
   "docs/briefs/mark-before-status.md",
   "docs/briefs/terminal-mark-only.md",
   "docs/briefs/claude-code-mark.md",
   "docs/briefs/agent-marks-everywhere.md",
   "docs/briefs/agent-logo-marks.md",
   "docs/briefs/todo-column-collapse.md"
  ],
  "read_first": [
   "docs/history/wd-briefs/CONTEXT.md",
   "docs/history/wd-briefs/inventory.json"
  ],
  "action": "Write one owner brief per item listed below, at docs/briefs/<id>.md. Read docs/history/wd-briefs/CONTEXT.md first: D1-D8 are locked and define the format, the classification and the collision rule. The inventory row for each item (phase, merge state, cell counts, history files, next_action) is in docs/history/wd-briefs/inventory.json.\n\nWHO READS IT: the project owner, a non-developer, deciding from a task card on work.gogl.be whether to approve, park, drop, answer or test. Every brief must let them decide without opening anything else.\n\nSOURCES (D4, never the one-line cos alone): for a PBI, `.bee/bin/bee backlog pbi list --json` for its cos, then every doc the cos names, the feature it is bound to or deferred from (docs/history/<feature>/CONTEXT.md, plan.md, spec-drop and request files), docs/discovery/, and `.bee/bin/bee decisions search --text \"<keyword>\"`. For a feature: docs/history/<slug>/ (CONTEXT.md Feature Boundary and Locked Decisions, plan.md, spec-drop files), `.bee/bin/bee cells list --json` filtered to the feature, `git log main --oneline --grep <slug>` for whether and what it merged, and `.bee/bin/bee decisions search --text <slug>`. A history dir missing here may exist (read-only) under /mnt/data/Projects/goglbe/waggledance/docs/history/<slug>/ or /mnt/data/Projects/goglbe/waggledance--wt--<slug>/docs/history/<slug>/. When the sources are thin, say so plainly in the brief rather than inventing.\n\nCLASSIFY (D5): tooling, bee-internal mechanics, build, lint, CI, tech debt -> audience: internal; anything the owner sees or uses (board, inbox, terminal, guide, notifications, usage tab) -> audience: owner. Merged into main but workflow never closed -> ask: test (owner-visible: tell them exactly what to open and try; internal: say no action needed beyond agreeing to close). Built and awaiting acceptance -> test. Exploring/planning for two months with nothing built, or superseded by a later item -> drop, with the reason and what superseded it. A proposed item worth doing -> build or park, with your recommendation. An open question that blocks -> answer, naming the question. size: S under a day of agent work, M a few cells, L a multi-slice feature.\n\nFORMAT (D2, D3), exactly:\n---\ntitle: <Vietnamese, <=80 chars, states the outcome, not the mechanism>\naudience: owner|internal\nsize: S|M|L\nask: build|park|drop|answer|test\n---\n\n**Là gì:** 2-3 sentences: what happens today vs after.\n\n**Vì sao:** what pain it removes and who hits it.\n\n**Nguồn gốc:** one line: deferred from which work, found by which review, requested by whom, and when.\n\n**Chi phí và rủi ro:** size in words, what could break, what it depends on.\n\n**Anh cần làm:** options (e.g. (a) Duyệt làm (b) Hoãn (c) Bỏ), then **Đề xuất:** one option plus a one-line reason.\n\n**Chuẩn xong:**\n- 2 to 4 observable outcomes\n\nBody 120-220 words (whitespace-split), Vietnamese, addressing the owner as \"anh\". NO ids, slugs, paths, file names, function names, decision codes, cell ids or backticks in the body; describe things by what the owner sees. English product names (WaggleDance, Paseo, herdr, Telegram, bee) are fine.\n\nDo not change any PBI, workflow, cell or decision state other than your own cell's claim and cap. Write no file outside your listed files.\n\nITEMS:\n- rail-row-one-line (feature, phase compounding, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- badge-mark-is-status (feature, phase compounding, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- mark-state-tone (feature, phase compounding, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- drawer-row-stack (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- drawer-mark-sprite (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- drawer-agent-mark (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- mark-before-status (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- terminal-mark-only (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- claude-code-mark (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- agent-marks-everywhere (feature, phase exploring, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- agent-logo-marks (feature, phase scribing, created 2026-08-25, merged into main: False, cells {'capped': 1})\n- todo-column-collapse (feature, phase compounding, created 2026-08-25, merged into main: False, cells {})",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py rail-row-one-line badge-mark-is-status mark-state-tone drawer-row-stack drawer-mark-sprite drawer-agent-mark mark-before-status terminal-mark-only claude-code-mark agent-marks-everywhere agent-logo-marks todo-column-collapse",
  "must_haves": {
   "truths": [
    "Each listed item has a brief the owner can decide from without opening anything else",
    "The verify checker exits 0 over the listed ids"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/rail-row-one-line.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/badge-mark-is-status.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/mark-state-tone.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/drawer-row-stack.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/drawer-mark-sprite.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/drawer-agent-mark.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/mark-before-status.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/terminal-mark-only.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/claude-code-mark.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/agent-marks-everywhere.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/agent-logo-marks.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    },
    {
     "path": "docs/briefs/todo-column-collapse.md",
     "substantive": "four-key flat frontmatter, six Vietnamese sections, 120-220 words, built from linked sources"
    }
   ],
   "key_links": [
    "Content traces to the item history, decisions, cells or commit log, not the one-line cos alone"
   ],
   "prohibitions": [
    "No PBI, workflow, cell or decision state changed",
    "No file outside the listed brief paths",
    "No id, path, slug or function name in a brief body"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 },
 {
  "id": "brf-8",
  "feature": "wd-briefs",
  "title": "Reconcile the lint briefs with clippy red on main today",
  "lane": "tiny",
  "role": "docs",
  "files": [
   "docs/briefs/p-ebd4ce69.md",
   "docs/briefs/p-950905ce.md",
   "docs/briefs/lint-gates-green.md"
  ],
  "read_first": [
   "docs/briefs/p-6fab30aa.md"
  ],
  "action": "Leader completeness check found three lint briefs claiming the gates are green, while cargo clippy -p waggledance --all-targets -- -D warnings is red on this tree at crates/waggledance/src/views.rs:2094 (needless borrow). p-6fab30aa (build, the one-line fix) is the surviving item; p-ebd4ce69 is the same request filed twice -> drop as duplicate; p-950905ce -> drop as duplicate with the correct reason; lint-gates-green -> keep test (agree to close the August record) but state the new red and point to the backlog item.",
  "verify": "python3 .bee/spikes/wd-briefs/check_briefs.py p-ebd4ce69 p-950905ce lint-gates-green",
  "must_haves": {
   "truths": [
    "No brief claims clippy is green on main today",
    "Exactly one lint item asks to build the fix"
   ],
   "artifacts": [
    {
     "path": "docs/briefs/p-ebd4ce69.md",
     "substantive": "drop as duplicate"
    },
    {
     "path": "docs/briefs/p-950905ce.md",
     "substantive": "drop as duplicate, correct reason"
    },
    {
     "path": "docs/briefs/lint-gates-green.md",
     "substantive": "names the new red"
    }
   ],
   "key_links": [
    "The surviving fix is the build brief for the same lint request"
   ],
   "prohibitions": [
    "No code change"
   ]
  },
  "affects_skills": [],
  "affects_specs": [],
  "deps": []
 }
]
```

## Proof

Each cell: `python3 .bee/spikes/wd-briefs/check_briefs.py <its ids>` exits 0 —
frontmatter flat with the four keys and legal values, title ≤ 80 chars, six
labels in order, 120–220 words, 2–4 done-when bullets, no id, path, file,
function or slug in the body. The leader then runs the checker over all of
`docs/briefs/`, reads a sample of briefs against their sources (completeness
check), and reports the count per audience and ask.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | 29 PBIs are open (proposed, ready or in-flight) | ran | `bee backlog pbi list --json` | `Counter({'proposed': 28, 'done': 10, 'declined': 1, 'in-flight': 1})` |
| 2 | 70 feature workflows are active besides wd-briefs | ran | `bee state workflows list --json` | `99 Counter({'feature': 70, 'pbi': 29})` |
| 3 | PBI `board-run-reaper` collides with the feature slug of the same name | ran | `.bee/tmp/wdb/part.py` | `wdb-1 15 board-run-reaper p-0f60abae …` and `board-run-reaper` in the feature list |
| 4 | 26 active features have no history dir in this worktree | ran | inventory build | `features with no history dir: 26` |
| 5 | The seven cells validate with disjoint files covering 98 briefs | ran | `bee cells add --dry-run` | `dry-run: 7 cell(s) valid — nothing written.`; `7 98` |

## Out of scope

Changing any PBI or workflow status the briefs recommend (drop, close): the
owner decides from the briefs; acting on them is later work.
