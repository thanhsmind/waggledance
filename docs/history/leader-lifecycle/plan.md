---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Leader lifecycle (handoff, one leader per family, 30-minute close)

## Summary

A context-limit handoff joins the old leader's family and retires that
leader once the successor has its task; every follow-up in the family lands
on the newest leader; idle leaders close after 30 minutes (configurable)
unless something in their family is still open; `ask_state` shows which
leader is current for each family.

Mode: `standard` — risk flags: public contract (a new MCP argument, new
`ask_state` fields, a new config key), data migration (one `runs` column),
behavior change (leaders archive after 30 minutes and on handoff).
Why this is the least workflow that protects the work: one store/config cell,
then the delivery and MCP cells in parallel over disjoint files, then one docs
and release cell. The delivery tests drive the fake Paseo through every
branch the spec names.

No hat wave: the spec is clear and owner-chosen, with every behavior and test
named; the decisions were locked from the code read (recorded deviation).

## Requirements (from CONTEXT.md)

- D1: `handoff` argument, validation, `runs.handoff` column.
- D2: handoff delivery starts a fresh leader and retires the older ones.
- D3: family resolution counts handoff runs created later.
- D4: implicit handoff from a leader caller, with a warning.
- D5: `leader_idle_minutes` (global + project), new sweep rules, doctor row.
- D6: archive paths are sweep, handoff retirement, wd accept.
- D7: `family_root` / `current` on `leaders[]`; docs teach `follow_up_of`.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | The idle limit is a 24-hour constant | read | `crates/waggledance/src/delivery.rs:23` | `const LEADER_IDLE_LIMIT: time::Duration = time::Duration::hours(24);` |
| 2 | Family resolution looks only at runs created before the run | read | `crates/waggledance/src/delivery.rs:525` | `let earlier: Vec<&Run> = family.iter().take_while(\|r\| r.id != run.id).collect();` |
| 3 | The sweep skips only leaders behind a working or blocked run | read | `crates/waggledance-core/src/repository.rs:395` | `FROM runs WHERE status IN ('working','blocked') AND pane_id LIKE 'paseo:%'` |
| 4 | The dispatch caller is the Paseo agent id | read | `crates/waggledance/src/mcp.rs:1449` | `fn dispatch_caller(paseo_agent: Option<String>, waggledance: Option<String>) -> Option<String> {` |
| 5 | New `runs` columns use a `has_column` migration | read | `crates/waggledance-core/src/repository.rs:1294` | `if runs == 1 && !has_column(conn, "runs", "follow_up_of")? {` |

## Work breakdown

| Cell | Files | Depends on | Proof |
|---|---|---|---|
| ll-1 store+config | `crates/waggledance-core/src/{repository,engine,config,project_settings}.rs` | — | `cargo test --profile fast -p waggledance-core` |
| ll-2 delivery | `crates/waggledance/src/delivery.rs` (+ the fake in its test module) | ll-1 | `cargo test --profile fast -p waggledance delivery` |
| ll-3 MCP+doctor | `crates/waggledance/src/mcp.rs`, `crates/waggledance/src/doctor.rs` | ll-1 | `cargo test --profile fast -p waggledance mcp doctor` |
| ll-4 docs+release | `docs/knowledge/patterns/paseo-leader-dispatch.md`, `docs/waggledance-supervisor-skill-template.md`, README config section, `Cargo.toml` version 0.5.3, `Cargo.lock` | ll-2, ll-3 | `cargo test --profile fast --workspace` + `cargo fmt --all --check` |

### ll-1 store+config

- Migration 11 adds `runs.handoff INTEGER NOT NULL DEFAULT 0` the same way
  migration 10 adds `follow_up_of`.
- `insert_follow_up_run` gains `handoff: bool`, written in the same INSERT
  (engine wrapper too; existing callers pass `false`).
- New reads, each with an engine wrapper: `run_is_handoff(id) -> bool`;
  `runs_on_pane(pane_id) -> Vec<Run>` across projects, newest first;
  `family_handoffs(root) -> HashSet<String>` (ids of handoff runs in the
  family) or an equivalent flag on the family read.
