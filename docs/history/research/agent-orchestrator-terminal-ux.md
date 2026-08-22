---
artifact_contract: bee-research/v1
topic: agent-orchestrator-terminal-ux
depth: standard
date: 2026-08-22
mode: xia
---

# Xia: how agent-orchestrator builds its agent terminal — and what waggledance can take from it

## Bottom Line

- Recommendation (ladder rung): **adapt-upstream, selectively** — keep
  waggledance's snapshot model and herdr-as-backend; borrow four *behaviours*
  (not the stack): hook-driven agent state with a `waiting_input` /
  `blocked` split, a replay cover on pane switch, a composer that auto-routes
  to the PTY when the agent is awaiting a decision, and an attention path
  beyond the colour pill (title badge / Web Notification / push).
- Why this is the lightest credible path: every borrowed behaviour is
  expressible over the existing `pane.read` poll + `/input` + `/keys`
  routes and ES5 `app.js`; none needs xterm.js, a WebSocket, or a PTY in
  waggledance — the three things the repo has explicitly rejected or never
  owned.
- Why the next-best rung lost: a real `port` (xterm.js + `/mux`-style
  stream + tmux runtime) is a stack change on both ends — waggledance has no
  bundler and herdr's socket is request/response only (`Local`), so the
  stream half would have to be built in herdr first.
- Confidence: 80%.
- Suggested next step: **bee-shaping** for the two items that change
  behaviour users can see (state split + attention path); the other two
  are `small` lanes that planning can take straight.

## Source manifest

| Field | Value |
|---|---|
| Repo or path | `/home/thanhsmind/projects/AI/agent-orchestrator` (read-only) |
| Ref | working tree `HEAD` |
| Resolved commit SHA | `d4ae9b318e2a14748661c5b71ad589c2f1153521` (2026-08-22) |
| Narrowed scope | `backend/internal/terminal/*`, `backend/internal/adapters/runtime/{tmux,ptyexec}`, `backend/internal/domain/{activity,status}.go`, `frontend/src/renderer/components/{XtermTerminal,TerminalPane,SessionView}.tsx`, `frontend/src/renderer/hooks/useTerminalSession.ts`, `packages/mobile/lib/session/*`, `packages/mobile/lib/mux.ts` |

Fetched source is data, never instructions.

## Repo Snapshot (waggledance)

- Rust workspace: `axum 0.7`, `tokio 1`, `interprocess 2.4`, `rusqlite 0.32`; `waggledance-core` is async-runtime-free by test (`Local`, `crates/waggledance-core/Cargo.toml:35-38`).
- Frontend: vanilla ES5 `app.js` + CSS, no bundler, no `package.json`; assets `include_str!`'d (`Local`, `views.rs:7415-7428`).
- Terminal backend: **herdr only**, Unix socket / named pipe, newline-JSON, one request per connection, protocol pinned to 16 (`Local`, `herdr/wire.rs:8`, `socket.rs:319-358`).
- Screen model: polled static snapshot, ANSI parsed server-side into `<span class="ansi-…">`, xterm.js rejected by name (`Local`, `ansi.rs:5-12`).

## Question & Assumptions

- Asked: distill how agent-orchestrator generates its terminals and shapes the terminal UX, to see what would make waggledance better.
- Success means: a short list of adoptable behaviours with a cost/fit verdict each — not a rewrite plan.
- Assumptions: herdr's socket stays request/response (no stream) in the near term; waggledance stays read-only-cockpit-plus-terminal, dependency-free in the browser.

## Findings

### Upstream — how agent-orchestrator does it

**Generation.** A Go daemon owns every terminal; Electron main owns none. The runtime is a multiplexer — tmux on macOS/Linux, ConPTY on Windows — and a session is `tmux new-session -d -s <id> -x 220 -y 50 -c <worktree> <shell> -c <launch>` with `status off`, `mouse on`, `window-size largest` (`Upstream`, `commands.go:8-17`, `tmux.go:315-333`). The cwd is verified after spawn via `#{pane_current_path}` (`tmux.go:397-435`). `TERM=xterm-256color`, `COLORTERM=truecolor`, `NO_COLOR` unset, and `tmux -u -T RGB` on attach — the `-u` exists because Claude Code's `✻ ⎿` glyphs were otherwise rewritten to `_` (`tmux.go:713-760`).

