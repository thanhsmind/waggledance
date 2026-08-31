---
type: bee.pattern
title: "A reference list written from memory is fiction — extract it from the vendor's own bundle"
description: "Pitfall: a list of someone else's commands, flags, or models recalled from memory looks authoritative and is wrong in ways nothing in the build can catch, and the extraction that fixes it introduces its own trap when the field read is unbounded."
timestamp: 2026-08-31
bee:
  id: a-reference-list-written-from-memory-is-fiction
  lifecycle: active
  areas: [agent-terminal]
  sources: [.bee/cells/sbc-1.json]
  polarity: pitfall
  signature: vendor list authored by hand
---

# A reference list written from memory is fiction — extract it from the vendor's own bundle

## The trap

`slash-builtin-commands` needed the list of an agent's own built-in commands.
Written from memory, the list carried `/cost`, `/review`, `/doctor`, `/todos`,
`/rewind` and `/vim` — **none** of them registered by claude 2.1.251.

Nothing in the build can catch this. The rows render, the tests pass, the menu
looks complete, and the only symptom is a user picking a command that does not
exist. A list of someone else's surface is data about a foreign artifact; the
only honest source is that artifact.

## The practice

Extract, then commit the extractor. The list ships from the vendor's own bundle,
and the tool that pulled it lives in the repo — `crates/waggledance/tools/extract-agent-builtins.py`
— so refreshing after a vendor upgrade is a rerun, never a retype. A list you
cannot regenerate is a list that starts rotting the day it lands.

## The trap inside the extraction

Extracting is not automatically correct. The first version read each field with
an **unbounded window** and silently mis-paired names with descriptions: it
described `/login` as "Sign out from your Anthropic account", which is
`/logout`'s text. Every row was present, every row looked plausible, and the
pairing was wrong.

Bound every field read at the next object marker. An unbounded read across a
record boundary produces well-formed output that is simply about a different
record — the failure mode with no error message.

## The shape of the rule

Applies to any list this repo restates from a foreign surface: another agent's
commands, a provider's model ids, a tool's flags. Two questions decide it —
*where did these strings come from?* and *what command regenerates them?* If the
answer to either is "me", the list is not evidence yet.
