---
okf_version: 0.1
---
<!--
GENERATED FILE — do not hand-edit.
Rendered by `bee knowledge index` from concept frontmatter inside docs/knowledge/ (okf-foundation D21).
Regenerate: `bee knowledge index`. Check freshness: `bee knowledge index --check`.
Deterministic: byte-identical for the same bundle contents — path-sorted entries, LF endings,
never a generation timestamp or any other wall-clock value.
-->

# Knowledge Bundle

## Sections

- [patterns/](patterns/index.md) — 10 concept(s)
- [work/](work/index.md) — 98 concept(s)

## Critical patterns

- [Assertions that pin literal adjacency decide where new code is allowed to go](patterns/assertions-that-pin-literal-adjacency.md) — Pitfall: tests that locate markup or style by literal substring, whole-string equality, or first-versus-second match turn the ordering of a rendered file into a contract — so the natural place to insert a new attribute or rule is the one place that breaks a dozen tests for no behavioural reason.
- [A cell that promises a user-visible outcome owes one proof of the whole path](patterns/prove-the-whole-path.md) — Pitfall: slicing a user-visible outcome into per-unit cells lets every slice go green while the feature stays inert, because no proof ever crosses the seams between them.
