---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: Paseo Leader Mode

## Summary

A leader that waggledance opens through Paseo now starts in `bypassPermissions`
unless the project or the host config names another mode. Waggledance then reads the
mode the agent really runs in, sets it again once if it is wrong, and says so in the
daemon log and the delivery reply if it is still wrong. The same check runs before a
task goes to a leader that already exists, so a leader stuck in "Always Ask" is
repaired on its next task.

Mode: `standard` — 3 risk flags: external-systems, public-contracts, covered-contract-change
Why this is the least workflow that protects the work: one code cell over the files
the leader path already lives in, one docs cell, and one live walk against the real
Paseo daemon, because the bug only shows in Paseo's runtime state.

## Requirements (from CONTEXT.md)

- D1: a new leader always gets an explicit `--mode`.
- D2: mode = `.waggledance.toml` `[paseo] mode`, else global `[paseo] mode`, else
  `bypassPermissions`; global `mode = ""` leaves it to Paseo and skips the check.
- D3: after create, read `runtimeInfo.modeId`; on mismatch run
  `paseo agent mode <id> <mode>` once and read again; still wrong → daemon-log warning
  plus a reply line naming agent, asked and runtime mode; the task is still delivered.
- D4: the same check before a send to an existing leader.
- D5: the delivery leader path is the only Paseo create site; nothing else changes.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command run, output kept).

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | The CLI's inspect `Mode` is the configured mode, not the runtime one, so it cannot be the check | ran | `sed -n 92p ~/.local/share/mise/installs/node/24.19.0/lib/node_modules/@getpaseo/cli/dist/commands/agent/inspect.js` | `Mode: snapshot.currentModeId ?? "default",` |
| 2 | Paseo's agent record carries the runtime mode and matches the live value | ran | `python3 -c 'json.load(open("~/.paseo/agents/home-thanhsmind-Projects-goglbe-beehive/ff24c29b-….json"))'` and MCP `get_agent_status ff24c29b` | `.runtimeInfo.modeId = default` / `"runtimeInfo":{…"modeId":"default"…},…"currentModeId":"bypassPermissions"` |
| 3 | `paseo agent mode <id> <mode>` calls the daemon's setAgentMode, the call that fixed the leader by hand | ran | `grep -n "await client.setAgentMode" …/@getpaseo/cli/dist/commands/agent/mode.js` | `await client.setAgentMode(resolvedId, normalizedMode);` |
| 4 | The two CLIs on this host background `run` with different flags, so the start must stay a waiting `run` | ran | `paseo run --help` (AppImage 0.12.0) and `@getpaseo/cli/dist/commands/agent/run.js:281` (npm 0.10.3) | `--no-wait  Return immediately without waiting for` / `return Boolean(options.background \|\| options.detach);` |
| 5 | Today `--mode` is passed only when non-empty and the default is empty | read | `crates/waggledance/src/paseo.rs:250`, `crates/waggledance-core/src/config.rs:41` | `("--mode", &spec.mode),` / `mode: String::new(),` |
| 6 | A new leader is started by one blocking call in delivery, the only create site | read | `crates/waggledance/src/delivery.rs:324` | `.run_and_wait(&start.spec(&run.task), timeout)` |
| 7 | waggledance already reads a Paseo store file read-only under `PASEO_HOME` or `~/.paseo` | read | `crates/waggledance/src/paseo.rs:110-114` | `.or_else(\|\| dirs::home_dir().map(\|h\| h.join(".paseo")))` |
| 8 | Existing tests assert the empty mode default | read | `crates/waggledance-core/src/config.rs:971` | `c.paseo.provider.is_empty() && c.paseo.mode.is_empty() && c.paseo.thinking.is_empty()` |

## Discovery

Read the leader path (`paseo.rs`, `delivery.rs`, `config.rs`), the Paseo CLI's `run`,
`inspect` and `mode` commands, and Paseo's agent records under `~/.paseo/agents/`. The
beehive leader `ff24c29b` is still in runtime mode `default` today, so the bug is live.

## Approach

Recommended (D1–D4):

1. `PaseoConfig::default().mode` becomes `"bypassPermissions"` (D2). `resolve_leader`
   keeps its file-over-global rule; a pure `leader_mode(config, settings)` returns the
   mode for both new and existing leaders.
2. `Paseo` trait gains `runtime_mode(id) -> Result<Option<String>>` (reads
   `runtimeInfo.modeId` from `<PASEO_HOME or ~/.paseo>/agents/*/<id>.json`) and
   `set_mode(id, mode)` (`paseo agent mode <id> <mode> --json`).
3. `ensure_mode(paseo, id, mode, settle)` returns `Skipped` (empty mode), `Matched`,
   `Repaired`, or `Mismatch { runtime }`: read, poll briefly while the session settles,
   set once on mismatch, poll again.
4. Delivery, new leader: the waiting `run_and_wait` runs beside a finder that polls the
   project's labelled leaders for an id not known before the run, then runs
   `ensure_mode` on it. Existing leader: `ensure_mode` before `send_and_wait`. A
   `Mismatch` logs `tracing::warn!` and adds a header line to the reply; the task still
   goes through.