- `PaseoConfig.leader_idle_minutes: u64`, default 30, serde default so old
  configs load. `ProjectPaseo.leader_idle_minutes: Option<u64>` and the key
  added to the known `[paseo]` keys so no unknown-key warning fires.
- Tests: migration on a table without the column; handoff flag round trip;
  `runs_on_pane`; config default and parse; project override parse.

### ll-2 delivery

- The limit comes from config (global, then the project override of the
  leader's newest carried run's project); `idle_for` takes the limit.
  Strictly greater: 31 minutes archives, 29 and 30 do not.
- The sweep also skips a leader with pending permissions, a leader carrying
  a `queued`/`working`/`blocked` run, and a leader any of whose carried runs'
  family (root via `follow_up_chain`, then `follow_up_family`) has a
  `queued`/`working`/`blocked` run.
- `follow_up_of` resolution (D3): candidates = earlier family runs plus every
  handoff run of the family other than this run, kept in family order. Wait
  if any candidate is open; leader = newest candidate with a started pane.
- A handoff run (D2): no wait, no send to an existing leader; it starts a
  fresh leader with its own task text; header line
  `[waggledance] handoff of run <of>: takes over from leader <old>` (or no
  `takes over` part when no older leader started). Once `new_leader` returns
  the successor id, spawn retirement for every distinct older family leader
  (started panes of the other family runs, not the successor): poll
  `inspect` every `permission_poll` until it is archived (stop), or idle with
  no open run on its pane (archive it, log it), or the turn timeout passes
  (stop, the sweep takes over).
