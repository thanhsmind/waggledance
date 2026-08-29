# Terminal Reply Composer — Context

**Feature slug:** term-reply-composer
**Date:** 2026-08-29
**Shaping session:** complete
**Scope:** Quick
**Domain types:** SEE

## Feature Boundary

Unify the terminal reply widget into one paseo-style composer card: the
textarea and every control live inside a single rounded bordered box. Pure
markup/CSS restructure — the posting wiring in `assets/app.js` keeps working
against the same selectors, and no server route changes.

Triage: class=feature, lane=small, flags=[covered-contract-change] (existing
tests pin the current reply markup), product files=2 (views.rs, server.rs
tests). Same route-record deviation as term-keys-grid does not apply — the
session was bound from main before entering the worktree.

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | The reply area is ONE rounded composer card modeled on the paseo composer: borderless textarea on top; a controls row INSIDE the same bordered box at the bottom — + attach on the left, Approve/Stage/Send right-aligned. The separate `.term-reply__actions` row below the box is gone as a separate visual block. Send keeps its filled primary style; attach chips and the attach error stay inside the card; the focus ring sits on the card (focus-within). | User's screenshot pair: current split layout vs the unified paseo composer, "nút và textarea cùng trong một khung cho thống nhất". |

### Agent's Discretion

- Exact spacing/radius within existing atelier tokens (the card reads
  rounded-large like the reference; no hard-coded colors), 44px touch targets
  preserved on all buttons.
- Where the no-attach variant (pane without attach support) puts its bottom
  row — same card anatomy, just without the + button.
- Whether `.term-reply__actions` remains as the class name of the in-card
  controls row (preferred: keep class names so `assets/app.js` selectors and
  most tests survive) or a new class is introduced.

## Existing Code Context

- `crates/waggledance/src/views.rs` — reply markup `term_reply` (~2726–2790):
  `.term-attach` wrapper (input, chips, `.term-reply__field` with
  `.term-attach__btn` + textarea, error), then `.term-reply__actions`
  (Approve/Stage/Send) as a separate row. CSS ~1955–2004 (`.term-reply__field`
  border box, `__actions` flex row, `__send` primary fill, handset stretch
  rules ~1955–1957).
- `crates/waggledance/assets/app.js` — posts via `.term-reply[data-pane-id]`,
  `.term-reply__text`, `__send`/`__stage`/`__approve`, `.term-attach__btn` —
  selectors must keep working unchanged.
- Pinned tests: views.rs/server.rs assertions on reply markup and the
  Approve-disabled state (~2726) — sweep for every assertion naming
  `term-reply` classes, as the term-keys-grid cell learned to do.

## Outstanding Questions

- None.

## Handoff Note

D1 is in the main decision store (logged from main before the worktree
opened). Cell executes in this worktree; proof is the related waggledance
tests plus a fast-profile build.