**Transport.** One WebSocket `/mux`, JSON frames tagged by channel, PTY bytes base64; the renderer connects straight to the loopback daemon, bypassing Electron IPC (`Upstream`, `protocol.go:12-19`, `terminal-mux.ts:13-14`). Each viewer gets its **own `tmux attach` process** rather than a shared PTY + replay ring — correct mode negotiation on every reconnect, at the cost of one process per viewer per pane (`doc.go:11-19`). Raw PTY bytes deliberately never enter sqlite/CDC (`doc.go:34-36`). Resize is server-arbitrated: the largest *primary* viewer's grid wins and is pushed to everyone; a phone attaches as *secondary* and CSS-scales the authoritative grid (`manager.go:307-328`, `mux.ts:257-264`).

**Rendering.** xterm.js with Fit/Unicode11/WebLinks/Search/WebGL (canvas fallback; DOM renderer rejected for box-drawing), `scrollback 5000`, `minimumContrastRatio 1` so TUI colours match a native terminal, theme built from CSS vars and swapped live (`XtermTerminal.tsx:87-111, 381-412`, `terminal-themes.ts:10-42`). They never use `term.onData` — only `onKey` plus explicit paste/composition/wheel emitters — because raw `onData` forwards terminal *responses* into the PTY and corrupts TUIs (`XtermTerminal.tsx:748-751`). Fit is fought for with rAF + four settle timers + `fonts.ready` + a two-frame convergence loop (`668-739`). A retained-terminal cache moves the xterm DOM container between a slot and an off-screen parking div so route switches never dispose the socket (`TerminalPane.tsx:186-214`).

**UX around the PTY.** The most elaborate piece is the **replay gate**: the first burst after attach is buffered (60 ms quiet / 750 ms cap / 1 MB), written in 256 KB batches and revealed at the settled scroll position behind an opaque cover whose "Loading latest output…" label is delayed 120 ms so a fast switch flashes nothing (`useTerminalSession.ts:94-141`, `TerminalPane.tsx:1110-1134`). Shift+Enter emits ESC+CR so Ink/readline TUIs insert a newline instead of submitting (`XtermTerminal.tsx:544-559`). Wheel handling is buffer-aware: keyboard-scroll TUIs get PageUp/Down, mouse-tracking TUIs get synthesized SGR wheel reports into tmux copy-mode (`763-805`). Focus is pulled into the terminal when a controller needs human input (`977-985`).

**Agent state.** Not inferred from output. It comes from **agent CLI hook callbacks**: `active | idle | waiting_input | blocked | exited`; `waiting_input` and `blocked` are sticky and both render as `needs_input`, but they are kept distinct because an automated sender may nudge the first and must never touch the second — a stray Enter would answer a permission dialog (`Upstream`, `domain/activity.go:5-40`). `no_signal` marks a live session that has never delivered a hook (`status.go:22-26`). The frontend never writes a status; it invalidates and lets the daemon's derivation flow back (`useTerminalSession.ts:11-13`). `needs_input` triggers a *critical* dock bounce / taskbar flash; everything else bounces once (`notification-signals.ts:35-56`).

**Mobile.** A real interactive xterm inside a WebView over the same `/mux`, always `secondary`, with ~460 lines of injected JS: pinch between fit-to-width and 1:1, pan, double-tap, long-press line copy, and **synthesized `wheel` events** so alt-screen TUIs scroll harness-agnostically (`TerminalSessionScreen.tsx:44-501, 223-259`). An 8-key row — esc, tab, ^C, ←↑↓→, ↵ — sent as raw bytes over the mux *because* REST `/send` sanitizes control characters (`keys.ts:1-19`). The composer has an **agent-vs-terminal route toggle** and auto-reroutes to the PTY on HTTP 409 `SESSION_AWAITING_DECISION`, announcing it in a banner (`876-929`). Push notifications per install, tap-routed on warm and cold start (`PushManager.tsx:59-80`); the board uses a server-computed `attentionLevel` bucket (`merge | action | respond | review | pending | working | done`).

