# Spec drop — sup-20260830-9c4e

**Provenance:** from waggledance@2b04509
**Filed:** 2026-08-30
**Correlation id:** sup-20260830-9c4e (same id as the registered PBI)
**Reference in the bee repo:** `docs/knowledge/areas/human-mailbox/overview.md`

## Request text (verbatim)

Surface bee human-mailbox information in the SAME inbox section of the waggledance UI.

What the inbox must show, in one section (not a new section):

- **Letters:** one markdown file per run, in a project's `.bee/human-mailbox/`. Typed
  frontmatter is the machine contract; the body is human prose. The subject is one plain
  sentence.
- **Digests:** files beside the letters, named `digest-YYYY-MM-DD.md` (UTC day) or
  `digest-YYYY-Www.md` (ISO week), frontmatter `type: digest`. Show them in the same
  inbox section as the letters. But never count or fold a digest as a letter.
- **Read state:** it lives inside the letter file. The inbox flips it ONLY with
  `bee mailbox mark --id <letter> --status read|unread`. A repeat flip is a no-op. The
  inbox NEVER writes the letter files itself (bee decisions D6, D17: bee is the only
  writer of its store; a consumer only reads and calls that one command).
- **Unfinished letters:** a letter can be marked as "run went silent"; show it as-is, do
  not invent status.

## Notes

- This item overlaps in domain with the already-proposed `p-e9386ebb` ("As the human, I
  want an inbox showing the letters bee leaves after each unattended run...") — filed
  as-is per the spec-drop contract; whether the two merge, supersede, or stay separate is
  this repo's own triage call, not decided here.
