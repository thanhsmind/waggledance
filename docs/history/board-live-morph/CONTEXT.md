# board-live-morph — locked context

The board is the one surface a human watches continuously while sessions
move. Today every change signal throws the whole page away
(`location.reload()`), so the board blinks and the reader loses their
place. This feature makes the board update in place, and spends motion on
the one thing a glance is asking about: which work moved.

## Locked decisions

### D1 — only the board surfaces stop reloading
`5dce3301-a7e3-4cb9-b427-e611dcf75fea`

Only the home Kanban tab and `/p/<id>/_bee` stop full-page reloading. They
refetch their own HTML and patch the live DOM in place. Document, search,
code and project-home pages keep `location.reload()` exactly as today.

**Why.** A markdown page reloads on an edit the reader just made, and a
repaint there costs nothing; the board's reload is the one that reads as a
flash. Holding the other surfaces still keeps the change to one markup
family and one state-preservation problem.

### D2 — motion is spent on the card, not on its contents
`7871e652-4ba4-495a-a970-1a3aef7b0cc1`

A card that changes column or order slides to its new box; a card that
appears fades in; a card that leaves fades out. Text, badges and counts
inside a card swap with no animation and no highlight.

**Why.** The reader's question at a glance is which work moved, not which
word changed. Animating in-card content would put motion everywhere and
reintroduce the visual noise the reload already causes. Under
`prefers-reduced-motion: reduce` the same in-place patching runs with the
movement removed.

## What must not move

The signal-filtering contract is untouched by this feature, and its
existing proof stays green without an edit:

- `shouldReload`, `isBoardRelevant`, `isBeeSignal`, `isBeeSignalBurst`,
  `RELOAD_DEBOUNCE_MS`, `scheduleReload` and `modalOpen` keep their exact
  current shapes and bodies in `crates/waggledance/assets/app.js`.
- Their server-side pins — `the_home_board_reload_filter_counts_bee_state_as_board_relevant`
  (`crates/waggledance/src/server.rs`) and
  `the_reload_debounce_is_scoped_to_bee_signals_on_a_board` (same file) —
  are not rewritten.
- The `.term-screen` guard stands: a page showing a live terminal screen
  is never force-updated, in-place or otherwise.

Only `reloadNow()`'s body changes: it routes to an in-place patch on a
board surface and falls back to `location.reload()` everywhere else and on
every failure.

## Where things live

- Both boards render one shared root:
  `<section class="fg-card bee-hub" data-feature-hub="…">` —
  `crates/waggledance/src/views.rs` (`bee_render_hub_section`,
  `bee_cross_project_features_section`). That section is the whole patch
  surface; nothing outside it is touched.
- Columns already carry identity: `id="hub-<key>"` and
  `data-hub-group="<key>"` (`bee_hub_group`), and the archive bar carries
  `data-hub-group="finished"`.
- Board CSS lives in `bee_hub_style()` inside `views.rs`, not in
  `crates/waggledance/assets/app.css`.
- `app.js` and `app.css` are hand-written source pulled in at compile time
  by `include_str!`/`concat!` — there is no generation step to re-run.
- There is no HTML-fragment endpoint for the board, and this feature adds
  none: the patch refetches `location.href` and takes the section out of
  the parsed document. A local daemon rendering one page per signal is
  cheaper than a new route plus its own proof.
