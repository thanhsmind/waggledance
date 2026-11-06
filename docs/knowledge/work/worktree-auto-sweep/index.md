<!--
GENERATED FILE — do not hand-edit.
Rendered by `bee knowledge index` from concept frontmatter inside docs/knowledge/ (okf-foundation D21).
Regenerate: `bee knowledge index`. Check freshness: `bee knowledge index --check`.
Deterministic: byte-identical for the same bundle contents — path-sorted entries, LF endings,
never a generation timestamp or any other wall-clock value.
-->

# work/worktree-auto-sweep/

## Concepts

- [worktree-auto-sweep — the daemon reclaims finished bee worktrees](worktree-sweep.md) — On a timer, the daemon runs each registered bee project's own bee worktree prune with a configured age floor, so merged, clean, idle worktrees disappear without anyone running prune by hand. bee alone decides what is finished; the daemon only schedules it and logs the verdicts.