Rejected:

- Switch the start to a background `run`: the two installed CLIs disagree on the flag
  (claim 4).
- Trust `paseo inspect` `Mode`: it is the field that lied (claim 1).
- Talk to the Paseo daemon websocket or MCP: a new protocol client for one field the
  agent record already carries (claim 2).

Risk map:

| Component | Risk | Lands in | Proof |
|---|---|---|---|
| Agent record path/shape is Paseo-internal | MEDIUM | plm-1 | A missing record or field reads as `None`, which counts as mismatch → set once → reported, never a crash; live walk reads a real record |
| Finder races the run (id appears late) | MEDIUM | plm-1 | Fake-Paseo test where the leader id appears after several polls; the finder stops when the run returns |
| Default change to `bypassPermissions` | LOW | plm-1 | Asked for by the spec; `mode = ""` keeps the old behavior |

Waves: plm-1, then plm-2 (docs describe the shipped behavior), then the live walk.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "30ff876890293b1cea96673b722ea95a7b27780259af61e6c3b2be09a0c2b112",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "not-applicable", "role": "hat-facts-gaps", "reason": "clear spec with exact asks; threshold keeps the fast path (logged)"},
    {"stage": "hat-wave-alternatives", "classification": "not-applicable", "role": "hat-alternatives", "reason": "clear spec; SMALLER PATH checked inline"},
    {"stage": "hat-wave-user-impact", "classification": "not-applicable", "role": "hat-user-impact", "reason": "clear spec"},
    {"stage": "hat-wave-risks", "classification": "not-applicable", "role": "hat-risks", "reason": "not high-risk"},
    {"stage": "hat-wave-value", "classification": "not-applicable", "role": "hat-value", "reason": "not high-risk"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "standard lane, no high-risk consult"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "plm-1 is red-first Rust test-plus-code work; precedent a66a46c0 moved Rust cells to this seat"},
    {"stage": "docs", "classification": "required", "role": "docs", "reason": "plm-2 knowledge and README sync"},
    {"stage": "code", "classification": "not-applicable", "role": "code", "reason": "Rust cell runs on the test seat"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "every cell names its role"},
    {"stage": "review", "classification": "conditional", "role": "review", "condition": "the user invokes bee-reviewing", "reason": "agents-review-user-invoked"},
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

SMALLER PATH: only passing `--mode bypassPermissions` (asks 1–2) would be cheaper, but
D3 needs the read-back and D4 the existing-leader repair; claim 1 shows the cheap read
cannot see the bug. No cheaper shape honors D1–D4.

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| plm-1 | Start Paseo leaders in the asked mode and prove the runtime mode | `crates/waggledance-core/src/config.rs`, `crates/waggledance/src/paseo.rs`, `crates/waggledance/src/delivery.rs` | — | a new or reused leader runs in `bypassPermissions`; a leader that stays wrong is named in the reply and the log | `cargo test -p waggledance -p waggledance-core` green |
| plm-2 | Describe the leader mode default and runtime check | `docs/knowledge/patterns/paseo-leader-dispatch.md`, `README.md` | plm-1 | the docs say the default is `bypassPermissions` and what happens when the runtime mode is wrong | pointer/parity check: every changed sentence matches the code |

