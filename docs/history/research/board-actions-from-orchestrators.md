---
artifact_contract: bee-research/v1
topic: board-actions-from-orchestrators
depth: standard
date: 2026-08-23
mode: xia
---

## Bottom Line

- **Recommendation (ladder rung): adapt-upstream** — the herdr-board / Symphony
  shape ("the board is a trigger and a mirror, never the executor") maps onto
  two primitives waggledance already owns: *run the project's own `bee` CLI as a
  subprocess* (what `POST /api/projects/:id/pbi` does today) and *send input /
  start an agent in a herdr pane* (what the terminal Approve button and
  `waggledance_dispatch` do today). Every action the user named is one of those
  two primitives plus a policy rule borrowed from upstream.
- **Why this is the lightest credible path:** no new result channel is needed —
  bee already writes the outcome into `.bee/` and the board already re-renders
  on file change. The only new code is one action endpoint + card buttons +
  three guardrails (opt-in, confirm, single-active-run).
- **Why the next-best rung lost:** pure *reuse* loses because the board has
  zero card-level write paths and its README invariant says "the board never
  writes" — that invariant must be consciously superseded (it already has a
  deferred PBI for exactly this). *Build* (a queue/daemon of our own like
  `boardd`) loses because `bee herding run` already is that daemon.
- **Confidence:** 80% on the mechanism, 60% on the concurrency story (see Risks).
- **Suggested next step: bee-shaping** — this is a feature with real product
  decisions (which actions, confirm or not, what "run review" spawns).

## Repo Snapshot

- Rust workspace (`crates/waggledance`, `waggledance-core`, `waggledance-desktop`), axum
  server + server-rendered HTML + vanilla `assets/app.js`; WebSocket `/ws` pushes
  `{"changed":[…]}` from a file watcher and the client does `location.reload()`.
- Agent transport is **herdr 0.8.2** over a local socket (`crates/waggledance/src/herdr/`);
  tmux is explicitly not implemented (`README.md:151-152`).
- bee 2.21.0 vendored at `.bee/bin/bee`; `uat_stop: "close"`, `gate_bypass: "normal"`,
  `herding.agents` registry with `claude-sonnet`, `pi-agy-flash-3.7`, `agy-flash`.
- Daemon auth posture: no token, Host-header allowlist only (`server.rs:552-587`);
  every pane route is cwd-contained to the project boundary.

## Question & Assumptions

- **Asked:** how do agent orchestrators execute actions from a board, and what
  can waggledance learn to add board actions — approve UAT, run review, run
  compound, kick off a todo, approve right on the board.
- **Success looks like:** a human on the board can press one button per
  situation and the right thing happens in bee, with the outcome visible on
  the board without opening a terminal.
- **Assumption:** "approve ngay trên board" means both (a) the agent's
  permission prompt (already an Approve button on the terminal page) and
  (b) bee gates (uat, merged shape+execution). Both are covered below.

## Findings

### Local

**What exists (reusable).**

- **Board** — five feature columns Todo / In Progress / Review / Compound /
  Ready to merge (`views.rs:2863-2867`, placement `views.rs:3604-3631`,
  membership `views.rs:3942-4017`). Pure `.bee/` filesystem reader
  (`waggledance-core/src/bee.rs:1-100`); no bee subprocess on the read path.
- **Write path #1 — run the project's bee CLI:** `POST /api/projects/:id/pbi`
  spawns `.bee/bin/bee backlog pbi add --title … --json` and 409s when the
  project vendors no bee (`server.rs:1455-1478`). This is the exact precedent
  for "board calls `bee gate`".
- **Write path #2 — talk to a pane:** `POST /p/:id/_terminal/:pane_id/input`
  (`server.rs:2942`) and `/keys`; the terminal Approve button posts
  `{text:"Approve", submit:true}` and is enabled only when the session's
  `activity.state == "blocked"` (`views.rs:1581-1584`, `:2290-2295`; decision
  `docs/history/terminal-approve-button/CONTEXT.md:20-23`).
- **Write path #3 — start an agent:** `POST /p/:id/_terminal/create/agent`
  (preset label only, `server.rs:3380-3393`) and MCP `waggledance_dispatch /
  waggledance_await / waggledance_runs` (`mcp.rs:227-276`), gated by the
  per-project `orchestration.enabled` opt-in (`docs/history/orchestrator-dispatch/CONTEXT.md` D6).
- **Session → pane bridge:** `.bee/sessions/<id>.json` `activity.pane` =
  `HERDR_PANE_ID` (`bee.rs:599-601`), plus `activity.feature`, `activity.cell`,
  `activity.state ∈ working|waiting_input|blocked|idle|exited`. A board card
  already knows which pane to address for a feature.
- **"Waiting on you" signal:** `waiting_on {kind, subject, asked_at, session}`
  on the lane record; badge rule = live `waiting_on` only, never bare
  `run_state: awaiting-approval` (`bee.rs:203-216`,
  `docs/knowledge/work/waiting-on-badge/delivery.md:18`).

**bee primitives behind each requested action** (all `Local`, from `bee <verb> --help`):

