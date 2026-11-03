# home-terminal-new-shell — plan

Lane: tiny · 1 file · covered contract change

## Ask

The homepage Terminals tab's pane bar must match the project terminal page's
picker exactly. It already shares `pane_bar` / `pane_strip` / `pane_tab` and
their CSS — the one difference left is the creation row: the project page
offers **New shell** beside its agent presets, the homepage tab offers presets
only.

## Why the difference exists, and why it is now stale

`terminals_tab` passes `plain_shell: false` to `terminal_create_controls`
(`crates/waggledance/src/views.rs:2715`). The recorded reason
(`home-terminal-header`, in that function's doc comment at
`views.rs:3195-3205`) is that the homepage tab "is not scoped to one project",
so a New shell button there would guess a project.

That premise no longer holds. `terminals-tab-project-scope-1` scoped the tab:
`create` is already built from `effective.project_id`, and the switcher itself
already lists only the panes sharing that project. A pane outside every
registered project (`project_id: None`) still renders no creation controls at
all. So "New shell" here starts a shell in exactly the project the reader is
watching — the same meaning it carries on the project page.

## Shape

1. `terminals_tab` (`views.rs:2715`) passes the plain-shell button through, and
   its comment records the reversal rather than the retired reasoning.
2. `plain_shell` then has one value at both call sites, so the parameter and its
   `!plain_shell && preset_buttons.is_empty()` early return go away — a flag with
   one live value is dead code. `terminal_create_controls(project_id, presets)`
   is the surviving signature.
3. `terminal_create_controls_offer_the_plain_shell_button_only_when_asked`
   (`views.rs:14118`) pins the removed parameter; it is replaced by a test that
   pins the surviving contract — the button always renders, presets or not.
4. `terminals_tab_creation_controls_follow_the_selected_panes_project`
   (`views.rs:14772`) gains an assertion that the homepage tab's controls now
   carry `New shell`, inside the pane bar, and still render nothing for a pane
   with no project.

No CSS, no `app.js`, no route change: `.term-create__pane` is already wired in
`app.js:4164`, scoped to `.term-create[data-project-id]`, which this tab already
renders. `POST /p/:id/_terminal/create/pane` is the existing route.

## Proof

`cargo test -p waggledance-core -p waggledance --no-fail-fast` over the views
tests, plus a live check of `/?tab=terminals` against a freshly built binary
(resolved from `--message-format=json`, confirmed by content).

## Risk

Low and reversible. The one behavior change beyond parity: a homepage tab with
a selected project pane and **no** configured presets used to render no
`.term-create` box at all and now renders one holding New shell. That is the
parity being asked for.
