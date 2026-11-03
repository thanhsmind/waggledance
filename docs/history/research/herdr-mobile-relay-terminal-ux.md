---
artifact_contract: bee-research/v1
topic: herdr-mobile-relay-terminal-ux
depth: deep
date: 2026-08-30
---

## Bottom Line

- **Recommendation (ladder rung): adapt-upstream.** Take the *behaviours* from
  herdr-mobile-relay, not its code. Nothing in it is importable — it is Go +
  a Svelte PWA against a Rust server-rendered app with no bundler — but it is
  the most thoroughly worked-out answer in existence to the exact question
  waggledance's Terminal tab asks, built on the *same* herdr socket, and it
  proves the architecture waggledance already chose is the right one.
- **Why this is the lightest credible path:** rung 1 (reuse) covers the screen
  poll, ANSI rendering, keys, attach and preset-start — all already built.
  It covers none of the six things the relay does that waggledance does not:
  structured approvals, masked secret prompts, stitched scrollback, live
  attention plumbing, a real screen revision, and command discovery. Rung 2
  (built-in) is empty — there is no framework here to lean on.
- **Why the next-best rung lost:** rung 4 (build from scratch) is what a
  waggledance-only design would do, and it would re-derive by trial the things
  the relay already learned the hard way — the 2-poll tolerance before tearing
  down a blocked UI, the 10s release grace on a size lease, the 2-confirmation
  refusal before overwriting an ambiguous history tail. Those constants are the
  research; taking them is free.
- **Confidence: 85%.** High on the inventory and the gaps (both sides read
  directly). Lower on effort estimates for the Rust/JS side, which no cell has
  costed yet.
- **Suggested next step: bee-shaping** — pick the slice from "Ranked adoption
  list" below. The whole list is a programme, not a feature.

## Source Manifest

| Field | Value |
|---|---|
| Repo or path | `/home/thanhsmind/Projects/refs/herdr-mobile-relay` (github.com/0cv/herdr-mobile-relay) |
| Ref | `main` |
| Resolved commit SHA | `7400537a429e754fc5af82ee9ac9d28f6e058c1d` (2026-08-29, v0.19.1) |
| Narrowed scope | Wire contract, mobile client display + interaction model, and the Go packages that make a *polled* remote terminal feel live. Excluded: transports (WebRTC/E2EE/gateway), self-hosting, updates, push infrastructure. |
| Mode | `xia` — distill and discuss; builds nothing |

## Repo Snapshot

- **Local (waggledance):** Rust workspace, 3 crates; herdr client at
  `crates/waggledance/src/herdr/` (6.9k lines across `mod.rs`, `socket.rs`,
  `wire.rs`, `pane_scroller.rs`); server-rendered HTML from `views.rs` (20.7k
  lines); one hand-written `assets/app.js` (3.7k lines), no bundler, no
  framework; ANSI→HTML in `waggledance-core/src/ansi.rs` (1.1k lines).
  `HERDR_PROTOCOL` pinned to 20. `Local`
- **Source (relay):** Go gateway (545 files) + Svelte 5 PWA built with Vite/Bun;
  36 internal packages; a JSON fixture contract (`contracts/fixtures/`) covering
  37 inbound commands and every outbound shape. `Upstream`
- **The decisive shared fact:** both talk to the *same* herdr over the *same*
  request/response socket, and **neither streams a PTY**. The relay polls a
  visible pane every 100–1000 ms (default 250 ms) and subscribes to herdr's
  `events.subscribe` for topology. waggledance polls every 1500 ms and has no
  event subscription. `Upstream` + `Local`

## Question & Assumptions

- **What was asked:** learn from herdr-mobile-relay — a project that has done
  the herdr-on-mobile connection very thoroughly — to improve how waggledance
  *displays* herdr and how a person *interacts* with it in the terminal surface.
- **What success appears to mean:** a ranked, evidence-backed list of what the
  relay does better, what waggledance already has, and what is worth taking —
  with the locked decisions it would touch named explicitly.
