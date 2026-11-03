---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: Changes Diff Screen

Mode: `standard` — 2 risk flags: covered-contract-change, audit-security
Why this is the least workflow that protects the work: a new public route plus
a first-ever git integration on an unauthenticated daemon needs a frozen shape
and per-cell proof, but every security control is reused unchanged, so no
hard-gate ceremony.

## Requirements (from CONTEXT.md)

- D1 — Changes/diff screen only; no files-explorer tree, breadcrumb-zoom
  sidebars untouched.
- D2 — Diff = working tree vs HEAD (staged + unstaged + untracked-as-A); no
  base picker.
- D3 — Non-repo / no-git project: explained empty state; entry link stays.
- D4 — Reviewed marks + N/M counter client-side in localStorage per project.
- D5 — Denylist + gitignore hold: `.git/` and denied paths never appear.

## Discovery

Inspected `engine.rs:509 code_path`, `code_source.rs` (resolve/list/read),
`render.rs:240-274 highlight_source` (per-line output is span-balanced by
construction — safe in table cells), `server.rs:614` route block,
`views.rs:9511-9660` (Section enum, section_switch, code_page), the xia
report, and a three-seat hat wave over the first draft
(`reports/hat-wave-synthesis.md`). Key wave facts absorbed: `git diff HEAD`
compares working tree to HEAD directly, so statuses taken from it never have
the porcelain XY-pair or toplevel-relative-path problems; untracked files are
invisible to it and need `ls-files -o`; a deleted file's old content cannot
come from the working tree.

## Approach

Shell out to the system `git` binary — no new crate. Three read-only calls
per page load, all `git -C <project_root>` with a 10 s timeout and kill:

1. `git diff HEAD -M --name-status -z --relative -- .` — the authoritative
   changed-file list: NUL-safe paths (handles spaces/non-ASCII), statuses
   straight from the worktree↔HEAD comparison (no staged/unstaged XY pairs),
   `--relative` yields project-relative paths and auto-scopes a project root
   nested below the repo toplevel. Status letters beyond M/A/D/R (T, C, U…)
   map conservatively: T→M, C→A, U→M labeled "conflicted"; unknown → M.
   Renames render as R with an `old → new` label (planning's call on the
   CONTEXT open question).
2. `git diff HEAD -M --no-color --no-ext-diff --relative -U100000 -- .` —
   one parse: the giant context radius returns each changed file whole, so
   the FULL old text (context+del lines) and FULL new text (context+add
   lines) reconstruct from a single linear walk — no `git show` blob fetch,
   no hunk-pairing alignment step, and the old side of a deleted file comes
   from the diff itself. Sections are matched to list (1) by path.
3. `git ls-files -o --exclude-standard -z` — untracked files (git's own
   exclude rules), each then read via the `read_source` discipline (binary
   sniff + 2 MiB cap) and rendered as a full-add A section.

Failure shape: `git` missing, not a repo, or a timed-out/killed call →
typed `GitUnavailable`/`NotARepo` → the D3 empty state, never a 500.

D5 gate: every path from (1) and (3) passes
`code_source::resolve_source_path` with the project's exclude patterns; a
refused path is SKIPPED and counted into one aggregate "N files hidden by
project excludes" line — no per-file disclosure of denied names, matching
`code_response`'s no-disclosure rule.

Caps (named numbers): 2 MiB per reconstructed side per file (past it the
section renders a truncation banner, unhighlighted); 100 file sections per
page (sidebar still lists everything; sections past the cap render a
"section hidden — open the file in Code view" stub); 48 MiB total git stdout
read, past it the page banners "diff truncated". Submodule entries (path is
a directory) render a labeled "submodule changed" row, no hunks. An M entry
whose reconstructed sides are byte-equal (CRLF/filter artifacts) renders
"no content changes (line endings or filters)".

Rendering: `Engine::changes(project_id)` wraps the git layer (pattern of
`engine.rs:509`); both panes syntax-highlighted by `highlight_source(path,
full_text)` per side — full texts make multi-line parse state correct, and
per-line fragments drop straight into table cells. `Section` enum gains a
`Changes` variant; `section_switch` renders Docs | Code | Changes at every
call site (Docs and Code pages included — that is the covered-contract
surface the flag names).

