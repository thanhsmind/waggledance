# Leader Delegated Work — Context

**Feature slug:** leader-delegated-work
**Date:** 2026-11-11
**Shaping session:** complete
**Scope:** Standard
**Domain types:** RUN | CALL

## Feature Boundary

A Paseo leader that ends its turn while work it handed out (a Claude background subagent, a herdr worker, a Paseo child agent) is still running is not reported done, archived or nudged until that work reports or a ceiling expires, and a reply the leader gives after its run already ended still reaches the caller.

## Locked Decisions

Recorded as decision `leader-delegated-work` (bee decisions log, 2026-11-11, touches `e9d64248`, `c6881832`).

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The leader declares a wait itself with a `waggledance hold` command (a reason, an optional duration) and can release it. waggledance also counts a live Paseo child agent of the leader (`ParentAgentId` names the leader) as held work. waggledance never infers delegated work from a provider timeline. | Paseo cannot see Claude background subagents or herdr workers; the wake turn is too short to catch by polling status. |
| D2 | A hold lasts at most 2 hours by default; the limit is configurable. When it expires the behavior before this feature resumes and the caller is told the hold expired. | A forgotten hold must not keep a run open forever. |
| D3 | While a hold or a live child exists, the run stays `working` with a note naming the wait; no new run status is added. A turn end does not end the run done, the idle sweep does not archive the leader, and the heartbeat does not nudge it. | Callers keep reading the statuses they know. |
| D4 | The hold clears when the leader mails a `result` or releases it. | — |
| D5 | A leader reply that arrives after its run already ended `done` or `lost` still reaches the caller as a late-result message; it is never dropped. | — |

### Agent's Discretion

The command's exact flags, the storage shape, the poll cadence while a hold is live, and how a late reply is detected and framed for each caller kind, within D1-D5.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| hold | A leader's own declaration that work it handed out is still running, with a reason and an expiry |
| live child | A Paseo agent whose `ParentAgentId` is the leader, not archived, status `running` or `initializing` |
| late reply | Leader output produced in a turn that ended after the leader's run had already ended |

## Read set

### Knowledge

| Concept | Grade at read | Why it matters |
|---------|---------------|----------------|
| docs/knowledge/patterns/paseo-leader-dispatch.md | unverified | Run lifecycle, idle sweep, follow-ups |
| docs/knowledge/patterns/wd-direct-leader.md | unverified | Mail kinds, tasks table, heartbeat nudge |

### Code

| Path | What it decides |
|------|-----------------|
| `crates/waggledance/src/delivery.rs` | `follow_turn` ends a run done on turn end (line 674); `sweep_leaders` archives idle leaders (216-262); `end` notifies callers (924) |
| `crates/waggledance/src/heartbeat.rs` | `beat` and `nudge` nudge an idle silent wd leader (182-262) |
| `crates/waggledance/src/paseo.rs` | `AgentState`, the `Paseo` trait, `wd_task` leader prefix (268) |
| `crates/waggledance/src/mail.rs` | `mail send` kinds and validation |
| `crates/waggledance-core/src/repository.rs` | `MIGRATIONS` (last is 11) |
| `crates/waggledance-core/src/config.rs` | `[paseo]` settings, `leader_idle_minutes` |

## Specific Ideas And References

- Live probe 2026-11-11: a Claude haiku agent on Paseo launched a background subagent running `sleep 180` and ended its turn; Paseo reported `idle` 11 s later, an interim notification woke it once at 09:56:14, and it woke on its own at 09:59:12 with the final answer. Polling status every 5 s never saw `running` during either wake turn; `UpdatedAt` and the logs did change.
- Paseo sets `PASEO_AGENT_ID` in every agent's environment, so `waggledance hold` run by the leader names its leader without a run id.
- Prior art: retired decision `c6881832` (working mail held a task open until result).

## Outstanding Questions

### Deferred To Planning

- [ ] How the daemon waits while a hold is live without a busy loop — read `Paseo::wait` semantics for an already idle agent.
- [ ] Whether Paseo archive kills an in-process background subagent — unverified; D3 makes it moot while a hold is live.

## Deferred Ideas

- Setting Claude Code `crossSessionInbound` on this machine, so no leader can type into another Claude session through its control socket — separate backlog item; bee steering may use that channel.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable.
