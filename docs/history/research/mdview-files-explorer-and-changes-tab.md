---
artifact_contract: bee-research/v1
topic: mdview-files-explorer-and-changes-tab
depth: standard
date: 2026-08-30
---

## Bottom Line

- **Recommendation (ladder rung): `build` — with a large `reuse` base.** There is
  nothing left to port from mdview. The code viewer this repo already ships *is*
  the mdview feature, brought across whole on 2026-08-13
  (`docs/knowledge/work/upstream-code-viewer/delivery.md`). Everything the
  screenshot adds — a persistent nested file tree, per-filetype icons, a
  **Changes** tab — exists in neither repo.
- **Why this is the lightest credible path:** the expensive, security-critical
  half is already built and tested. `code_source.rs` (path canonicalisation,
  denylist, gitignore-awareness, binary sniffing, 2 MiB cap) and
  `render.rs::highlight_source` (syntect, per-line) cover file access and
  rendering. Only the *navigation shell* and the *git layer* are new.
- **Why the next-best rung lost:** `adapt-upstream` (rung 3) has no source.
  mdview @ `6875119` (v0.7.3) is **behind** this fork on every relevant file —
  `views.rs` 933 lines vs 21 602 here, `server.rs` 1 168 vs 33 759 — and its
  shipped sidebar is the *same* breadcrumb-zoom widget, deliberately chosen over
  a tree. `reuse` (rung 1) alone cannot deliver: no tree, no icons, no git.
- **Confidence: 90%.** Both repos read directly at their current commits; the
  only soft spot is what the screenshot is meant to imply (see Assumptions).
- **Suggested next step: `bee-shaping`.** Three gray areas below decide the size
  of this work before a plan can be honest.

## Repo Snapshot

`Local` — Rust workspace, edition 2021, v0.5.2. `crates/waggledance-core`
(engine, indexer, code_source, render) + `crates/waggledance` (axum 0.7 server,
CLI, MCP, views) + an excluded Tauri shell. Storage rusqlite 0.32 + FTS5;
walking via `ignore` 0.4; highlighting via `syntect` 5; markdown via `comrak`
0.29 + `ammonia` 4. The web UI is **server-rendered HTML built by string
`format!` inside `views.rs`**, with one hand-written `assets/app.js` (3 735
lines) and `assets/app.css` (3 274 lines) — no framework, no build step, no
bundler. That is the constraint that shapes every answer here.

## Question & Assumptions

- **What was asked:** distil `refs/mdview`; the code-file viewer is already
  supported there, so upgrade the docs viewer to the "Files" experience in the
  screenshot — a full nested project tree with a sibling **Changes** tab.
- **What success appears to mean:** the viewer stops being a two-mode thing
  (Docs section / Code section, each showing one folder at a time) and becomes a
  single IDE-shaped surface: one persistent tree of the whole project on the
  left, any file openable from it, and a second tab showing what changed.
- **Assumptions still needing confirmation:**
  1. The screenshot is **this repo's own root**, rendered by an IDE explorer, not
     by mdview or waggledance — the entry list matches
     `ls -a /home/thanhsmind/Projects/goglbe/waggledance` exactly, dot-dirs and
     order included. Read as *the target shape*, not as something to copy code
     from. `Inference`
  2. It shows `.git/` and `target/`. Both are **unservable today by design** (see
     Risks). Assumed to be incidental IDE behaviour, not a requirement.
  3. "Changes" is assumed to mean git working-tree changes (status + diff), not
     the file-watcher's change events.

## Findings

### Local

`Local` — **The code viewer is complete and shipped.**

- Routes: `crates/waggledance/src/server.rs:614-615` —
  `/p/:id/_code/` → `code_root`, `/p/:id/_code/*path` → `code_dir_or_file`
  (handlers at `server.rs:7047-7056`). Registered before the `/p/:id/*path`
  catch-all; `server.rs:592` notes these are the only two catch-alls.
- Views: `views.rs:9553 code_page`, `views.rs:9634 code_dir_page`,
  `views.rs:9717 code_tree`. `code_page` emits a `<table class="codeview__table">`
  with `<tr id="L{n}">` rows and `#L{n}` gutter anchors.
