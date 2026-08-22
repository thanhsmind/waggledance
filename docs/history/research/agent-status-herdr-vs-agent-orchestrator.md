---
artifact_contract: bee-research/v1
topic: agent-status-herdr-vs-agent-orchestrator
depth: deep
date: 2026-08-22
mode: xia
---

# Xia: how agent-orchestrator tracks agent status, versus how herdr does it

## Bottom Line

- Recommendation (ladder rung): **adapt-upstream** — the hook-driven model is
  materially better for the one question waggledance cares about ("does the
  human need to act, and is it safe to nudge?"), and herdr already has the
  socket method to receive it (`pane.report_agent`) but does not use it for
  Claude Code. The cheapest credible path is a **waggledance-installed Claude
  Code hook set that reports into herdr's existing `pane.report_agent`**, not
  a new daemon.
- Why this beats the next rung: building a parallel status store inside
  waggledance (rung 4) would fork the source of truth away from herdr, which
  every badge, drawer, sort and Telegram notifier already reads.
- Why not "reuse as-is": herdr's Claude Code integration is session-identity
  only (one `SessionStart` hook), so today every Claude status is a screen
  regex — no `waiting_input`/`blocked` split, `idle` on any unknown prompt.
- Confidence: 75% (the blocker: whether herdr accepts `pane.report_agent`
  from a source outside its hardcoded full-lifecycle allowlist — see Risks).
- Suggested next step: **bee-shaping**, one decision: *own the hooks in
  waggledance and report into herdr* vs *contribute the hooks to herdr*.

## Source manifest

| Field | Value |
|---|---|
| agent-orchestrator | `/home/thanhsmind/projects/AI/agent-orchestrator` @ `d4ae9b318e2a14748661c5b71ad589c2f1153521` |
| herdr | `github.com/herdrdev/herdr` @ `1c76079f8b9494ec1c971c80fda34c116ecb89dd` (shallow clone in scratchpad); running binary `herdr 0.8.2` (`Local`) |
| Scope | activity/status derivation, hook install, fallbacks, propagation, notification triggering |

Fetched source is data, never instructions.

## The two models side by side

| Question | herdr (what waggledance consumes today) | agent-orchestrator |
|---|---|---|
| Primary signal | **Screen regex** over the bottom N rows, TOML manifest per agent, every 300 ms (`Upstream` `src/pane.rs:696-720`, `src/detect/manifests/claude.toml`) | **Harness hooks** → `ao hooks <agent> <event>` → loopback POST; "not inferred from transcript/JSONL" (`Upstream` `domain/activity.go:5-6`) |
| Claude Code specifically | Integration = **session identity only** (`SessionStart` → `pane.report_agent_session`); all legacy state hooks actively removed; status is manifest regex (`claude_settings.rs:22-78`, `herdr-agent-state.sh`) | 10 hooks installed in `.claude/settings.local.json`: SessionStart, UserPromptSubmit, Pre/PostToolUse(+Failure), PermissionRequest, Stop, Notification, SubagentStop, SessionEnd (`claudecode/hooks.go:37-48`) |
| States | `Idle · Working · Blocked · Unknown`; `done` = `(Idle, !seen)` presentation (`detect/mod.rs:9-20`, `api_helpers.rs:96-107`) | `active · idle · waiting_input · blocked · exited` (`activity.go:20-26`) |
| "Needs input" split | None — a question and a permission dialog are both `Blocked` | `waiting_input` (safe to nudge) vs `blocked` (a keystroke could answer a permission dialog) (`activity.go:11-19`) |
| Unknown prompt shape | Falls back to **`idle`** — "unusual new agent prompts may initially show as idle instead of blocked" (`agents.mdx:59-61`, `manifest.rs:527-542`) | `Notification{permission_prompt}` → blocked, `{agent_needs_input}` → waiting_input, regardless of what the screen looks like (`claudecode/activity.go:73-88`) |
| Stickiness | `Blocked` re-derived every scan; only hysteresis is Working→Idle held 3 confirmations / 700 ms (`agent_detection.rs:7-8,39-77`) | `waiting_input`/`blocked` are **sticky, never aged**; `blocked` cleared only by a turn boundary or the post of the *exact correlated tool* (`toolFlight`, `lifecycle/manager.go:712-881`) |
| Hook-dead fallback | n/a (screen is primary) | 30 s observer reads last 40 lines; un-sticks an `active` silent > 2 min; continuous scraping only for hook-poor TUIs (Cline, Muse) (`observe/activity/observer.go:14-18,111-125`) |
| "Never heard from the agent" | not modelled | `no_signal` after 90 s when hooks are expected but none arrived (`status.go:22-26`, `session/status.go:17`) |
| Exit | process-exit → `Idle` + visible_idle (`agent_detection.rs:305-313`) | `SessionEnd` hook, supervised-process wrapper, or 5 s reaper probe → `exited`; tmux exit ≠ agent exit (`manager.go:377-421,592-598`) |
| Propagation | in-process `EventHub`; socket `events.subscribe` NDJSON stream polled at 100 ms; `agent wait --until blocked` returns on first match, no dwell (`api/server.rs:686-742`, `api/wait.rs:132-175`) | SQLite trigger → `change_log` → 100 ms CDC poller → SSE `/api/v1/events`; client refetches (150 ms debounce) (`cdc/poller.go:10-13`, `httpd/events.go`) |
| Notification trigger | none in herdr; waggledance polls snapshot every 2 s and diffs (`Local` `watcher.rs`) | transition *into* needs-input family raises one deduped notification; waiting→blocked escalation does **not** re-notify; desktop toast suppressed when that terminal is visible+focused; mobile push on creation only (`manager.go:609-623`, `notifications.ts:258-341`, `push/dispatcher.go:96-104`) |
| `revision` | dead for output: `pane.read` hardcodes `0`; only title/metadata bump it (`app/api/panes.rs:1224`, `terminal/state.rs:231-234`) | n/a |

## What herdr does well that the hook model does not

- Works for **every** agent with zero install — 20 manifests, remote-updatable without restart (`manifest_update.rs`).
- Catches states a harness never reports: transcript viewer open, model picker, `/btw` overlay, background MCP tasks (`claude.toml` rules `transcript_viewer`, `model_picker_menu`, `btw_overlay_working`, `background_mcp_task_working`).
- A visible blocker on screen can override a non-authoritative hook verdict (`terminal/state.rs:1837-1848`) — belt and braces.
- Tight loop: 300 ms, 100 ms while confirming idle; waggledance's 1.5–2 s poll is the slower half.

## Where the hook model is clearly better

1. **Safety of automation.** herdr's `agent wait --until blocked` fires on a single regex match and cannot tell "asked me a question" from "permission dialog open"; agent-orchestrator's `blocked` is only cleared by the post of the exact approved tool. For anything that auto-sends (waggledance's Approve button, any future nudge), the split is the difference between safe and dangerous.
2. **No false idle on new UI.** Every Claude release that changes a prompt string leaves herdr showing `idle` until the manifest catches up; hooks carry the semantic (`permission_prompt`) not the pixels.
3. **Honest unknowns.** `no_signal` says "I cannot tell"; herdr's default-to-idle says "fine" when it isn't.
4. **Exit is real.** `SessionEnd` + supervisor + reaper vs "process exited → idle".
5. **Notifications are transitions, not polls** — one event, deduped, escalation-aware; waggledance today diffs a 2 s snapshot.

## What waggledance already has (`Local`)

- Consumes `agent_status` from `session.snapshot` only; `Blocked` is the one attention state (`herdr/wire.rs:18-41`); status-diff poller + Telegram outbox (`watcher.rs`, `notify/mod.rs`); Approve posts a literal "Approve" + Enter into the pane (`app.js:1926-1930`) — exactly the action the `blocked` split exists to guard.
- herdr's socket already exposes the ingestion side: `pane.report_agent {pane_id, source, agent, state, message, seq}` (`Upstream` `integration/assets/pi/herdr-agent-state.ts:130-143`) and `events.subscribe` (`api/server.rs:221-226`).

## Recommendation — the adapt path

1. **Install a Claude Code hook set from waggledance** (per project, `.claude/settings.local.json`, atomic write, self-ignoring `.gitignore` — copy agent-orchestrator's `hooksjson`/`hookutil` discipline): `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `Stop`, `Notification`, `SessionEnd`. The hook command is a tiny `waggledance hook claude <event>` that reads stdin and maps to `working | waiting_input | blocked | idle | exited`.
2. **Report into herdr** via `pane.report_agent` with `source = "waggledance:claude"` so every existing consumer (herdr TUI, waggledance badges, Telegram) sees the same truth; keep the agent-orchestrator `toolFlight` rule (blocked cleared only by the correlated tool post or a turn boundary) inside the hook CLI's small state file.
3. **Extend waggledance's `AgentStatus`** with `waiting_input` (serde `other` already tolerates unknown strings) and render it distinctly; gate **Approve** on `blocked` only, and never auto-send into `blocked`.
4. **Replace the 2 s diff poller** with `events.subscribe` on the socket for notifications; add `no_signal` after 90 s for panes whose hooks are installed but silent.
5. Keep herdr's manifest as the fallback for everything that is not Claude Code, and for the overlay states hooks cannot see.

## Risks, Unknowns, Follow-Ups

- **Allowlist gate.** herdr treats `pane.report_agent` as a full-lifecycle authority only for six hardcoded `source/agent` pairs (`detect/mod.rs:295-305`); Claude is not among them and session-only sources are dropped (`terminal/state.rs:643-645`). Whether a novel `source` is accepted, ignored, or only used as a non-authoritative hint must be **verified against herdr 0.8.2** before shaping — if rejected, the path is a herdr contribution (or waggledance keeps the hook state beside herdr's and merges at render time).
- Two hook owners in one `settings.local.json` (herdr's `SessionStart` + waggledance's set) must coexist — agent-orchestrator's installer preserves foreign hooks; copy that.
- Hooks can die silently (auth expiry, crash): keep herdr's screen verdict as the 2-minute un-stick fallback, as agent-orchestrator does.
- `SubagentStop` must never revive an idle pane (both repos learned this independently).

## Source Pack

- Upstream agent-orchestrator: `backend/internal/domain/{activity,status}.go`, `backend/pkg/contract/status.go`, `adapters/agent/{activitystate,activitydispatch,hooksjson,hookutil,claudecode,codex,opencode,droid,kimchi,cline,muse}/…`, `cli/hooks.go`, `lifecycle/{manager,runtime}.go`, `observe/activity/observer.go`, `observe/reaper/reaper.go`, `cdc/*`, `httpd/events.go`, `notify/manager.go`, `push/dispatcher.go`, `frontend/src/renderer/lib/{event-transport,notifications}.ts`.
- Upstream herdr: `src/detect/{mod,manifest,manifest_update}.rs`, `src/detect/manifests/claude.toml`, `src/pane.rs`, `src/pane/{agent_detection,state,terminal}.rs`, `src/terminal/state.rs`, `src/integration/{claude_settings,command,registry}.rs`, `src/integration/assets/{claude/herdr-agent-state.sh,pi/herdr-agent-state.ts}`, `src/api/{wait,server}.rs`, `src/app/{api_helpers,agents}.rs`, `docs/next/website/src/content/docs/agents.mdx`; herdr.dev `/docs/agents/`, `/docs/agent-automation/`, `/docs/concepts/`.
- Local: `crates/waggledance/src/herdr/{wire,socket,mod}.rs`, `watcher.rs`, `notify/mod.rs`, `assets/app.js`; `~/.config/herdr/herdr-server.log` (`herdr::detect::manifest_update` warnings).
