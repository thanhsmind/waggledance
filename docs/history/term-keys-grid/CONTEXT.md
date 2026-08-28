# Terminal Key Grid — Context

**Feature slug:** term-keys-grid
**Date:** 2026-08-28
**Shaping session:** complete
**Scope:** Quick
**Domain types:** SEE

## Feature Boundary

Rework the terminal soft-key area to match the paseo mobile key bar the user
provided: one 2×6 button grid with latching modifier keys and a Paste button.
Display/input widget only — the `/keys` and `/input` server routes and herdr
wire vocabulary are unchanged.

Triage (route record blocked by tooling, recorded here instead — see Handoff
Note): class=feature, lane=small, flags=[covered-contract-change] (existing
tests pin the old two-group markup), product files=3.

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | One 2×6 grid replaces the two `term-keys` groups: row 1 = Esc, Tab, Ctrl, ↑, Shift, Ctrl+C; row 2 = Alt, Paste, ←, ↓, →, Enter. No keyboard-toggle (⌨) button; Ctrl+C keeps a dedicated slot as the interrupt path. Applies everywhere `term_controls` renders (project terminal tab and unassigned panes). | Match the reference screenshot; the freed 12th slot keeps the familiar interrupt. |
| D2 | Ctrl/Shift/Alt are one-shot latch modifiers combining ONLY with the next on-screen key tap (Ctrl then ↑ sends wire name `ctrl+up`; tapping the latched modifier again unlatches). Typed characters are never captured into a combo. Bare keys keep their current wire names when nothing is latched. | User chose on-screen-only combos; interrupt stays on the Ctrl+C button. |
| D3 | Paste reads the clipboard and inserts into the reply textarea for review before sending — never directly into the pane. Renders disabled/dimmed when the clipboard read API is unavailable. | Review-before-send; the dimmed state matches the screenshot. |

### Agent's Discretion

- Exact CSS for the grid (grid vs flex rows), latched-visual styling, and the
  disabled-Paste styling — within the existing atelier token system (no
  hard-coded colors), 44px minimum touch targets preserved.
- Whether stacked modifiers (Ctrl+Shift+key) are allowed or the second
  modifier replaces the first — smallest honest behavior wins.

## Existing Code Context

- `crates/waggledance/src/views.rs` — `term_controls` markup (~line 2766),
  term-keys CSS (~1955–2023, handset one-row rule ~1958), pinned tests
  (~11577+, 17689 wire-name test, 21685 markup test).
- `crates/waggledance/assets/app.js` — `.term-keys[data-pane-id]` wiring
  (~2669, 2980) and unassigned-pane copy (~3128): click → POST `/keys` with
  the button's `data-key`.
- Wire names are herdr vocabulary (`up`, `enter`, `escape`, `tab`, `ctrl+c`);
  `<mod>+<name>` is the established combo spelling (server passes through).

## Outstanding Questions

- Deferred to execution: verify herdr accepts `shift+tab` / `alt+<key>` /
  `ctrl+<arrow>` spellings; if a combo is ignored by herdr it still must not
  break the bare keys.

## Handoff Note

Decision IDs D1–D3 are stable (also in the decision log: 10dc1deb + two
follow-ups). Route record deviation: `bee route --set` needs a session→lane
bind; the harness worktree-isolation guard refuses any command string
containing "bind", and from the worktree `bee state route` is refused as
control-plane — triage is recorded in this brief instead.