- Access layer: `crates/waggledance-core/src/code_source.rs` —
  `resolve_source_path` (`:52`), `list_dir` (`:69`), `read_source` (`:103`),
  plus `DirEntry` / `DirListing` / `SourceContent`. `read_source` caps at 2 MiB,
  sniffs binary by NUL byte / invalid UTF-8, truncates at the last whole line.
- Highlighting: `render.rs::highlight_source` returns
  `HighlightedSource { lines: Vec<String>, syntax_name }` — line numbers and
  anchors are the view's job, so a diff view can reuse the same per-line output.

`Local` — **Both sidebars are breadcrumb-zoom, not trees.**

- Docs: `views.rs:9848 file_tree` server-renders a no-JS fallback of the active
  folder's files; `app.js:160-278` then re-renders client-side from a
  `#filelist` JSON blob — breadcrumbs (`.chap-crumbs`) to zoom out, one
  collapsible **Subfolders** disclosure (`.chap-folders`) to zoom in,
  `.chap-file` links for this folder only.
- Code: `views.rs:9717 code_tree` server-renders the identical markup from a
  single-directory `DirListing`. Its sidebar search input is
  `disabled` (`views.rs:9719`) — a placeholder, not a feature.
- **No nested expandable filesystem tree exists anywhere in the repo.** The only
  other tree-shaped UI are the TOC outline (`views.rs:9442`, flat, indent-only)
  and the home projects rail (`app.js:531-760`, collapsible groups).

`Local` — **No git integration at all.** `git2`: 0 hits. `Command::new("git")`:
0 hits across `crates/*/src`. The only git-adjacent code is `ignore`'s
gitignore *file* parsing (`code_source.rs:218 build_gitignore` reads literal
`.gitignore` files, explicitly working with no `.git` present) and the denylist
that refuses `.git` outright (`code_source.rs:149-163`).

`Local` — **The index is markdown-only and stays that way.**
`indexer.rs:14 const MARKDOWN_EXTS: &[&str] = &["md", "markdown"];` Code files
are never indexed — they are read from disk on demand. This is a deliberate
locked decision (mdview D1, inherited): keep FTS5 md-only, avoid index bloat on
large repos. A whole-project tree must therefore be built from a live `ignore`
walk, **not** from the SQLite store.

`Local` — Knowledge coverage is thin: `docs/knowledge/work/upstream-code-viewer/`
records the port; nothing in `docs/knowledge/` covers the sidebar model or
navigation UX.

### Upstream

`Upstream` — Source manifest:

| Field | Value |
|---|---|
| Repo or path | `/home/thanhsmind/Projects/refs/mdview` |
| Ref | `main` |
| Resolved commit SHA | `6875119e2fb84f3dab6c3c10f7ef81206dc1f204` (v0.7.3, 2026-08-14) |
| Narrowed scope | `_code` routes/views, `code_source.rs`, `render.rs`, `app.js`, `app.css`, plans + history for file-nav and diff |

**Dependency matrix**

| Component | mdview (source) | waggledance (local) | Verdict |
|---|---|---|---|
| `_code` routes | `server.rs:168-169` `Upstream` | `server.rs:614-615` `Local` | `EXISTS` — identical |
| `code_page` / `code_dir_page` / `code_tree` | `views.rs:287/367/449` `Upstream` | `views.rs:9553/9634/9717` `Local` | `EXISTS` — same markup |
| `code_source.rs` | 412 lines `Upstream` | 412 lines `Local` | `EXISTS` — byte-for-byte same size |
| `highlight_source` | `render.rs:262` `Upstream` | `render.rs` (696 vs 675 lines) `Local` | `EXISTS` |
| Nested file tree | absent `Upstream` | absent `Local` | `NEW` |
| Per-filetype icons | absent — only a generic folder emoji on `.chap-subfolder::before`, `app.css:399-403` `Upstream` | absent `Local` | `NEW` |
| "Changes" / diff tab | absent in code; **deferred** in `plans/260812-1458-code-viewer-section/plan.md:46` and `docs/history/code-viewer-section/CONTEXT.md:14` `Upstream` | absent `Local` | `NEW` |
| git dependency (`git2` / shell git) | 0 hits `Upstream` | 0 hits `Local` | `NEW` |
| Auth (`auth.rs`, `cf_access.rs`) | present `Upstream` | absent — this fork dropped it `Local` | `CONFLICT` — see Risks |

