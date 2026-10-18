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
[{"id": "lme-1", "feature": "leader-mail-envelope", "title": "Teach the supervisor and its leaders the turn-end mail envelope", "action": "Add the [mail waggledance#<corr> <kind>] envelope to the leader preamble and every supervisor-to-leader prompt; read it first in 'When a leader finishes', with working as a silent wait; record it in the paseo-native-supervisor pattern.", "lane": "tiny", "role": "docs", "files": ["docs/waggledance-supervisor-agent.md", "docs/knowledge/patterns/paseo-native-supervisor.md"], "affects_skills": [], "affects_specs": ["docs/knowledge/patterns/paseo-native-supervisor.md"], "acceptance": "The leader preamble and every prompt the supervisor sends a leader ask for the envelope; 'When a leader finishes' reads the envelope first and treats working as a silent wait; the pattern doc records it; the seat embedded-doc test is green.", "verify": "cargo test --profile fast -p waggledance seat::", "read_first": ["docs/waggledance-supervisor-agent.md", "docs/knowledge/patterns/paseo-native-supervisor.md"], "must_haves": {"truths": ["A leader turn that ends while its own work still runs never reaches the user as a result or a question"]}}, {"id": "lme-2", "feature": "leader-mail-envelope", "title": "Follow a working leader until it sends a final mail", "action": "Heartbeat cron */10; on each beat nudge an idle leader whose last mail was working and silent 15 minutes, report progress once if it stays silent; ledger states nudged and silent-reported; pattern doc updated.", "lane": "tiny", "role": "docs", "files": ["docs/waggledance-supervisor-agent.md", "docs/knowledge/patterns/paseo-native-supervisor.md"], "affects_skills": [], "affects_specs": ["docs/knowledge/patterns/paseo-native-supervisor.md"], "acceptance": "The heartbeat section nudges a silent working leader once and reports progress once after a second silent beat; the seat tests are green.", "verify": "cargo test --profile fast -p waggledance seat::", "read_first": ["docs/waggledance-supervisor-agent.md"], "must_haves": {"truths": ["A task whose leader stopped on working is followed until the leader sends result, question or blocked, or the user hears it went silent"]}}]
```

### Slice 2 (plan_rev 1): follow a working leader to its end

`t-bd665a` still reported mid-task, for two reasons. The supervisor ran the
instructions from before slice 1, because the installed binary predates it. And a
`working` mail alone does not track anything: the supervisor hears from a leader only
when that leader's own next turn ends, so a leader whose wake never comes leaves the
task hanging. Decision `c6881832`: the heartbeat runs every 10 minutes. A leader
that is idle, whose last mail was `working`, and has had no turn end for 15 minutes
gets one nudge. If it stays silent through the next beat, the supervisor reports
`progress` to the user once.

| Cell | Files | Proof |
|---|---|---|
| lme-2 | `docs/waggledance-supervisor-agent.md`, `docs/knowledge/patterns/paseo-native-supervisor.md` | `cargo test --profile fast -p waggledance seat::` |

lme-2 is listed in the cell block under "Cells".

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | The leader preamble asks only for a plain reply | read | `docs/waggledance-supervisor-agent.md:98` | "When you finish or need a decision, end your turn with a plain reply." |
| 2 | The supervisor judges every turn end as a result, question, permission or error | read | `docs/waggledance-supervisor-agent.md:121` | "Read its last reply and decide what kind of turn it was" |
| 3 | The binary embeds this file and a test pins it to the repo copy | read | `crates/waggledance/src/seat.rs:15` | `include_str!("../../../docs/waggledance-supervisor-agent.md")` |
| 5 | The installed binary carries no mail rule | ran | `strings ~/.local/bin/waggledance \| rg -c "mail waggledance#"` | no match; `~/.waggledance/supervisor/CLAUDE.md` also has 0 matches |
| 6 | The t-bd665a leader ended its turn waiting on a child worker, with a Monitor armed | ran | `paseo logs bbfebc6`, `paseo inspect 24a2b37` | "[Monitor]" then "Tôi chờ worker."; worker `ParentAgentId bbfebc69…`, Status running |
| 4 | `ensure` rewrites the supervisor's CLAUDE.md when it differs from DOC | read | `crates/waggledance/src/seat.rs:65` | `if std::fs::read_to_string(&path).is_ok_and(\|have\| have == DOC) {` |

## Out of scope

- A durable mailbox command in waggledance. Rejected in decision `9d5fbab7`.
- A supervisor that is already running keeps the CLAUDE.md it started with. The new
  text takes effect the next time `waggledance supervisor ensure` writes the file and
  a supervisor session starts.
