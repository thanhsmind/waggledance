# supervisor-active-loop — plan

Lane: small. Class: feature. Decisions: `15cc4a55` (report to wd), `c505ac8d` (done
report), `83d65d2e` (advisor first), `4d03d229` (Sonnet), `221af324` (heartbeat
board).

## Problem

The user hands work to wd on OpenClaw. wd hands it to the waggledance supervisor, and
the supervisor opens a leader. After that the flow stops being active:

- `waggledance supervisor report` posts to OpenClaw `/hooks/agent`. That endpoint runs
  an isolated, fresh turn and delivers straight to Telegram. wd never sees what the
  supervisor said about the task wd handed off.
- The supervisor runs Haiku. On `t-39cfa3` it put a question to the user with no
  advisor step in its ledger.
- The heartbeat beats every 10 minutes but only talks to itself. Its list of tasks
  waiting on the user was wrong: `t-bd665a` still showed as open after its leader
  closed.
- A `result` mail can be a single envelope line, with no account of what was done.

## Shape

1. **Reports go to wd's main session.** `report` runs the live OpenClaw CLI:
   `openclaw agent --agent <agent_id> --message <framed report> --deliver --channel
   telegram --json`. The turn runs in `agent:<agent_id>:main`, so wd has the context
   of the task it handed off. The framing asks wd to check the report against that
   task and write the message the user should read; `--deliver` sends wd's reply to
   Telegram. The CLI is found from the running `openclaw-gateway` process: its
   `APPDIR` environment names the AppImage mount, and the CLI is
   `<APPDIR>/resources/openclaw/openclaw.mjs`. `[openclaw] cli` overrides that.
   With no gateway running, `report` fails, and the supervisor's ledger retry covers
   it; it never falls back to a local run. A new kind `board` carries the task board.
   The hook path leaves `report`.
2. **The supervisor's default provider is `claude/claude-sonnet-5-5`.**
3. **Supervisor instructions** (`docs/waggledance-supervisor-agent.md`):
   - A `result` mail must carry a done report: what was done, the proof (command and
     outcome), what is left, and where it lives. A result without one goes back to
     the leader.
   - Every question goes to an advisor first. A question goes up to wd only with an
     `advisor-undecided` ledger line for that question.
   - Each beat reconciles the ledger against the map. A task whose leader is closed or
     archived gets `stale-closed` and drops off the board. The beat sends `board` to
     wd when any task changed state since the last board, and every 2 hours while any
     task is open. The board lists every open task: project, step, waiting on whom,
     since when. Questions waiting on the user sit on the board with how long they
     have waited.
4. **wd's skill** (`integrations/openclaw/waggledance-orchestrator/SKILL.md`): how wd
   reads a supervisor report and a board, and what it sends the user.

## Cells

| Cell | Role | Files | Proof |
|---|---|---|---|
| sal-1 | code | `crates/waggledance/src/seat.rs`, `crates/waggledance-core/src/config.rs` | `cargo test --profile fast -p waggledance seat::` and `-p waggledance-core config::` |
| sal-2 | docs | `docs/waggledance-supervisor-agent.md`, `docs/knowledge/patterns/paseo-native-supervisor.md`, `integrations/openclaw/waggledance-orchestrator/SKILL.md` | `cargo test --profile fast -p waggledance seat::` (embedded doc) |

