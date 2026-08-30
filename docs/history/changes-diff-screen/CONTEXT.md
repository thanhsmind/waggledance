# Changes Diff Screen — Context

**Feature slug:** changes-diff-screen
**Date:** 2026-08-30
**Shaping session:** complete
**Scope:** Standard
**Domain types:** SEE | READ

## Feature Boundary

A per-project **Changes** screen in the waggledance web viewer showing the git
working-tree diff (vs HEAD) as stacked side-by-side file diffs, with a
changed-files sidebar (M/A/D badges) and a client-side reviewed counter —
matching the user's screenshot (2026-08-30). It ends there: no nested
files-explorer tree, no commit/base picker, no server-side review state.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Build the Changes/diff screen (layer 4 of the xia report) — and ONLY it. The nested files-explorer tree (layers 1–3) stays not-built; the breadcrumb-zoom sidebars stay as they are. Supersedes decision 070dd341. | The screen's own sidebar lists changed files only — it is part of this screen, not a project explorer. |
| D2 | Diff scope v1: working tree vs HEAD — staged + unstaged + untracked (untracked shows as A). No base/commit picker. (a5f24805) | Screenshot header says "working tree". |
| D3 | Non-repo project (or git unavailable): the screen renders a clear "not a git repository" empty state; the Changes entry link stays visible. (58bf3ed2) | An explained empty tab beats a hidden or broken one. |
| D4 | Reviewed state (per-file mark + N/M counter) is client-side only, persisted in localStorage per project, following the existing persistence idiom. No server state. (800c53f2) | Matches `waggledance-rail-collapsed` / `waggledance-folders-open` idiom. |
| D5 | The tree/screen shows exactly what is servable: existing denylist + gitignore rules hold — `.git/`, secrets, and denied paths never appear in the diff surface. | Security: the daemon is unauthenticated on LAN (xia report challenge #1). |

### Agent's Discretion

- How git is invoked (`git2` crate vs shelling out to `git`) — planning decides;
  weigh build cost vs runtime dependency.
- Route shape (e.g. `/p/:id/_changes`), entry-point placement in the topbar,
  exact layout/CSS, diff algorithm/renderer, hunk collapsing, large-diff limits.
- Reuse of `render.rs::highlight_source` for syntax colouring inside diff panes.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Changes screen | The per-project working-tree diff page; NOT a files explorer. |
| Reviewed | A per-file client-side checkmark; counted as N/M in the header. Cleared marks are the viewer's business — the server never knows. |
| Working tree | Everything `git status` reports vs HEAD: staged, unstaged, untracked. |

## Specific Ideas And References

- User's screenshot (2026-08-30): left sidebar = changed files grouped by
  directory with M/A/D letter badges; main pane = per-file sections stacked
  vertically, each a side-by-side two-column diff with line numbers, syntax
  highlighting, red removed-line / green added-line row tinting; header shows
  "working tree — 0/27 reviewed" and per-file `+n −m` counts.
- Research (read first): `docs/history/research/mdview-files-explorer-and-changes-tab.md`.

## Existing Code Context

### Reusable Assets

- `crates/waggledance-core/src/code_source.rs` — path canonicalisation, denylist,
  gitignore awareness, binary sniffing, 2 MiB cap. The security layer D5 relies on.
- `crates/waggledance-core/src/render.rs::highlight_source` — per-line syntect
  output; a diff view can reuse the same per-line HTML.
- `crates/waggledance/src/views.rs:9553` `code_page` — line-numbered code table
  markup (`<tr id="L{n}">`) to pattern the diff table on.

### Established Patterns

- `#filelist` JSON blob + client render (`app.js:160-278`) — walk once server-side,
  hand the client a blob; the precedent for the changed-files sidebar.
- `localStorage["waggledance-rail-collapsed"]` / `sessionStorage["waggledance-folders-open"]`
  — the persistence idiom D4 follows.
- Server-rendered HTML via `format!` in `views.rs`; no framework, no build step.

### Integration Points

- `crates/waggledance/src/server.rs:614` — `_code` route registration; the new
  `_changes` route registers beside it, before the `/p/:id/*path` catch-all.
- Topbar centre slot (Docs|Code switch) — where the Changes entry link lands.

## Canonical References

- `docs/specs/web-interface.md` — the read-only compatibility surface; must be
  synced with the new screen at capture time.
- `docs/history/research/mdview-files-explorer-and-changes-tab.md` — the xia
  report; dependency matrix and risk table for this exact feature.

## Outstanding Questions

### Deferred To Planning

<!-- bee:not-a-deferral: all three were answered by planning on 2026-08-30 — resolutions recorded in plan.md (Approach) and decision ad275999; kept as the historical question list, nothing left to act on -->
- [x] git2 vs shell-out — resolved: shell-out, 3 read-only calls (plan.md Approach).
- [x] Large-diff limits — resolved: 2 MiB/side, 100 sections, 48 MiB stdout (plan.md).
- [x] Rename detection — resolved: R with `old → new` label via `-M` (plan.md).
<!-- /bee:not-a-deferral -->

## Deferred Ideas

<!-- bee:not-a-deferral: out-of-scope register mirrored in plan.md "Out of scope"; these become work only through a new user ask or backlog item, no promise to act rides here -->
- Base/commit picker (diff vs arbitrary ref) — v2; D2 locks working-tree-only.
- Nested files-explorer tree + Docs/Code unification (layers 1–3) — explicitly
  out, per D1.
- Live refresh of the diff via the watcher — watcher is md-only today; separate ask.
<!-- /bee:not-a-deferral -->

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