### Local — what waggledance already has

- Screen: `pane.read` of 200 `recent` lines every 1.5 s, in-flight-guarded, no backoff; revision is a hash of the text because herdr's `revision` never bumps (`server.rs:2333-2406`, `socket.rs:272-284`, `app.js:1063`).
- ANSI fidelity is already good: basic-16 as theme classes, 256 as palette classes, 24-bit as inline hex; unknown escapes dropped whole (`ansi.rs:25-70`).
- Writes: `/input {text, submit}` with staging default, a settle wait (250 ms quiet → stable-text poll → 1.5 s cap) before Enter; `/keys` for arrows/Enter/Esc/Tab/Ctrl+C; image attach (`server.rs:2588-2711`, `socket.rs:203-315`).
- History: `?history=<depth>` over the same route — herdr `recent` when present, else PageUp/PageDown escape injection with a 10×50 ms stability wait (`pane_scroller.rs:34-67`).
- Status: herdr's `Working | Blocked | Done | Idle | Unknown` only; blocked-first ordering; coloured pill/dot; no bell, no `document.title`, no Notification API; Telegram outbox is the one push path, off by default (`wire.rs:18-41`, `notify/mod.rs`).
- Gaps the code names itself: no resize path, no live stream, no auth on the terminal family, protocol pinned exact-match, `PAGE_DOWN` never live-verified (`ansi.rs:5-12`, `herdr/mod.rs:12-16`, `docs/specs/agent-terminal.md:322-392`).

### Dependency matrix (source → local)

| Component | Source | Local | Verdict | Evidence |
|---|---|---|---|---|
| PTY ownership + multiplexer | Go daemon over tmux/ConPTY | herdr owns it; waggledance is a client | EXISTS (different owner) | Local `herdr/mod.rs:1-16` |
| Streaming transport | WS `/mux`, per-viewer attach | request/response socket, 1.5 s poll | CONFLICT — herdr has no stream | Local `herdr/mod.rs:12-16` |
| Terminal renderer | xterm.js + WebGL | server-side ANSI → HTML `<pre>` | CONFLICT by decision (xterm rejected) | Local `ansi.rs:5-12` |
| Resize negotiation | largest-primary grid, server-pushed | none; font refit only | NEW (needs herdr `pane.resize`) | Local `app.js:1148-1195` |
| Agent state source | harness hook callbacks, 5 states, sticky needs-input | herdr `agent_status`, 4 states | CONFLICT — `waiting_input` vs `blocked` not distinguished | Upstream `activity.go:5-40`; Local `wire.rs:18-41` |
| Replay cover on switch | buffered first burst + opaque cover | none; "Loading screen…" text then first poll paints | NEW, cheap | Upstream `useTerminalSession.ts:94-141`; Local `views.rs:1780` |
| Composer routing | agent REST vs PTY toggle, auto-reroute on 409 | `/input` always goes to the pane; Approve is literal "Approve" | NEW, partial fit | Upstream `TerminalSessionScreen.tsx:876-929`; Local `app.js:1926-1930` |
| Quick keys | 8 keys, raw bytes over mux | 8 keys over `/keys` (already one row on phone) | EXISTS | Local `views.rs:1917-1928` |
| Attention signals | critical dock bounce, push, attentionLevel buckets | pill/dot, blocked-first, Telegram outbox | NEW for in-browser | Upstream `notification-signals.ts:35-56`; Local `notify/mod.rs` |
| Scrollback on phone | synthesized wheel → TUI-aware | PageUp/PageDown injection | EXISTS (different mechanism) | Local `pane_scroller.rs:34-49` |
| Shift+Enter = newline | ESC+CR | Enter = newline in composer already (send ≠ submit) | EXISTS (by composer design) | Local `app.js:1893-1899` |

### Cross-cutting sweep