- **Assumptions still needing confirmation:**
  - That "terminal" means waggledance's Terminal/Transcript surface
    (`/p/:id/_terminal`, `/?tab=terminals`), not a TUI. Everything below
    assumes this. `Inference`
  - That herdr protocol 20 exposes no pane-resize verb — waggledance's
    `socket.rs` calls none, and the relay resizes out-of-band via `stty` on the
    TTY it resolves from the pane's foreground PID. `Inference`

## Findings

### The one-line verdict on architecture

The relay independently arrived at the architecture waggledance already
chose — **poll a text snapshot, render it server-agnostically, send input as
discrete commands** — and it did so with a full engineering budget and no
constraints. It uses no xterm.js and no PTY stream. `Upstream`

That validates three rejections already locked in this repo
(`docs/history/research/agent-orchestrator-terminal-ux.md`): xterm.js, a
WebSocket PTY mux, and waggledance owning a multiplexer. The gap between the two
apps is **not architectural**. It is entirely in the layer above the poll.

### Dependency matrix — source component → local

| # | Source component | Local status | Evidence |
|---|---|---|---|
| 1 | `question` — classify pane text into approval / question / chat / unknown, extract options, plan the keystrokes for an answer (5,169 lines, agent-specific) | **NEW** | Local has one blind button that types the literal word "Approve" (`views.rs:2880-2889`) `Local` / `internal/question/attention.go:42`, `parser.go:70`, `input.go:21` `Upstream` |
| 2 | `history` + `seqmatch` — stitch poll snapshots into a rolling 10,000-line buffer by sequence alignment | **CONFLICT** | Local scrolls by *injecting PageUp/PageDown into the live pane* (`pane_scroller.rs:307-412`) — it writes into the user's real terminal to read history `Local` / `internal/history/history.go:60` `Upstream` |
| 3 | `noecho` — detect `[sudo] password for…`, ssh passphrase, gpg, PIN; offer a masked field; never persist, never log | **NEW** | Nothing local classifies a no-echo prompt `Local` / `internal/noecho/noecho.go:37` `Upstream` |
| 4 | `panesize` — lease the pane's columns/rows to the viewer's real width, TTL 120 s, release grace 10 s, restore baseline | **CONFLICT** | Local instead shrinks the font 13px→10px then wraps (`app.js:2202-2280`) `Local` / `internal/panesize/manager.go:130` `Upstream` |
| 5 | Attention plumbing — `document.title` badge, `navigator.setAppBadge`, vibrate on new blocked, Web Push with an *"Approve once"* action button in the OS tray | **NEW** | Local has a colour pill only; Telegram notify is a separate opt-in duty `Local` / `App.svelte:181-191, 421-446` `Upstream` |
| 6 | `panedelta` — send copy-segments instead of full frames, gated at 25% savings | **EXISTS (client half)** | The screen route hashes its own text (`ansi::revision_of`, `server.rs:4316`) — it does *not* use herdr's dead `revision` field — and the client skips the repaint when it matches (`app.js:2346`). But the dedupe happens on the **client, after** the server has already run `to_html` plus two linkify passes and shipped the payload (`server.rs:4316-4329`). The saving is DOM-only; the server work is unconditional `Local` / `internal/panedelta/delta.go:18` `Upstream` |
| 7 | `coordinator` — per-pane FIFO, one in-flight op, generation counters against recycled pane ids, idempotent request ledger | **NEW** | Local posts straight to `/input`; a double-tap over a flaky link sends twice `Local` / `internal/coordinator/coordinator.go:233` `Upstream` |
| 8 | `slashcmd` — walk `.claude/skills`, `.agents/skills`, settings files; build a per-agent command palette with autocomplete | **NEW** | No command discovery locally `Local` / `internal/slashcmd/catalog.go:37` `Upstream` |
| 9 | Key-hint detection — when the last lines name arrows / Enter / Esc / Y/N / a chord, offer matching one-tap buttons | **NEW** | Local key grid is fixed 2×6, context-free `Local` / `docs/mobile-app.md` `Upstream` |
| 10 | `terminal-find` — search loaded rows, highlight, walk the first 1,000 matches | **NEW** | No search over screen output `Local` / `lib/terminal-find.ts:27-42` `Upstream` |
| 11 | Render quality — box-drawing glyphs as CSS gradients + SVG arcs; fractional-pixel cell probe; near-white-bg / near-black-fg contrast normalisation | **NEW** | Local emits `ansi-*` spans + inline RGB, no box-drawing or contrast repair `Local` / `lib/terminal.ts:67-200, 541-587` `Upstream` |
| 12 | Anti-jitter — tolerate 2 missed polls before tearing down a blocked UI; hold the last stable frame up to 4 s during a resize | **NEW** | Local dims the screen (`.term-screen--stale`) but the Approve button's state is server-rendered *at page load* and never updates (`views.rs:2856-2858`) `Local` / `lib/agents.ts:230-265` `Upstream` |
| 13 | `events.subscribe` on the herdr socket for topology push, 15 s poll only as reconciler | **NEW** (already wanted) | Local polls only; already on this repo's wanted list `Local` / `internal/herdr/events.go:175`, `coordinator/poller.go:19` `Upstream` |
| 14 | `conversation` — native transcript readers per agent, paginated, collapsible tool calls, searchable, copy-markdown | **EXISTS (partial)** | Local Transcript tab reads the JSONL activity log (`views.rs:3156-3208`) but has no pagination, no tool-call collapse, no search `Local` |
| 15 | `copyresponse` — transactional `/copy` with a clipboard sentinel, restoring the host clipboard and the unsent draft | **NEW** | No copy affordance locally `Local` / `internal/copyresponse/copy.go:53` `Upstream` |
| 16 | Armed modifiers combining with *typed* characters ("arm `^`, type `c`") with a live chord readout | **CONFLICT** | `term-keys-grid` D2 explicitly rejected capturing typed characters into latches `Local` |
| 17 | Text / keys / attach / preset-start / pane strip / cross-project drawer / status tones | **EXISTS** | `views.rs:2913-3070`, `server.rs:4564-4740` `Local` |

