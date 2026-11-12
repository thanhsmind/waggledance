# anphabe-intake — plan

Lane: standard (2 flags: external systems — the pass calls the `claude` CLI and, through it, the anphabe MCP; data model — a new ledger table and migration). Decisions: `CONTEXT.md` D1-D7.

## Summary

waggledance gains an intake pass. Each pass runs Claude Haiku 5.5 once, which reads the user's Visual Feedback comments and anphabe tasks and returns the code items with a guessed project and a confidence. waggledance records every item once, sends each confident item to its project's leader, and holds the rest. `waggledance intake run` makes one pass by hand and prints what was sent, held and skipped; `waggledance intake list` shows the held items. With `[intake] enabled = true` the daemon makes a pass every 15 minutes and announces new held items once through the notify channel. Nothing is written back to anphabe.

Mode: `standard` — 2 risk flags: external systems, data model
Why this is the least workflow that protects the work: the model only classifies; the steps that can double-send (ledger, threshold, dispatch) are plain Rust behind unit tests with a fake runner, and dispatch reuses the existing `waggledance_dispatch` handler with its `request_id` duplicate guard.

## Load-bearing claims

Labels: `read` = opened at that line this session; `ran` = command run this session. Evidence is a verbatim substring of the anchored line.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | A crate-internal entry runs any MCP tool against an engine, so the pass can dispatch through the same handler as MCP | read | crates/waggledance/src/mcp.rs:106 | `pub(crate) fn call_tool_on(engine: &Engine, name: &str, args: Value) -> Value {` |
| 2 | `waggledance_dispatch` takes a caller-chosen `request_id` that stops a retry from sending twice | read | crates/waggledance/src/mcp.rs:282 | `"request_id": { "type": "string", "description": "Caller-chosen id for this request — pass one on every call so a retry never sends twice" },` |
| 3 | Background loops are switched by config inside `reconcile`, one slot per loop | read | crates/waggledance/src/main.rs:249 | `self.reconcile_sweep(engine.clone().filter(\|e\| e.config.worktree_sweep.enabled));` |
| 4 | The registry schema is at version 14, so the ledger is migration 15 | read | crates/waggledance-core/src/repository.rs:977 | `pub const SCHEMA_VERSION: i64 = 14;` |
| 5 | A notifier delivers one alert from a kind and a body | read | crates/waggledance/src/notify/mod.rs:44 | `async fn send(&self, kind: &str, body: &str) -> Result<()>;` |
| 6 | A headless Haiku run with only the mcp-gateway tools fetches and classifies the user's items in about 36 s for about 0.012 USD | ran | `claude -p --model claude-haiku-5-5 --output-format json --allowedTools "mcp__mcp-gateway__call_tool,mcp__mcp-gateway__get_tool_definition,mcp__mcp-gateway__search_tools" < prompt.txt` | `cost 0.011900580000000001 turns 4 ms 36000` |
| 7 | `--allowedTools` swallows a trailing prompt argument, so the prompt must go on stdin | ran | `claude -p --model claude-haiku-5-5 --output-format json --allowedTools "..." "<prompt>"` | `Error: Input must be provided either through stdin or as a prompt argument when using --print` |

## Discovery

Read `main.rs` `TerminalBackground::reconcile` and `worktree_sweep.rs` (periodic opt-in loop with a first-tick delay), `repository.rs` `MIGRATIONS`, `mcp.rs` `call_tool_on` and the dispatch schema, `notify/mod.rs` and `notify/telegram.rs`, `daemon.rs` `telegram_credentials`. Ran the Haiku pass twice (rows 6-8); the anphabe tool schemas (`list_my_tasks`: email, mode owner|collab; `list_review_comments`: author, mentioned, status) came from `mcp-gateway get_tool_definition`.

## Approach

Recommended path (D3, D4): a `claude -p` subprocess behind a runner trait, so tests inject canned output. The prompt lists the registered projects (id, name, root path) and asks for a JSON array of items `{key, source, id, title, url, project, confidence, reason, twin_of}`. Rust parses it, drops non-code items (D1), and for each new item writes one ledger row with a verdict: `dispatched` (D4), `held` (D5), `skipped` (not code) or `covered` (a twin of an item already dispatched). Dispatch goes through `call_tool_on(engine, "waggledance_dispatch", {project, task, leader: true, request_id: "intake-<source>-<id>"})`. An item already in the ledger is never re-dispatched by a later pass.

