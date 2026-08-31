---
type: bee.pattern
title: Hand-added prose inside a managed marker block is deleted on the next doctor sync
description: "Pitfall: waggledance doctor --fix overwrites the <!-- waggledance:START/END --> block in AGENTS.md/CLAUDE.md with its canonical template on every sync, so anything hand-added INSIDE that block — even prose that has lived there for a while — is silently deleted the next time doctor runs."
timestamp: 2026-08-31
bee:
  id: managed-block-hand-edits-lost-on-doctor-fix
  lifecycle: active
  areas: [doctor]
  sources: [AGENTS.md, CLAUDE.md, crates/waggledance/src/doctor.rs]
  polarity: pitfall
---

# Hand-added prose inside a managed marker block is deleted on the next doctor sync

## The trap

`waggledance doctor --fix` treats everything between
`<!-- waggledance:START -->` and `<!-- waggledance:END -->` in `AGENTS.md` and
`CLAUDE.md` as fully owned, canonical content — it overwrites the whole block
with its shipped template on every sync. Prose added by hand inside that
block, however long it has lived there, is deleted with no warning the next
time doctor runs.

## The tell

Project-specific instructions that used to be in `AGENTS.md`/`CLAUDE.md`
disappear after running `waggledance doctor --fix`, with no error and no diff
review — the file still parses and looks normal.

## What to do instead

Project-owned prose always goes **outside** the managed markers — either
above `<!-- waggledance:START -->` or below `<!-- waggledance:END -->`, never
between them. Before adding anything inside the block, check whether it
belongs to waggledance's own template (and should be proposed upstream) or is
project-specific (and belongs outside it).

## Recurrence

- 2026-08-30 — the "Building (Waggledance)" section had been living inside the
  managed block and was lost on a `--fix` run; the repair moved the START
  marker below it.