Rejected:
- `git2` crate — C build cost against the ~1 s fast-profile dev loop.
- `git status --porcelain` parsing — toplevel-relative paths, XY staged
  pairs, C-quoting, conflict records: a second parser and five edge-case
  families for nothing list (1) doesn't already give.
- Per-hunk highlighting — wrong across block comments/raw strings and more
  code than full-file highlighting (render.rs:250-262).
- `#filelist` JSON-blob sidebar — pays off only for breadcrumb re-render;
  the changed list is static per load, server-rendered is fewer lines.
- Server-side review state — rejected in shaping (D4, 800c53f2).
- `git add -N` or any index mutation to fold untracked in — prohibited.

Risk map:

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| -U100000 body parse + reconstruction | MEDIUM | header/path parsing, whole-file reconstruction correctness | unit tests on tempdir fixture repos: M/A/D/R, spaces + non-ASCII paths, nested project root, deleted file, CRLF-only change, submodule, conflict status |
| D5 filtering | HIGH | leaking a denied path on an unauthenticated daemon | unit test: excluded path in git output never reaches WorkingTreeDiff; aggregate count only |
| caps & timeout | MEDIUM | unbounded git stdout / hung subprocess pins a tokio worker | truncation tests at each cap; timeout test with a stub slow command |
| Section enum change | LOW | mechanical, but touches every Docs/Code page | existing view tests + markup assertion for the third link |
| perf on big diffs | LOW | caps above bound the payload | cap tests |

## Shape

Phase plan:

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1. Walking skeleton | `git_diff.rs` (3-call design, parse, reconstruct, caps, timeout), `Engine::changes`, `/p/:id/_changes` route, minimal `changes_page` (real side-by-side, unstyled) | end-to-end proof incl. D3/D5 before polish | open `/p/<id>/_changes`, see the real working-tree diff | everything |
| 2. Screen polish | Sidebar (dir-grouped rows reusing `.chap-*` CSS, M/A/D badges, +n/−m counts, click-scrolls-to-section, scrollspy active state), sticky file headers, highlighted panes, theme-token diff palette (light+dark), Section::Changes in topbar, mobile drawer inherited via `#sidebar`/`.layout` markup | skeleton proved the data | screen matches screenshot | phase 3 |
| 3. Reviewed state | per-file checkbox (section header) + sidebar mirror, N/M counter, complete state at N==M, localStorage keyed by content-hash so an edited file's stale mark drops (D4) | pure client layer on a finished DOM | mark files, reload, marks persist; edit a marked file, mark clears | — |

Current slice: phases 1–3 as 3 sequential cells (same files, real
dependency); phase 3 stays its own cell to keep phase 2 — already the
fattest — from growing past review size.

## Test matrix

- Happy: fixture repo with modified + added + deleted + renamed + untracked
  files → WorkingTreeDiff lists all with right statuses (R carries old→new),
  full old/new texts reconstruct byte-exact (asserted against the fixture),
  page renders one section per file.
- Edge: non-repo project → D3 empty state. Untracked binary → "binary" row,
  no hunks. >2 MiB side → truncation banner. >100 sections → stub sections.
  CRLF-only M → "no content changes" row. Submodule dirty → labeled row.
  Conflict (U) status → M "conflicted". Path with spaces + non-ASCII →
  parsed whole (NUL list is authoritative). Project root nested below repo
  toplevel → only project files, project-relative. Empty diff → "no
  changes" state distinct from D3's.
- Error: git binary absent → D3 state, no 500. Timeout → D3-style error
  state, subprocess killed. Excluded/denied path in git output → skipped +
  counted, never named (D5).

## Out of scope

<!-- bee:not-a-deferral: out-of-scope register mirroring CONTEXT.md's fenced Deferred Ideas; no promise to act rides here. Plan-rev bumped 2026-08-30 for this fence; the commit picker later entered scope as D6 (cds-5/6), the rest stays out. -->
- Files-explorer tree + Docs/Code unification (D1), watcher-driven live
  refresh, server-side review state — out of scope. The base/commit picker,
  out in this plan's first revision, was pulled INTO scope by D6 during uat.
<!-- /bee:not-a-deferral -->
