---
artifact_contract: bee-plan/v2
mode: high-risk
---

# Plan: Paseo Dispatch Default On

## Summary

Every project waggledance manages now takes dispatched work without a
`.waggledance.toml`. A project turns dispatch off only by writing
`orchestration = false` in its own `.waggledance.toml`. The board's Approve/Reject
and run buttons follow the same rule, and the Settings page stops offering a
per-project checkbox that would no longer decide anything; it shows each project's
state and how to turn one off.

Mode: `high-risk` — 3 risk flags: authorization, covered-contract-change, public-contracts
Why this is the least workflow that protects the work: it removes a consent default,
so it gets the five-seat hat wave as the plan check; the code is one predicate and its
readers, done in one code cell, plus one docs cell and a live dispatch to a-blog.

## Requirements (from CONTEXT.md)

- D1: every registered project accepts dispatch by default, no file needed.
- D2: only `orchestration = false` in the project's `.waggledance.toml` turns it off.
- D3: one computed consent value for MCP dispatch, leader dispatch, trigger, mailbox scan, board actions and Settings.
- D4: Settings drops the per-project checkbox and its POST route; it lists effective state; the DB column stays unread.
- D5: the refusal names project, file path and the `orchestration = false` line.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command run, output kept).

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | Today consent is the DB flag or the file set to true | read | `crates/waggledance-core/src/engine.rs:390-393` | `project.orchestration_enabled \|\| self.project_settings(project).0.orchestration == Some(true)` |
| 2 | A new project is registered with the flag off | read | `crates/waggledance-core/src/engine.rs:86` | `orchestration_enabled: false,` |
| 3 | The refusal tells the user to opt in | read | `crates/waggledance/src/mcp.rs:1248-1251` | `has not opted into orchestrator dispatch — enable it from the project's settings page, or set \`orchestration = true\`` |
| 4 | Board actions read the DB flag, not the predicate | read | `crates/waggledance/src/server.rs:3091` | `if !project.orchestration_enabled {` |
| 5 | The file parser already yields `Some(false)` for `orchestration = false` | read | `crates/waggledance-core/src/project_settings.rs:97-101` test | `parse("orchestration = false", p).0.orchestration,` |
| 6 | a-blog has no `.waggledance.toml` | ran | `ls /mnt/data/Projects/anphabe/a-blog/.waggledance.toml` | owner report: file does not exist; dispatch refused |

## Discovery

Gather job-1792736906600-4133640-1 listed every reader, refusal string, test and doc
of the opt-in (see CONTEXT.md "Existing Code Context"). Four readers go through
`Engine::orchestration_allowed`; the board and Settings read `Project.orchestration_enabled`
directly.

## Approach

Recommended (D1–D5):

1. `Engine::orchestration_allowed(project)` becomes `project_settings(project).0.orchestration != Some(false)`.
   `Engine::list_projects` and `get_project` fill `Project.orchestration_enabled` with that
   same value on every read, so the board and Settings readers need no signature change
   (D3). `register` keeps writing the column; nothing reads the stored value.
2. `mcp.rs` `not_opted_in` becomes the opted-out refusal (D5): `project <id> has turned
   off orchestrator dispatch with \`orchestration = false\` in <path> — remove that line to
   allow it`, plus parse warnings. The board 403 text names the same line.
3. Settings: remove the per-project checkbox form and `POST /api/projects/:id/orchestration`
   (route, handler, form struct); the Orchestration section lists each project with
   `on` or `off (orchestration = false in .waggledance.toml)` (D4). `Engine::set_orchestration_enabled`
   and the store setter are deleted if nothing else calls them.
4. Copy: MCP tool descriptions (`mcp.rs:229`, `:249`), `setup.rs` doctor line, config doc
   for the trigger. Tests flip from "refused by default" to "accepted by default; refused
   with `orchestration = false`".
5. Docs: README, `docs/specs/mcp-surface.md`, `docs/specs/system-overview.md`,
   `docs/knowledge/patterns/paseo-leader-dispatch.md`,
   `docs/knowledge/patterns/an-opt-in-that-blocks-the-live-proof.md`,
   `docs/waggledance-supervisor-skill-template.md`. Delivery records under
   `docs/knowledge/work/*/delivery.md` are history and stay as written.

Rejected:

- Write `orchestration = true` into each discovered repo: writes owner files in every project.
- Flip the DB default with a migration: two sources of truth; the file still decides off.
- Keep the Settings checkbox as an extra off switch: a second off switch the owner did not ask for.

Risk map:

