---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Same-cluster steer tier for leader dispatch

## Summary

The leader dispatch rule gains a middle tier. A request in the same cluster as
a running leader still opens its own leader, and the running leader gets a
steer note that carries the marker. `waggledance_dispatch` takes `steer: true`.
The daemon writes the marker. A steer whose leader is gone ends `lost`
instead of opening a leader. `ask_state` flags steer runs.

Mode: `standard`. Risk flags: public contract (one new MCP argument and one
new `ask_state` field) and behavior change (doctrine every supervisor and
leader follows). There is no migration, because steer is read from the stored
task text. Why this is the smallest honest shape: one code cell and one docs
and release cell, run in that order, because the docs describe the code's
contract.

No hat wave: the spec names every tier and the open choices. The decisions
were locked from the code read (recorded deviation, as in leader-lifecycle).

## Requirements (from CONTEXT.md)

- D2/D3/D7: `steer` argument, validation, marker, lost-on-gone, `steer` field.
- D1/D4/D5/D6/D8/D9/D10: template, tool descriptions, pattern pages, release.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | A follow-up whose leader is gone starts a new leader with the earlier task | read | `crates/waggledance/src/delivery.rs:507` | `(leader, _) => {` then `format!("{}\n\n{}", follow.prior, run.task)` |
| 2 | A leader's dispatch without follow_up_of becomes a handoff unless `handoff: false` | read | `crates/waggledance/src/mcp.rs:1548` | `(Some(false), _) \| (_, None) => None,` |
| 3 | `waggledance_runs` rows carry the full task text | read | `crates/waggledance/src/mcp.rs:2064` | `"task": r.task,` |
| 4 | The template is compiled into setup and doctor | read | `crates/waggledance/src/setup.rs` | `include_str!` of `docs/waggledance-supervisor-skill-template.md` |

## Work breakdown

| Cell | Files | Depends on | Proof |
|---|---|---|---|
| cs-1 steer code | `crates/waggledance/src/mcp.rs`, `crates/waggledance/src/delivery.rs`, `crates/waggledance/src/paseo.rs` | — | `cargo test --profile fast -p waggledance` |
| cs-2 docs+release | `docs/waggledance-supervisor-skill-template.md`, `docs/knowledge/patterns/{paseo-leader-dispatch,wd-direct-leader,paseo-native-supervisor}.md`, `Cargo.toml`, `Cargo.lock` | cs-1 | `cargo test --profile fast --workspace` and `cargo fmt --all --check` |

### cs-1 steer code

- `paseo::STEER_MARKER` constant, `paseo::steer_task(task)` and
  `paseo::is_steer(task)`. `is_steer` is true when the first non-empty line
  of the body (after any wd preamble) equals the marker.
- `handle_dispatch` reads `steer`. It refuses `steer: true` without
  `leader: true`, without `follow_up_of`, or with `handoff: true`.
  `dispatch_to_leader` puts the marker on the task before the wd wrap.
  The answer carries `steer`.
- `task_summary` skips the marker line. `leader_runs` rows gain `steer`.
- Delivery: in the gone branch of a `Route::Follow`, a steer run is lost with
  a reason that names the follow-up run, and no leader starts.
- Schemas: `steer` property and the three-tier rule in the
  `waggledance_dispatch` and `waggledance_ask_state` descriptions.
- Tests: dispatch validation (three refusals and one accepted case with the
  marker stored first), `task_summary`/`steer` on an ask_state row, and the
  delivery test that a steer to a gone leader ends `lost` and starts nothing.

### cs-2 docs+release

- Template Leader mode steps 1-3 become the three tiers, with the steer call,
  the line for the new leader's task, `handoff: false` for a leader caller, and
  the receiving side's rule.
- `paseo-leader-dispatch.md` § follow_up_of gains the tier rule and a
  `steer` subsection. `wd-direct-leader.md` and `paseo-native-supervisor.md`
  get one pointer each.
- Workspace version 0.5.4, then `Cargo.lock`.

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
      "reason": "owner spec names every behavior and test; skip recorded in plan"
    },
    {
      "stage": "hat-wave-alternatives",
      "classification": "not-applicable",
      "role": "hat-alternatives",
      "reason": "no cheaper shape: the spec fixes the contract"
    },
    {
      "stage": "hat-wave-user-impact",
      "classification": "not-applicable",
      "role": "hat-user-impact",
      "reason": "behavior change named in D1, D3, D6"
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
      "classification": "conditional",
      "role": "test",
      "reason": "tests ride inside each code cell; no separate test cell",
      "condition": "a cell's tests need a separate pass"
    },
    {
      "stage": "docs",
      "classification": "required",
      "role": "docs",
      "reason": "cs-2 template, pattern pages and version bump"
    },
    {
      "stage": "code",
      "classification": "required",
      "role": "code",
      "reason": "cs-1 Rust changes plus their tests"
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
      "reason": "the version bump is a docs-cell edit; tagging stays with the owner"
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

## Proof at close

- `cargo test --profile fast --workspace` green.
- `cargo fmt --all --check` clean.
- `cargo clippy --profile fast -p waggledance -- -D warnings` clean.

## Cells — current slice (preview)

```json
[
{"id":"cs-1","feature":"leader-cluster-steer","lane":"standard","role":"code","deps":[],"affects_skills":[],"affects_specs":[],"title":"Add the steer flag to leader dispatch","decisions":["D2","D3","D7"],"files":["crates/waggledance/src/mcp.rs","crates/waggledance/src/delivery.rs","crates/waggledance/src/paseo.rs"],"read_first":["docs/history/leader-cluster-steer/CONTEXT.md","docs/history/leader-cluster-steer/plan.md"],"action":"Follow plan.md section cs-1.","verify":"cargo test --profile fast -p waggledance","verify_owner":"worker","must_haves":{"truths":["steer true is stored with the marker as its first line","steer without leader or follow_up_of, or with handoff true, is refused","a steer to a gone leader ends lost and starts no leader","ask_state runs carry steer and a summary without the marker"],"artifacts":[],"key_links":[]}},
{"id":"cs-2","feature":"leader-cluster-steer","lane":"standard","role":"docs","deps":["cs-1"],"affects_skills":[],"affects_specs":["docs/knowledge/patterns/paseo-leader-dispatch.md","docs/knowledge/patterns/wd-direct-leader.md","docs/knowledge/patterns/paseo-native-supervisor.md"],"title":"Teach the three-tier leader dispatch rule and release 0.5.4","decisions":["D1","D4","D5","D6","D8","D9","D10"],"files":["docs/waggledance-supervisor-skill-template.md","docs/knowledge/patterns/paseo-leader-dispatch.md","docs/knowledge/patterns/wd-direct-leader.md","docs/knowledge/patterns/paseo-native-supervisor.md","Cargo.toml","Cargo.lock"],"read_first":["docs/history/leader-cluster-steer/CONTEXT.md","docs/history/leader-cluster-steer/plan.md"],"action":"Follow plan.md section cs-2.","verify":"cargo test --profile fast --workspace","verify_owner":"worker","must_haves":{"truths":["the template names three tiers checked by every caller including leaders","the template says how a leader treats a steer note","the pattern pages say the same","workspace version is 0.5.4"],"artifacts":[],"key_links":[]}}
]
```