| Board action | bee mechanism | Needs an agent? |
|---|---|---|
| Approve UAT | `bee gate --name uat --approved true --lane <feature>` (actor defaults to `user`; `--actor auto` on uat is refused — uat is human-only by design) | No — CLI call. The session learns on its next turn; optionally also `send_input` "uat approved" to `activity.pane` so a waiting session wakes. |
| Approve merged gate (shape+execution) | `bee gate --merge --approved true --lane <feature>` — carries advisor + plan-conflict preconditions; refuses loudly | No — CLI call, same wake-up note |
| Approve agent permission prompt | existing `/input` Approve, condition `activity.state == blocked` | pane only |
| Run review | user-invoked skill `bee-reviewing` → needs a session: `send_input` to the feature's live pane, or spawn one | Yes |
| Run compound | `bee-capturing` flush (42 stubs pending today) + `bee state compounding-run` receipt | Yes |
| Kick off a todo | = one herding dispatch: `bee worktree new --feature <slug>` → `herdr pane split` → `herdr agent start` → `bee herding record-worker` (`role-dispatch.md:312-460`), or the single-shot `bee herding run --task … --cwd <worktree>` which splits a pane, hands a brief over `.bee/mailbox/<job>/`, and waits for `result-N.json` | Yes |

**Constraints already on record.**

- `README.md:128-131`: "**The board never writes.** It approves no gate, claims
  no cell … the daemon verifies that by comparing each project's store
  byte-for-byte". The gather found no code implementing the byte-for-byte check
  — the invariant is prose + the absence of routes, not a guard.
- `.bee/backlog.jsonl:13` — deferred PBI *"Agent board V2: on-board
  Approve/Reject buttons (real write path to bee with concurrency-safety
  story)"*, P3: "a write path races live agents and needs its own shaping."
- `docs/history/terminal-approve-button/CONTEXT.md:71-73` — a row of canned
  replies (Reject, Retry) was explicitly deferred as "a different feature".
- Orchestrator-dispatch D1: waggledance never decides *what* to dispatch; D6:
  write surfaces are off until a human opts the project in.

**Genuinely missing:** a card-level action endpoint; a single-active-run guard
per feature for board-triggered runs; a UI state for "action in flight" (the
board is reload-on-change, so a 2–5 s CLI call needs optimistic feedback).

### Upstream

| Source | Pattern worth taking | Fit |
|---|---|---|
| **herdr-board** (nelsonPires5) — cards dispatched to herdr panes | Board is trigger + mirror: moving a card into an *automatic* column queues a run; the `boardd` daemon resolves the session, opens a stable `card-<id>` tab, starts the agent in a visible pane; *manual* columns are the human gates; the agent reports back with a CLI (`board card run done`) and the daemon applies the transition | Near-identical stack (herdr 0.8.2, local daemon, visible panes). bee already plays the `board` CLI role: the agent reports via `bee cells finish` / mailbox `result-N.json`, and waggledance already mirrors `.bee/`. |
| **OpenAI Symphony** SPEC.md — Linear as control plane | Tracker status is the state machine; orchestrator *reconciles every tick* (human moves a card → agent stopped/started); explicit claim set prevents double dispatch; "approval requests MUST NOT stall a run indefinitely — satisfy, surface, auto-resolve, or fail, per documented policy"; tracker writes run host-side with orchestrator credentials, never the child's | Reconciliation = waggledance's file watcher; the claim set = bee reservations / wave ledger; the "surface to operator" policy is literally the Approve button on a card. |
| **Heym agentic kanban** | Automatic vs manual columns; first two columns manual ("human judgment adds value at problem definition, not execution"); single active run per card ("a board never races against itself"); `rerun: true/false` distinguishes fresh arrival from a follow-up round; comments are the steering channel | Maps to: Todo (manual gate = "Start"), In Progress (auto), Review/Compound (button-triggered rounds), Ready to merge (manual gate). `herding run --continue <job>` is the follow-up round. |
| **Vibe Kanban** (Rust + worktree per task) | Per-task worktree; follow-up prompts sent to the agent from the UI; tool approvals surfaced in UI; MCP server so agents update the board | bee worktrees already; follow-up = `/input`; tool approvals = existing Approve; MCP = already there for dispatch. |
| **Paperclip** | Atomic task checkout; approval gates enforced server-side and *revisioned*; agents wake on heartbeat and resume context | bee `cells claim-next` + heartbeat sessions already match; "revisioned approval" = bee's `approved_for_plan_rev` stamp. |

Common law across all five: (1) the board never runs the agent in-process — it
enqueues, a daemon spawns; (2) human gates are columns/buttons, not typed
commands; (3) exactly one active run per card; (4) results return through a
structured channel the agent writes, which the board mirrors; (5) approval
policy is explicit and documented, never "stall until someone notices".

### Docs

- bee 2.21.0 `--help` text (version-matched, local binary) for `gate`, `state
  gate`, `state waiting-on`, `herding run/wave/occupancy/record-worker/
  control-loop`, `triggers add/list/resolve`. Notable: `bee triggers` IS built
  (help prints) though unused on disk — a possible "deferred decision inbox",
  but it is a decision-tracking store, not a command queue; not the right tool
  for board actions.
