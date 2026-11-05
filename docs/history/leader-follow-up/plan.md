---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Leader follow-up (`follow_up_of`)

## Summary

`waggledance_dispatch` with `leader: true` takes `follow_up_of`, and the
delivery loop routes such a run into the leader that already carries the
referenced work: it waits while that leader runs, sends to it when idle, and
starts a new leader with the chain's task texts when it is gone. A `done` run
no longer archives its leader; the 24-hour idle sweep does. `ask_state`
leaders list the runs each leader carries, and the tool descriptions, the
supervisor skill template and two knowledge patterns teach callers to use it.

Mode: `standard` — risk flags: public-contracts (a new MCP argument and new
`ask_state` fields that other sessions read), data-migration (one nullable
`runs` column), behavior change (no archive on done).
Why this is the least workflow that protects the work: one store cell, then
the delivery and MCP cells in parallel over disjoint files, then one docs
cell. The delivery tests drive the fake Paseo through every branch; the MCP
tests drive the tool through `call_tool`.

## Requirements (from CONTEXT.md)

- D1: `follow_up_of` argument, validation, stored as `runs.follow_up_of`.
- D2: family resolution, the four delivery branches, the chain prompt.
- D3: no archive on `done`; the sweep archives.
- D4: `leaders[].runs` with `task_summary`; `follow_up_of` on runs rows.
- D5: tool descriptions, skill template, two knowledge patterns.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | Only a wd run looks for a follow-up leader today | read | `crates/waggledance/src/delivery.rs:346` | `let prompt = match wd.then(\|\| self.follow_up(run)).flatten() {` |
| 2 | A done non-wd run archives its leader | read | `crates/waggledance/src/delivery.rs:582` | `if let (false, Some(agent)) = (wd, agent) { self.archive(agent).await;` |
| 3 | New `runs` columns are added with a `has_column` migration | read | `crates/waggledance-core/src/repository.rs:1141` | `if !has_column(conn, "runs", "caller")? {` |
| 4 | Dispatch checks a duplicate request_id before routing to the leader | read | `crates/waggledance/src/mcp.rs:1241` | `if let Some(request_id) = request_id { match engine.run_by_request(&project.id, request_id) {` |
| 5 | `follow_on` assumes a wd run | read | `crates/waggledance/src/delivery.rs:455` | `self.sent(run, leader, true, &now_rfc3339());` |

## Work breakdown

| Cell | Files | Depends on | Proof |
|---|---|---|---|
| lf-1 store | `crates/waggledance-core/src/repository.rs`, `engine.rs` | — | `cargo test --profile fast -p waggledance-core` |
| lf-2 delivery | `crates/waggledance/src/delivery.rs` | lf-1 | `cargo test --profile fast -p waggledance delivery` |
| lf-3 MCP | `crates/waggledance/src/mcp.rs` | lf-1 | `cargo test --profile fast -p waggledance mcp` |
| lf-4 docs | skill template, `paseo-leader-dispatch.md`, `wd-direct-leader.md` | lf-2, lf-3 | `cargo test --profile fast -p waggledance` (the doctor tests pin the template) |

### lf-1 store

Add a nullable `runs.follow_up_of` column with the same `has_column`
migration the `caller` column uses. `insert_run_with_request` gains a
`follow_up_of` argument written in the same INSERT, so a run is never
delivered before its link exists. Add reads: `run_follow_up_of(id)`,
`run_request_id(id)`, `follow_up_family(project, root)` returning the root and
every run whose chain reaches it, oldest first, and `leader_runs(project,
agent, limit)` returning `(Run, request_id, follow_up_of)` newest first.
Engine wrappers for each. Tests: the migration on an old table, the family of
a two-step chain, the leader-run read.

### lf-2 delivery

`deliver` resolves a follow-up for a run with `follow_up_of` before the wd
path. The resolution walks to the root, reads the family, requeues while an
earlier family run is `queued`/`working`, and picks the newest earlier family
run with a started paseo pane as the leader. The three leader branches follow
D2; the fresh-leader branch builds the chain prompt and adds a header line
naming the gone leader. `follow_on` takes the `wd` flag instead of assuming
true. `done` stops archiving. Tests: idle leader → `follow_on` and no new
leader; running leader → requeued; archived leader → new leader whose prompt
is the original then the follow-up, header names the gone leader; chain of
two follow-ups lands on the newest leader; a done run keeps its leader (the
existing archive assertions flip).