### Cross-cutting sweep

Wiring outside the feature folder that any adoption would touch:

- **`crates/waggledance/src/herdr/socket.rs`** — every new herdr verb lands
  here, and `HERDR_PROTOCOL = 20` is pinned (`herdr-protocol-20` D4). A real
  screen revision (#6) can be computed *locally* by hashing the rendered text,
  needing no protocol change; `events.subscribe` (#13) does need one. `Local`
- **`crates/waggledance/src/herdr/pane_scroller.rs`** — 1,202 lines that exist
  *only* to inject scroll keystrokes. Adopting stitched history (#2) retires it
  rather than extending it, and with it the unverified `PAGE_DOWN` gap. `Local`
- **`crates/waggledance/assets/app.js`** — one file, no bundler, no test
  harness. Every client-side item (#9, #10, #11, #12) grows it. It is already
  3,735 lines; a fourth of the relay's client work would double it. This is the
  single largest hidden cost in the list. `Local`
- **`crates/waggledance/src/server.rs`** `/input` and `/keys` routes — the
  idempotency ledger (#7) and structured answers (#1) both land here. The
  `terminal-open-access` D10 rule (JSON body only, never form POST) constrains
  their shape. `Local`
- **`crates/waggledance/src/views.rs`** — server-renders the composer. Anything
  whose state must track the poll (#1, #12) has to move out of server render
  into `app.js`, or gain a JSON endpoint. This is a structural change, not a
  cosmetic one. `Inference`
- **`docs/specs/agent-terminal.md`** — the living spec; D1–D10 there and the
  CONTEXT files listed below all describe the current surface and would need
  syncing. `Local`

### Locked decisions this would touch

Noted with evidence, never silently overridden — superseding any of these is
the user's move.

- **`terminal-approve-button` D2/D3** — one tap sends the literal text
  `"Approve"`, no confirmation, ignoring the draft. Structured approval buttons
  (#1) *extend* this rather than contradict it: the blind button stays for
  unclassified screens, and named option buttons appear when the screen parses.
  D1 (Approve sits first in the actions row) is unaffected. `Local`
- **`term-keys-grid` D2** — modifier latches combine **only** with the next
  on-screen key tap; typed characters are deliberately not captured. The relay's
  armed-modifier model (#16) contradicts this directly. Flagged; not
  recommended below. `Local`
- **`agent-terminal` D3** and `agent-orchestrator-terminal-ux` — waggledance
  never runs a terminal of its own; herdr owns the PTY. The size lease (#4)
  runs `stty` against the pane's TTY on the host, which sits *on* that boundary
  even though it is not a multiplexer. Flagged as the item needing an explicit
  ruling before shaping. `Local`
- **`bee-agent-activity-contract` (110d9120)** — Approve is enabled only on
  `blocked`. The relay's finer split (`approval` vs `question` vs `chat`) is
  exactly the `waiting_input` / `blocked` distinction this repo already lists as
  *wanted but not built* — #1 delivers it. No conflict; it closes an open item.
  `Local`

### Already wanted here, and the relay shows how

Five items on this repo's own "wanted, not built" list are solved upstream, in
working code, against the same daemon:

| This repo wanted | The relay's answer |
|---|---|
| `waiting_input` vs `blocked` split | `question.Classify` → approval / question / chat / unknown `Upstream` |
| title badge, favicon, Web Notification on blocked | `document.title` `(N) 🐑`, `setAppBadge`, `vibrate([120,80,120])`, push action buttons `Upstream` |
| replay cover on pane switch to hide the first poll burst | last-stable-frame hold + "Resizing terminal…" up to 4 s `Upstream` |
| `events.subscribe` instead of a diff poller | `EventClient.Bootstrap` + 15 s reconciler poll `Upstream` |
| verify `PAGE_DOWN` injection against a live pane | made moot — stitched history needs no injection at all `Upstream` |

### Inference

- The relay's single largest investment is `question` at 5,169 lines — larger
  than waggledance's entire herdr client. That ratio is the finding: on a
  polled terminal, **turning screen text into a structured, tappable decision is
  the hard part**, and everything else is support for it. `Inference`
- waggledance's `revision`-based repaint skip is dead code in practice, so the
  screen is re-rendered from scratch ~40 times a minute per open pane. A local
  content hash would revive it in a few lines and is almost certainly the
  cheapest visible win on the list. `Inference`
- Retiring keystroke-injected scrollback is a *correctness* change, not a
  performance one: today, reading history writes into a pane someone else may be
  typing in. `Inference`

## Ranked adoption list

Ordered by payoff ÷ cost, with the cheapest real wins first.

1. **Move the existing dedupe to the server** — the hash and the skip already
   exist, but on opposite sides of the wire: the client compares, so the server
   still renders and ships every unchanged frame. Have the poller send
   `?since=<revision>` and answer an unchanged pane with a bare
   `{"revision": …}`. Net *reduction* in server work, not an addition.
   See "Appendix — where the server time actually goes". *Small.*
2. **Attention plumbing** — title badge, `setAppBadge`, vibrate on a new
   blocked pane, Web Notification on transition. Pure `app.js`, no protocol
   change, closes a wanted item. *Small.*
3. **Structured approvals + key hints** — parse the option list off the pane and
   render named buttons with approve/deny/trust tones; offer one-tap keys when
   the last lines name them. Keep the blind Approve for unparsed screens.
   *Medium — the highest payoff on the list.*
4. **Anti-jitter** — tolerate 2 missed polls before tearing down a blocked UI;
   hold the last good frame briefly on pane switch. *Small, and #3 needs it to
   not flicker.*
5. **Masked secret prompt** — `noecho`'s eight regexes plus a masked field that
   never persists and never logs. Self-contained, and a real safety gap today.
   *Small.*
6. **Stitched history** — merge snapshots into a rolling buffer via sequence
   alignment; retire `pane_scroller.rs` and its live-pane writes. *Large, and
   the one that removes a whole class of wrongness.*
7. **Slash-command palette** — discover skills and commands, autocomplete in the
   composer. Especially apt in a bee repo. *Medium.*
8. **Find in screen** — search loaded rows with match navigation. *Small.*
9. **Render repair** — box-drawing via CSS gradients, contrast normalisation for
   near-white backgrounds. *Medium; visible on every TUI frame.*
10. **Idempotent input ledger** — per-pane FIFO and a request ledger. *Medium;
    invisible until a double-send bites.*
11. **Column lease** — deferred pending the PTY-boundary ruling above.

## Risks, Unknowns, Follow-Ups

- **`app.js` has no bundler and no test harness.** Items 2, 3, 4, 8, 9 all land
  there. Growing it past ~5k lines by hand is the main structural risk on this
  list, and nothing in this research costed it. `Local`
- **The composer is server-rendered.** Making its state track the poll (#1, #3)
  needs either a JSON state endpoint or a move into `app.js`. Decide once,
  before #3. `Inference`
- **The column lease crosses a locked boundary.** `stty` against a herdr-owned
  TTY is not "waggledance owns a PTY", but it is close enough to need the
  user's explicit ruling rather than an agent's judgment. `Local`
- **`question`'s parsers are agent-version-specific.** The relay's own README
  says newer agent releases change edge-case discovery before it catches up.
  Any adoption inherits that maintenance tail — scope it to the agents this repo
  actually runs. `Upstream`
- **No live-herdr proof was run.** Everything here is read from source on both
  sides; nothing was executed against a running daemon. `Local`

## Open questions for the user

1. The column lease (#11) touches the "herdr owns the PTY" boundary — in or out?
2. Item 3 is the big one and the rest are small. Take the four small wins first
   (#1, #2, #4, #5) as one slice, or go straight at structured approvals?

## Source Pack

- **Local files read:** `crates/waggledance/src/herdr/{mod,socket,wire,pane_scroller}.rs`,
  `crates/waggledance/src/{views,server,orchestrate,supervisor}.rs`,
  `crates/waggledance/assets/app.js`, `crates/waggledance-core/src/ansi.rs`,
  `docs/specs/agent-terminal.md`, `docs/specs/bee-cockpit.md`,
  `docs/history/research/{agent-orchestrator-terminal-ux,agent-status-herdr-vs-agent-orchestrator,bee-agent-activity-contract}.md`,
  and the CONTEXT.md of `agent-terminal`, `terminal-open-access`,
  `terminal-pane-scope`, `terminal-image-attach`, `terminal-approve-button`,
  `term-keys-grid`, `term-reply-composer`, `homepage-terminals`,
  `homepage-terminal-full`, `herdr-protocol-20`, `console-phone-layout`.
- **Upstream repo checked:** `herdr-mobile-relay` @ `7400537` —
  `contracts/fixtures/{inbound,outbound,push}/`, `internal/protocol/`,
  `internal/herdr/{client,events,socket_api}.go`,
  `internal/{panedelta,panesize,seqmatch,stablestate,noecho,question,activity,conversation,history,copyresponse,slashcmd,clipboard,upload,session,coordinator,framing}/`,
  `frontend/src/App.svelte`, `frontend/src/components/`, `frontend/src/lib/`,
  `README.md`, `docs/{mobile-app,transports}.md`.
- **Docs pages checked:** none external — no web research was needed or done;
  every claim above is `Local` or `Upstream`.

---

# Appendix — How the relay makes scrolling smooth

Added on the user's narrowing: interaction here is already good; the question is
purely **display, and specifically why scrolling up and down feels smooth**.

The short answer: **the relay never scrolls the terminal.** It scrolls a copy of
the history that the server stitched together itself, and it holds the reading
position by *line content* rather than by line number. Five layers, each solving
a different failure.

## Layer 1 — The server owns the history, so the pane is never touched

`internal/history` keeps one buffer per pane, up to `MaxLines = 10000`,
persisted to `<cache>/claude-history/<pane>.json` and flushed at most every
`SaveInterval = 10s` (`history.go:17-21, 60`). Every poll snapshot is *merged
into* that buffer. The pane itself is only ever read. `Upstream`

This is the whole foundation. Scrolling becomes a local operation over data the
server already holds — not a remote-control operation on somebody's live
terminal. Compare `crates/waggledance/src/herdr/pane_scroller.rs:307-412`, where
"Older" injects `\x1b[5~` (PageUp) into the real pane and pauses the live poll to
read the result. `Local`

## Layer 2 — Stitching a moving window onto a fixed buffer

The hard part: each snapshot is a *window*, and successive windows overlap
unpredictably — the pane scrolls, the agent redraws its whole frame, a status
bar repaints every tick. `Manager.merge` (`history.go:60-125`) handles it in
seven steps: `Upstream`

1. **Split the footer off.** The last `FooterLines = 6` lines are treated as a
   volatile status bar, stored separately, and never merged into history
   (`history.go:230`). This alone is why an inline status line that repaints
   several times a second does not accumulate thousands of near-duplicate rows.
2. **Hash the body.** Identical to last time, with no pending ambiguity → return
   immediately, no work and no re-render (`history.go:74-78`).
3. **Normalize for comparison only.** Strip ANSI, strip `\r`, right-trim
   (`NormalizeLine`, `history.go:241`). Comparison runs on normalized text;
   storage keeps the original coloured line.
4. **Try exact tail overlap first** — the largest *k* where the last *k* lines of
   history equal the first *k* lines of the new frame (`tailOverlap`,
   `history.go:247`). Overwrite those *k* (they may have gained colour) and
   append the rest. This is the common, cheap path.
5. **Fall back to sequence alignment.** `seqmatch` is a faithful port of Python's
   `difflib.SequenceMatcher` with `autojunk=False` — Ratcliff/Obershelp gestalt
   matching. Take the longest matching block of ≥2 lines that has ≥2 non-empty
   lines; that block is the anchor tying old to new (`sequenceMatch`,
   `history.go:267`).
6. **Then judge what the alignment means** — `applyMatch` (`history.go:155-181`),
   and this judgment is what makes it feel right rather than merely work:
   - nothing follows the match → the desktop pane was scrolled up and is
     re-showing known content → **change nothing**;
   - the match sits implausibly deep (`historyTail >= len(body)`) → coincidence →
     treat the whole frame as new;
   - the match ends within 3 lines of the tail → normal → rebase: truncate at the
     match end, append the suffix;
   - anything else → **ambiguous → refuse to act.** Increment `StaleRefusals`;
     only when **two consecutive** polls agree does it rewrite.
7. Trim to the last 10,000 lines.

Step 6's last branch is the anti-flicker rule *for history itself*: a single odd
frame can never rewrite what you are reading.

## Layer 3 — The client renders only the visible rows

`VirtualTerminalIndex` (`frontend/src/lib/virtual-terminal.ts:9-92`) is a Fenwick
tree over per-row pixel heights: `offset(i)` and `indexAt(px)` in O(log n), point
`update()` in O(log n) and ignoring deltas below 0.25px. `range(scrollTop,
viewportHeight, overscan)` returns the visible row window plus the exact top and
bottom spacer heights, and only that window is in the DOM between two spacer
spans. Overscan is 1.5× the viewport. A `ResizeObserver` on the rendered rows
feeds *measured* heights back into the tree, so wrapped rows of differing heights
never accumulate scroll drift. `Upstream`

## Layer 4 — The reading position survives every disturbance

This is the layer that actually reads as "smooth"
(`TerminalView.svelte:494-514, 672-793`): `Upstream`

- **Stick-to-bottom is a distance test, not a flag:**
  `scrollHeight - scrollTop - clientHeight < 48`. Within 48px of the bottom the
  view follows the tail; beyond it, the view holds. Re-evaluated on every frame
  and every layout change, so it is never stale.
- **Anchoring by content.** Before each re-render it captures an anchor: the
  first row whose bounding box intersects the viewport, the pixel offset into
  that row, **and the row's text** (`currentVirtualAnchor`). After the re-render,
  `matchingAnchorIndex` finds that row again *by text* — exact match scores 2,
  containment scores 1, ties broken by nearest index — not by index. So when 200
  rows are cropped off the front, the eye stays on the same line.
- **Row-shift pre-correction.** `renderedRowShift` counts how many rows fell off
  the front and shifts the anchor index before matching, so the text search
  starts in the right neighbourhood.
- **One mechanism, three callers.** The same anchor path serves a new content
  frame, a container resize (`ResizeObserver`), and an interface-scale change.
- **`jumpVisible`** — the floating jump-to-latest button appears exactly when the
  view is not sticking. It is a consequence of the state, not a separate flag.

## Layer 5 — Only send what changed

`panedelta.Build` (`internal/panedelta/delta.go:18`) turns the new frame into
copy-segments (`{copy_start, copy_lines}`) plus literal text, keyed on 3-line
rolling hashes with up to 64 candidate offsets, and `Efficient()` only sends the
delta when it saves at least 25% (`literalBytes + 64·segments < ¾·len(current)`).
`Upstream`

## What waggledance does instead, today

| Concern | Relay | waggledance |
|---|---|---|
| History | server-side 10k-line stitched buffer, persisted `Upstream` | none — PageUp injected into the live pane, live poll paused (`pane_scroller.rs:307-412`) `Local` |
| Status bar | last 6 lines split off, never merged `Upstream` | not separated `Local` |
| Repeat frames | body hash short-circuits **before** any further work `Upstream` | own hash exists and works, but is compared on the **client** — the server has already rendered and shipped the frame (`server.rs:4316-4329`, `app.js:2346`) `Local` |
| DOM | virtualized window + spacers `Upstream` | full screen replaced every 1500 ms (`app.js:2148`) `Local` |
| Scroll position | text anchor + 48px stick test `Upstream` | none — browser default after each replace `Local` |
| Fit | column lease to the real width `Upstream` | font shrunk 13→10px, then wrap (`app.js:2202-2280`) `Local` |

## If this were adopted, the cheap-to-expensive order

1. **Layer 4 alone, in `app.js`** — the 48px stick test plus a text anchor
   captured before and restored after each screen replace. ~60 lines, no
   protocol change, no server change, and it works *even with the current
   1.5-second full-screen replace*: it is the single change that would stop the
   poll from disturbing the reading position. `Inference`
2. **Layer 1 + 2, in Rust** — a `PaneHistory` beside `pane_scroller.rs`: footer
   split, body hash, tail overlap, sequence alignment, and the four-branch
   judgment with its 2-poll refusal. This is the change that retires
   keystroke-injected scrolling and its unverified `PAGE_DOWN`. Ratcliff/Obershelp
   is ~270 lines to port, or a crate. `Inference`
3. **Layer 3** — only pays above roughly a thousand rendered rows, which is
   exactly where layer 1 puts us. Defer until layer 1 exists. `Inference`
4. **Layer 5** — bandwidth only; irrelevant on a local daemon. Skip. `Inference`

The ordering matters: layer 4 is independently useful and reversible, layer 2 is
the correctness fix, and layer 3 only becomes necessary because layer 1
succeeded.

---

# Appendix — Where the server time actually goes

Written after the question "cách nào nhẹ nhàng cho server hơn — capture giống
herdr-mobile thì khá nặng". The instinct is reasonable but points at the wrong
cost centre. Measured by reading the actual request path, not by inference.

## Correction to the matrix above

An earlier draft of this brief said waggledance's repaint-skip never fires
because herdr returns `revision: 0`. **That is wrong.** The `revision: 0`
finding (`socket.rs:272-285`) concerns the *settle-wait* logic, not the screen
route. The screen route computes its own hash — `ansi::revision_of(&read.text)`
(`server.rs:4316`, `ansi.rs:268`) — and the client does skip the repaint on a
match (`app.js:2346`). Row 6 of the matrix and item 1 of the adoption list have
been corrected. `Local`

## What one `/screen` poll costs today

Every 1500 ms, per pane open in a browser (`app.js:2148`), `terminal_screen`
runs, in order (`server.rs:4290-4331`): `Local`

| Step | Cost | Conditional? |
|---|---|---|
| `st.herdr.snapshot()` — the **entire** workspace/tab/pane inventory over the socket | one full round-trip + parse | **no** — and it exists only to answer "is this pane inside the project root" |
| `scroll_aware_read` — per-pane lock, plus a sweep over every *other* pane's stale scroll record | lock + bookkeeping | no |
| `pane.read` recent, 200 lines (`SCREEN_READ_LINES`) | one round-trip | no |
| `ansi::revision_of` — hash the text | trivial | no |
| `ansi::to_html` over 200 lines | the real CPU | **no** |
| `doc_links::linkify_docs` over the HTML | second pass | **no** |
| `doc_links::linkify_urls` over the HTML | third pass | **no** |
| ship the full HTML payload | bandwidth | **no** |
| client compares `revision`, throws it all away if unchanged | — | yes |

For a pane that is blocked or idle — which is most panes, most of the time —
**every row above runs and is then discarded.** The dedupe is real, but it sits
on the far side of the wire from the work it could avoid.

## So how heavy would the relay's capture actually be?

Per poll, `Manager.merge` on a 200-line frame is: one line split, one 64-bit
rolling hash, one ANSI-strip regex per line, and a tail-overlap scan that
early-exits on the common case (`history.go:60-125, 247`). That is **less work
than the single `to_html` pass waggledance already runs unconditionally**, and
it runs at 1500 ms here versus 250 ms there — six times less often. `Inference`

Memory is ~1 MB per pane at the full 10,000 lines, and the relay persists to
disk every 10 s because a phone may reconnect at any moment. `Upstream`

**The capture is not the expensive part. The unconditional render is.**

## Three ways to make it lighter than it is today

Ordered by server cost *saved*, not by effort:

1. **Move the dedupe across the wire.** The poller sends `?since=<revision>`;
   the server does `snapshot` + `pane.read` + hash, and on a match returns
   `{"revision": …}` with no `text`. Skips `to_html` and both linkify passes and
   the payload, on the majority of polls. No new state — the hash is already
   computed. *Net reduction.* `Inference`
2. **Stop taking a full `session.snapshot` per poll.** It is a containment check
   answered by data that changes on the scale of minutes, not 1.5 seconds. Cache
   the pane→project membership with a short TTL, or invalidate it from the same
   herdr event stream the relay already subscribes to. *Net reduction, larger
   than 1 on a busy herdr.* `Inference`
3. **Then, if scrollback is still wanted:** capture, but **lazily and narrowly** —
   the two cuts the relay does not need and we do:
   - **only while a browser is actually watching that pane**, dropped when the
     last poller stops; the relay must capture continuously because a phone can
     reconnect at any time, but waggledance's viewer and herdr are on the same
     machine and the poll *is* the presence signal. `Inference`
   - **only for alt-screen panes.** A shell pane already gets real scrollback
     from herdr itself — `SCREEN_READ_LINES`'s own comment measures it: a shell
     with 423 lines returns 200, while an alt-screen agent has
     `max_offset_from_bottom == 0` and returns exactly the visible frame
     (`server.rs:4341-4348`). Stitching adds nothing for shells; it is the *only*
     way to get history for a full-screen agent. The relay reached the same
     conclusion by a different road — its cache directory is literally named
     `claude-history` (`history.go:41`). `Local` + `Upstream`

   With both cuts, capture is bounded by "panes currently on screen running a
   TUI agent" — typically one or two — instead of every pane herdr knows about.

## And the answer to the original question

None of this is needed for smooth scrolling. **Layer 4 — the 48px stick test and
the text-anchor — is pure client-side `app.js` and costs the server exactly
zero.** It is both the lightest item on this page and the one that answers what
was asked. Items 1 and 2 above are worth doing on their own merits; item 3 is a
separate feature (scrollback for TUI agents) that should not be smuggled in
under the word "smooth".