- `bee config get` is not built into this binary — read `.bee/config.json` directly.

### Inference

- **All five actions reduce to a 2×2:** {CLI call, pane action} × {feature has
  a live bound pane, it does not}. A single endpoint
  `POST /p/:id/_bee/actions {kind, feature}` can dispatch: `uat-approve` and
  `gate-approve` → CLI; `permission-approve` → pane `/input`; `review`,
  `compound`, `start-todo` → if a live pane is bound to the feature, `send_input`
  a slash-command line (`/bee-reviewing …`, `/bee-capturing flush`), else
  `bee herding run --task <brief> --cwd <worktree>` (or `bee worktree new` first
  for a todo). This is herdr-board's `boardd` with bee as the queue.
- **No new result channel is needed.** `bee gate` rewrites the lane file →
  watcher → board reload; `herding run` writes `.bee/logs/dispatch.jsonl` and the
  wave ledger → the Runs view already reads these. The missing piece is purely
  *in-flight* feedback (button spinner until the `changed` event lands).
- **The README invariant must be superseded by a decision, not quietly
  broken.** The honest replacement wording is Symphony's: "the board writes only
  through the project's own `bee` CLI and only on an explicit human click; it
  never decides, only relays."
- **Concurrency story for the deferred PBI:** the race is "human approves on
  the board while the session is mid-turn". `bee gate` is an idempotent
  file write; the session rereads gates at its next `orient`/`status`; the
  waiting-on mark clears on the user's next *message*, which a board click is
  not — so the board should also post one line to `activity.pane`
  ("uat approved from board") so the mark clears and the session wakes. Single
  active run per feature is enforced by refusing a second `review`/`compound`/
  `start-todo` while `bee herding occupancy` reports that feature's worker unresolved.

## Risks, Unknowns, Follow-Ups

- **Security widening (medium):** today the daemon has no auth; Host-header
  allowlist is the only CSRF defense. Adding gate approval as a POST makes a
  rebinding/Host bypass worth more. Mitigation in scope: reuse
  `orchestration.enabled` opt-in per project, require a confirm dialog for
  `uat`/`gate`, and keep the terminal-family switch as the second key.
- **uat is hard-gate territory** (human-only by bee doctrine) — a button is
  fine precisely because the clicker *is* the human; but `--actor user` from a
  daemon loses "who": consider writing `--reason "board click <ts>"` via the
  gate's reason field only if bee accepts it for actor=user (unverified —
  proof obligation for planning).
- **Spawn cost / runaway:** `review` and `compound` without a live pane spawn a
  full agent. Bound by `herding run --ceiling` and the occupancy cap (4).
- **In-flight UI on a reload-on-change board:** a click must not double-fire
  across the reload; idempotency key = `{feature, kind, lane mtime}`.
- **Open questions for shaping:** (1) which actions ship first — the
  zero-agent ones (uat/gate/permission approve) are a small lane, the
  spawn ones are standard; (2) review/compound brief wording — does the board
  send a slash-command to a live pane, or always spawn fresh? (3) should the
  Todo "Start" button be the herding dispatch role (worktree + classify-lane +
  interlock) or the lighter `herding run` one-shot?
- **Memory note to correct:** the MEMORY.md claim that bee ≥2.21.0 writes
  `merge_ready` on lanes is not observed — 0 of 81 lane files carry it.

## Source Pack

- Local: `crates/waggledance/src/{server,views,mcp,watcher}.rs`,
  `crates/waggledance/src/herdr/{mod,socket,wire}.rs`, `assets/app.js`,
  `crates/waggledance-core/src/{bee,transcript}.rs`, `README.md`,
  `.bee/{state.json,config.json,backlog.jsonl,wave-ledger.jsonl}`,
  `.bee/lanes/*.json`, `.bee/sessions/*.json`, `.bee/mailbox/`,
  `docs/history/{terminal-approve-button,orchestrator-dispatch}/CONTEXT.md`,
  `docs/history/research/bee-agent-activity-contract.md`,
  `docs/knowledge/work/{waiting-on-badge,bee-agent-activity,board-topbar-polish}/delivery.md`,
  `.claude/skills/bee-{hive,herding,reviewing,capturing,swarming}/…`,
  `bee gate/state gate/state waiting-on/herding/triggers --help`.
- Upstream: [herdr-board](https://github.com/nelsonPires5/herdr-board),
  [openai/symphony SPEC.md](https://github.com/openai/symphony/blob/main/SPEC.md),
  [Heym — Agentic Kanban Board](https://heym.run/blog/agentic-kanban-board),
  [BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban),
  [paperclipai/paperclip](https://github.com/paperclipai/paperclip),
  [awesome-agent-orchestrators](https://github.com/andyrewlee/awesome-agent-orchestrators),
  [Cursor agent-kanban (news)](https://pasqualepillitteri.it/en/news/1717/cursor-kanban-agent-sdk-2026).
