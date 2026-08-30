# Board Visibility — Slice 2: the inbox

**Feature:** board-visibility · **Lane:** standard · **Slice:** 2
**Decisions:** D1 (cross-project home is the target), D3 (a section is refused unless the
owner can act on it AND its number can go down), D6 (user, 2026-08-30: spec-correct,
letters only, with a self-explaining empty state)
**Backlog:** `p-e9386ebb`
**Worktree:** `waggledance--wt--board-visibility` (branch `wt/board-visibility`)
**Revision:** rev 1 · slice 1 merged at `2ca86f7`

Every load-bearing claim below carries how it was established: **[đã đọc]** I opened that
file, **[đã chạy]** I ran it and this is the output, **[đoán]** I inferred it. There are no
`[đoán]` claims in this plan — that is the point. This dogfoods the discipline proposed to
beehive after slice 1's review found five inferred-and-wrong premises.

## First contact, done before this plan was written

- **[đã chạy]** `find` over all three checkouts: **zero** `.md` letters anywhere;
  `entries/*.jsonl` present in waggledance (5) and beehive (20), none in jarvis.
- **[đã chạy]** 13 entry rows in this session's own file, **all `kind: "cap"`**, and
  `needs_you` empty in every one.
- **[đã chạy]** `bee mailbox mark --id does-not-exist-000 --status read` →
  *"no letter … remedy: list that directory and pass a name it holds"*. The verb exists and
  refuses cleanly.
- **[đã đọc]** `render.rs:363` sets comrak's `front_matter_delimiter` — it **strips**
  frontmatter for rendering. Nothing in this repo **parses** it into typed fields.
- **[đã chạy]** `Cargo.lock:3052` — `yaml-rust 0.4.5` is already in the tree transitively.

## Why the mailbox is empty, and why that is not a bug

**[đã đọc]** `mailbox.rs:740-742`:

```rust
pub(crate) fn armed(root: &Path) -> bool {
    herding_configured(root) && owner_armed_the_loop(root)
}
```

Two ANDed signals. **[đã chạy]** Signal 1 (a `herding` block in `.bee/config.json`) is
satisfied in both waggledance and beehive. Signal 2 is the owner's marker file
`<main-root>/.bee/tmp/bee-herding.enable` — **absent in all three checkouts**. So `armed()`
is false everywhere, entries accumulate, and by D9 *"only an unattended run composes and
files a letter"*. Composition additionally fires when a run's work record reaches
`done`/`dropped` (**[đã đọc]** `work.rs:298-300`).

The user chose the spec-correct build knowing this (D6). The empty state is therefore a
first-class deliverable, not a placeholder: it must **say why it is empty**, because bee's
own source names the failure mode we would otherwise create (**[đã đọc]**
`mailbox.rs:1817-1819`): *"a silently missing letter is worse than a noisy one: the human
would read an empty mailbox as a quiet night rather than as a broken store."*

## What bee locked, and what it left to us

