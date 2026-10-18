# leader-mail-envelope — plan

Lane: tiny. Class: feature. Decision: `9d5fbab7`.

## Problem

The supervisor reads every leader turn end as a result or a question. A leader that
stops while its own advisors, subagents or builds still run looks finished. In
`t-334d78` the beehive leader ended its turn with "The plan check (3 advisors) and the
baseline build are still running", and the supervisor sent the user a misleading
status plus a question that was not theirs to answer.

## Shape

Borrow piggery's split between `reply` (a turn ended) and `settled` (the last message
is sent and nothing is still working). Each leader turn ends with a mail envelope:

```
[mail waggledance#<corr> <kind>]
```

`<kind>` is `result`, `question`, `blocked` or `working`. When the kind is `working`,
the supervisor sends the user nothing and waits for the leader's next turn end.

## Cells

| Cell | Files | Proof |
|---|---|---|
| lme-1 | `docs/waggledance-supervisor-agent.md`, `docs/knowledge/patterns/paseo-native-supervisor.md` | `cargo test --profile fast -p waggledance seat::` (embedded doc matches) |

```json
[{"id":"lme-1","feature":"leader-mail-envelope","title":"Teach the supervisor and its leaders the turn-end mail envelope","action":"Add the [mail waggledance#<corr> <kind>] envelope to the leader preamble and every supervisor-to-leader prompt; read it first in 'When a leader finishes', with working as a silent wait; record it in the paseo-native-supervisor pattern.","lane":"tiny","role":"docs","files":["docs/waggledance-supervisor-agent.md","docs/knowledge/patterns/paseo-native-supervisor.md"],"affects_skills":[],"affects_specs":["docs/knowledge/patterns/paseo-native-supervisor.md"],"acceptance":"The leader preamble and every prompt the supervisor sends a leader ask for the envelope; 'When a leader finishes' reads the envelope first and treats working as a silent wait; the pattern doc records it; the seat embedded-doc test is green.","verify":"cargo test --profile fast -p waggledance seat::","read_first":["docs/waggledance-supervisor-agent.md","docs/knowledge/patterns/paseo-native-supervisor.md"],"must_haves":{"truths":["A leader turn that ends while its own work still runs never reaches the user as a result or a question"]}}]
```

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | The leader preamble asks only for a plain reply | read | `docs/waggledance-supervisor-agent.md:98` | "When you finish or need a decision, end your turn with a plain reply." |
| 2 | The supervisor judges every turn end as a result, question, permission or error | read | `docs/waggledance-supervisor-agent.md:121` | "Read its last reply and decide what kind of turn it was" |
| 3 | The binary embeds this file and a test pins it to the repo copy | read | `crates/waggledance/src/seat.rs:15` | `include_str!("../../../docs/waggledance-supervisor-agent.md")` |
| 4 | `ensure` rewrites the supervisor's CLAUDE.md when it differs from DOC | read | `crates/waggledance/src/seat.rs:65` | `if std::fs::read_to_string(&path).is_ok_and(\|have\| have == DOC) {` |

## Out of scope

- A durable mailbox command in waggledance. Rejected in decision `9d5fbab7`.
- A supervisor that is already running keeps the CLAUDE.md it started with. The new
  text takes effect the next time `waggledance supervisor ensure` writes the file and
  a supervisor session starts.
