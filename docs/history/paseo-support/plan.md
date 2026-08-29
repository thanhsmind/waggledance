# Paseo Support — Plan

**Feature:** paseo-support · **Lane:** standard · **Route flags:** external-systems, cross-platform
**Decisions honored:** D1–D5 (docs/history/paseo-support/CONTEXT.md) — cited inline below.
**Revision:** rev 2 — rev 1 was redrafted after the planning review wave found it
duplicating a shipped feature (see "Review findings applied").

## What will be built

Waggledance's board shows the paseo agents currently live on this machine as
badges on the project they run in, and folds the folders paseo is working in
that waggledance does not track into the **existing** "Suggested projects"
block, which already carries a Register button. Display-only (D1): no input,
no control, no daemon API.

## Shape

One slice, two cells. Cell 1 is a new leaf module; cell 2 is additive wiring
into three existing seams — no new route, no new section, no new markup block.

### Cell 1 — core reader: `paseo.rs`

New module `crates/waggledance-core/src/paseo.rs` (+ one `pub mod paseo;` in
`lib.rs`). Sync filesystem work only; the core crate is sync-only and its
`no_web_framework_dependency_declared` guard test (bee.rs:5089) forbids
axum/tokio/hyper — resolves the deferred "core vs binary" question: **core**.

- `PaseoAgent { id, provider, cwd: PathBuf, title, last_status,
  last_activity_at: String, model: Option<String> }`, parsed with
  `#[serde(rename_all = "camelCase")]` because the store is camelCase
  (`archivedAt`, `lastStatus`, `lastActivityAt`); `model` comes from the
  nested `config.model`. `last_activity_at` stays the raw RFC-3339 string —
  the render side formats it, so the seam carries no time type.
- `list_live_agents(store_root: &Path) -> Vec<PaseoAgent>`: enumerate exactly
  two levels, `<store_root>/<slug>/<uuid>.json` (a non-directory at level one
  and anything deeper are skipped), keep records with `archivedAt`
  absent/null AND `lastStatus != "closed"` (D4). Tolerant: unreadable dir,
  unparseable file, or missing required field is skipped, never an error.
- `default_store_root() -> Option<PathBuf>` = `dirs::home_dir()?.join(".paseo/agents")`,
  matching the `dirs::home_dir()` convention at config.rs:185.
- Tests: inline `#[cfg(test)]` + `tempfile` (both already deps) — live kept,
  closed excluded, archived excluded, malformed skipped.

### Cell 2 — board integration through the existing seams

`crates/waggledance/src/server.rs` + `crates/waggledance/src/views.rs`.

1. **Test seam.** `AppState` gains `paseo_store_root: Option<PathBuf>`,
   `None` meaning `default_store_root()` — the exact shape `transcript_root`
   (server.rs:52, defaulted at 269 and 6747, overridden per-test at 16065)
   already uses. An axum handler takes no extra argument; this is the only
   seam that lets the exit test point at a fixture store.
2. **Read.** `index_page` reads live agents inside `spawn_blocking` (sync FS
   work off the request thread, the discipline `register_project` uses at
   server.rs:2752 — index_page itself does not do this today, so this is
   borrowed precedent, not existing shape).
3. **Mapping (D5).** Per project, the page already builds
   `Boundary::new(vec![root])` and calls `project_panes`, whose membership
   test is `boundary.validate_existing(cwd)` (server.rs:4480) —
   canonicalizing, symlink-resolving, fail-closed. Paseo agents map through
   that same `validate_existing`, **not** the raw `is_contained_in_root`
   predicate, which documents itself (paths_boundary.rs:257-266) as a second
   guard only, never a replacement.
4. **Matched agents (D1).** A matched agent renders as a badge on its project
   row beside the existing herdr badges: provider, model, and relative age of
   `lastActivityAt` (an old "running" record shows its age — no liveness
   probe, per CONTEXT discretion). The agent's `title` is prompt text, so it
   is **not** rendered: the page's precedent is agent kind/status, never
   prompt content.
5. **Untracked folders (D3).** `suggested_projects` (server.rs:5409) already
   dedups unregistered cwds, drops traversal paths, applies the owned-subtree
   guard, and views.rs:1258 already renders each as a Register form POSTing
   `path` to `/api/projects/register`. Paseo cwds fold into that same
   aggregation; `ProjectSuggestion` gains `paseo_count`, and the row's meta
   text appends ", N paseo agent(s)" only when that count is non-zero — so
   every existing pane-only assertion stays byte-identical. No new block, no
   new route, no auto-register.
6. **Disclosure gate.** The suggestions block is gated on
   `terminal_family_enabled` (server.rs:790-796) because host-wide cwds on an
   unauthenticated `/` are a disclosure (toa-4). Paseo rows are the same
   class and ride the same switch — no new config surface, and switched off
   the page is byte-identical to today.
7. **Whole-path proof** (pattern: prove-the-whole-path): one route test
   through `router(st)` + `get(app, "/")` with a fixture store and a tracked
   project asserts the badge appears on that project's row, and that an
   untracked cwd produces a suggestion row whose form action is
   `/api/projects/register`.

### Not in this slice

- MCP tool for the mapped agents — discretionary; the browser view is the
  committed surface. Headline only, no cell.
- Interactive control / daemon API — out of scope (D1).
- Showing the agent title — deliberately withheld (see 4); a later toggle if
  the user wants it.

## Why this size

Rev 1 planned its own untracked-projects block with its own Register button;
the review found that block already shipped. Rev 2 deletes that work and
spends the cell on three small extensions instead. Smaller-path check: PASS
after redraft — the surviving cost is one leaf module plus `paseo_count`, and
no locked decision is shrunk to get it.

## Cost if the shape is wrong

Low: one new module, one additive `AppState` field, one additive struct field.
Nothing existing changes behavior with the switch off. Known risk from the
bundle (assertions-that-pin-literal-adjacency, 21 recorded hits): board tests
pin markup ordering, so both cells run the FULL package suite, not scoped
anchors, before capping.

## Review findings applied

The planning review wave raised three BLOCKERs and one WARNING against rev 1,
all structural, all fixed above: the mapping helper (`validate_existing`, not
`is_contained_in_root`), the duplicate suggestions block (fold into
`suggested_projects`), and the missing disclosure gate
(`terminal_family_enabled`). Its CRITICALs against the cells — the missing
`AppState` seam, the unanchored render test, the unnamed mapping helper — are
in cell ps-2; its MINORs (camelCase serde, enumeration depth, verify scope)
are in ps-1.

## Proof / test scope

- ps-1: `cargo test -p waggledance-core` (new paseo tests + crate green).
- ps-2: `cargo test -p waggledance` (whole-path route test + full package
  suite per the adjacency pattern).
- CI runs `cargo test --workspace` on push, the deterministic net.

## Execution notes

Work lands in worktree `waggledance--wt--paseo-support` (branch
`wt/paseo-support`), already merged up to main.
