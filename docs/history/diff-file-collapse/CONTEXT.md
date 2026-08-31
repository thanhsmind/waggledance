# diff-file-collapse — locked context

**Ask (verbatim):** "trong diff, tôi muốn thêm tính năng cho phép collapse và
expanse các file diff"

The Changes screen (`/p/<id>/_changes`) already folds a file's diff — but only
as a side effect of ticking **Reviewed**, and the header click that brings a
folded section back is deliberately one-way. There is no control that simply
closes a file you are done looking at.

## Decisions

Store id `61b0cf9c-1c26-4fde-b9fc-0cac4cdcfab2`.

**D1 — A two-way fold button per file.** Each `.changeset__head` gets its own
button (a chevron) that folds and unfolds that one section. It ships `hidden`
in the markup and `app.js` unhides it, exactly the contract
`.changeset__review` already follows: a control that remembers nothing must
not appear on a page with scripting off.

**D2 — The header click stays expand-only.** `app.js` already states the
reason and it still holds: a stray click on a header must never hide content
the reader was reading. The explicit button is the deliberate gesture; the
header is the recovery gesture.

**D3 — One "Collapse all / Expand all" button in `.changes__head`.** Its label
and `aria` flip by state — *Collapse all* while any section is open,
*Expand all* once every one is folded. One control rather than two, because
the head already carries the count, the base picker, the reviewed counter and
the hidden-files note.

**D4 — Folding is independent of the reviewed mark.** Ticking Reviewed still
folds, as today. Folding by hand never ticks, unticks, or reads a mark.

**D5 — Fold state is not persisted.** The reviewed marks in `localStorage`
stay the single durable store for this screen; on reload the fold derives from
them alone. A second store would race the reviewed-derived fold for the same
class attribute, for view state that costs one click to redo.

## Rejected

- Making the header click toggle both ways — reverses D2's rationale, already
  locked in `app.js`.
- Persisting the fold — see D5.
- Separate *Collapse all* and *Expand all* buttons — more chrome in an already
  busy header for no extra reach.