### lf-3 MCP

Parse `follow_up_of`; refuse it beside `preset`/`pane_id` or without
`leader`; after the duplicate check, resolve it by run id then request id in
the project, refusing an unknown one; pass the resolved id to
`dispatch_to_leader`. `attach_leaders` adds `runs` per leader with
`task_summary`. `waggledance_runs` rows carry `follow_up_of`. Update both tool
descriptions. Tests: exclusivity, unknown run refused, duplicate request_id
still returns the first run, the stored link, `task_summary` skipping
provenance lines, the description naming `follow_up_of`.

### lf-4 docs

Template: a "Follow-ups" passage in leader and execute mode: read `ask_state`
leaders, dispatch with `follow_up_of: <run_id>` and no fresh corr when the
spec extends a live leader's work, otherwise mint a new corr. Knowledge
patterns: the follow-up section covers wd and `follow_up_of`; the archive
text says done keeps the leader and the sweep archives it.

## Deviation

Cells run inline on the session model instead of through dispatched workers:
lf-2 and lf-3 build on lf-1's signatures in the same worktree, and the
herding model configured for `code` is a flash-tier pane model; the change is
coupled enough that a handoff would cost more than it saves.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "094d64e24e68c0b6fabaed19ae6619f3ac5e32bac9e9da2ab51728125c724d9b",
  "stages": [
    {
      "stage": "plan",
      "classification": "required",
      "role": "plan",
      "reason": "this plan, drafted by the leader"
    },
    {
      "stage": "hat-wave-facts-gaps",
      "classification": "not-applicable",
      "role": "hat-facts-gaps",
      "reason": "owner-written requirement-by-requirement spec; skip recorded in plan"
    },
    {
      "stage": "hat-wave-alternatives",
      "classification": "not-applicable",
      "role": "hat-alternatives",
      "reason": "the one cheaper shape the spec invited is weighed in D3"
    },
    {
      "stage": "hat-wave-user-impact",
      "classification": "not-applicable",
      "role": "hat-user-impact",
      "reason": "additive MCP argument and fields; behavior change named in D3"
    },
    {
      "stage": "hat-wave-risks",
      "classification": "not-applicable",
      "role": "hat-risks",
      "reason": "not high-risk"
    },
    {
      "stage": "hat-wave-value",
      "classification": "not-applicable",
      "role": "hat-value",
      "reason": "not high-risk"
    },
    {
      "stage": "advisor",
      "classification": "not-applicable",
      "role": "advisor",
      "reason": "standard lane"
    },
    {
      "stage": "test",
      "classification": "required",
      "role": "test",
      "reason": "lf-1..lf-3 Rust code plus tests on the native Rust-and-tests seat, precedent a66a46c0"
    },
    {
      "stage": "docs",
      "classification": "required",
      "role": "docs",
      "reason": "lf-4 template and knowledge sync"
    },
    {
      "stage": "code",
      "classification": "not-applicable",
      "role": "code",
      "reason": "Rust cells run on the test seat (a66a46c0)"
    },
    {
      "stage": "read",
      "classification": "conditional",
      "role": "read",
      "reason": "gathers only",
      "condition": "a worker needs a multi-file lookup"
    },
    {
      "stage": "extraction",
      "classification": "conditional",
      "role": "extraction",
      "reason": "cheap reader",
      "condition": "a single-fact lookup is delegated"
    },
    {
      "stage": "generation",
      "classification": "not-applicable",
      "role": "generation",
      "reason": "every cell names its role"
    },
    {
      "stage": "review",
      "classification": "conditional",
      "role": "review",
      "reason": "agents-review-user-invoked",
      "condition": "the user invokes bee-reviewing"
    },
    {
      "stage": "deploy",
      "classification": "not-applicable",
      "role": "deploy",
      "reason": "no release"
    },
    {
      "stage": "supervisor",
      "classification": "not-applicable",
      "role": "supervisor",
      "reason": "not used"
    },
    {
      "stage": "design",
      "classification": "not-applicable",
      "role": "design",
      "reason": "no UI"
    },
    {
      "stage": "lane-1",
      "classification": "not-applicable",
      "role": "lane-1",
      "reason": "one design"
    },
    {
      "stage": "lane-2",
      "classification": "not-applicable",
      "role": "lane-2",
      "reason": "no blind lanes"
    },
    {
      "stage": "lane-3",
      "classification": "not-applicable",
      "role": "lane-3",
      "reason": "no blind lanes"
    }
  ]
}
```

## Hat wave

Skipped, recorded with the inline-execution decision: the spec is owner-written requirement by requirement, the risk sits in delivery branches the fake-Paseo tests drive one by one, and D3 weighed the only cheaper shape the spec invited.

## Proof at close

- `cargo test --profile fast -p waggledance-core` and `cargo test --profile fast -p waggledance` green.
- The recorded `commands.test` targets `packages/bee-rs`, which this repo does not have; the cargo commands above are the real suite (named gap).

## Cells — current slice (preview)

```json
[
 {
  "id": "lf-1",
  "feature": "leader-follow-up",
  "lane": "standard",
  "status": "open",
  "role": "test",
  "deps": [],
  "affects_skills": [],
  "affects_specs": [],
  "title": "Store a run's follow_up_of link and read a follow-up family",
  "decisions": [
   "D1",
   "D2",
   "D4"
  ],
  "files": [
   "crates/waggledance-core/src/repository.rs",
   "crates/waggledance-core/src/engine.rs"
  ],
  "read_first": [
   "docs/history/leader-follow-up/plan.md"
  ],
  "action": "Follow plan.md section lf-1.",
  "verify": "cargo test --profile fast -p waggledance-core",
  "must_haves": {
   "truths": [
    "runs.follow_up_of is written in the insert"
   ],
   "artifacts": [],
   "key_links": []
  }
 },
 {
  "id": "lf-2",
  "feature": "leader-follow-up",
  "lane": "standard",
  "status": "open",
  "role": "test",
  "deps": [
   "lf-1"
  ],
  "affects_skills": [],
  "affects_specs": [],
  "title": "Deliver a follow_up_of run to its family's leader and keep done leaders",
  "decisions": [
   "D2",
   "D3"
  ],
  "files": [
   "crates/waggledance/src/delivery.rs"
  ],
  "read_first": [
   "docs/history/leader-follow-up/plan.md"
  ],
  "action": "Follow plan.md section lf-2.",
  "verify": "cargo test --profile fast -p waggledance delivery",
  "must_haves": {
   "truths": [
    "an idle leader gets the follow-up",
    "a done run keeps its leader"
   ],
   "artifacts": [],
   "key_links": []
  }
 },
 {
  "id": "lf-3",
  "feature": "leader-follow-up",
  "lane": "standard",
  "status": "open",
  "role": "test",
  "deps": [
   "lf-1"
  ],
  "affects_skills": [],
  "affects_specs": [],
  "title": "Accept follow_up_of at dispatch and list each leader's runs in ask_state",
  "decisions": [
   "D1",
   "D4"
  ],
  "files": [
   "crates/waggledance/src/mcp.rs"
  ],
  "read_first": [
   "docs/history/leader-follow-up/plan.md"
  ],
  "action": "Follow plan.md section lf-3.",
  "verify": "cargo test --profile fast -p waggledance mcp",
  "must_haves": {
   "truths": [
    "an unknown follow_up_of is refused"
   ],
   "artifacts": [],
   "key_links": []
  }
 },
 {
  "id": "lf-4",
  "feature": "leader-follow-up",
  "lane": "standard",
  "status": "open",
  "role": "docs",
  "deps": [
   "lf-2",
   "lf-3"
  ],
  "affects_skills": [],
  "affects_specs": [],
  "title": "Teach callers follow_up_of in the skill template and leader patterns",
  "decisions": [
   "D5"
  ],
  "files": [
   "docs/waggledance-supervisor-skill-template.md",
   "docs/knowledge/patterns/paseo-leader-dispatch.md",
   "docs/knowledge/patterns/wd-direct-leader.md"
  ],
  "read_first": [
   "docs/history/leader-follow-up/plan.md"
  ],
  "action": "Follow plan.md section lf-4.",
  "verify": "cargo test --profile fast -p waggledance",
  "must_haves": {
   "truths": [
    "the template names follow_up_of"
   ],
   "artifacts": [],
   "key_links": []
  }
 }
]
```
