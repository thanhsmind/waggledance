---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Home Terminal Panel

Mode: `standard` — 1 risk flag: covered-contract-change
Why this is the least workflow that protects the work: two files of view
composition over an already-shipped, already-judged embed mechanism; the one
new surface (`nav=1`) is a render-mode subset of existing pages.

Hat wave: SKIPPED, named reason — the threshold ("big, vague, or high-risk")
is not met: the ask is fully sketched by the user's two screenshots and the
mechanism (embed iframes, D9) shipped and passed judge this same day.

## Requirements (from CONTEXT.md)

- D1 — right sidebar Files | Diff on the homepage terminals tab, scoped to the
  selected pane's project; Diff tab also fills the panel above the terminal;
  no-project pane → explained empty state.
- D2 — `nav=1` mode on `_changes`/`_code` embeds + `<base target="wd-term-panel">`;
  three iframes total; server emits every URL.
- D3 — Diff nav rows deep-link `#f<i>` into the embedded Changes page; Files
  nav folders self-navigate (nav mode kept), file rows fill the panel.

## Discovery

`views.rs:2778 terminals_tab` renders the tab inside the home layout;
`TerminalsMenuPane.project_id` is the scoping key and is `None` for
unassigned panes. `PageChrome`/embed threading, `changes_nav`, `code_tree`,
and the project-terminal split panel (`term-embed*`, cds-8) all shipped in
changes-diff-screen and are the assets this composes.

## Approach

Phase 1 (`nav=1`): in the `_changes` and `_code` handlers, read `nav` beside
`embed`; when both are set, render the nav-only body — `_changes`: the
`changes_nav` list plus the base picker context it needs, each file row an
absolute `/p/<id>/_changes?embed=1#f<i>` link; `_code`: the tree sidebar
alone, folder links keeping `embed=1&nav=1` with `target="_self"`, file links
absolute `/p/<id>/_code/<path>?embed=1`. Both inject
`<base target="wd-term-panel">`. Nav mode without embed renders normal pages.

Phase 2 (homepage): `terminals_tab` gains a right sidebar (Files | Diff tab
buttons + two lazy nav iframes, URLs emitted server-side from the selected
pane's project) and a `wd-term-panel`-named panel iframe above the terminal
that splits the area half/half when filled (reuse the `.term-split` idiom);
selecting the Diff tab also loads `_changes?embed=1` into the panel; a close
control restores the full-height terminal; open-tab state in sessionStorage
(`waggledance-home-panel:<project-id>`), try/catch idiom; no-project pane →
empty-state sidebar, no iframes.

SMALLER PATH check: PASS — no new endpoint, no new render engine, two cells;
the only cheaper shape (skip `nav=1`, put whole embed pages in the sidebar)
double-renders the diff at sidebar width and cannot deep-link the panel.

Rejected: postMessage frame coordination (more JS than `<base target>`);
rendering the lists directly into the homepage (a second render path for
content the embeds already serve).

Risk map:

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| nav=1 subset render | LOW | reuses existing fragments | markup tests |
| base-target link routing | MEDIUM | folder links must NOT hit the panel | markup tests assert target="_self" on folder links, base tag present |
| homepage layout squeeze | MEDIUM | third column on an already busy tab | markup + manual check; narrow widths keep existing drawer behavior |
| pane with no project | LOW | project_id: None handled at render | markup test |

## Test matrix

- Happy: `_changes?embed=1&nav=1` renders only the nav list with base target
  + `#f<i>` absolute links; `_code/?embed=1&nav=1` renders only the tree with
  self-targeted folder links; terminals tab with a projected pane renders the
  two tab buttons, lazy iframes (no src until opened), and the named panel.
- Edge: pane with `project_id: None` → empty-state sidebar, zero iframes;
  `nav=1` without `embed=1` → normal page; empty diff → nav shows the
  no-changes state.
- Error: sessionStorage throwing → defaults closed (harness idiom from cds-3).

## Out of scope

<!-- bee:not-a-deferral: out-of-scope register; work arrives only through a new ask -->
- Editing files, staging, or any write action from the panel; per-pane (rather
  than per-project) sidebar scoping; the project terminal page (already has
  its panel from cds-8).
<!-- /bee:not-a-deferral -->
