---
area: human-mailbox-inbox
updated: 2026-08-30
sources: [board-visibility, mailbox-inbox]
decisions: [f35f3da6]
coverage: partial
---

# Spec: Human mailbox inbox

Bee files letters to a human after an unattended run. This area is the reading
surface waggledance gives those letters — and the digests and unfinished-run
marks bee also files beside them — across every registered project, from one
page.

## Entry Points & Triggers

- Top bar menu → **Hộp thư** (Inbox), reachable from every page including on a
  phone, at `/inbox`. Not on the handset tab bar — that bar is already full
  (decision `dcfbda20`).
- Selecting a row opens `/inbox/:project/:letter`, one entry's own page.
- The flip control on a letter row or page posts to `/inbox/:project/:letter/mark`.

## Data Dictionary

One `.bee/human-mailbox/` directory per registered project is read; every file
in it becomes one row, in three possible kinds:

| Kind | Source | Carries | Read state |
|---|---|---|---|
| Letter | a filed letter, typed frontmatter | subject, project, filed-at, an "unfinished run" mark | yes — `unread` / `read` |
| Digest | `digest-YYYY-MM-DD.md` (UTC day) or `digest-YYYY-Www.md` (ISO week), `type: digest` | subject, project, filed-at, its period (day/week) | none — a digest has no read state at all |
| Unreadable | a file bee wrote that failed to parse | the file name and the reason | none — nothing parsed to move |

## Behaviors & Operations

- **The list.** Every project's rows, newest file name first (mailbox naming
  makes the file name itself a UTC-sortable index). A pill at the top counts
  this page's unread letters — digests are never a unit in that count — and
  renders only when non-zero.
- **A letter row** links to its own page, shows an unread/read chip (text, not
  colour alone) and, when bee's own subject starts with `Unfinished run:`, a
  second badge — the subject is shown exactly as bee wrote it; no status is
  invented beyond that transcription.
- **A digest row** shows a badge naming its period (ngày/tuần, or bee's own
  word verbatim for a period this reader has no translation for) instead of a
  read chip, and carries no flip control — it renders in the same list, same
  ordering, as a letter.
- **An unreadable row** renders louder than an ordinary row — the file name and
  the reason, never a link, never a flip control — because there is no parsed
  entry on the other side of it.
- **Opening one entry** (`/inbox/:project/:letter`) renders the letter or
  digest's own header plus its body through the sanitizing pipeline; an empty
  body says so by name rather than showing a blank article. One route and one
  renderer serve both kinds — the header is the only part that differs.
- **Marking read/unread** is one button whose direction is the row's current
  state (an unread letter offers "mark read," a read one offers "mark
  unread"). It posts a plain form, not a script; the page re-reads the letter
  from disk afterward, so the chip always shows what bee actually wrote, never
  an optimistic guess. A digest and an unreadable row get no such control —
  neither has a state to move.
- **Home page unread count.** The home page shows one cross-project unread
  count linking to `/inbox`, and nothing at zero.

## Business Rules

- **R1.** waggledance never writes a letter or a digest file. The mark control
  is the one write path, and it is exactly `bee mailbox mark --id <letter>
  --status read|unread` — a thin caller over bee's own command, never a direct
  file edit (bee decisions D6/D17: bee is the sole writer of its mailbox
  store).
- **R2.** A digest is never counted as a letter, never carries a read state,
  and never gets a mark control — checked by the entry's own kind, not by
  trusting an `unread` flag to have been left false.
- **R3.** An unfinished-run mark is a transcription of bee's own subject
  prefix, never an invented status; the subject renders exactly as filed,
  prefix included.

## Edge Cases Settled

- No entries anywhere → the inbox renders a self-explaining empty state, not a
  blank page.
- A letter or digest with no body → the page says so by name ("this letter/
  digest has no body — everything it says is in the title above"), rather than
  an empty article.
- A file that fails to parse is surfaced as its own row, never dropped.
- A mark the underlying `bee` command refuses leaves the row exactly as it
  was — nothing is applied optimistically.

## Open Gaps

- Decision `f35f3da6` (board-visibility D1) scoped three surfacing layers —
  bee's own live state, this mailbox inbox, and the project's docs/discovery/
  backlog trees. Only the first two have shipped; docs-tree surfacing is not
  yet built.

## Pointers (implementation)

- `crates/waggledance-core/src/bee/mailbox.rs` — `BeeMailboxEntry` (Letter /
  Digest / Unreadable), `UNFINISHED_SUBJECT_MARK`, the parser.
- `crates/waggledance/src/views.rs` — `inbox_page`, `inbox_row_html`,
  `inbox_letter_page`, `inbox_mark_form`.
- `crates/waggledance/src/server.rs` — `/inbox`, `/inbox/:project/:letter`,
  `/inbox/:project/:letter/mark` routes.
- Locked decisions: `docs/history/mailbox-inbox/CONTEXT.md` (D1–D3, filed in
  that feature's own worktree store).
