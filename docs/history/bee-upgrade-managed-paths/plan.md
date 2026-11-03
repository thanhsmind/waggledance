---
artifact_contract: bee-plan/v2
mode: small
---

# Plan: bee-upgrade-managed-paths

## Summary

The first live bulk upgrade skipped 17 projects. Almost every overlapping
file was bee's own output from an earlier upgrade that nobody committed.
After this change, uncommitted files inside bee-managed paths are
overwritten and committed with the upgrade. Any other uncommitted file
that bee would write still skips the project. The change also stops
read-only git calls from taking the index lock, and adds `--only` to the
commit.

Mode: `small` — 1 risk flag: external systems (writes and commits in other
repos).
Why this is the least workflow that protects the work: one core file
changes, and the existing temp-repo tests already pin the commit isolation
this change must keep.

## Requirements

- bdc40074 (user, 2026-10-31): these paths are bee-managed:
  `.claude/skills/bee-*`, `.agents/skills/bee-*`, `.opencode/skills/bee-*`,
  `.bee/onboarding.json`, `.pi/extensions/bee-guard*`. Uncommitted files
  under them no longer skip a project; they are overwritten and committed.
  Any other overlap still skips the project and is named.
- 3439d31f (D3) still holds: the user's other staged or uncommitted work is
  never committed.

## Load-bearing claims

Labels: `read` means the file was opened at that line. `ran` means the
command was executed. The evidence column holds bytes copied from the
anchor.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | The overlap skip is one filter over the dirty set | read | `crates/waggledance-core/src/bee_upgrade.rs:244` | `            reason: "uncommitted edits in files bee would write".to_string(),` |
| 2 | The git helper does not set GIT_OPTIONAL_LOCKS | read | `crates/waggledance-core/src/bee_upgrade.rs:538` | `        .env("LC_ALL", "C");` |
| 3 | The live run's overlaps are bee-managed paths | ran | `waggledance bee upgrade --project survey-platform --apply --json` | `.claude/skills/bee-design/additions/wcag-2-2.md` |

## Discovery

The first live `waggledance bee upgrade --apply` on 2026-10-31 left 4
projects `up_to_date` and skipped 26. Of those 26, 17 were skipped for
overlapping uncommitted files. Sampled overlaps were skill files and
`.bee/onboarding.json`. memorypad also had `AGENTS.md` and
`.bee/config.json` in the overlap.

## Approach

