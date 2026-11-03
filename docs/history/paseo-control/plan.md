# Paseo Control — Plan

**Feature:** paseo-control · **Lane:** high-risk
**Route flags:** external-systems, audit-security, cross-platform, multi-domain
**Decisions honored:** D1–D5 (docs/history/paseo-control/CONTEXT.md) — cited inline.
**Revision:** rev 3 — rev 1 was rewritten after the review wave (three BLOCKERs);
rev 2 after the high-risk advisor consult (four edits). See "What review changed".

## What will be built

Clicking a paseo agent row opens that agent's own page. The page shows the
conversation the agent is having, a composer to send it a message, and — when
the agent is blocked on a permission request — Allow / Deny controls. Every
failure of the paseo daemon or CLI is named on the page, never swallowed.

## Why this is high-risk

- **User text into a subprocess.** The composer takes text typed on a page
  reachable from the internet and hands it to the `paseo` CLI.
- **A proxied authorization decision.** Allow/Deny answers a permission request
  on the user's behalf: the page decides what an AI agent may do on this machine.

## Verified facts this plan rests on

Established by direct inspection, not assumption.

1. **`paseo logs <id>` emits a text conversation stream, already collapsed.**
   Lines are `[User] <message>`, plain paragraphs for the agent's replies, and
   one-line tool calls. Observed labels on a live agent: `User, Read, Grep,
   Shell, Bash, Edit, Write, Task, Task Notification, Skill, Thought,
   ToolSearch, AskUserQuestion, EnterWorktree, ExitWorktree`.
2. **There is NO JSON form of `paseo logs`.** `--json`, `-o json`, and
   `--format json` all return the same text stream (verified). The label
   grammar is an unstructured stream owned by another project with no contract —
   which is why S9 makes an upstream rename fail CLOSED.
3. **`paseo send` WAITS for the agent to finish by default.** `--no-wait`
   returns immediately. A timeout-bounded send without `--no-wait` would report
   every real send as a failure — the mirror of D5.
4. **`--prompt <text>` is safe against flag injection** (empirically, a
   dash-leading value is accepted as a value); the bare positional `[prompt]`
   is not. This plan pins `--prompt`.
5. **`validTermBase` (`app.js:21`) hard-rejects any base outside
   `/p/<project>/...`.** It is the second, deliberate gate of review-p1-fixes D3
   against attacker-controlled `data-term-base` in rendered markdown. Widening
   it for `/paseo/` would let a hostile rendered file aim a composer at a live
   agent. **It is not widened.**
6. **`require_loopback_host` is NOT CSRF protection.** `host_is_allowed`
   (`server.rs:619`) accepts any request whose `Host` matches loopback or the
   configured hostname — which a cross-site request to `waggle.gogl.be` does.
   What actually blocks a cross-origin POST is the `Json<T>` extractor demanding
   `application/json`, forcing a CORS preflight the router never answers. This
   is **already documented as D10** on `update_terminal_config`
   (`server.rs:2618-2628`), naming the exact Cloudflare-Access-cookie threat
   model this deployment has. S3 restates that invariant per route rather than
   inventing it; four existing POST handlers use `Form` (CORS-simple) instead,
   which is why it must be named and not assumed.
7. **The core crate cannot host this** — its `no_web_framework_dependency_declared`
   guard (`bee.rs:5086`) forbids tokio in core. Resolves CONTEXT's deferred
   core-vs-binary question: **binary crate**. `core/src/process.rs` holds only
   `apply_detach`; nothing there is reusable.
8. **No existing subprocess call in this repo is timeout-bounded** — the four
   `bee` invocations are unbounded. Bounding the CLI is NEW precedent here.
9. **`paseo` resolves for the daemon**: the systemd user manager's PATH includes
   `~/.local/bin` (verified). It is a bash wrapper, so binary-not-found must
   still be a named outcome (D5), never an assumption.
10. **Refresh is polling** — `POLL_MS` 1500 on terminal pages, 5000 on the
    agents list. No SSE.