**Cross-cutting sweep** `Upstream` — wiring outside the feature folder that a
tree would touch: `app.js:380-404` mobile drawer keyed on `.layout` / `.sidebar`
/ `#sidebar-toggle`; `app.css:317-381` the `.chap-folders` grid `0fr→1fr` reveal
animation; `topbar_full` centre slot carrying the Docs|Code switch;
`sessionStorage["waggledance-folders-open"]` (`app.js:190`) and
`localStorage["waggledance-rail-collapsed"]` (`app.js:531+`) as the existing
persistence idiom. The watcher (`watch.rs`) filters markdown, so a tree gets no
live-reload for free. Nothing else was found; components not listed here are
**unchecked**, not confirmed clean.

`Upstream` — **The flat model was a decision, not an oversight.**
`docs/history/file-nav-ux/plan.md` replaced an earlier flat full-path list with
the "C2 breadcrumb-zoom chapter" sidebar *because the full list was clutter*,
and locked "always show exactly one folder's contents".
`plans/.../phase-03-section-ui.md` reaffirms it: "Mirrors the md sidebar's
'always show exactly one folder' model", "**no new JavaScript**",
"colours come from existing CSS variables — **no new palette**".
A nested always-visible tree **reverses** that decision. It may well be the
right reversal — a code repo is not a book — but it is a reversal, and it is
the user's to make, not this brief's.

### Docs

`Local` — `docs/specs/web-interface.md` (this repo's own spec, the read-only
compatibility surface) describes the breadcrumb-zoom chapter sidebar and
contains no mention of a Files or Changes tab. Any tree work must sync it.
No external documentation was consulted: every question here was answerable
from the two repositories, and the relevant crate versions are pinned in
`Cargo.toml`. No web search was run — no claim below depends on one.

### Inference

- The work splits cleanly into four layers of increasing cost and risk:
  1. **Tree sidebar** — replace one-folder-at-a-time with a nested, expandable,
     persistent tree over a live `ignore` walk. Touches `views.rs`, `app.js`,
     `app.css`, plus one new core function beside `list_dir`. Reversible;
     no server contract change.
  2. **Unify Docs and Code** — the tree makes the Docs|Code switch redundant:
     one tree, `.md` opens rendered, everything else opens highlighted. This is
     the actual *"nâng cấp docs lên tính năng files"* — and it is a **routing
     contract change** (`/p/:id/*path` vs `/p/:id/_code/*path`), so old links
     need a redirect.
  3. **Per-filetype icons** — cheap and self-contained; inline SVG or a small
     extension→glyph map in `views.rs`, styled from existing tokens.
  4. **Changes tab** — the whole cost. Needs a git dependency this project has
     never had, a diff renderer, and a decision about what happens in a project
     that is not a git repo at all.
- Layer 1 alone very likely delivers most of what the screenshot promises. It
  does not touch the server contract, does not add a dependency, and is
  reversible.
- A whole-repo `ignore` walk on every page load is the obvious naive
  implementation and the obvious performance trap on a large repo. The existing
  `#filelist` JSON idiom (walk once, hand the client a blob, render client-side)
  is the precedent that already solved this for markdown.

## Risks, Unknowns, Follow-Ups

**Challenge — five adversarial questions**