```json
[{"id":"sal-1","feature":"supervisor-active-loop","title":"Deliver supervisor reports into wd's main session through the OpenClaw CLI","action":"Replace the /hooks/agent POST in `waggledance supervisor report` with a run of the live OpenClaw CLI: `<cli> agent --agent <openclaw.agent_id> --message <framed report> --deliver --channel telegram --json`, adding `--reply-to <openclaw.telegram_to>` when set. Find the CLI from the running process whose comm is `openclaw-gateway`: read its APPDIR from /proc/<pid>/environ and use `<APPDIR>/resources/openclaw/openclaw.mjs`, run by `node` on PATH; `[openclaw] cli` in config overrides the discovery with a full command. No gateway process and no override: fail with a message naming both. Never pass --local. The framed message carries the tag `[waggledance#<corr>] <project> — <label>: <text>` and asks wd to check it against the task it handed off and write the message the user should read on Telegram, in Vietnamese, quoting numbers, paths and the leader's words verbatim, asking the user plainly when the report needs a decision. Add kind `board` (label `bảng việc`). Exit non-zero when the CLI exits non-zero or its JSON status is not ok. Remove hook_body and the HTTP post from report; leave the hook config fields and `setup openclaw` as they are. Change the supervisor default provider to claude/claude-sonnet-5-5 and update the tests that pin it.","lane":"small","role":"code","files":["crates/waggledance/src/seat.rs","crates/waggledance-core/src/config.rs"],"affects_skills":[],"affects_specs":["docs/knowledge/patterns/paseo-native-supervisor.md"],"acceptance":"Unit tests pin: the argv built for a report (agent id, deliver, channel, reply-to only when set, json, no --local); the framed message carrying the tag and the instruction; CLI discovery from a fake proc root with an openclaw-gateway entry and APPDIR, from the config override, and the refusal with neither; kind board accepted, unknown kinds refused; supervisor default provider is sonnet. Existing seat and config tests stay green.","verify":"cargo test --profile fast -p waggledance seat:: && cargo test --profile fast -p waggledance-core config::","read_first":["crates/waggledance/src/seat.rs","crates/waggledance-core/src/config.rs","docs/history/supervisor-active-loop/plan.md"],"must_haves":{"truths":["A supervisor report runs a turn in the wd agent's main session and never posts to /hooks/agent","report fails loudly when no OpenClaw gateway is running instead of running locally"]}},
{"id":"sal-2","feature":"supervisor-active-loop","title":"Teach the supervisor and wd the active loop","action":"In docs/waggledance-supervisor-agent.md: reports go to wd (not the user) and wd writes the Telegram message; a result mail must carry a done report (what was done, proof, what is left, where) or goes back to the leader; every question goes to an advisor first and goes up only with an advisor-undecided ledger line; each heartbeat reconciles the ledger with the map (stale-closed) and sends kind board when any task changed state since the last board, and every 2 hours while any task is open, listing every open task with project, step, waiting on whom and since when, questions waiting on the user included; add ledger states advisor-undecided, stale-closed, board-sent. Update the mail rule text so result requires the done report. Update docs/knowledge/patterns/paseo-native-supervisor.md to match. In integrations/openclaw/waggledance-orchestrator/SKILL.md add how wd handles a supervisor report and a board arriving in its session: check it against the task it handed off, write the user one clear Vietnamese message, keep the tag, never answer a leader's question on the user's behalf.","lane":"small","role":"docs","files":["docs/waggledance-supervisor-agent.md","docs/knowledge/patterns/paseo-native-supervisor.md","integrations/openclaw/waggledance-orchestrator/SKILL.md"],"affects_skills":[],"affects_specs":["docs/knowledge/patterns/paseo-native-supervisor.md"],"acceptance":"The supervisor doc carries the done report, advisor-first, reconcile and board rules and the new ledger states; the pattern doc and wd's skill match; the embedded-doc test is green.","verify":"cargo test --profile fast -p waggledance seat::","read_first":["docs/waggledance-supervisor-agent.md","integrations/openclaw/waggledance-orchestrator/SKILL.md","docs/history/supervisor-active-loop/plan.md"],"must_haves":{"truths":["A task the supervisor opened is followed by the heartbeat and shown to wd on a board until it closes","A question reaches wd only after an advisor said it cannot decide"]}}]
```

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | `openclaw agent --agent wd` runs in wd's main session and returns JSON | ran | probe 2026-10-18, `node <openclaw>/openclaw.mjs agent --agent wd --json` | `.status ok`, `.result.payloads[0].text PROBE-OK`, sessionKey `agent:wd:main` |
| 2 | The live CLI is found from the gateway process environment | ran | `/proc/1494049/environ` | `APPDIR=/tmp/.mount_AICowomoTthn`, comm `openclaw-gateway` |
| 3 | `/hooks/agent` always runs an isolated turn | ran | `rg -n -i isolated <openclaw>/docs/automation/webhook.md` | `94:- Runs an **isolated** agent turn (own session key)` |
| 4 | The report builds the hook body here | read | `crates/waggledance/src/seat.rs:205` | `pub fn hook_body(` |
| 5 | The supervisor default provider is Haiku | read | `crates/waggledance-core/src/config.rs:69` | `provider: "claude/claude-haiku-5-5".into(),` |
| 6 | The supervisor skipped the advisor on t-39cfa3 | read | `~/.waggledance/supervisor/ledger.md` | `t-39cfa3 … question-open` with no `advisor-open` line |

## Out of scope

- Removing the hook config fields, `hook.token` and the hook half of
  `setup openclaw`. They stay until nothing reads them.
- Moving the running supervisor to Sonnet. That is a deploy step after merge.
