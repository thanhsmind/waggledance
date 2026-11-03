# Hat wave synthesis — changes-diff-screen (plan step)

Date: 2026-08-30 · Seats: hat-facts-gaps (opus, dispatch b2626660),
hat-alternatives (opus, dispatch bb971310), hat-user-impact (sonnet,
dispatch 32d4cf49). All three returned inside the ceiling; none dropped.
Wave-open reason logged in decisions (standard lane, first git integration).

## Verdicts

- hat-alternatives: **REDRAFT** — single `git diff HEAD -U100000 --relative`
  + `ls-files -o` is materially less code than status-parse + hunk-parse +
  blob fetch. ACCEPTED; plan.md Approach rewritten around it.
- hat-facts-gaps: 7 BLOCKER / 11 WARNING. All blockers dissolved or answered
  by the redraft (see mapping below).
- hat-user-impact: 12 findings (11 GAP / 1 FINE). Folded into phases 2–3.

## Disposition of blockers (facts-gaps)

1. porcelain toplevel-relative paths → moot: porcelain dropped; `--relative`
   + cwd-scoped `ls-files` give project-relative paths. Kept as a test row.
2. untracked dirs collapse in porcelain → moot; `ls-files -o` lists files.
3. no git command named / untracked not in `git diff HEAD` → plan now names
   the 3 exact calls and the union.
4. rename form undecided → decided: R badge with `old → new` label (-M).
5. old side of deleted file → reconstructed from the -U100000 body, not
   read_source.
6. caps had no mechanism → named: 2 MiB/side, 100 sections, 48 MiB stdout,
   each with its render behavior.
7. conflict statuses → conservative mapping table (T→M, C→A, U→M
   "conflicted", unknown→M).

Warnings absorbed: NUL-safe list is authoritative for paths (9); D5 refusals
skip + aggregate count, no names (10/12); gitignore divergence noted —
tracked files unaffected, untracked governed by git's own excludes then
denylist (11); submodule labeled row (13); CRLF empty-diff row (14);
full-file highlighting only (15); Section enum change named as the
covered-contract surface (16); 10 s timeout + kill (17); route-order claim
downgraded to comment (18).

## Disposition of user-impact gaps

Loading (1): caps bound the payload — named instead of a spinner. Sidebar
scroll + scrollspy (2,3): phase 2 must-haves. Grouping depth (4): dir-header
rows + indent, not a nested tree (D1). Checkbox placement (5): section
header + sidebar mirror. Complete state (6): phase 3. Stale marks (7):
content-hash key, stale marks drop. Mobile (8): `#sidebar`/`.layout` markup
inherited. Theme (10): palette as CSS variables over existing tokens.
Truncation UX (11): banner + "open in Code view" stub. Binary label (12):
named row.

## Alternatives-seat point not taken

P4 (3 cells → 2): NOT taken — phase 2 is already the fattest cell; the seat
itself flags a >400-line split-back. Sequential 3-cell chain kept.