| # | Question | Source answer (mdview) | Local answer | Risk if wrong |
|---|---|---|---|---|
| 1 | Should the tree show `.git/` and `target/`, as the screenshot does? | Never. D3 locked *gitignore + denylist*; `.git` is hard-denied independent of config (`code_source.rs:149`, test `git_directory_denied_even_with_empty_exclude_patterns:287`). | Same code, same guarantee. | 🔴 **Red flag.** The daemon is **unauthenticated** and can bind wildcard on LAN — this fork dropped mdview's `auth.rs`/`cf_access.rs` entirely. Serving `.git/` leaks the full history and any credential in `.git-credentials`. Showing it in the tree but 404-ing the click is worse than not showing it: it advertises what exists. **Recommend: keep the denylist; the tree shows exactly what is servable.** |
| 2 | Can the tree be built from the existing index? | No — index is md-only by locked decision D1. | `indexer.rs:14`, unchanged. | 🟢 Green. Answer is a live `ignore` walk. Cost is bounded by the same gitignore filter the code viewer already uses. |
| 3 | Does a persistent tree fit "no new JavaScript"? | mdview's phase-03 forbade new JS explicitly. | This fork already broke that rule — `app.js` is 3 735 lines vs mdview's 776, with a client-rendered chapter sidebar, a collapsible rail, and scrollspy. | 🟢 Green. The constraint is already spent; a client-rendered tree is idiomatic *here*. Keep the no-JS server fallback the current sidebar has. |
| 4 | Does merging Docs and Code break existing links? | Not applicable — mdview keeps them separate. | `/p/:id/_code/*path` is a published route; short links (`short_link.rs`) and MCP `waggledance_view_file` hand out URLs that agents and humans have saved. | 🟡 **Amber.** Layer 2 needs a redirect from `_code/*` and a check of every URL-emitting surface (MCP, `short_link.rs`, the CLI's `open`). Cheap if planned, a silent breakage if not. |
| 5 | What does "Changes" show in a project with no git repo? | Undefined — mdview deferred diff entirely. | `build_gitignore` is explicitly written to work with no `.git` present (`code_source.rs:214-216`), so registered projects are *not* assumed to be repos. | 🟡 **Amber.** Unanswered product question. A Changes tab that is empty or broken for non-repo projects is a worse surface than no tab. Needs a locked answer before it is built. |

**Open questions for shaping**

1. **Scope**: layer 1 only (tree), 1+2 (tree replaces the Docs/Code split), 1+2+3,
   or all four? These are four different features with four different sizes.
2. **The reversal**: is the breadcrumb-zoom chapter sidebar being *replaced* on
   the Code side only, or on both, or kept as a toggle? It was locked
   deliberately for the Docs reading experience; a book's chapter list and a
   repo's file tree are not obviously the same want.
3. **Changes tab**: git dependency (`git2` = a C library and a build cost, vs
   shelling out to `git` = a runtime dependency and a subprocess per request),
   and the non-repo answer from challenge #5.
4. **Hidden entries**: confirm the tree respects the existing denylist +
   gitignore, i.e. that the screenshot's `.git`/`target` rows are not part of
   the ask.

**Evidence gaps**

- The performance ceiling of a whole-repo `ignore` walk was **not measured** on a
  real project — the largest registered project's file count is unknown.
  `Inference` only; a proof obligation for the shape gate.
- The cross-cutting sweep covers what was searched. The Tauri desktop shell
  (`crates/waggledance-desktop`, excluded from the workspace build) was not
  examined for sidebar assumptions.

## Source Pack

**Local files read** — `Cargo.toml`; `crates/waggledance/src/server.rs`
(routes 592-615, handlers 7047-7056); `crates/waggledance/src/views.rs`
(9349, 9442-9478, 9553, 9634, 9717-9800, 9848-9895);
`crates/waggledance-core/src/code_source.rs` (full);
`crates/waggledance-core/src/indexer.rs:14,104,165-170`;
`crates/waggledance-core/src/render.rs`; `crates/waggledance/assets/app.js`
(160-313, 380-404, 531-760, 1267-1286); `crates/waggledance/assets/app.css`
(468-479, 571-662, 738-765); `docs/specs/web-interface.md`;
`docs/knowledge/work/upstream-code-viewer/{index,delivery}.md`.

**Upstream inspected** — `/home/thanhsmind/Projects/refs/mdview` @
`6875119e2fb84f3dab6c3c10f7ef81206dc1f204`: `Cargo.toml`;
`crates/mdview/src/{server,views}.rs`;
`crates/mdview-core/src/{code_source,render}.rs`;
`crates/mdview/assets/{app.js,app.css}`;
`plans/260812-1458-code-viewer-section/{plan,phase-03-section-ui}.md`;
`docs/history/{file-nav-ux,ui-polish-settings-sidebar,code-viewer-section}/`;
`docs/specs/web-interface.md`.

**Docs pages checked** — none; no claim here depends on external documentation.
