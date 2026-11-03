# mailbox-inbox — locked context

Spec: `docs/history/mailbox-inbox/spec-drop-sup-20260830-9c4e.md`
(spec drop sup-20260830-9c4e, PBI in-flight; originally relayed into
`docs/discovery/human-mailbox-inbox/`, relocated here when the feature started). Upstream contract:
beehive `docs/knowledge/areas/human-mailbox/overview.md` — bee is the only
writer of its store (human-mailbox D6/D17); the consuming inbox reads files
and calls exactly one command, `bee mailbox mark --id <letter> --status
read|unread`.

## Locked decisions (logged in this worktree's bee store, 2026-08-30)

- **D1 — digest is its own entry kind.** Detected by frontmatter
  `type: digest` (the contract, per beehive `mailbox_digest.rs::render_digest`;
  the `digest-` filename is convention only). Rendered in the SAME `/inbox`
  section as letters; never counted as a letter or as unread; carries no read
  state and no mark control. Digest frontmatter: `subject`, `type`, `period`
  (`day`|`week`), `period_id`, `filed_at`, `letters[]`, `unreadable[]`.
- **D2 — unfinished-run status is transcription, never invention.** A letter
  whose subject starts with `Unfinished run:` (beehive
  `UNFINISHED_SUBJECT_MARK`, D12) gets a display badge; the subject itself is
  shown as-is; no new field is read, no status is invented (beehive D3 froze
  the letter frontmatter field list — the mark rides the subject by design).
- **D3 — one route, one renderer.** A digest opens through the same
  `/inbox/:project/:file` route and sanitized markdown pipeline as a letter,
  read-only, with no flip control on its page.

## Cells

- `mi-1` — core reader: `BeeMailboxEntry::Digest` + `BeeLetter::is_unfinished`
  (crates/waggledance-core/src/bee/mailbox.rs, bee.rs).
- `mi-2` — inbox UI: digest rows, badges, digest page
  (crates/waggledance/src/views.rs, server.rs). Depends on mi-1.