**Binding on this slice** (all **[đã đọc]** from beehive's `CONTEXT.md` / `mailbox.rs`):

- **D1** — *"no rendering surface, no listing UI, no viewer ships from bee"*. The inbox is ours.
- **D6** — *"Read/unread is a field bee owns inside the letter file. The waggledance inbox
  flips it by calling a bee command, never by writing the file."* One sanctioned mutation:
  `bee mailbox mark --id <letter-file-name> --status read|unread [--json]`, idempotent,
  returning `{letter, path, status, previous_status, changed}`.
- **D3 (theirs)** — the frontmatter field list is the machine contract and closed:
  `subject`, `run`, `project`, `filed_at`, `status`, `items[]`, `needs_you[]`. Required at
  read: the first five; a letter missing any is `Unreadable`.
- **D11** — one letter per run; filename `<UTC-timestamp>-<short-run-slug>.md`, which is
  also the letter's only id. *"a directory listing is the index"* — there is no manifest.
- `status` is a closed set `unread|read`. `needs_human_decision` in a letter is **derived on
  read, never authoritative** (`mailbox.rs:459-462`) — we must not trust it.
- Letters live in the **control root** (main checkout, never a worktree) and the directory is
  **git-ignored** — runtime state.
- A run reaching its end again **re-composes its letter in place**, keeping `filed_at`,
  filename and the human's `status` (`mailbox.rs:1749-1755`). Content and mtime can change
  after the human has read it; the id cannot.

**Explicitly not settled by bee, so ours to decide and record:** per-project vs
cross-project, sort order, unread counts, where the inbox lives, and the empty state.

## Decisions this plan takes, for the gate to accept or reject

1. **Cross-project, newest first.** D1 puts the target on the home side; letters carry a
   `project` frontmatter field, so aggregation is free. Filenames are timestamp-led
   precisely so a bare listing sorts (their D11 rationale).
2. **A dedicated `/inbox` route, with only an unread count on the home page.** Reading
   letters is a reading activity and would swamp the home page. The count passes D3 on both
   clauses: it **goes down** as letters are read, and the action that lowers it is one click
   away. A list of letter rows on the home page would not.
3. **Parse with `yaml-rust`, already in the lockfile.** Zero new dependency.
   **[đã đọc]** `mailbox.rs:1224` says bee's emitter *"must be valid YAML for the consuming
   inbox's real parser"* — bee wrote it expecting a real parser, so hand-rolling one would
   work against its own stated contract. Trade-off named: `yaml-rust 0.4.5` is unmaintained;
   it is chosen because it is **already compiled into this build**, so using it adds no new
   supply-chain surface. Rejected: adding a maintained YAML crate (new dependency for a
   parse we can already do), and hand-parsing (contradicts bee's stated assumption).
4. **An unreadable letter is shown as unreadable, never hidden.** Same reasoning as the
   empty state: a broken store must not read as a quiet night.

## Shape — five cells

### `bi-1` — Read the letters (waggledance-core)

New reader: list `<root>/.bee/human-mailbox/*.md` per project, parse frontmatter into a typed
record, expose it on the rollup the way `bee.rs` exposes everything else. Required-field
failures become an `Unreadable { file, reason }` variant — **surfaced, never dropped**.
`needs_human_decision` is ignored on read. Reader only; nothing renders yet.

**Proof:** fixture-directory tests — a well-formed letter parses every field; a letter
missing each required field in turn is `Unreadable` and names which; `items: []` /
`needs_you: []` parse as empty, not missing; a `needs_you` entry with no `kind` parses (their
younger-field rule); a directory with no letters yields an empty list, not an error.

### `bi-2` — The `/inbox` page

New route beside the existing ones (`server.rs:447-476` is the table). Cross-project list,
newest first, one row per letter: subject, project, filed-at, unread marker. Clicking a row
shows that letter's body — the markdown pipeline already exists.

The empty state is part of this cell and is written to be read by a human who does not know
bee's internals: no letters yet, letters are filed only by unattended runs, and this machine
has not armed one.

**Proof:** view tests over a fixture registry — letters from two projects interleave newest
first; an unreadable letter renders as unreadable with its filename; zero letters renders the
explaining empty state and not a bare "0".

### `bi-3` — Mark read and unread, through bee

A control on each row shelling to `bee mailbox mark --id <file-name> --status …`, parsing its
JSON result. **The letter file is never opened for writing anywhere in waggledance** — D6.
A failed mark reports the failure; it never optimistically flips the UI.

**Proof:** the command is invoked with the letter's exact file name and the expected status;
a refusal surfaces rather than silently succeeding; re-marking to the same status is treated
as success (`changed: false`), matching bee's idempotence contract.

### `bi-4` — The unread count on the home page

One number, cross-project, linking to `/inbox`. Renders nothing at zero — the home page keeps
its "zero is the honest answer" rule (**[đã đọc]** `views.rs:6341-6342`).

**Proof:** view tests — a count renders and links; zero renders nothing; the count is the sum
across projects.

### `bi-5` — Prove it against a letter bee actually composed

There are no letters to render, and hand-forging one would prove nothing about bee's real
emitter — it would prove our parser reads our own fixture.

Instead: in a **scratch checkout** (never the user's real tree, never the user's real
`.bee/human-mailbox/`), arm herding, drive a run to a terminal work status so **bee itself**
composes a letter from real entries, then render that letter through the new inbox and flip
its read state with `bee mailbox mark`.

**Do not arm herding in any of the three registered checkouts** — that is the user's
operational decision, and D6 recorded that they have not taken it.

**Proof:** the composed letter's path and frontmatter recorded verbatim beside what the
inbox renders for it; the mark command's JSON result recorded before and after; the real
store untouched, evidenced by a before/after listing of all three checkouts.

Binary traps this repo has already sprung, both **[đã chạy]** verified during slice 1: builds
land in `~/.cache/cargo-target` and the in-repo `target/fast/waggledance` is stale and reports
the same version; and two trees sharing `CARGO_TARGET_DIR` can leave a byte-identical binary
while cargo reports success. Give any comparison tree its own target dir and verify the
binaries differ before trusting a comparison.

## Smaller-path check

*Is there a cheaper shape that still honours D6?* Considered: put the letter list directly on
the home page and skip the `/inbox` route. Rejected — it fails D3's second clause the moment
letters accumulate, and D3 exists because a list-shaped block was already deleted from this
board once for exactly that.

## What this slice does not do

- No reading of `entries/*.jsonl`. **[đã đọc]** the spec neither permits nor forbids it, and
  the entry line is explicitly free to grow keys while the letter frontmatter is frozen —
  so it is not a contract to build on. The user chose letters-only (D6).
- No answering a `needs_you` item — bee's D13 ships no reply path and says so.
- No writing to `.bee/human-mailbox/`, ever, in any form.
- No arming of herding in the user's checkouts.
- No push notification of letter subjects — beehive's handover names it out of scope and
  ours to decide later.
