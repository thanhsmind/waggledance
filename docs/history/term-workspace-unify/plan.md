# term-workspace-unify — plan

Lane: small · 4 files · behavior change on two pages

## Ask

The project terminal page (`/p/:id/_terminal`) must present like the homepage
Terminals tab (`/?tab=terminals`), so the two read as one UI. The owner chose the
stronger form: extract ONE shared layout both pages call, with class names that
do not say "home".

## Today

Both pages already share `pane_bar` / `pane_strip` / `pane_tab`, the pane card
(`pane_cards` / `screen_frame` — neither renders an identity header any more),
and the `.term-split` class on `<main>`. What differs is the frame around them
and the Files|Diff control.

| | homepage Terminals tab | project terminal page |
|---|---|---|
| frame | `.home-term` two columns | single column in `<main>` |
| Files\|Diff | `.home-term__side` sidebar on the right, over a lazy nav frame | `.term-embed__tabs`, two buttons above the terminal |
| opening a file | nav page's `<base target=wd-term-panel>` lands it in `.home-term__panel` above the terminal — no script between the click and the frame | `.term-embed__panel` iframe, `src` set by script |
| split | `main.fg-page.term-split` divides `.home-term__panel` / `.home-term__screen` | the same class divides `.term-embed` / `.term-panes` |
| storage key | `waggledance-home-panel:<id>` | `waggledance-term-panel:<id>` |

`terminal_page` is `terminal_embed_panel`'s ONLY caller, and `.term-embed` has no
other renderer — so the project page's variant can be deleted rather than kept
alongside.

## Shape

1. **One layout function.** `term_workspace(bar, screen_html, project_id)` renders
   the whole frame — the two columns, the panel above the screen, and the
   sidebar — and both pages call it. `terminals_tab` stops composing the frame
   inline; `terminal_page` stops rendering `.term-embed` entirely.
2. **Neutral names.** `.home-term*` → `.term-work*` throughout (`__col`,
   `__screen`, `__panel`, `__panel-head`, `__panel-name`, `__close`, `__frame`,
   `__side`, `__side--empty`, `__side--open`, `__tabs`, `__tab`, `__tab--on`,
   `__hint`, `__navs`, `__nav`), and `home_term_panel` / `home_term_sidebar` →
   `term_work_panel` / `term_work_sidebar`. The name is the whole point of the
   owner's choice: a page that is not the homepage must not render `home-term`.
3. **Delete the project variant.** `terminal_embed_panel`, the `.term-embed*` CSS,
   and the `.term-embed[data-project-id]` block in `app.js` all go. The rule
   `main.fg-page.term-split { height: calc(100dvh - 53px); … }` lives in that CSS
   block but belongs to BOTH pages — it moves into the unified block rather than
   dying with its neighbours.
4. **One storage key.** `waggledance-term-panel:<id>` for both pages (the project
   page's existing name; the homepage's `waggledance-home-panel:` retires). One
   key means the Files/Diff choice follows the project across both pages, which is
   the unification the owner asked for. Cost: every viewer's remembered tab resets
   once. It is a per-viewer convenience, not state anyone can lose work from.
5. **Fix the responsive scoping.** The `@media (max-width: 700px)` rules that undo
   the fixed-height split are written `.home-shell > main.fg-page.term-split …`,
   and `.home-shell` exists only on the homepage. Left as-is, the project page on
   a handset would keep a `100dvh`/`overflow: hidden` frame while the columns
   stack inside it — a terminal you cannot scroll to. The `.home-shell >` prefix
   is dropped so the breakpoint covers both pages.

## Unchanged, deliberately

- The project page keeps its topbar, its `name · terminal` crumb, and the
  Overview/Terminal/Transcript project nav. That is page navigation, not terminal
  presentation, and the homepage has its own equivalent.
- `<main data-project-id>` stays on the project page: `pane_cards` passes
  `base: None` and the screen poller reads the page-root id. The homepage keeps
  passing its per-pane `data-term-base`. `terminal_page_controls_carry_no_data_term_base`
  and `terminals_tab_controls_carry_the_selected_panes_own_base` both keep holding.
- `transcript_page` and `unassigned_terminal_page` keep `.term-panes`; neither
  uses the split, so the rule that leaves with `.term-embed` costs them nothing.
- `PANEL_FRAME_NAME` and `panel_base_tag()` are untouched — the `<base target>`
  mechanism is exactly what makes a file click land in the panel with no script
  in between, and it now serves both pages.

## Cells

- **twu-1** — `views.rs`: extract `term_workspace`, rename the two helpers, rewire
  both pages, delete `terminal_embed_panel`.
- **twu-2** — `app.css`: rename the `.home-term*` rules to `.term-work*`, fold the
  shared `term-split` height rule in, delete the `.term-embed*` block, drop the
  `.home-shell >` prefix from the breakpoint.
- **twu-3** — `app.js`: retarget the panel/sidebar block to `.term-work`, switch
  the storage key, delete the `.term-embed` block.
- **twu-4** — tests: rewrite the five `.term-embed` tests as `.term-work` tests
  covering BOTH pages, and re-point every `.home-term` assertion.

twu-1 and twu-2/3 touch different files and can run together; twu-4 depends on
all three.

## Proof

`cargo test -p waggledance-core -p waggledance --no-fail-fast`, then a live check
of both `/p/:id/_terminal` and `/?tab=terminals` against a binary built into a
private `CARGO_TARGET_DIR` and confirmed by content — the shared target dir is
overwritten by sibling sessions (see
`docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md`,
disguise 4).

## Risk

Medium, and concentrated on the homepage, which currently works and was just
accepted by the owner. Every `.home-term` string is a rename with a mechanical
counterpart in three files; the danger is a missed one, which shows as an unstyled
or dead control rather than a crash. The tests in twu-4 are what make a missed
rename fail loudly — `rg -c 'home-term'` returning 0 across the crate is part of
that cell's proof.