11. **`/input` has no body-size cap**; `attach` has `DefaultBodyLimit`, `/keys`
    caps at `MAX_KEYS_PER_REQUEST = 1000`. The send route follows the capped
    precedent, not the omission.

## Security invariants (contracts, not preferences)

- **S1 — argv only.** `Command::new(prog).arg(..)`, never a shell, never string
  concatenation. Matches all nine existing call sites.
- **S2 — `--prompt`, never the positional prompt** (fact 4).
- **S3 — JSON extractor only** on every write route: no `Form`, no `text/plain`
  tolerance (fact 6 / D10). Proven two ways, because the extractor test alone
  gives false confidence: a form-encoded body is refused, **and**
  `OPTIONS /paseo/:id/send` returns no successful preflight — no
  `access-control-allow-origin`. The second test fails the day anyone adds a
  permissive CORS layer anywhere in the router, which the first would not catch.
- **S4 — the agent id is never passed to the CLI unless it names a currently
  live agent.** Enforced on every route: page, poll, send, permit.
- **S5 — control is scoped to agents inside registered projects.** An agent in
  a folder waggledance does not track is listed but not controllable. Its
  refusal names the **per-agent** remedy — register that folder as a project —
  so the fine-grained grant is project registration, not the coarse
  unassigned-group switch, which a user flips once and never flips back. That
  switch remains the escape hatch, not the advertised path. No new config key,
  so D4 holds.
- **S6 — no prompt text, CLI stdout, or CLI stderr reaches `tracing`.** The
  typed error carries a kind only — the discipline
  `transcript_read_failed_response` (`server.rs:3987`) applies to an `io::Error`
  that might carry a path fragment.
- **S7 — D4's switch gates every route**, read and write alike; off means the
  page gives the HTML disabled shape and the JSON routes give
  `terminal_disabled_json_404`.
- **S8 — `Sec-Fetch-Site` is asserted positively.** `require_loopback_host`
  gains a check that the header, when present, is `same-origin` or `none`.
  Absent is allowed (non-browser clients send none; a browser mounting this
  attack always sends it). This turns "no CORS layer exists" from an absence
  into an assertion, and covers every POST in the router at once.
- **S9 — an unrecognized conversation label fails CLOSED.** It renders as the
  LABEL ONLY, never its body. If paseo renames `[Thought]` to `[Thinking]`, the
  withhold rule stops matching — and a body-rendering fallback would publish the
  agent's private reasoning to a page reachable at waggle.gogl.be. A transcript
  where NO line matches any known label renders a named "conversation format not
  recognized" state, not a wall of paragraphs that all read as agent replies.

## Shape — one slice, five cells, serial

The split is by blast radius, not convenience: the read path proves itself
before any write route exists, the shared middleware hardens before the first
button ships, and the one irreversible action gets its own cell.

### pc-1 — the CLI adapter (`crates/waggledance/src/paseo_cli.rs`)

The single door to the `paseo` binary; nothing else invokes it.

- `logs(id, tail)`, `send(id, text)` → `--prompt <text> --no-wait --json`
  (facts 3, 4 / S1, S2), `permit_ls()`, `permit_allow(id, req)`,
  `permit_deny(id, req)`.
- Every call bounded by `tokio::time::timeout` at **`PASEO_CLI_TIMEOUT = 10s`**
  (new precedent, fact 8; follows the `INDEX_HERDR_SNAPSHOT_TIMEOUT` idiom).
- Typed error naming exactly four states D5 needs apart: binary-not-found,
  daemon-unreachable, timed-out, failed(exit code). **S6**: kind only, never
  captured output.
- Program path injectable so tests drive a fixture shell script.
- **Proof includes one recorded smoke against the REAL binary.** A fixture
  script cannot catch a wrong argv *order* — `permit allow <agent> <req>` would
  pass green and fail only in production.

### pc-2 — the agent page, read path (walking skeleton)

`server.rs` + `views.rs`.

