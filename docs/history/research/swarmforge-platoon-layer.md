---
artifact_contract: bee-research/v1
topic: swarmforge-platoon-layer
depth: deep
date: 2026-08-25
mode: xia
---

## Bottom Line

- **Recommendation (ladder rung): adapt-upstream — but only SwarmForge's *platoon*
  layer, and it lands as a prompt-plus-manifest layer over tools waggledance
  already ships, not as new daemon machinery.** SwarmForge's *pack* layer (roles,
  role-worktrees, tmux sessions, handoff daemon, cockpit) is 80% a re-implementation
  of things this colony already has in two places: bee owns lanes/cells/gates/
  feature-worktrees, and waggledance owns the cross-project board, board actions,
  and a verified dispatch protocol. The one idea in that repo with **no local
  counterpart** is the unbuilt `platoon-brainstorm.md`: a Lieutenant coordinating
  several independently-deployable components under one system objective — which
  is exactly "điều phối các beehive project liên quan".
- **Why this is the lightest credible path:** the Lieutenant needs five verbs —
  *who is in the swarm, what state is each in, send work, wait, read runs*. Four
  of the five already exist as MCP tools (`waggledance_projects`,
  `waggledance_ask_state`, `waggledance_dispatch`, `waggledance_await`,
  `waggledance_runs`). The genuinely missing piece is small: a **swarm manifest**
  declaring which registered projects form one swarm, each one's role, level and
  `depends_on`. Everything else is a skill prompt.
