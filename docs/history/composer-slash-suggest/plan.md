# Composer Slash Suggest — Plan

**Lane:** standard · **Class:** feature · **Flags:** none · **Product files:** 4
**Worktree:** `waggledance--wt--composer-slash-suggest` (branch `wt/composer-slash-suggest`)

## Shape

Two cells, one slice, parallelizable (disjoint files):

1. **slash-endpoint** — a new `crates/waggledance/src/slash.rs` module:
   `slash_entries(project_root: Option<&Path>, home: &Path) -> Vec<SlashEntry>`
   scanning the D2 directories; `SlashEntry { name, kind, description }`,
   kind `command|skill`, serialized with serde. Project entries shadow
   user-level entries of the same name+kind; results sorted by name.
   Description: for commands, the first non-frontmatter, non-heading line of
   the `.md` (or frontmatter `description:` when present); for skills, the
   frontmatter `description:` of `SKILL.md`. Truncated ~120 chars.
   Routes in `server.rs`: `GET /p/:id/_slash` (project via
   `st.engine.get_project`, root + home scan) and `GET /_slash` (home only),
   both `Json(Vec<SlashEntry>)`, shaped after `jump_search` (server.rs:7471).
   Unit tests in `slash.rs` over tempdir fixtures.

2. **slash-popup** — `assets/app.js`: one shared `wireSlashSuggest(input, url)`
   applied at each of the three composer wiring sites (D1); trigger/keys per
   D3; fetches the endpoint once per page on first `/`, caches; menu is a
   JS-created element positioned above the textarea, `.slash-menu` /
   `.slash-item` classes. `assets/app.css`: styles imitating `.jump-list`
   (app.css:1952-2027) with the same tokens. `views.rs`: handshake tests in
   the idiom of `agent_page_composer_class_matches_what_app_js_finds_it_by`
   (views.rs:23898) — APP_JS contains the wiring selector and the endpoint
   paths `"/_slash"` / `"/_slash"`-with-project-base.

SMALLER PATH check: the cheapest shape honoring D1–D3 — no views.rs markup
change (menu is JS-created like the jump overlay), no core-crate change, no
config. A static list would drop D2; inlining data in markup was rejected on
the D2 record. PASS.

Hat wave: SKIPPED — recorded (`106f2c49`), clear-ask fast path, same exception
board-run-reaper took.

## Load-bearing claims

| # | Claim | Anchor | Label | Evidence |
|---|-------|--------|-------|----------|
| 1 | `Project.root_path` exists for the scan | `crates/waggledance-core/src/domain.rs:11` | read | `pub root_path: PathBuf` seen in this session |
| 2 | Per-project GET precedent returns `Json(hits)` after `get_project` lookup | `crates/waggledance/src/server.rs:7471-7484` | read | `jump_search` read in full |
| 3 | Route table takes `/p/:id/_jump`-style entries | `crates/waggledance/src/server.rs:508` | read | `.route("/p/:id/_jump", get(jump_search))` |
| 4 | Composer forms are wired once at init; pollers repaint only pane screens | `assets/app.js:3490-3530`, innerHTML sweep | read | forms forEach at 3490; innerHTML hits are screen/overlay only |
| 5 | Jump palette is the popup idiom (overlay, arrows, Escape) | `assets/app.js:315-426`, `assets/app.css:1952-2027` | read | gather digest + spot-read |
| 6 | Handshake test idiom pins APP_JS↔selector both ways | `crates/waggledance/src/views.rs:23898-23918` | read | test read in gather digest |
| 7 | Three composer wiring sites exist (project/unassigned/paseo) | `assets/app.js:3490,3713,4222` | read | gather digest anchors |

## Discovery

Gather digest: `.bee/mailbox/job-1788147778123/report-1.md` (main checkout).
No open questions; no external surfaces.

## Proof

- Cell 1: `cargo test -p waggledance slash` (module tests) — plus build.
- Cell 2: `cargo test -p waggledance views` (handshake tests).
- Cap/merge: `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" cargo test -p waggledance-core -p waggledance --no-fail-fast` — the recorded standard command.

## Later slices

None planned. Possible follow-ups (headlines only): argument-hint display;
plugin-provided commands; fuzzy (not prefix) matching.