- `GET /paseo/:agent_id` and `GET /paseo/:agent_id/conversation` (poll fragment,
  1500ms cadence). No route collision: the only catch-alls are `/p/:id/*path`
  and `/p/:id/_code/*path`.
- **S4/S5 guard on both**, resolved through `paseo::list_live_agents` read via
  `spawn_blocking` from the `AppState::paseo_store_root` seam — never
  `default_store_root()` directly, or every route test reads the developer's
  real `~/.paseo` and every call blocks the reactor.
- **S7** switch gate.
- **Conversation rendering (D2)** — parse by line prefix, HTML-escape every
  segment, collapse to human phrasing:

  | Source line | Rendered |
  |---|---|
  | `[User] <text>` | the user's own turn |
  | plain paragraph | the agent's reply |
  | `[Read] <path>` | "read `<basename>`" — adjacent reads collapse to "read 3 files" |
  | `[Edit]` / `[Write] <path>` | "edited `<basename>`", collapsing the same way |
  | `[Shell]` / `[Bash]` | "ran a command" |
  | `[Grep]` / `[ToolSearch]` | "searched the code" |
  | `[Task]` / `[Task Notification]` | "delegated a task" / "a task finished" |
  | `[Skill]` | "loaded a skill" |
  | `[Thought]` | **withheld** — private reasoning |
  | unrecognized `[Label]` | **the label alone, never its body** (S9) |

- `--tail 200` (a real session ran 622 lines); a short or empty transcript
  renders an empty state, not an error.
- Attribution is best-effort (fact 2): a reply line that itself begins with
  `[User] ` or `[Shell] ` must not be mis-attributed.
- Fills `paseo_agent_row`'s empty `url`, making the drawer row a link.
- D5's four states each render distinctly; S5's refusal names the per-agent
  remedy.
- **Seam proof:** assert the write routes do NOT exist yet.

### pc-3 — assert `Sec-Fetch-Site` in the shared host middleware (S8)

`server.rs`. ~8 lines inside `require_loopback_host`, landing BEFORE any write
route ships. Its own cell because it changes a security path every route in the
router passes through — a different blast radius from the feature's own surface.
Absent header allowed; present-and-cross-site refused.

### pc-4 — send a message

`server.rs` + `views.rs` + `assets/app.js`.

- Composer: extract ONLY the textarea + send-button sub-block of
  `views::pane_controls`. The soft-key grid, Approve, Stage, Paste and attach
  post to herdr routes that do not exist here, and `data-agent-state` Approve
  gating is bee-session-derived and meaningless for paseo — dropped, not carried.
- Its own scoped IIFE reading `data-paseo-base` (the `data-unassigned-base`
  precedent). **`validTermBase` is not widened** (fact 5).
- `POST /paseo/:agent_id/send`, JSON `{text}`. **S3** both tests.
  **`PASEO_SEND_MAX_BYTES = 32 KiB`** app-level named refusal under a
  **`DefaultBodyLimit::max(64 KiB)`** layer, so the app's own message is what
  the user sees.
- Full guard chain repeated (S4, S5, S7).
- In-flight guard: composer disabled while a send is outstanding, poller skips a
  tick during it (`app.js` records this exact defect shipping once before).
- **D5**: a failed send never renders as sent.

### pc-5 — answer a permission request

`server.rs` + `views.rs`. The one irreversible action, alone, on pc-4's guard
helper.

- `POST /paseo/:agent_id/permit` with allow/deny. **S3, S4, S5, S7.**
- Control shown only when `permit_ls` reports a pending request for that agent.
- **Stale `req_id`**: `permit_ls` said pending, the agent moved on, and the id
  now names a DIFFERENT request. The request id sent back is validated against a
  fresh `permit_ls` before the answer is issued; a mismatch is a named refusal,
  never an answer to the wrong question.
- Answering an already-answered request renders the non-zero exit as a named
  state, never a second success.

## Test matrix (high-risk — applicable dimensions)