- **Why the next-best rung lost:** *reuse* alone loses — nothing in waggledance or
  bee expresses that beehive and waggledance are related, so no agent can answer
  "who else does this change break". *build* (porting `handoffd`, `pack_web`,
  `swarmforge.conf` into Rust) loses twice: it duplicates cross-board / board
  actions / dispatch-await-runs / `bee cells claim-next`, and it collides head-on
  with locked decision D1 of `orchestrator-dispatch` ("no autonomous control loop
  inside waggledance").
- **Confidence:** 85% on the layering (manifest + skill + existing tools); 55% on
  the ripple carrier — whether a cross-project handoff needs its own queue or a
  PBI is enough (see Open Questions).
- **Suggested next step: bee-shaping.** Three real product decisions are open and
  they change what gets built (Open Questions Q1–Q3).

## Repo Snapshot

- **Local:** Rust workspace `crates/{waggledance, waggledance-core}` (+ excluded
  Tauri shell), v0.5.2, edition 2021. axum 0.7 + minijinja server-rendered HTML,
  rusqlite 0.32 registry, comrak/syntect/ammonia rendering, `notify` watcher →
  WebSocket reload. 42 Rust source files; `server.rs` 29.8k lines, `views.rs`
  18.9k, `bee.rs` 10.5k. (`Cargo.toml`, `wc -l crates/**/*.rs`)
- **Local:** agent transport is **herdr over a local socket only**
  (`crates/waggledance/src/herdr/`); tmux as a transport is explicitly not
  implemented (`README.md:151-152`).
- **Local:** bee 2.21.0, CLI at `/home/thanhsmind/Projects/goglbe/beehive/.bee/bin/bee`,
  166 commands, store at `waggledance/.bee/`. `bee orient` here reports
  `phase=scribing feature=todo-column-collapse gates=false/true/true/false/false`.
- **Upstream:** SwarmForge `main` @ `0b69a51ca154025087569e6f880ca24b0bce6f54`
  (2026-08-24). Babashka/Clojure + zsh, ~6.1k lines of `.bb` scripts; runnable
  configs live on branches `two-pack` / `four-pack` / `six-pack` / `squad` /
  `adversaries`; `main` is documentary + shared scripts + default constitution.

## Question & Assumptions

- **What was asked:** distill SwarmForge and explore applying it to waggledance as
  a swarm layer that coordinates related beehive projects from the state
  waggledance already manages.
- **What success appears to mean:** one surface where several bee projects
  (waggledance, beehive, jarvis, memorypad, …) are coordinated as one system —
  work assigned across them, dependencies respected, attention aggregated — using
  the state waggledance already reads rather than a new store.
- **Assumptions still needing confirmation:** the swarm's unit is a *project*, not
  a component inside one project (Q1); the human stays the gate-keeper (Q2).

## Findings

### Local

**What already exists — and it is most of the cockpit.**

- `Local` **Cross-project board.** `cross-board` D1–D10 put a roll-up of every
  registered project carrying a `.bee/` on `/`: *Waiting on you / In Progress /
  Finished*, **flat, never grouped per project** (D4), every card project-labelled
  (D5). This is already the platoon board SwarmForge only sketches — and it is
  one level *above* SwarmForge's per-role swimlanes.
  (`docs/history/cross-board/CONTEXT.md`)
- `Local` **A verified dispatch protocol in Rust.** `orchestrate.rs` (1128 lines):
  preflight → fresh split marker → pre-send baseline → send → poll. Fail-closed:
  refuses `Working`/`Blocked`/`Unverifiable`, proves completion only by a marker
  absent from the run's own baseline, falls back to three unchanged
  `ansi::revision_of` reads when status is `Unknown`.
  (`crates/waggledance/src/orchestrate.rs:1-60`)
- `Local` **Seven MCP tools, not the four the README advertises**: `view_file`,
  `search`, `projects`, `ask_state`, **`dispatch`**, **`await`**, **`runs`**.
  `dispatch(project, preset|pane_id, task)` → `run_id`; `await(run_id, ≤60s)` →
  `working|done|blocked|timeout` + transcript delta; `runs(project?)` read-only.
  (`crates/waggledance/src/mcp.rs:227-280`) — *README drift worth fixing.*
- `Local` **Durable run state** (`Run` in `domain.rs`: project, pane, preset, task,
  baseline, marker, status, timestamps + a `runs.feature` column). D7's whole
  point: a restarted orchestrator recovers the fleet by *reading state*, not by
  carrying a roster in its prompt. That is the shrunk form of SwarmForge's
  `.swarmforge/handoffs/` audit tree.
- `Local` **Board actions.** The board writes in exactly two ways — the project's
  own `bee` CLI, or one line into a herdr pane — and only on a human click in a
  project switched on in settings. Approve/reject UAT, the merged shape+execution
  gate, a permission prompt; Start / Run review / Run compound.
  (`README.md:128-148`)
- `Local` **`+ New task` already is a cross-project write path**: POSTs to
  `/api/projects/:id/pbi`, which shells the *target project's own*
  `.bee/bin/bee backlog pbi add`, producing a `proposed` PBI the Todo column
  renders. (`docs/history/board-new-task/CONTEXT.md` N2)
- `Local` **Rich parsed bee state**: `bee.rs` exposes `BeeSnapshot`,
  `BeeProjectRollup`, `BeeAttentionItem`, `BeeSession`, `BeeWorktree`,
  `BeeReservation`, `BeeLane`, `BeeHandoff`, `BeeVelocity`, `BeeFindings`.
  `ask_state()` with no project returns a rollup across every registered project;
  a project with no `.bee/` reports `present:false`, never an error.
  (`crates/waggledance-core/src/bee.rs`, `mcp.rs` tests at 1286-1309)
- `Local` **Blocked-run notification** already has a feature
  (`docs/history/dispatch-blocked-notify/`) and a Telegram channel
  (`crates/waggledance/src/notify/telegram.rs`).

**What is genuinely missing** (checked code, config, docs, tests):

- `Local` **No relation between projects.** `Project` is `{id, name, root_path,
  created_at, last_seen_at, orchestration_enabled}` — no group, no role, no
  `depends_on`. `rg 'depends_on|relation|project_group|upstream_project'` over both
  crates returns nothing but unrelated prose. (`domain.rs:7-22`)
- `Local` **No agent→agent mail.** Dispatch is orchestrator→pane, one hop. bee's
  `cells claim-next` / `cells finish` is intra-project only (one store per repo,
  shared across that repo's worktrees). Nothing carries "beehive changed the CLI
  contract, waggledance's board reader must follow".
- `Local` **Cross-project pane targeting is deliberately refused** —
  `DispatchRefusal::OutsideBoundary`, "refusing to dispatch across project
  boundaries" (`orchestrate.rs:80-84`). Dispatching *into* project B by naming B
  is fine; reaching B's pane from A's boundary is not. Any swarm design must sit
  on the first, never try to defeat the second.

### Upstream

SwarmForge @ `0b69a51`, narrowed to topology, handoff, cockpit and platoon.

- `Upstream` **Config-driven topology** — `swarmforge/swarmforge.conf`, one line
  per agent: `window[-invisible] <role> <agent> <worktree> [task|batch] [args…]`.
  The swarm's shape is data, not code; the launcher normalises it into
  `.swarmforge/roles.tsv`, which every helper then reads instead of re-parsing.
  **This is the borrowable idea**: a declared topology plus a normalised runtime
  view of it. (`README.md` "The `swarmforge.conf` File")
- `Upstream` **Daemon-owned transport.** `handoffd.bb` (363 lines) is the only
  process touching the tmux socket. Agents write a *draft* with headers only;
  the daemon validates, copies into each recipient's `inbox/new/`, moves the
  original to `sent/` or `failed/`, and sends a **generic wake-up** — never the
  payload. Queue state is file location; audit is filename + headers.
  (`swarmforge/handoff-protocol.md:1-60`)
- `Upstream` **A three-verb agent vocabulary**: `swarm_handoff.sh <draft>` (send),
  `ready_for_next.sh` (accept — prints `NO_TASK` / `TASK:` / `BATCH:`),
  `done_with_current.sh` (complete — prints `MAIL_WAITING` / `NO_TASK`). Agents
  never write bodies, branch names, queue filenames, or tmux commands. The
  protocol's surface is deliberately tiny so the agent cannot invent it.
- `Upstream` **Two message types only** — `git_handoff` (points at a committed
  10-hex-char SHA the helper resolves and canonicalises) and `note` (one line,
  ≤80 chars). Priority `00`–`99` sorts the queue; `batch` mode lets a role
  consume all equal-priority mail as one unit.
- `Upstream` **Terminal handoff = Done.** The last role broadcasts `to:` every
  other role; *that*, not merely several recipients, moves the card to the Done
  well. Completion is a protocol event, not a UI guess.
- `Upstream` **Attention has two distinct shapes** — an *approval* (specifier
  handoff held until the human approves, with a Documents menu for artifacts) and
  a *clarification request* (`pack_dashboard_request.sh clarify <file>`, minting a
  durable `clar-<ts>` id, answer injected back into that agent's pane). The
  constitution is explicit: **do not ask in the pane**, and do not abuse `note`
  for a question.
- `Upstream` **Layered constitution** — shared articles on `main`
  (`engineering.prompt`, `workflow.prompt`, `handoffs.prompt`) installed into each
  branch and each worktree only when absent; a branch overrides by committing the
  same filename, or *extends* with `local-<name>.prompt`. Never edit a shared
  article to add a local exception.
- `Upstream` **Platoon (design only, unbuilt)** — `platoon-brainstorm.md`: a
  Lieutenant is the only agent alive at startup; it brainstorms the system with
  the human, produces a component plan, **waits for approval**, then chooses a
  pack type per component and starts squads one directory below. It maintains the
  dependency graph, enforces that dependencies point toward the higher-level
  component, and owns the system test. Its dashboard is the pack board "scaled up
  one level": horizontal squad rows, work queue subdivided by squad, attention
  aggregated across squads with labels. Every one of its seven Open Design
  Questions is still open upstream.

### Docs

- `Docs` (project-local, version-matched) `README.md:128-148` — "The board relays;
  it never decides… it originates none of its own." Any swarm layer either honours
  this or supersedes it consciously; `board-actions-from-orchestrators.md`
  (2026-08-23) already recorded that this invariant "must be consciously
  superseded" and it was, for three named human-clicked actions.
- `Docs` `docs/history/orchestrator-dispatch/CONTEXT.md` — **the binding
  constraint set**:
  - **D1** hybrid architecture: waggledance implements the *mechanical* protocol;
    the orchestrator stays an **external LLM agent** holding the MCP tools.
    "Waggledance never decides what to dispatch — it only executes dispatches
    safely." *"Never codes" is enforced by tool surface, not prose.*
  - **D2** V1 is *exactly three* dispatch-family tools. No broadcast in v1
    (deferred as PBI `p-bf161077`).
  - **D3** presets only — raw argv/env/cwd from a caller is never accepted.
  - **D4** `await` ≤ 60s, clamped silently.
  - **D6** per-project `orchestration.enabled`, effective only when
    `terminal.enabled` is on; default off for every project.
  - **D7** durable run state; no prompt-carried roster.
  - Deferred, explicitly out: **"Any autonomous control loop inside waggledance"**.
- `Docs` SwarmForge `AGENTS.md`: prompt wording is not to be pinned by automated
  tests — relevant because the Lieutenant layer proposed here is *mostly prompt*.

### Inference

- `Inference` **The pack layer is redundant here and the platoon layer is not.**
  SwarmForge's roles are pipeline *stages* (specifier→coder→cleaner→…), each in
  its own worktree. bee already stages work as phases + lanes + cells, and already
  puts a worktree per *feature*. Two worktree schemes over one repo would fight:
  a role-worktree and a feature-worktree cannot both own the same file set, and
  bee's reservation/hold guard would refuse one of them. Verdict: **CONFLICT — do
  not port role-worktrees.**
- `Inference` **waggledance is already the platoon dashboard SwarmForge wants.**
  Everything in the brainstorm's dashboard section — squad rows, aggregated
  attention with squad labels, a work queue subdivided by squad — is a *grouping*
  of things `/` already renders flat. The missing input is not rendering; it is
  the membership fact that says which projects form a squad.
- `Inference` **The Lieutenant must live outside the daemon.** D1 makes this not a
  preference but a locked boundary: a Lieutenant *is* a control loop. Implemented
  as a bee skill driving the seven MCP tools, it is fully compatible with D1 and
  needs no supersession. Implemented as Rust inside `waggledance serve`, it
  requires the user to overturn D1.
- `Inference` **A PBI is already a cross-project handoff.** `bee backlog pbi add`
  in the downstream project, via the existing `/api/projects/:id/pbi` path, is
  durable, appears in that project's Todo column, and is picked up by that
  project's own bee flow — with zero new storage. It lacks priority ordering and
  batch semantics; whether that shortfall bites is Q3.
- `Inference` **"Swarm" is a colliding word here.** bee already uses swarm/
  swarming for its intra-project worker lane (`bee-swarming`, `bee cells`,
  dispatched workers). Naming this layer *swarm* will make every later document
  ambiguous. Prefer SwarmForge's own **platoon**, or a bee-native word (**colony**,
  **fleet**).

## Dependency Matrix

One row per SwarmForge component, mapped to this colony. `EXISTS` = already here,
`NEW` = must be built, `CONFLICT` = would fight something settled.

| # | SwarmForge component | Local counterpart | Verdict | Evidence |
|---|---|---|---|---|
| 1 | `swarmforge.conf` topology (role · backend · worktree · receive-mode) | `config.terminal.agent_presets` (label→argv) + project registry — **no roles, no membership** | **NEW** *(small — the one real build)* | `Local` `config.rs:92-123`, `domain.rs:7-22` |
| 2 | `.swarmforge/roles.tsv` normalised runtime view | none | **NEW** *(falls out of #1)* | `Upstream` `handoff-protocol.md:44-58` |
| 3 | Per-role git worktree under `.worktrees/` | bee worktree **per feature**, `bee worktree new --feature` | **CONFLICT** — drop | `Inference` + AGENTS.md worktree rule |
| 4 | tmux session per role, terminal adapters, window watchdog | herdr only; tmux transport not implemented | **CONFLICT** — drop | `Local` `README.md:151-152` |
| 5 | `handoffd.bb` daemon: outbox→inbox, validate, wake-up | none cross-project; `dispatch` is one hop | **NEW** *(or substitute PBI — Q3)* | `Local` `orchestrate.rs`; `Upstream` `handoffd.bb` |
| 6 | `swarm_handoff` / `ready_for_next` / `done_with_current` | `bee cells claim-next` / `bee cells finish` — intra-project | **EXISTS intra-project, NEW cross-project** | `Local` `bee --help --names` |
| 7 | Priority + `batch` receive mode | bee lanes/waves order intra-project; nothing cross | **NEW** *(only if Q3 says yes)* | `Upstream` `handoff-protocol.md:60-80` |
| 8 | `git_handoff` = pointer to a canonicalised commit | bee cell cap + `cell:` trailer + one commit per cell | **EXISTS** (equivalent) | `Local` AGENTS.md "Care for the session" |
| 9 | Board with swimlanes + Done well | cross-board D1–D10 (flat, cross-project, project-labelled) | **EXISTS — and better** | `Local` `cross-board/CONTEXT.md` |
| 10 | Attention → approval (Approve/Reject) | board actions: UAT gate, merged shape+exec gate, permission prompt | **EXISTS** | `Local` `README.md:128-148` |
| 11 | Attention → clarification (`clar-<id>`, answer injected) | `bee state waiting-on` + `BeeAttentionItem` + terminal reply | **EXISTS, thin gap**: the answer round-trip is a human typing into a pane, not a routed answer | `Local` `bee.rs:798-827`; `Upstream` `pack_dashboard_request.bb:80-90` |
| 12 | Chat rail to the master agent, durable request id | `dispatch`→`run_id`, `await` delta, `runs` list (D4/D7/D8) | **EXISTS** | `Local` `mcp.rs:227-280` |
| 13 | New Task → card in the master lane | `+ New task` → target project's own `bee backlog pbi add` | **EXISTS** | `Local` `board-new-task/CONTEXT.md` N2 |
| 14 | Layered constitution (shared + `local-*` override) | `AGENTS.md` + `@AGENTS.md` import + `skills/` + `docs/waggledance-agents-template.md` | **EXISTS** | `Local` CLAUDE.md / AGENTS.md |
| 15 | Per-role backend (`claude`/`codex`/`copilot`/`grok`) | agent presets + `.bee/config.json` `herding.agents` | **EXISTS** | `Local` `config.rs:116-123` |
| 16 | Teardown, sleep inhibitor, cleanup window | herdr owns pane lifecycle; `supervisor.rs` | **EXISTS / N-A** | `Local` `supervisor.rs` |
| 17 | Commit byline `By <role>.` via commit-msg hook | bee: one commit per cell, `cell:` trailer | **EXISTS** (equivalent) | `Local` AGENTS.md |
| 18 | **Lieutenant / platoon control layer** | none | **NEW — this is the target** | `Upstream` `platoon-brainstorm.md` (design only) |

## Cross-Cutting Sweep

Wiring outside any feature folder that a platoon layer would touch. A component
absent here is *unchecked*, not clean.

- **`config.rs`** — a swarm manifest is a new top-level table beside
  `[terminal]`/`[search]`. It must **not** become a second, weaker permission gate:
  D6's per-project `orchestration_enabled` (+ `terminal.enabled`) stays the only
  thing that authorises a dispatch. Membership in a swarm must never imply consent.
  (`Local` `config.rs:12-123`, `Engine::orchestration_allowed`)
- **`mcp.rs` tool table** — D2 locks V1 at exactly three dispatch-family tools. A
  `waggledance_swarm(...)` tool is a **supersession of D2**, the user's move. The
  cheap alternative: extend `waggledance_projects()`'s existing payload with the
  manifest's role/level/`depends_on` fields — additive, no new tool, no D2 conflict.
- **Boundary/containment** — `project_and_verify_pane_in_boundary` and
  `DispatchRefusal::OutsideBoundary` are load-bearing. A platoon dispatches *by
  naming project B*; it never reaches B's pane from A's boundary. Any design that
  needs the second thing is wrong. (`Local` `orchestrate.rs:80-140`)
- **Watcher → WebSocket reload** — `.bee/` changes already push a reload, so a
  manifest stored under a project's `.bee/` gets liveness for free; one stored in
  `~/.waggledance/` does not and would need `waggledance restart` or a new watch.
- **`notify/`** — the blocked-run path already exists; platoon-level "squad B is
  blocked on you" should reuse it rather than mint a channel.
- **Security posture** — `README.md` "Security — read this before exposing it": no
  auth anywhere. A manifest that lets one agent reach N projects raises blast
  radius on an already unauthenticated daemon. Localhost-only stays mandatory.
- **README drift** — the README documents *four* MCP tools; seven ship. Worth a
  capture and a one-line fix independent of this work.
- **Naming** — `BeeHandoff` already means bee's *session* handoff record
  (`pause` / `planned-next`). Do not overload "handoff" for cross-project mail.
- **Licensing** — SwarmForge `main` carries **no LICENSE file** at `0b69a51`.
  Borrow structure and vocabulary; copy no code. (Moot in practice: it is
  Babashka/zsh, nothing is portable into a Rust daemon anyway.)

## The Shape This Suggests

Three layers, thinnest first. Only layer 1 is new code.

1. **Swarm manifest (NEW, small, data-only).** A declared membership fact:
   swarm name → member projects by registry id, each with `role`, `level`
   (distance from IO, per the brainstorm's dependency rule) and `depends_on`.
   waggledance reads it and does two things: groups the existing cross-project
   board into squad rows, and returns the fields on `waggledance_projects()`.
   Read-only, no new authority. This is SwarmForge's `swarmforge.conf` idea
   (#1/#2) with roles reinterpreted as *projects*, not pipeline stages.
2. **The Lieutenant is a bee skill, not daemon code.** One session whose entire
   tool surface is the seven existing MCP tools: `projects` → membership,
   `ask_state` → each project's phase/gates/attention, `dispatch`/`await`/`runs`
   → work, `search` → docs. It decomposes the system objective, keeps the
   dependency graph honest, waits for human approval before starting squads, and
   owns the system-level test. **D1 is satisfied exactly, not bent**: the decision
   lives in the agent, waggledance stays the safe executor.
3. **Ripple carrier — start with the PBI, not a queue.** A contract change in
   beehive becomes `bee backlog pbi add` in each dependent project through the
   existing `/api/projects/:id/pbi` path, carrying its provenance line. Durable,
   already rendered in the Todo column, zero new storage. Port SwarmForge's
   priority/batch queue (#5/#7) only when that shortfall demonstrably bites.

## Risks, Unknowns, Follow-Ups

- **D1/D2 collision is the live risk.** As a skill, nothing is superseded. The
  moment anyone asks for "waggledance should keep the squads moving by itself",
  that is D1's explicitly-deferred item and needs the user's decision, not an
  agent's judgment.
- **`await` is clamped to 60s (D4).** A Lieutenant supervising several long squads
  polls; it does not sleep. Cost and wake-up cadence are real design inputs.
- **One board-started run per feature stands down that card's buttons**
  (board-run-actions D3). Fan-out across *projects* is unaffected; fan-out within
  one feature is not.
- **`orchestration_enabled` is off for every project by default (D6).** A platoon
  over N projects means flipping N switches — deliberately. Do not let manifest
  membership auto-enable them.
- **Confidence gap on the ripple carrier (55%).** The PBI path is proven for
  *creating* work; it has never been used to express *ordering* between projects.
- **Unverified upstream detail:** only SwarmForge `main` was read. The runnable
  branches (`two-pack`/`four-pack`/`six-pack`/`squad`/`adversaries`) carry the role
  prompts and could change the picture of how much behaviour lives in prompts vs
  scripts. Claims about role behaviour are `Inference` from `main`'s README.
- **Evidence gap:** no web research was run; every `Upstream` claim comes from the
  local checkout at the pinned SHA, and no `Docs` claim is external.

## Open Questions

1. **Q1 — What is a squad?** A whole registered project (beehive, waggledance,
   jarvis…), or a component *inside* one project? This brief assumes **project**.
   The other reading turns the manifest into a per-repo file and changes the
   board grouping entirely.
2. **Q2 — May the Lieutenant dispatch unattended, or only propose?** "Propose"
   means it writes PBIs and the human presses Start (nothing new to authorise).
   "Dispatch" means `orchestration.enabled` across the fleet and a real autonomy
   posture. This decides the security story.
3. **Q3 — Does the ripple need priority and batch, or is a PBI enough?** PBI =
   zero new code. Priority/batch = SwarmForge's queue, which means a new store, a
   delivery step, and a receive vocabulary agents must learn.

## Source Pack

- **Local files read:** `Cargo.toml`; `README.md`; `AGENTS.md`; `CLAUDE.md`;
  `crates/waggledance-core/src/domain.rs`; `crates/waggledance-core/src/config.rs`;
  `crates/waggledance-core/src/bee.rs` (type index);
  `crates/waggledance/src/orchestrate.rs:1-140`;
  `crates/waggledance/src/mcp.rs:150-310, 1260-1360`;
  `docs/history/orchestrator-dispatch/CONTEXT.md`;
  `docs/history/cross-board/CONTEXT.md`;
  `docs/history/console-rail-orchestrator/CONTEXT.md`;
  `docs/history/board-new-task/CONTEXT.md`;
  `docs/history/research/board-actions-from-orchestrators.md`;
  `docs/knowledge/index.md`; `.bee/state.json`; `bee orient`; `bee --help --names`.
- **Upstream read** (`/home/thanhsmind/Projects/refs/swarm-forge` @ `0b69a51`):
  `README.md`; `platoon-brainstorm.md`; `AGENTS.md`;
  `swarmforge/handoff-protocol.md:1-150`;
  `swarmforge/constitution/articles/{engineering,workflow,handoffs}.prompt`;
  `swarmforge/scripts/pack_board.bb:1-120`;
  `swarmforge/scripts/pack_dashboard_request.bb:1-90`; script inventory + line counts.
- **Docs pages checked:** none external — no web research was run this session.

---

## Addendum (2026-08-25) — Cơ chế thật đã có sẵn, và ba chỗ hở

Người dùng chỉnh lại khung: waggledance + bee gần như đã đủ; cách nó chạy là
*biết task này do agent nào chạy (claude code, codex, pi, agy) → mở pane → bảo nó
chạy → bee ghi state → waggledance gom lại*. Kiểm chứng lại code và config trên đĩa:
**đúng nguyên văn**, và điều đó thu nhỏ phần việc còn lại xuống rất nhiều.

### Đường dây đã nối, đúng như mô tả

- `Local` **"Agent nào chạy" là một registry có thật, phía bee**:
  `.bee/config.json` → `herding.agents`. Trong `beehive` nó chứa
  `claude-sonnet` → `["claude","--model","sonnet","--permission-mode","bypassPermissions"]`,
  `pi-opencode-free`, `pi-agy-flash-3.7` → `["pi","-a","--model","agy/gemini-3.7-flash:high"]`,
  `agy-flash` → `{"argv":["agy","--dangerously-skip-permissions"], "workspace_trust":{…}}`.
  `herding.agent_command` chọn cái mặc định. (`beehive/.bee/config.json`)
- `Local` **Board đã đọc đúng registry đó và mở pane**: `Start` / `Run review` /
  `Run compound` gọi `waggledance_core::bee::herding_agent_argv(root)` — đọc
  `.bee/config.json` của *chính project đích*, giải `agent_command` (mảng argv, hoặc
  một tên tra trong `herding.agents`), spawn pane với nhãn preset `bee:herding`,
  gửi lệnh, ghi `Run`. Không có `herding` thì từ chối có tên:
  *"this project's .bee/config.json records no herding.agent_command"*.
  (`server.rs:1949, 2154-2157, 2280-2308`; `bee.rs:2007-2042`)
- `Local` **bee cũng có sẵn nửa còn lại**: `bee herding wave` đọc
  `herding.agent_command`, tách token 0 thành herdr agent kind + argv, thay `{MODEL}`,
  dựng `HerdrBackend`, chạy fleet choreography, ghi một dòng vào
  `.bee/wave-ledger.jsonl`. (`bee herding wave --help`)
- `Local` **Vòng gom về đã đóng**: bee ghi `.bee/`, watcher đẩy reload qua WebSocket,
  `bee.rs` parse thành `BeeSnapshot`/`BeeProjectRollup`/`BeeAttentionItem`, board
  cross-project render, `waggledance_ask_state` trả rollup cho agent.

Kết luận: **không cần handoff daemon, không cần hàng đợi, không cần cockpit mới.**
Mục #5 và #7 trong Dependency Matrix rơi khỏi phạm vi.

### Ba chỗ hở chặn đúng mô hình đó chạy xuyên dự án

| # | Chỗ hở | Bằng chứng | Cỡ |
|---|---|---|---|
| A | **Chính waggledance không có `herding`** trong `.bee/config.json` → board không Start được gì trên chính nó. `jarvis`, `jarvis-mcp`, `memorypad` cũng vậy; chỉ `beehive` và `collab-review` có. | `Local` khảo sát 6 project dưới `goglbe/` | vài dòng config mỗi project |
| B | **`waggledance_dispatch` không nhìn thấy `herding.agents`** — nó chỉ giải `preset` theo `config.terminal.agent_presets` *toàn cục*, mà list đó đang là `[]`. Nên một agent điều phối **không thể** nói "chạy task này ở project X bằng `pi-agy-flash-3.7`"; chỉ `pane_id` (pane đã sống) là dùng được. Board làm được điều tool không làm được — cùng một máy, hai đường phân giải khác nhau. | `Local` `mcp.rs:664-679, 99-109` (`unknown agent preset`), `~/.waggledance/config.toml` `agent_presets = []`, đối lại `server.rs:2280` | **đây là điểm chốt** — resolver đã tồn tại, chỉ chưa được nối vào tool |
| C | **Resolver không hiểu dạng object mà chính bee ghi ra**: `herding_agent_argv_from_config` chỉ nhận `Value::Array` cho một entry trong `agents`; `agy-flash` trong beehive là `{"argv":[…],"workspace_trust":{…}}` → trả `None` → board từ chối. Hôm nay chưa lộ vì `agent_command` đang trỏ `claude-sonnet` (dạng mảng). | `Local` `bee.rs:2016-2029` đối chiếu `beehive/.bee/config.json` | vài dòng, có mùi lỗi |

### Hệ quả cho tầng platoon

Với mô hình này, **bản kê thành viên trở thành tuỳ chọn, không phải điều kiện tiên
quyết**: `waggledance_projects()` đã liệt kê mọi project đăng ký, nên một Lieutenant
chỉ cần được *bảo* nó phụ trách những project nào. Manifest chỉ còn giá trị làm cho
nhóm đó sống sót qua nhiều phiên và cho board gom theo hàng — một tiện nghi, không
phải nền móng. Thứ tự việc đảo lại: **B trước, rồi A, rồi C**; manifest sau cùng,
hoặc không cần.

### Chỉnh lần hai — "một herdr, có pane thì dùng pane"

Người dùng chốt thêm một luật vận hành: **máy luôn chạy đúng một herdr, và mọi
action nếu đã có pane thì dùng lại pane đó.** Nửa đầu đã là kiến trúc sẵn có —
waggledance không bao giờ tự chạy terminal, herdr là transport duy nhất
(`README.md:139-152`), và board đã theo luật đó một phần: `Run review` /
`Run compound` đi vào pane sống của chính feature khi có, chỉ mở pane mới khi không
(`README.md:132-138`). Nửa sau mới là chỗ vỡ.

- `Local` **Agent bị mù về pane.** `project_panes` (`server.rs:4419-4444`) dựng
  đủ inventory — `pane_id`, `kind` (chính là agent kind: claude/codex/pi/agy, hoặc
  `shell`), status, đã lọc containment theo boundary của project. Nhưng **không tool
  MCP nào phơi nó ra**. `BeeSession` không mang pane id, chỉ có `workspace_id`
  (`bee.rs:616-640`). `waggledance_runs` có `pane_id`, nhưng chỉ của những run do
  chính nó dispatch — không thấy pane người dùng tự mở, pane do nút `Start` mở, hay
  pane của phiên khác. Trong đúng cách anh vận hành (một herdr chung, phần lớn pane
  do người mở), đó là gần như toàn bộ đội hình.
- `Inference` **Nên đảo lại thứ tự chốt.** Điểm chốt không còn là "dispatch spawn
  được theo `herding.agents`" (B) mà là **"agent nhìn thấy được pane đang có"** —
  vì không thấy thì đường tái dùng không bao giờ đi được, và spawn chỉ còn là đường
  lùi. B tụt xuống hạng hai, làm cho nhánh spawn khi không có pane nào.
- `Inference` **Cách rẻ nhất, không đụng D2**: gấp inventory pane vào chính
  `waggledance_ask_state` (nó đã trả sessions / attention / handoff cho từng
  project) thay vì thêm tool thứ tám. Additive, không phải lật D2.
- `Inference` **Luật "có pane thì dùng" là chính sách, nên nó thuộc về agent, không
  thuộc daemon** — đúng D1. waggledance *phơi* pane; Lieutenant *quyết* tái dùng
  trước, spawn sau. Lưu ý D5 vẫn chặn: chỉ pane `Idle`/`Done` mới nhận được lệnh,
  pane `Working`/`Blocked` bị từ chối — nên "tái dùng" nghĩa là tái dùng pane rảnh.

Thứ tự chốt sau chỉnh lần hai: **phơi pane cho agent → B (spawn theo
`herding.agents`) → A (điền `herding` cho các project còn trống) → C (dạng object)**.