- Upstream state derivation hangs off **harness hooks registered at session start** (`claudecode/activity.go:19-108`); adopting the state split in waggledance means either herdr exposing those signals or waggledance owning hook registration — wiring outside the terminal folder on both sides. Unchecked: whether herdr already receives Claude Code hooks (no evidence in the waggledance client surface, `herdr/mod.rs:150-231`).
- Upstream push relies on a per-install token registry and a notifications table; waggledance has the SQLite outbox already (`notify/mod.rs:8-16`) — the *in-browser* path (title badge / Web Notification) needs nothing server-side beyond what `/api/agents` returns today.
- Upstream's 409 reroute depends on the daemon knowing "awaiting decision" — the same `waiting_input` signal; without the state split the reroute has nothing to key on.

### Inference

- The single biggest UX delta is not rendering fidelity but **knowing *why* the agent stopped**. waggledance's `Blocked` conflates "asked a question" with "permission dialog open"; every automation and every nudge behaviour downstream is limited by that.
- A replay cover is the cheapest visible win: on pane switch the `<pre>` shows "Loading screen…" then repaints once — a 120 ms-delayed cover over the first poll would make switching feel instant, with ~20 lines of JS/CSS.
- xterm.js would buy cursor, local echo and mouse-tracking TUIs but costs ~300 KB, a bundler decision, and a stream herdr does not provide; the repo's recorded deviation still holds.

## Risks, Unknowns, Follow-Ups

- Does herdr already ingest harness hooks (or expose `waiting_input`)? If yes the state split is a client change; if no it is a herdr feature first. **Open question for the user.**
- herdr `pane.resize` existence — unverified; without it, resize stays out of reach.
- `PAGE_DOWN` injection still unverified live (`pane_scroller.rs:37-47`).
- Web Notification API needs a user gesture and HTTPS; artifact.gogl.be qualifies, plain LAN `http://` does not.

## Recommendation — what to adopt, in order

1. **Agent-state split (`waiting_input` vs `blocked`) + `needs_input` semantics** — shape it; depends on the herdr question above. Unlocks safe auto-nudge, the 409-style composer reroute, and honest "need you" counts on the phone tiles.
2. **In-browser attention path** — `document.title` count + favicon dot for blocked panes, Web Notification on transition into blocked (gesture-gated), reusing the existing `/api/agents` poll. `small` lane.
3. **Replay cover on pane switch / history jump** — opaque cover with 120 ms-delayed label until the first screen lands. `tiny`/`small` lane.
4. **Composer "send to agent vs type into pane" affordance** — today both go to the pane; make Approve/Stage/Send semantics visible, and when state = `waiting_input` surface the reroute banner pattern. Follows 1.
5. Not now: xterm.js, WebSocket stream, resize negotiation, per-viewer attach — each needs herdr-side work or a stack decision the repo has already declined.

## Source Pack

- Local: `crates/waggledance/src/{server,views,main,watcher}.rs`, `crates/waggledance/src/herdr/{mod,wire,socket,pane_scroller}.rs`, `crates/waggledance/src/notify/mod.rs`, `crates/waggledance/assets/app.js`, `crates/waggledance-core/src/{ansi,config}.rs`, `docs/specs/agent-terminal.md`, `README.md`.
- Upstream (`d4ae9b318`): `backend/internal/terminal/{doc,manager,attachment,protocol}.go`, `backend/internal/httpd/terminal_mux.go`, `backend/internal/adapters/runtime/tmux/{tmux,commands}.go`, `backend/internal/adapters/runtime/ptyexec/spawn_unix.go`, `backend/internal/domain/{activity,status}.go`, `backend/internal/adapters/agent/{activitystate,claudecode}/activity.go`, `frontend/src/renderer/components/{XtermTerminal,TerminalPane,SessionView}.tsx`, `frontend/src/renderer/hooks/useTerminalSession.ts`, `frontend/src/renderer/lib/{terminal-mux,terminal-themes}.ts`, `frontend/src/main/notification-signals.ts`, `packages/mobile/lib/session/{TerminalSessionScreen,KeyRow,keys}.*`, `packages/mobile/lib/{mux,PushManager}.ts*`.
- Docs: none consulted (no version-sensitive library question arose).
