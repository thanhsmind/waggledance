<!--
GENERATED FILE — do not hand-edit.
Rendered by `bee knowledge index` from concept frontmatter inside docs/knowledge/ (okf-foundation D21).
Regenerate: `bee knowledge index`. Check freshness: `bee knowledge index --check`.
Deterministic: byte-identical for the same bundle contents — path-sorted entries, LF endings,
never a generation timestamp or any other wall-clock value.
-->

# patterns/

## Concepts

- [Assertions that pin literal adjacency decide where new code is allowed to go](assertions-that-pin-literal-adjacency.md) — Pitfall: tests that locate markup or style by literal substring, whole-string equality, or first-versus-second match turn the ordering of a rendered file into a contract — so the natural place to insert a new attribute or rule is the one place that breaks a dozen tests for no behavioural reason.
- [A cell that promises a user-visible outcome owes one proof of the whole path](prove-the-whole-path.md) — Pitfall: slicing a user-visible outcome into per-unit cells lets every slice go green while the feature stays inert, because no proof ever crosses the seams between them.
- [A frozen plan's own first commit reads as an edit to it](the-first-commit-of-a-frozen-plan.md) — Pitfall: the plan-freeze guard treats any mention of an approved plan file in a git path as a revision, so committing that file for the first time is refused — and satisfying the guard by bumping the revision records a revision that never happened.
- [The lock trail names the owner; session start times only correlate with it](the-lock-trail-names-the-owner.md) — Practice: to find which session wrote an untracked file, match the contention log's lock trail against file mtimes instead of bracketing by session start times — the log survives the case that defeats inference, an owner whose transcript lives under a different project than the repo it writes into.
- [A test that builds the collaborator itself cannot see that production builds it differently](the-test-builds-the-collaborator-production-does-not.md) — Pitfall: when a unit test hands the function the dependency it wants, the test proves the function and nothing about the wiring — so a lazily-built handle can be missing on every real call while the whole suite stays green.