- Tests required by the spec: handoff archives the old leader after the
  successor is initialized (and not before the old leader's run ends);
  `follow_up_of` naming an old family run resolves to the successor, also for
  a follow-up queued before the handoff; sweep archives at 31 minutes and not
  at 29; a blocked run's leader is never archived; a family with a queued run
  is never archived. Existing 24-hour tests move to the new limit.

### ll-3 MCP+doctor

- `dispatch_schema` documents `handoff`; validation: `handoff: true` needs
  `leader: true` and `follow_up_of` (after D4's fill), refused otherwise with
  a message naming both.
- D4: with `leader: true`, no `follow_up_of`, `handoff` not `false`, and a
  caller that is a Paseo agent id: the newest run in this project whose pane
  is `paseo:<caller>` becomes `follow_up_of` with handoff true; the result's
  `warnings` gains `leader <caller> dispatched a new leader without
  follow_up_of; treated as a handoff of run <id> — pass follow_up_of with
  handoff: true`. `structuredContent` carries `handoff: true|false`.
- `ask_state` `leaders[]`: `family_root` and `current` per D7; the
  description names both.
- Doctor row `leader idle close`: Skip when Paseo is off; Ok with
  `idle leaders close after <n> min ([paseo] leader_idle_minutes)`; Warn when
  0.
- Tests through `call_tool`: explicit handoff stored with the flag and family
  link; handoff without follow_up_of refused; implicit handoff from a caller
  that leads a project run, warning present; `handoff: false` opts out;
  `family_root`/`current` for two leaders of one family.

### ll-4 docs+release

- `paseo-leader-dispatch.md` and the supervisor template: follow-up rule,
  handoff rule, the 30-minute close and its key, `family_root`/`current`.
- README config section names `leader_idle_minutes`.
- Version 0.5.3 in `Cargo.toml` (and `Cargo.lock`), the repo's release step
  (`chore: bump version to 0.5.3`, as for 0.5.2).

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
      "reason": "behavior change named in D2, D5, D6"
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
      "reason": "ll-4 pattern, template, README and version bump"
    },
    {
      "stage": "code",
      "classification": "required",
      "role": "code",
      "reason": "ll-1..ll-3 Rust changes plus their tests"
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

`cargo test --profile fast --workspace`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo fmt --all --check` — the same three
commands CI runs. The recorded `commands.test` points at
`packages/bee-rs/Cargo.toml`, which this repo does not have; the CI commands
are the recorded fallback.

## Cells — current slice (preview)

```json
[
{"id":"ll-1","feature":"leader-lifecycle","lane":"standard","role":"code","deps":[],"affects_skills":[],"affects_specs":[],"title":"Store the handoff flag and the leader idle limit","decisions":["D1","D5"],"files":["crates/waggledance-core/src/repository.rs","crates/waggledance-core/src/engine.rs","crates/waggledance-core/src/config.rs","crates/waggledance-core/src/project_settings.rs","crates/waggledance/src/mcp.rs","crates/waggledance/src/delivery.rs"],"read_first":["docs/history/leader-lifecycle/CONTEXT.md","docs/history/leader-lifecycle/plan.md"],"action":"Follow plan.md section ll-1. Callers of insert_follow_up_run in the waggledance crate only gain a false argument so the workspace compiles; no behavior change there.","verify":"cargo test --profile fast -p waggledance-core","verify_owner":"worker","must_haves":{"truths":["runs.handoff exists after migration 11 and round-trips","leader_idle_minutes defaults to 30 globally and parses per project without an unknown-key warning","runs_on_pane lists runs across projects newest first"],"artifacts":[],"key_links":[]}},
{"id":"ll-2","feature":"leader-lifecycle","lane":"standard","role":"code","deps":["ll-1"],"affects_skills":[],"affects_specs":[],"title":"Retire the old leader on handoff and close idle leaders after the configured limit","decisions":["D2","D3","D5","D6"],"files":["crates/waggledance/src/delivery.rs"],"read_first":["docs/history/leader-lifecycle/CONTEXT.md","docs/history/leader-lifecycle/plan.md"],"action":"Follow plan.md section ll-2.","verify":"cargo test --profile fast -p waggledance delivery","verify_owner":"worker","must_haves":{"truths":["handoff archives the old leader only after the successor is initialized and the old leader is idle","follow_up_of on an old family run resolves to the successor, also when queued before the handoff","sweep archives at 31 minutes and not at 29","a blocked run's leader is never archived","a family with a queued run is never archived"],"artifacts":[],"key_links":[]}},
{"id":"ll-3","feature":"leader-lifecycle","lane":"standard","role":"code","deps":["ll-1"],"affects_skills":[],"affects_specs":[],"title":"Accept handoff in dispatch, detect implicit handoffs, and show the current family leader","decisions":["D1","D4","D5","D7"],"files":["crates/waggledance/src/mcp.rs","crates/waggledance/src/doctor.rs"],"read_first":["docs/history/leader-lifecycle/CONTEXT.md","docs/history/leader-lifecycle/plan.md"],"action":"Follow plan.md section ll-3.","verify":"cargo test --profile fast -p waggledance mcp doctor","verify_owner":"worker","must_haves":{"truths":["handoff true is stored with the flag and the family link","handoff without follow_up_of is refused","a leader caller without follow_up_of becomes a handoff with a warning","handoff false opts out","leaders[] carry family_root and current","doctor shows the idle limit"],"artifacts":[],"key_links":[]}},
{"id":"ll-4","feature":"leader-lifecycle","lane":"standard","role":"docs","deps":["ll-2","ll-3"],"affects_skills":[],"affects_specs":["docs/knowledge/patterns/paseo-leader-dispatch.md"],"title":"Teach follow_up_of and handoff, document the idle limit, bump to 0.5.3","decisions":["D5","D7"],"files":["docs/knowledge/patterns/paseo-leader-dispatch.md","docs/waggledance-supervisor-skill-template.md","README.md","Cargo.toml","Cargo.lock"],"read_first":["docs/history/leader-lifecycle/CONTEXT.md","docs/history/leader-lifecycle/plan.md"],"action":"Follow plan.md section ll-4.","verify":"cargo test --profile fast --workspace","verify_owner":"worker","must_haves":{"truths":["the template says a small follow-up always goes through follow_up_of","the pattern names leader_idle_minutes and handoff","workspace version is 0.5.3"],"artifacts":[],"key_links":[]}}
]
```