```json
[
 {"id":"plm-1","feature":"paseo-leader-mode","title":"Start Paseo leaders in the asked mode and prove the runtime mode","lane":"standard","role":"test","status":"open","deps":[],
  "decisions":["D1","D2","D3","D4","e833b46e-d5a7-4135-8d2d-9ad7b8fa2bd7"],
  "files":["crates/waggledance-core/src/config.rs","crates/waggledance/src/paseo.rs","crates/waggledance/src/delivery.rs"],
  "read_first":["docs/history/paseo-leader-mode/CONTEXT.md","docs/history/paseo-leader-mode/plan.md","crates/waggledance/src/paseo.rs","crates/waggledance/src/delivery.rs"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach steps 1-4. (1) PaseoConfig::default().mode becomes \"bypassPermissions\"; update the config default test (config.rs:971 and :994) to assert it; a global mode = \"\" still parses to empty. Add pub fn leader_mode(config, settings) -> String in paseo.rs (file value if non-empty, else global) and use it in resolve_leader. (2) Add to the Paseo trait: async fn runtime_mode(&self, id) -> Result<Option<String>> reading runtimeInfo.modeId from <PASEO_HOME or ~/.paseo>/agents/*/<id>.json (missing file or field = Ok(None), never an error; reuse the store_path home rule), and async fn set_mode(&self, id, mode) -> Result<()> running `paseo agent mode <id> <mode> --json` through exec. Implement both on PaseoCli and FakePaseo (FakePaseo: a scripted queue of runtime modes per id and Call::RuntimeMode/Call::SetMode records). (3) Add pub enum ModeCheck { Skipped, Matched, Repaired, Mismatch { runtime: Option<String> } } and pub async fn ensure_mode(paseo, id, mode, settle: Duration, poll: Duration) -> ModeCheck: empty mode -> Skipped with no calls; poll runtime_mode until it equals mode or settle elapses; on mismatch call set_mode once (a set_mode error counts as still mismatched, logged), poll again up to settle; return Repaired or Mismatch. (4) delivery.rs: for Leader::Start, record the known leader ids for the project (wd_leaders or leaders filtered as resolve_leader does) before the run, then run run_and_wait concurrently with a finder that polls those lookups every poll interval for an id not in the known set and runs ensure_mode on it; the finder ends when it has checked one id or when run_and_wait returns (if the run returned first, run ensure_mode on the returned id). For Leader::Existing (not busy) run ensure_mode before send_and_wait. On Mismatch: tracing::warn! naming run, agent, asked and runtime mode, and insert a line into self.state headers for the run: `[waggledance] leader <id> runs in mode <runtime or unknown>, asked <mode>; set it in Paseo` (append to any existing header with a newline). Delivery continues either way. Keep settle/poll injectable on Delivery so tests run fast (as permission_poll is). No code comments.",
  "must_haves":{"truths":["With no mode set anywhere the paseo run argv carries --mode bypassPermissions","A runtime mode that differs triggers exactly one paseo agent mode call","A mode still wrong after the set is named in the run reply header and the run still ends done","An existing leader is checked before the task is sent","Global mode empty skips the flag and the check"],"artifacts":[{"path":"crates/waggledance/src/paseo.rs","substantive":"ensure_mode, runtime_mode reading runtimeInfo.modeId, set_mode via paseo agent mode"}],"key_links":["delivery.rs Leader::Start and Leader::Existing both call ensure_mode"],"prohibitions":["Do not switch paseo run to --no-wait or --background","Do not read the mode from paseo inspect"]},
  "verify":"cargo test --profile fast -p waggledance-core && cargo test --profile fast -p waggledance",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"plm-2","feature":"paseo-leader-mode","title":"Describe the leader mode default and runtime check","lane":"standard","role":"docs","status":"open","deps":["plm-1"],
  "decisions":["D2","D3","D4","e833b46e-d5a7-4135-8d2d-9ad7b8fa2bd7"],
  "files":["docs/knowledge/patterns/paseo-leader-dispatch.md","README.md"],
  "read_first":["docs/history/paseo-leader-mode/CONTEXT.md","docs/knowledge/patterns/paseo-leader-dispatch.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"In paseo-leader-dispatch.md, replace 'mode and thinking unset are left to Paseo' with: mode defaults to bypassPermissions, global mode = \"\" leaves it to Paseo; thinking unset is left to Paseo. Add a short section 'Leader mode check' describing D3 and D4 (reads runtimeInfo.modeId from Paseo's agent record because paseo inspect shows the configured mode; sets once with paseo agent mode; reports in the daemon log and the reply header; delivery continues), with the ff24c29b incident as the why, and add paseo.rs/delivery.rs to Pointers if not there. In README.md near line 256 say the default mode is bypassPermissions. Match the code from plm-1 exactly.",
  "must_haves":{"truths":["The concept names bypassPermissions as the default and describes the runtime check"],"artifacts":[{"path":"docs/knowledge/patterns/paseo-leader-dispatch.md","substantive":"Leader mode check section"}],"key_links":[],"prohibitions":["No claim the code does not make"]},
  "verify":"rg -n 'bypassPermissions' docs/knowledge/patterns/paseo-leader-dispatch.md README.md",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":false}}
]
```

After plm-2 the leader runs the live walk: a fast-profile build, proven by content,
starts a real leader through the daemon path and reads its `runtimeInfo.modeId`.

## Test matrix

- Happy: no mode set anywhere → `run` argv carries `--mode bypassPermissions`; runtime
  mode already `bypassPermissions` → `Matched`, no `set_mode` call. Pass when the fake's
  call log has no `SetMode`.
- Happy: project file `mode = "acceptEdits"` → argv carries `--mode acceptEdits`.
- Edge: runtime mode `default` then `bypassPermissions` after one `set_mode` →
  `Repaired`. Pass when the call log has exactly one `SetMode(id, "bypassPermissions")`.
- Edge: global `mode = ""` → no `--mode` flag, no read, no `set_mode` (`Skipped`).
- Edge: the new leader's id appears only after several polls → the finder still checks it.
- Error: runtime mode stays `default` after `set_mode` → `Mismatch`; the run's reply
  header names agent id, asked `bypassPermissions`, runtime `default`; the run still ends
  `done`.
- Error: agent record missing → treated as mismatch, set once, reported; no panic.
- Existing leader with runtime `default` → `set_mode` before the `SendAndWait` call.
- Changed default: `config.rs` default test asserts `mode == "bypassPermissions"`; the
  same scenario on main asserts empty.

## Open Questions

(none)

## Out of scope

- `thinking` keeps paseo-auto-wiring D8 (passed only when set).
- Fixing Paseo's own create path that splits `currentModeId` from `runtimeInfo.modeId`.
- Repairing leaders that receive no new task (no sweep over idle leaders).
