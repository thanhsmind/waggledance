# Home Terminal Panel — Context

**Feature slug:** home-terminal-panel
**Date:** 2026-08-30
**Shaping session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

The homepage terminals tab (`/?tab=terminals&pane=<id>`) gains a right
sidebar with Files | Diff tabs scoped to the selected pane's project, and a
panel frame above the terminal that sidebar links load into. It reuses the
shipped embed pages (changes-diff-screen D9) — no new endpoint, no new
render engine.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Right sidebar on the homepage terminals tab with two tabs, Files and Diff, scoped to the selected pane's `project_id`. Files = the existing breadcrumb-zoom file tree (image #6); Diff = the changed-files list with badges and ± counts (image #7). Selecting Diff ALSO loads the embedded Changes page into the panel above the terminal at once; clicking a file in either sidebar loads the matching page into that panel, deep-linked to the file/section. A pane with no project shows an explained empty state in the sidebar. (user, 2026-08-30, two screenshots) | — |
| D2 | Mechanism: a `nav=1` mode on the two embed pages — `_changes?embed=1&nav=1` renders ONLY the changed-files nav list, `_code/?embed=1&nav=1` renders ONLY the tree sidebar — and both inject `<base target="wd-term-panel">` so every link loads into the named panel iframe. The right sidebar is two lazy nav iframes; the panel above the terminal is a third iframe named `wd-term-panel`. No new endpoint, no JS framework; the server emits every URL — a project id is never assembled in JS. | `<base target>` is the standard HTML frame-navigation mechanism; no postMessage. |
| D3 | Nav-mode deep links: the Diff nav's file rows link to `/p/<id>/_changes?embed=1#f<i>` (the full embedded diff, scrolled to that file); the Files nav's file rows link to `/p/<id>/_code/<path>?embed=1`; folder navigation inside the Files nav keeps `embed=1&nav=1` so the sidebar navigates itself, only file links carry the base target through to the panel. | Folder clicks re-render the nav; file clicks fill the panel. |
| D4 | Pages inside the homepage panel frame carry `panel=1`: the in-page sidebar (changed-files/tree) is dropped — the homepage right sidebar already navigates — while the page header (count, base picker, reviewed) stays. Every in-panel link (nav rows, data-panel-src, picker, code links, "Open in Code view") threads the flag. The project terminal page's own panel (cds-8) keeps plain `embed=1` — there its in-page sidebar is the only nav. (uat, image #8) | — |
| D5 | No auto-registration of bee worktrees in the viewer: a pane in an unregistered worktree lands in Unassigned with the sidebar's explained empty state; the homepage Suggestion row's one-press register is the door to worktree diffs. (user, 2026-08-30) | — |

### Agent's Discretion

- Exact tab styling (match `.fg-*`/existing tokens), sidebar width, panel
  split ratio (top half like the project terminal page), lazy-load and
  sessionStorage details (follow changes-diff-screen D8's idiom), how the
  base-target exemption for folder links is implemented (e.g. per-link
  `target="_self"` overriding the injected base — cheaper than two modes).
- Whether the sidebar defaults open or closed on the terminals tab; keep the
  tab usable and un-cramped on narrow widths (the existing drawer idiom).

## Existing Code Context

### Reusable Assets

- changes-diff-screen shipped: `PageChrome` embed threading (`views.rs`),
  `section_topbar`, `changes_nav`, `code_tree`, `.layout--embed` CSS,
  `terminal_embed_panel` + `.term-embed*`/`.term-split` (the project
  terminal page's own panel — the pattern D1 extends to the homepage).
- `views.rs:2778 terminals_tab` — the tab's renderer; `TerminalsMenuPane`
  carries `project_id: Option<String>` per pane (the scoping key).
- sessionStorage try/catch idiom (`app.js`), `waggledance-term-panel:<pid>` key
  from cds-8.

### Integration Points

- `views.rs terminals_tab` + the home `layout` wrapper (`views.rs:351-390`) —
  where the right sidebar and panel frame mount.
- `server.rs` `_changes`/`_code` handlers — where `nav` joins `embed` in the
  query structs.

## Canonical References

- `docs/specs/web-interface.md` — "Changes screen (git diff)" section; sync
  the homepage sidebar + nav mode there at capture time.
- `docs/history/changes-diff-screen/CONTEXT.md` D8/D9 — the mechanism this
  feature extends.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads
locked decisions, code context, and canonical references.