| Component | Risk | Lands in | Proof |
|---|---|---|---|
| Every agent can now dispatch into every managed project | HIGH (accepted by the owner, D1) | pd-1 | the owner asked for it; `terminal.enabled` still gates herdr dispatch; `orchestration = false` turns a project off; live dispatch to a-blog |
| Observer trigger now sees every project as consenting | MEDIUM | pd-1 | trigger stays off by default (`terminal.trigger_enabled`); test that an opted-out project is still skipped |
| Board Approve/Reject buttons appear on every bee project | MEDIUM | pd-1 | views tests for on-by-default and off-by-file |
| A malformed file no longer means "off" | LOW | pd-1 | test: a parse error leaves dispatch on and the warning shows in Settings |

Waves: pd-1, then pd-2 (docs describe the shipped behavior), then a live dispatch to a-blog.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "30ff876890293b1cea96673b722ea95a7b27780259af61e6c3b2be09a0c2b112",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "required", "role": "hat-facts-gaps", "reason": "high-risk: five-seat wave"},
    {"stage": "hat-wave-alternatives", "classification": "required", "role": "hat-alternatives", "reason": "high-risk: five-seat wave"},
    {"stage": "hat-wave-user-impact", "classification": "required", "role": "hat-user-impact", "reason": "high-risk: five-seat wave"},
    {"stage": "hat-wave-risks", "classification": "required", "role": "hat-risks", "reason": "high-risk: five-seat wave"},
    {"stage": "hat-wave-value", "classification": "required", "role": "hat-value", "reason": "high-risk: five-seat wave"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "the hat wave synthesis is the high-risk advisor consult"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "pd-1 is red-first Rust test-plus-code work; precedent a66a46c0"},
    {"stage": "docs", "classification": "required", "role": "docs", "reason": "pd-2 docs and README sync"},
    {"stage": "code", "classification": "not-applicable", "role": "code", "reason": "Rust cell runs on the test seat"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "every cell names its role"},
    {"stage": "review", "classification": "required", "role": "review", "reason": "goal-check judge on pd-1"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no release"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "herding cockpit role, not used"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "one design"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Shape

One slice.

SMALLER PATH: writing `orchestration = true` into a-blog alone would unblock today's
dispatch, but D1 asks for every project with no file, and the board and Settings would
keep disagreeing with dispatch (D3). No cheaper shape honors D1–D5.

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| pd-1 | Accept dispatch in every managed project unless its file turns it off | `engine.rs`, `repository.rs`, `mcp.rs`, `server.rs`, `views.rs`, `trigger.rs`, `setup.rs`, `config.rs`, `discovery.rs` | — | a dispatch to a-blog goes through; Settings shows each project on or off | `cargo test --profile fast -p waggledance-core -p waggledance` green |
| pd-2 | Describe dispatch as on by default with a file opt-out | README and five docs | pd-1 | docs say on by default and how to turn a project off | pointer check |

```json
[
 {"id":"pd-1","feature":"paseo-dispatch-default-on","title":"Accept dispatch in every managed project unless its file turns it off","lane":"high-risk","role":"test","status":"open","deps":[],
  "decisions":["D1","D2","D3","D4","D5","fb1b49ab-c734-461d-961f-20f60789d3b3"],
  "files":["crates/waggledance-core/src/engine.rs","crates/waggledance-core/src/repository.rs","crates/waggledance/src/mcp.rs","crates/waggledance/src/server.rs","crates/waggledance/src/views.rs","crates/waggledance/src/trigger.rs","crates/waggledance/src/setup.rs","crates/waggledance/src/config.rs","crates/waggledance/src/discovery.rs"],
  "read_first":["docs/history/paseo-dispatch-default-on/CONTEXT.md","docs/history/paseo-dispatch-default-on/plan.md","crates/waggledance-core/src/engine.rs","crates/waggledance-core/src/project_settings.rs"],
  "affects_skills":[],"affects_specs":[],
  "action":"Follow plan.md Approach steps 1-4. (1) Engine::orchestration_allowed returns project_settings(project).0.orchestration != Some(false); Engine::list_projects and get_project (and any other Engine fn returning Project) set orchestration_enabled to that same value on every Project they return. (2) mcp.rs not_opted_in becomes the opted-out refusal: `project <id> has turned off orchestrator dispatch with `orchestration = false` in <path> — remove that line to allow it`, then each parse warning; server.rs bee_action 403 says the project turned off board actions with `orchestration = false` in its .waggledance.toml. (3) Remove the Settings per-project checkbox forms, the POST /api/projects/:id/orchestration route, update_project_orchestration and OrchestrationForm; the Orchestration section lists each project with `on` or `off — orchestration = false in .waggledance.toml` plus any parse warning, and a hint that every project takes dispatch unless its file says otherwise and that Terminal still gates pane dispatch. Delete Engine::set_orchestration_enabled and the store setter if nothing else calls them (keep the DB column and migration). (4) Update copy: mcp.rs tool descriptions at ~229 and ~249, setup.rs paseo_unneeded_lines, config.rs trigger doc. Tests red-first: core — a fresh project with no file is allowed; `orchestration = false` is refused; `orchestration = true` allowed; a parse failure is allowed; get_project/list_projects return the effective flag. Daemon — dispatch and leader dispatch accepted with no file and refused with `orchestration = false` naming the path and the line; board actions 200 by default and 403 with the file off; trigger skips an opted-out project and its mailbox; Settings lists on/off and has no checkbox form or orchestration POST route (route returns 404/405); discovery's registered project reads as allowed. Rewrite the existing opt-in tests (engine.rs ~1306/1323, repository.rs ~2340, mcp.rs ~2805/2835/2859/2875/3334/3398, trigger.rs ~968/1170/1308/1818, server.rs ~14416/30227/31943, views.rs ~16113/16165/22472/22565, discovery.rs ~161, setup.rs ~1139) to the new rule rather than deleting their intent. No code comments.",
  "must_haves":{"truths":["A project with no .waggledance.toml accepts MCP and leader dispatch","orchestration = false in .waggledance.toml refuses dispatch and board actions, naming the path and line","Board Approve/Reject and run buttons appear for every project not turned off","Settings has no per-project checkbox and lists each project's effective state","terminal.enabled still gates pane dispatch"],"artifacts":[{"path":"crates/waggledance-core/src/engine.rs","substantive":"orchestration_allowed default on; effective flag on read"}],"key_links":["every reader of consent uses the same computed value"],"prohibitions":["Never write .waggledance.toml into any project","No code comments","Do not drop the DB column or migration"]},
  "verify":"cargo test --profile fast -p waggledance-core -p waggledance && cargo fmt --check",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"pd-2","feature":"paseo-dispatch-default-on","title":"Describe dispatch as on by default with a file opt-out","lane":"high-risk","role":"docs","status":"open","deps":["pd-1"],
  "decisions":["D1","D2","D4","D5","fb1b49ab-c734-461d-961f-20f60789d3b3"],
  "files":["README.md","docs/specs/mcp-surface.md","docs/specs/system-overview.md","docs/knowledge/patterns/paseo-leader-dispatch.md","docs/knowledge/patterns/an-opt-in-that-blocks-the-live-proof.md","docs/waggledance-supervisor-skill-template.md","docs/knowledge/patterns/index.md"],
  "read_first":["docs/history/paseo-dispatch-default-on/CONTEXT.md","crates/waggledance-core/src/engine.rs","crates/waggledance/src/mcp.rs"],
  "affects_skills":[],"affects_specs":["docs/specs/mcp-surface.md","docs/specs/system-overview.md"],
  "action":"Follow plan.md Approach step 5. Rewrite every passage that says a project must opt in (README.md ~207-259, docs/specs/mcp-surface.md ~50/115/141/161/219/237 including the mermaid node, docs/specs/system-overview.md ~124, docs/knowledge/patterns/paseo-leader-dispatch.md ~4/37-62/76-83/123/297/372, docs/knowledge/patterns/index.md ~28 description line, docs/waggledance-supervisor-skill-template.md ~185-189 including the quoted refusal) to: every managed project takes dispatch by default; `orchestration = false` in its .waggledance.toml turns it off; Settings shows the state; terminal.enabled still gates pane dispatch. Re-scope docs/knowledge/patterns/an-opt-in-that-blocks-the-live-proof.md to the opt-out (or mark it superseded by decision fb1b49ab with a pointer), keeping it truthful. Leave docs/knowledge/work/*/delivery.md history records unchanged. Regenerate indexes with bee knowledge index if descriptions change and run bee knowledge check.",
  "must_haves":{"truths":["No living doc tells a user to opt a project in","Docs name orchestration = false as the only per-project off switch"],"artifacts":[{"path":"docs/knowledge/patterns/paseo-leader-dispatch.md","substantive":"default-on rule and opt-out"}],"key_links":["the quoted refusal matches mcp.rs not_opted_in"],"prohibitions":["Do not rewrite docs/knowledge/work/*/delivery.md history"]},
  "verify":"rg -n 'opted into orchestration|orchestration = true' README.md docs/specs docs/knowledge/patterns docs/waggledance-supervisor-skill-template.md",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":false}}
]
```