Rejected: a Rust anphabe client (credentials and schema live in mcp-gateway); letting Haiku call `waggledance dispatch` itself (no ledger, no threshold); a JSON-file ledger (the registry already holds runs, and the migration pattern is established).

Risk map:
- Haiku output shape drifts (MEDIUM, lands in ai-2): the parser accepts a fenced or bare array and a pass with unparseable output records nothing and logs the raw head; proof is a unit test per shape.
- Double dispatch (MEDIUM, lands in ai-1/ai-2): the ledger primary key and the dispatch `request_id` are both `intake-<source>-<id>`; proof is a test that runs two passes over the same canned output and counts one dispatch.
- The `claude` binary missing under the daemon's PATH (LOW, ai-3): the binary is a config value defaulting to `claude`; a spawn failure logs once per pass and records nothing.

Waves: ai-1 → ai-2 → ai-3 → ai-4, serial because each builds on the previous types and ai-2/ai-3 both touch `main.rs`.

## Cells

```json
[
 {
  "id": "ai-1",
  "feature": "anphabe-intake",
  "title": "Add the intake config section and the intake ledger table",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance-core/src/config.rs",
   "crates/waggledance-core/src/repository.rs",
   "crates/waggledance-core/src/engine.rs",
   "crates/waggledance-core/src/lib.rs",
   "crates/waggledance-core/src/intake.rs"
  ],
  "action": "Per D2, D3, D4, D5, D7. (1) config.rs: add an `[intake]` section to Config with fields enabled (default false), interval_minutes (default 15), user_email (default empty; the pass refuses to run while it is empty), model (default \"claude-haiku-5-5\"), claude_binary (default \"claude\"), min_confidence (default 0.8); a config file without the section keeps loading. (2) repository.rs: migration 15 adds an intake ledger table keyed by item key (`<source>-<id>`), holding source, item id, title, url, project (nullable), confidence, reason, verdict (dispatched | held | skipped | covered), run_id (nullable), notified flag, first_seen timestamp; add it to SCHEMA too, bump SCHEMA_VERSION to 15, and keep the migrate-from-older-version test pattern. Store methods: insert an item only when its key is absent (report whether it was inserted), list items by verdict, mark items notified. (3) Expose these through Engine the way other store calls are exposed, with the plain data types in a new core intake module. Tests: config defaults and an explicit section; migration from a v14 database keeps runs and adds the table; insert-if-absent returns false on the second insert and leaves the first row unchanged; list by verdict. No code comments.",
  "verify": "cargo test --profile fast -p waggledance-core",
  "affects_skills": [],
  "affects_specs": [],
  "deps": [],
  "decisions": ["389f77b0-e799-4624-ac9c-448f2c6fc672", "905ef00e-6f33-492b-af45-5d93c224cac9"],
  "read_first": [
   "docs/history/anphabe-intake/CONTEXT.md",
   "docs/history/anphabe-intake/plan.md",
   "crates/waggledance-core/src/config.rs",
   "crates/waggledance-core/src/repository.rs",
   "crates/waggledance-core/src/engine.rs"
  ],
  "must_haves": {
   "truths": [
    "A config with no [intake] section loads with intake disabled, interval 15, model claude-haiku-5-5, min_confidence 0.8",
    "Opening a v14 registry migrates it to v15 with its runs intact and an empty intake ledger",
    "Inserting the same item key twice keeps one row and reports the second insert as not inserted"
   ],
   "artifacts": [
    {"path": "crates/waggledance-core/src/intake.rs", "substantive": "intake item and verdict types"},
    {"path": "crates/waggledance-core/src/repository.rs", "substantive": "migration 15, ledger table and its store methods"}
   ],
   "key_links": ["Engine exposes the ledger methods the waggledance crate will call"],
   "prohibitions": ["No change to existing tables or earlier migrations", "No literal user email in code", "No code comments"],
   "open_choices": ["Column names and the timestamp format"]
  }
 },
 {
  "id": "ai-2",
  "feature": "anphabe-intake",
  "title": "Run one intake pass: Haiku classifies, waggledance records, dispatches or holds; add the intake CLI verbs",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance/src/intake.rs",
   "crates/waggledance/src/cli.rs",
   "crates/waggledance/src/agent_cli.rs",
   "crates/waggledance/src/main.rs"
  ],
  "action": "Per D1-D6 in a new crates/waggledance/src/intake.rs. A runner trait takes the prompt and returns the model's text; the real runner spawns `<claude_binary> -p --model <model> --output-format json --allowedTools mcp__mcp-gateway__call_tool,mcp__mcp-gateway__get_tool_definition,mcp__mcp-gateway__search_tools` in a scratch directory under the data dir, writes the prompt on stdin (claim 7), enforces a timeout, and returns the `result` field of the JSON envelope. The prompt names the user email from config and states D1 and D2: Visual Feedback root comments (status open) the user wrote, that mention the user, or that sit on a review page the user created, and the user's open tasks in owner and collab mode; keep code work, drop digital/marketing work; match each to one of the registered projects (id, name, root path, passed in the prompt) or null; mark a twin when a task and a comment describe the same work; return only a JSON array with key, source (task|feedback), id, title, url, body, page_url, selector, image_urls, project, confidence, reason, is_code, twin_of. The parser accepts a bare array or one inside a markdown fence; unparseable output fails the pass with the first 300 characters in the error and records nothing. Decide per item, skipping keys already in the ledger: not code → skipped; twin of a key dispatched in this or an earlier pass → covered; project registered and confidence >= min_confidence → dispatch via call_tool_on(engine, \"waggledance_dispatch\", {project, task, leader: true, request_id: \"intake-<key>\"}) and record dispatched with the run_id; otherwise held. A dispatch error records held with the error as reason. The task text handed to the leader carries title, body, url, page_url, selector, image links and the D1 instruction to do the work in that project's own chain. Never call any anphabe write tool (D6); the allowed tool list has no write tools, and the prompt says read only. A pass returns a report {dispatched, held, skipped, covered, already_seen} of item rows. Refuse to run with an error naming `[intake] user_email` while it is empty. CLI: `waggledance intake run` makes one pass and prints the report as JSON; `waggledance intake list [--verdict held|dispatched|skipped|covered]` prints ledger rows (default held); errors go to stderr with exit 1, matching the agent CLI convention. Tests with a fake runner and a temp registry: fenced and bare output parse; a non-code item is skipped; a confident item on a registered project is dispatched once and a second pass over the same output dispatches nothing; a low-confidence item and an unknown project are held; a twin is covered; garbage output records nothing. Stub the dispatch call behind the same seam the tests can observe, without spawning Paseo. No code comments.",
  "verify": "cargo test --profile fast -p waggledance intake",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["ai-1"],
  "decisions": ["ad9b778b-8bd1-4040-99f6-99f803f7b93c", "389f77b0-e799-4624-ac9c-448f2c6fc672", "905ef00e-6f33-492b-af45-5d93c224cac9"],
  "read_first": [
   "docs/history/anphabe-intake/CONTEXT.md",
   "docs/history/anphabe-intake/plan.md",
   "crates/waggledance/src/mcp.rs",
   "crates/waggledance/src/agent_cli.rs",
   "crates/waggledance/src/cli.rs",
   "crates/waggledance-core/src/intake.rs"
  ],
  "must_haves": {
   "truths": [
    "Two passes over the same canned model output dispatch each confident item exactly once",
    "An item below min_confidence or with an unregistered project is held, never dispatched",
    "A non-code item is recorded skipped and never dispatched",
    "waggledance intake list prints held items by default"
   ],
   "artifacts": [
    {"path": "crates/waggledance/src/intake.rs", "substantive": "runner trait, real claude runner, prompt, parser, decision and pass report"}
   ],
   "key_links": ["dispatch goes through mcp::call_tool_on with waggledance_dispatch, leader true and request_id intake-<key>", "the intake verbs appear in waggledance schema and search"],
   "prohibitions": ["No anphabe write tool in the allowed list or the prompt", "No change to waggledance_dispatch behaviour", "No code comments"],
   "open_choices": ["Exact prompt wording and the subprocess timeout value"]
  }
 },
 {
  "id": "ai-3",
  "feature": "anphabe-intake",
  "title": "Run the intake pass in the daemon on its interval and announce new held items once",
  "lane": "standard",
  "role": "code",
  "files": [
   "crates/waggledance/src/intake.rs",
   "crates/waggledance/src/main.rs"
  ],
  "action": "Per D3, D5, D7. Add an intake loop slot to TerminalBackground reconciled like reconcile_sweep: running only when `[intake] enabled` is true and user_email is set; each tick (interval_minutes, first tick after a short delay, missed ticks skipped, one pass at a time) runs the ai-2 pass on a blocking-safe path and logs the report counts. After a pass, items with verdict held and notified false are announced in one notifier message (kind \"intake\", body listing title, project guess or none, confidence and url per item) and then marked notified; with no Telegram credentials the NullNotifier path only logs, as for other alerts. A failed pass logs a warning and the loop keeps running. Tests: the slot runs when enabled with an email and stops when disabled (pattern of the existing sweep_running tests); held items are announced once across two passes with a recording notifier. No code comments.",
  "verify": "cargo test --profile fast -p waggledance intake",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["ai-2"],
  "decisions": ["905ef00e-6f33-492b-af45-5d93c224cac9"],
  "read_first": [
   "docs/history/anphabe-intake/plan.md",
   "crates/waggledance/src/main.rs",
   "crates/waggledance/src/worktree_sweep.rs",
   "crates/waggledance/src/notify/mod.rs",
   "crates/waggledance/src/daemon.rs"
  ],
  "must_haves": {
   "truths": [
    "With [intake] enabled and user_email set the daemon starts the intake loop; disabling it stops the loop",
    "A held item is announced through the notifier once, not on every pass"
   ],
   "artifacts": [
    {"path": "crates/waggledance/src/main.rs", "substantive": "intake slot reconciled from config"}
   ],
   "key_links": ["reconcile starts the intake loop from engine.config.intake"],
   "prohibitions": ["No change to the other loops' switches", "No code comments"],
   "open_choices": ["First-tick delay length"]
  }
 },
 {
  "id": "ai-4",
  "feature": "anphabe-intake",
  "title": "Consolidate this feature's knowledge and write its delivery",
  "lane": "standard",
  "role": "writer",
  "files": [
   "docs/knowledge/**",
   "docs/history/anphabe-intake/delivery.md"
  ],
  "action": "Run the writer consolidation step on this feature: run `bee knowledge consolidate --feature anphabe-intake`. Record the intake pass in the supervisor-daemon area (the loop, the `[intake]` section, the ledger verdicts, the `intake run|list` verbs, that the model only classifies and never dispatches or writes to anphabe), anchored to the code. Dedupe and trim, never restate code, and write docs/history/anphabe-intake/delivery.md listing the concepts changed and why, one line each. Run `.bee/bin/bee knowledge check` green. One commit.",
  "verify": ".bee/bin/bee knowledge check",
  "affects_skills": [],
  "affects_specs": [],
  "deps": ["ai-1", "ai-2", "ai-3"],
  "decisions": ["ad9b778b-8bd1-4040-99f6-99f803f7b93c", "389f77b0-e799-4624-ac9c-448f2c6fc672", "905ef00e-6f33-492b-af45-5d93c224cac9"],
  "read_first": [
   "docs/history/anphabe-intake/CONTEXT.md",
   "docs/history/anphabe-intake/plan.md",
   "docs/knowledge/areas/supervisor-daemon.md"
  ],
  "must_haves": {
   "truths": [
    "the supervisor-daemon area describes the intake pass, anchored to code",
    "docs/history/anphabe-intake/delivery.md lists every concept the feature changed"
   ],
   "artifacts": [
    {"path": "docs/history/anphabe-intake/delivery.md", "substantive": "the feature's delivery record"}
   ],
   "key_links": ["the delivery lands in the same merge as the code"],
   "prohibitions": ["No restating of code in area concepts", "No code comments"],
   "open_choices": []
  }
 }
]
```

## Test matrix

- Happy: fake runner returns one confident survey-platform item. Pass when the ledger holds one `dispatched` row with a run_id and the dispatch seam saw `request_id` `intake-task-<id>`.
- Happy: `waggledance intake list` after a pass with one low-confidence item. Pass when stdout is JSON with that one row, verdict `held`.
- Edge: the same canned output on a second pass. Pass when the report's `dispatched` is empty and `already_seen` lists the key.
- Edge: a task and a comment marked twins. Pass when one is `dispatched` and the other `covered`.
- Edge: fenced vs bare JSON output. Pass when both parse to the same items.
- Error: garbage output. Pass when the pass errors with the first 300 characters and the ledger is unchanged.
- Error: empty `user_email`. Pass when `waggledance intake run` exits 1 and stderr names `[intake] user_email`.

## Proof at feature end

`bee test` in the worktree, then one live `waggledance intake run` from the fast build against the real registry with a temporary `[intake]` config and the artifact confirmed by content, reporting the dispatched, held and skipped counts. Live dispatch to real leaders happens on that run, so it runs only after the user confirms.

## Open Questions

(none)

## Out of scope

- Writing anything back to anphabe (D6).
- Enabling `[intake]` in the user's real config; the user turns it on.
- A release and version bump.
