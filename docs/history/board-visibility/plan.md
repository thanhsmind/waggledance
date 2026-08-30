# Board Visibility — Plan

**Feature:** board-visibility · **Lane:** standard · **Class:** feature
**Flags:** public-contracts, multi-domain · **Product files:** 3
**Decisions:** D1 (all three layers, sliced; cross-project home is the target),
D3 (a section is refused unless the owner can act on it AND its number can go down —
supersedes D2), D4 (the board over-reports "waiting on you"; truth before addition)
**Backlog:** `p-e9386ebb` (the inbox)
**Worktree:** `waggledance--wt--board-visibility` (branch `wt/board-visibility`)
**Revision:** rev 2 — rev 1 was rewritten after the plan review wave. Two of its premises were
false and its slice 1 would have made the board **worse**; see "What rev 1 got wrong".

## The ask

> "Xem có rất nhiều phần dưới tài liệu như các inbox của từng project chưa được thể hiện
> trên board. Hãy làm hoàn chỉnh phần này để có thể nhìn thấy hết được những gì đang diễn
> ra trên các projects liên quan."

**"Đang diễn ra"** — *happening now* — so live state, not archives. **"Các projects liên
quan"** — plural, from one place — which D1 settles as the cross-project home page
(`server.rs:852` `index_page`).

## What rev 1 got wrong

**Rev 1's centrepiece would have degraded the board.** It proposed rendering `state.json`'s
recorded `waiting_on.subject`, on the theory that the board knows *that* a project waits but
not *what for*. Measured against the live store, the recorded subjects are frequently
unusable — four lanes carry the literal string `"AskUserQuestion"`, a tool name — while the
card at `views.rs:5396-5405` already derives a strictly better sentence: *"Waiting on you —
Shape gate awaiting your decision"*. Rendering the raw subject would replace a good sentence
with a worse one.

**And it faced the wrong direction.** `waiting_on_is_live` (`bee.rs:208-210`) whitelists no
`kind`, so a `turn-end` mark reads as live — though AGENTS.md defines `turn-end` as *"control
back with the human and nothing owed"*. Lane `board-live-morph` carries
`{kind: "turn-end", subject: "Không còn gì chờ bạn"}` — literally *"nothing is waiting on
you"* — and the board counts it as waiting. There are already **three** needs-you surfaces on
this page (stat tile `views.rs:6339`, card reason `views.rs:5396`, hub grouping). A fourth
would have multiplied a false signal. Hence D4: **truth before addition.**