In `bee_upgrade.rs`, split the overlap into two sets: managed (under a
bee-managed prefix) and other. A non-empty "other" set still skips the
project. A managed overlap no longer skips it. Those files join the commit
set when they are still dirty after apply and fall under an expanded
planned path. The commit set becomes
((after − before) ∪ (managed overlap still dirty after apply)) ∩ planned.
`git()` sets `GIT_OPTIONAL_LOCKS=0`, and the commit passes `--only`.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "094d64e24e68c0b6fabaed19ae6619f3ac5e32bac9e9da2ab51728125c724d9b",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "not-applicable", "role": "hat-facts-gaps", "reason": "small lane, one file"},
    {"stage": "hat-wave-alternatives", "classification": "not-applicable", "role": "hat-alternatives", "reason": "small lane"},
    {"stage": "hat-wave-user-impact", "classification": "not-applicable", "role": "hat-user-impact", "reason": "the user chose the behaviour directly"},
    {"stage": "hat-wave-risks", "classification": "not-applicable", "role": "hat-risks", "reason": "not high-risk"},
    {"stage": "hat-wave-value", "classification": "not-applicable", "role": "hat-value", "reason": "not high-risk"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "small lane"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "red-first Rust test-plus-code cell"},
    {"stage": "docs", "classification": "not-applicable", "role": "docs", "reason": "the concept update rides the same cell"},
    {"stage": "code", "classification": "not-applicable", "role": "code", "reason": "Rust cells run on the test seat"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "every cell names its role"},
    {"stage": "design", "classification": "not-applicable", "role": "design", "reason": "no UI change"},
    {"stage": "review", "classification": "conditional", "role": "review", "condition": "the user invokes bee-reviewing", "reason": "agents-review-user-invoked"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no release"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "not used"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "one design"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| bum-1 | Overwrite and commit uncommitted bee-managed files during a bulk upgrade | `crates/waggledance-core/src/bee_upgrade.rs`, `docs/knowledge/work/bee-bulk-upgrade/bulk-upgrade.md` | — | projects whose only overlaps are bee's own leftover files upgrade and commit; a dirty AGENTS.md still skips | `cargo test --profile fast -p waggledance-core bee_upgrade` green |

```json
[
 {"id":"bum-1","feature":"bee-upgrade-managed-paths","title":"Overwrite and commit uncommitted bee-managed files during a bulk upgrade","lane":"small","role":"test","status":"open","deps":[],
  "decisions":["bdc40074","3439d31f","0b11d69f"],
  "files":["crates/waggledance-core/src/bee_upgrade.rs","docs/knowledge/work/bee-bulk-upgrade/bulk-upgrade.md"],
  "read_first":["docs/history/bee-upgrade-managed-paths/plan.md","crates/waggledance-core/src/bee_upgrade.rs","docs/knowledge/work/bee-bulk-upgrade/bulk-upgrade.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach. In bee_upgrade.rs (search `uncommitted edits in files bee would write` for the overlap skip inside the apply path), split the overlap into managed and other. A path is managed when it starts with `.claude/skills/bee-`, `.agents/skills/bee-` or `.opencode/skills/bee-`, equals `.bee/onboarding.json`, or starts with `.pi/extensions/bee-guard` (per decision bdc40074). Keep the Skipped outcome and its reason for a non-empty other set, naming only the other files. With only managed overlaps, continue to apply; the commit set becomes ((after minus before) union (managed overlap paths that are still dirty after apply)) filtered to the expanded planned prefixes. Upgraded.files lists the committed set. In `fn git(` add `.env(\"GIT_OPTIONAL_LOCKS\", \"0\")`. In the commit call (search `\"commit\",`) add `--only` before the pathspec flags. Extend the fake bee in mod tests only where needed. New tests: an uncommitted file under .claude/skills/bee-hive before apply no longer skips; the project is Upgraded and the commit contains that file; a dirty AGENTS.md together with a dirty managed file still skips and names only AGENTS.md; an unrelated staged file still stays out of the commit (existing test must stay green); a dry run leaves .git/index mtime unchanged. Update the existing test that expected a skip for a dirty .claude/skills/bee-hive/SKILL.md to the new behaviour. In docs/knowledge/work/bee-bulk-upgrade/bulk-upgrade.md, update the skip-rule and path-expansion text to cite bdc40074 and list the managed prefixes. No code comments.",
  "must_haves":{"truths":["An uncommitted file under a bee-managed prefix no longer skips the project and is committed with the upgrade","Any other overlapping uncommitted file still skips the project and is named","Work the user staged outside the planned paths is never committed","Read-only git calls set GIT_OPTIONAL_LOCKS=0","The commit passes --only"],"artifacts":[{"path":"crates/waggledance-core/src/bee_upgrade.rs","substantive":"managed-prefix split, commit-set union, git env, --only, tests"}],"key_links":["the knowledge concept cites bdc40074"],"prohibitions":["No change to other skip rules","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance-core bee_upgrade",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}}
]
```

## Test matrix

- Happy path: a managed-only overlap is upgraded and committed. Pass when `git show --name-only HEAD` lists the managed file.
- Edge cases:
  - A mixed overlap skips the project. Pass when the outcome is `skipped` and names only `AGENTS.md`.
  - A staged unrelated file stays out of the commit. Pass when it stays in `git diff --cached`.
- Error path: a dry run never touches the index. Pass when the `.git/index` mtime is unchanged.

## Open Questions

(none)

## Out of scope

- The `up_to_date` label after an apply that had nothing tracked to commit
  (filed in the backlog).
- Redirecting the board POST after an apply.