| Dim | Probe | Cell |
|---|---|---|
| 1 User types | Switch off → page gives the disabled HTML shape; conversation, send and permit give `terminal_disabled_json_404`; no CLI invoked | pc-2, pc-4, pc-5 |
| 2 Input extremes | Empty message refused; over-32-KiB refused with the app's named message, not a bare 413; text with shell metacharacters, a leading `-`, HTML and non-ASCII reaches the CLI as ONE argv element and renders escaped | pc-1, pc-4 |
| 3 Timing | Stale `req_id` refused; a permission answered twice renders a named failure; a send while one is in flight is blocked client-side; the poller skips a tick during a send | pc-4, pc-5 |
| 4 Scale | 600+ line transcript bounded by `--tail 200`; zero-entry transcript renders an empty state | pc-1, pc-2 |
| 5 State transitions | Sending to an id no longer live is refused (S4) | pc-4 |
| 6 Environment | `paseo` binary absent → named state, page still renders; daemon down → named state; the real-binary smoke pins argv order | pc-1, pc-2 |
| 7 Error cascades | Non-zero exit and timeout each produce their own named state, never a success shape | pc-1, pc-4, pc-5 |
| 8 Authorization | An id not in the live set, and a live id outside every registered project, refused on page, poll, send AND permit; a form-encoded POST refused; `OPTIONS` returns no `access-control-allow-origin`; a cross-site `Sec-Fetch-Site` refused | pc-2, pc-3, pc-4, pc-5 |
| 10 Integration | An unrecognized label renders label-only, never its body (S9); a transcript matching no known label renders the named unrecognized-format state; a reply line beginning with a label string is not mis-attributed | pc-2 |
| 11 Compliance | Prompt text, CLI stdout and stderr never reach `tracing` (S6) | pc-1, pc-4, pc-5 |

Dimensions 9 and 12 do not apply: no data model, no business rule with boundary
values.

## Why this size

Five cells is the floor once the two irreversibles are separated: a shared
security middleware that every route passes through, and an authorization
answer the user cannot take back. SMALLER PATH check: **PASS** — fact 1 already
deleted the expensive piece a naive shape would carry (an event-model parser),
and no locked decision is shrunk to reach this size.

## Cost if the shape is wrong

Moderate. The read path is additive and gated. The write path is the exposure: a
mistake sends text to an agent, or approves a permission, the user did not
intend. That is why S1–S9 are contracts, why the guard repeats on every route,
and why D5 forbids a silent failure.

## What review changed

**Review wave (three BLOCKERs, all confirmed by inspection):** `paseo send`
waits by default → `--no-wait`; the composer cannot be reused by widening
`validTermBase` without weakening a deliberate gate → own scoped IIFE;
`require_loopback_host` is not CSRF protection → S3 as a tested invariant. Plus:
control scoped to registered projects (S5), the D2 collapse mapping specified,
and a label-spoofing probe.

**Advisor consult (four edits, all taken):**
1. The form-encoded test alone gives false confidence → S3 adds the `OPTIONS`
   preflight test, which fails the day a CORS layer appears.
2. Absence-of-CORS became a positive assertion → S8, `Sec-Fetch-Site`, its own
   cell (pc-3) because it touches every route.
3. Send and permit were bundled; permit is the irreversible one → split into
   pc-4 and pc-5, and pc-5 gained the stale-`req_id` case the matrix had missed.
4. An upstream label rename would have leaked `[Thought]` to a public page →
   S9 makes unrecognized labels render label-only, with an unrecognized-format
   canary.

It also corrected this plan: the JSON/preflight property is **not** undocumented
— `server.rs:2618-2628` (D10) names it, with this deployment's exact threat
model. Fact 6 now cites it.

## Proof / test scope

- pc-1: `cargo test -p waggledance paseo_cli` plus one recorded real-binary smoke.
- pc-2..pc-5: `cargo test -p waggledance` — the FULL package suite, per
  `docs/knowledge/patterns/assertions-that-pin-literal-adjacency.md`.
- CI runs `cargo test --workspace` on every push.

## Execution notes

New feature worktree (`bee worktree new --feature paseo-control`); the
paseo-support worktree is a different branch and is not reused.