**Two "forgotten" fields were deliberate deletions.** Rev 1 listed `config.gate_bypass` and
`running_workers` as never-rendered oversights. They are named on record as removed content —
`views.rs:3922-3925` (board-trim D1: the Process health panel, *"gate bypass"*, *"readers …
are untouched"*) and `views.rs:3911` (board-declutter: *"running workers … this page just
stops rendering them"*). Re-adding them is reverting a recorded decision, which is not the
agent's to do quietly. **Cut from slice 1** — see "Open question for the owner".

**Two smaller corrections.** `promote_proposals` is not unrendered: `bee.rs:1831-1848` folds
it into `compute_attention`, and it already shows as a Docs row on the feature page. And the
debt arithmetic rev 1 wanted already exists — `bee.rs:1835`:

```rust
let knowledge_debt = scribing_debt.len() + capture_queue.waiting + promote_unapplied;
```

turned into one attention item titled `"{n} knowledge-debt item(s)"` naming each contributor.
A fourth copy would be duplication, not a feature.

## Slice 1 — make the existing signal true, then give the per-project rail its facts

The question this slice answers: **standing at the home page, which projects actually need me,
and what is each one doing right now?** No new file readers.

### `bv-1` — Stop counting a finished turn as a wait (D4)

`waiting_on_is_live` (`bee.rs:208-210`) accepts any `kind`. Exclude `turn-end`: per AGENTS.md
the Stop hook sets it on every ordinary turn end, control back with the human and **nothing
owed** — it is the "idle" mark, not a demand. `gate` and `question` remain live.

This changes a number the owner sees, downward, on every existing needs-you surface at once.
That is the point.

**Proof:** `bee.rs` tests over fixture `state.json` files — `turn-end` reads not-live,
`gate` and `question` read live, absent `waiting_on` reads not-live. Plus a check of how many
of the 20-odd live lanes change classification, recorded in the cap.

### `bv-2` — Carry the wait's kind and subject, additively

Add `waiting_on: Option<BeeWaitingOn>` **beside** `waiting_on_live`, not instead of it. The
review measured the cost of replacement: `waiting_on_live` has 54 occurrences (`views.rs` 44,
`bee.rs` 7, `mcp.rs` 3). Additive is ~10 lines and breaks nothing.

No view consumes it in this cell. It exists so `bv-3` can make a quality judgment the reader
must not make for it.

**Proof:** `bee.rs` tests — kind and subject survive into the snapshot; an absent `waiting_on`
stays `None` rather than becoming an empty struct.

### `bv-3` — Widen the sentence the card already writes, when the subject earns it

The cheaper seam the review found: `views.rs:5396-5405` already builds *"Waiting on you —
{label} gate awaiting your decision"*, gated on `!working_now`, narrowed by `is_active`
(`views.rs:5433`), covered by 40+ existing tests. Widen **that one site** instead of adding a
strip that inherits none of it.

Rule, and it is the whole cell: **the recorded subject replaces the derived sentence only when
it is more informative than the derived sentence.** A subject that is a bare tool name
(`AskUserQuestion`), empty, or equal to the derived text keeps the derived wording. When the
subject is real — `"Agent logo marks — shape + execution approval"` — it wins.

**Proof:** view tests via the existing idiom (`views.rs:15527-15548`: fixture root →
`read_rollup` → `pairs` → assert on HTML) — an informative subject renders; `"AskUserQuestion"`
falls back to the derived sentence; an empty subject falls back; a `turn-end` project renders
no waiting line at all (bv-1's behaviour, seen from the view).

### `bv-4` — Give the per-project rail what is happening in that project

The review named the real per-project element: `project_sidebar` (`views.rs:1118-1148`,
*"One row per project, not a grid of cards"*) — the home page's actual per-project surface,
which today receives projects, panes and paseo badges but **no bee rollup at all**. The
Features section is organised by feature column, so per-project facts sit awkwardly there;
this rail is where "per project, what is happening" belongs, and it is exactly what the user
asked to see from one place.

Pass the rollup in and add, per project row: the active feature and phase, and the project's
own needs-you state (post-bv-1, so it is true). One line, no lists.

**D3 check, stated because the cell must pass it:** both facts can go down (a feature finishes;
a gate gets answered) and both are actionable from this page (the row already links into the
project). Knowledge-debt is deliberately **excluded** — it only climbs, and nothing on this
page can lower it, so D3 refuses it.

**Proof:** view tests — a project with an active feature and a live gate renders both; an idle
project renders neither and no empty scaffolding; the existing sidebar tests stay green.

### `bv-5` — Prove it against the three real projects

Render the home page against the live registry (waggledance, beehive, jarvis) and record in
`proof.md` what each element shows **beside the underlying store value**, so the render is
checked against the data rather than against itself. Include the before/after count of
projects reading as "waiting on you", which is bv-1's whole claim.

Two traps this repo has already sprung: builds land in `~/.cache/cargo-target` and
`target/fast/waggledance` is stale and lies — resolve the binary from
`cargo build --message-format=json`. And `.bee/config.json` records `commands.test` as
`cargo test --manifest-path packages/bee-rs/Cargo.toml`, which is **beehive's** path and does
not exist here — use `cargo test -p waggledance -p waggledance-core` and record that the
configured command is wrong (filed separately).

## Open question for the owner, carried into the gate

`gate_bypass: "full"` is recorded for waggledance — this project auto-approves its own gates —
and `running_workers` says who is working right now. Both were **deliberately removed** from
the board (board-trim D1, board-declutter). Restoring either reverses a recorded decision, so
slice 1 leaves them out. If the owner wants the bypass level visible, that is a supersession
they make, not one the agent takes.

## Later slices — headlines only, no cells yet

- **Slice 2 — the inbox.** Read `.bee/human-mailbox/`, list letters newest-first across
  projects, render frontmatter (`subject`, `needs_you[]`), flip read/unread by shelling to
  `bee mailbox mark` — never by writing the file (beehive's D6). **Verified: zero letters
  exist in any checkout**; only `entries/*.jsonl` (5 files here) has content, because
  composition fires only for an armed unattended run. The slice must render the entry layer or
  it ships an empty box. Read beehive's `docs/discovery/human-mailbox/MAP.md` (D1–D14, D16)
  first — the spec exists and is not ours to redesign.
- **Slice 3 — the docs trees.** New readers for `docs/knowledge/`, `docs/discovery/`
  (including spec-drops) and `docs/backlog.md`. Largest, least urgent, and `docs/backlog.md`
  partly duplicates `.bee/backlog.jsonl` which the board already renders — that overlap needs
  settling before either is shown.

## Smaller-path check

*Is there a cheaper shape that still honours D1?* This IS the cheaper shape, and the review
found it: rev 1's new strip would have been a fourth needs-you surface inheriting none of the
existing gating or tests, while widening one existing sentence (`bv-3`) inherits all of it.
Slice 1 adds zero file readers.

## Concurrency

`bv-1` and `bv-2` both touch `bee.rs`; `bv-3` and `bv-4` both touch `views.rs`; `bv-3` depends
on `bv-2`'s field and `bv-1`'s filter. **Serial**, in order — a real dependency chain plus file
overlap, not caution.

## What this plan does not do

- No new needs-you strip — D4 and the review's "three surfaces already" finding.
- No `gate_bypass`, `running_workers`, `velocity`, `tier_mix`, or `backlog.findings` rendering
  — recorded deletions or refused by D3.
- No fourth copy of the knowledge-debt arithmetic; no knowledge-debt on the home page at all.
- No writes to `.bee/human-mailbox/` in any slice — bee owns that file's state.
- No change to the six-column kanban, the live strip, or the Backlog & Review panel.
- No change to the per-project board — D1 puts the cross-project home first, and the two hub
  implementations (`views.rs:~5640` and `:5930`) will diverge slightly as a known cost.
