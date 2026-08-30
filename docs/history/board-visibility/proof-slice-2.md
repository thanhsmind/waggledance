# Board Visibility — slice 2 proof: the inbox against a letter bee actually composed

**Cell:** `bi-5` · **Feature:** board-visibility · **Slice:** 2
**Run 1:** 2026-08-30, 18:43–18:52 (+07) · **Worker:** `w-bi-5` — returned `[BLOCKED]`,
having composed the real letter and produced legs (a), (d-commands), (e), (g).
**Run 2:** 2026-08-30, 18:53–19:05 (+07) · **Worker:** `w-bi-5b` — the render legs, on a
route harness instead of a daemon.
**Status: COMPLETE.** This document is self-contained: run 1's legs are folded in below
with their own commands and outputs, not referred to as done elsewhere.

What is being proved is the promise the slice makes to the user — *"I can see what
happened in every project"* — against data this repo did not write. Every fixture in
`bi-1`…`bi-4` was authored by the same hands that wrote the parser and the view, so it
agrees with them by construction. The letter below did not come from here: **bee composed
it**, out of its own real verbs.

## Method — how a real letter was obtained, and how it was rendered without a daemon

### Getting a real letter at all

There were **zero** `.md` letters on this machine, and that is not an accident: bee files
a letter only from an unattended run (its D9), and `armed()` requires BOTH a `herding`
block in `.bee/config.json` AND the owner's marker file
`<main-root>/.bee/tmp/bee-herding.enable`. No registered checkout has that marker — leg
(g) below is the measurement, taken again in run 2.

Arming a registered checkout to manufacture a letter would have changed the user's own
machine. So run 1 built a **throwaway store inside this worktree's `target/`** — a
directory that is git-ignored, is not registered with waggledance, and is not any
project's control root:

```
$ cat target/bi5-scratch/.bee/config.json
{
  "herding": { "agent_command": "claude-sonnet" },
  "commands": { "test": "true" },
  "gate_bypass": "total",
  "worktree_first": "off"
}
$ ls -la target/bi5-scratch/.bee/tmp/
total 0
drwxr-xr-x 1 thanhsmind thanhsmind  36 Aug 30 18:43 .
drwxr-xr-x 1 thanhsmind thanhsmind 172 Aug 30 18:45 ..
-rw-r--r-- 1 thanhsmind thanhsmind   0 Aug 30 18:43 bee-herding.enable
```

Both of `armed()`'s signals, satisfied in a scratch tree and **nowhere else**.

### Rendering it without starting a second daemon

Run 1 blocked here, correctly. `waggledance serve` resolves its registry and its
`daemon.lock` from `$HOME/.waggledance` with no `--data-dir` override (`config.rs:184`);
redirecting `HOME` is refused by this session's isolation guard; and using the real
`~/.waggledance` would have taken the lock out from under the user's live daemon
(`server.rs:344`, `:378`). There is no fourth option that starts a daemon safely.

Run 2 took the route harness instead. `crates/waggledance/src/server.rs`'s
`bee_route_tests` already drives `router()` — the **real** route table, the real handlers,
the real view layer — over a `fresh_root()` on disk with `register()` putting it in an
in-memory registry. That is the whole path from a letter file to rendered HTML with the
process boundary removed, and it is the same harness `bi-2`'s own tests use. **No daemon
was started, `~/.waggledance` was never opened, and no registered checkout was armed or
written.**

Each leg's rendered HTML below is the `<main>` element of the actual response, captured
by running the proof tests with `--nocapture`. The surrounding page chrome (head, top bar,
agent drawer, scripts) is elided as noise; nothing inside `<main>` is.

---

## Leg (a) — the letter bee composed, with its provenance

The composing store's own entry stream, which is what bee's composer reads. Two entries,
written by two different verbs — `bee cells finish` and `bee cells block`:

```
$ cat target/bi5-scratch/.bee/human-mailbox/entries/bi5-scratch-run.jsonl
{"at":"2026-08-30T11:45:54.706Z","kind":"cap","what":"Add the scratch reader file","files":["reader.txt"],"commit":null,"proof":"true — green — scratch fixture, one file","departure":null,"needs_you":[]}
{"at":"2026-08-30T11:46:06.705Z","kind":"blocker","what":"the store path is not settled — the scratch reader needs a decision on where it reads from","files":[],"commit":null,"proof":null,"departure":null,"needs_you":[{"id":"sx-2","what":"the store path is not settled — the scratch reader needs a decision on where it reads from","blocks":"Point the scratch reader at the store","kind":"question","needs_human_decision":true}]}
```

`bee work set --status done` then fired the composer (bee's `work.rs:298-300`: composition
fires when a run's work record reaches `done`/`dropped`). What it wrote:

```
$ ls target/bi5-scratch/.bee/human-mailbox/
20260830T114615Z-bi5-scratch-run.md
entries
$ sha256sum target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md
f48f7c17d453cb9ae2c18978c1d48a20dee909d0d96f22c7b3a1eb744ac15e00  target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md
```

```
$ cat target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md
---
subject: "Add the scratch reader file"
run: "bi5-scratch-run"
project: "bi5-scratch"
filed_at: "2026-08-30T11:46:15.336Z"
status: "read"
items:
  - what: "Add the scratch reader file"
    files:
      - "reader.txt"
    commit: null
    proof: "true — green — scratch fixture, one file"
    departure: null
  - what: "the store path is not settled — the scratch reader needs a decision on where it reads from"
    files: []
    commit: null
    proof: null
    departure: null
needs_you:
  - id: "sx-2"
    what: "the store path is not settled — the scratch reader needs a decision on where it reads from"
    blocks: "Point the scratch reader at the store"
    kind: "question"
    needs_human_decision: true
---

## Done

- Add the scratch reader file

## Broken or unfinished

- the store path is not settled — the scratch reader needs a decision on where it reads from

## Needs your call

- [sx-2] the store path is not settled — the scratch reader needs a decision on where it reads from — blocks: Point the scratch reader at the store
```

`status: "read"` rather than `"unread"` because leg (d) already flipped it. The file name
is the letter's only id (D11).

**Two shapes here that no fixture in this repo had ever forced**, and they are the reason
this leg comes first:

- `items[].files` is a **nested block sequence** under an indented key
  (`files:` then `      - "reader.txt"`), not an inline flow list.
- `commit`, `proof` and `departure` are explicit **`null` scalars**. A hand-written fixture
  would have omitted the keys; absent and null are different values to a YAML parser, and
  a parser can easily accept one and reject the other.

---

## Leg (a2) — the reader, pointed at those exact bytes

**This is the thing the proof was for.** The question is not whether the reader parses a
letter we wrote; it is whether it parses the letter bee wrote.

The letter's bytes were copied verbatim into `waggledance-core`'s
`bee::mailbox` tests as `REAL_BEE_LETTER` (same sha as above; kept as a literal rather
than read off `target/`, which is scratch a clean checkout does not have), and the new
test `the_letter_bee_actually_composed_parses` asserts every field against the
frontmatter printed above — including the nested `files` sequence and each explicit
`null`.

```
$ cargo test -p waggledance-core --lib bee::mailbox
running 15 tests
test bee::mailbox::tests::a_file_with_no_frontmatter_fence_is_unreadable ... ok
test bee::mailbox::tests::a_missing_or_empty_mailbox_is_an_empty_list_not_an_error ... ok
test bee::mailbox::tests::frontmatter_that_is_not_yaml_is_unreadable_not_a_panic ... ok
test bee::mailbox::tests::an_item_without_what_is_unreadable_and_names_the_item ... ok
test bee::mailbox::tests::empty_lists_parse_as_empty_and_stay_distinguishable_from_absent ... ok
test bee::mailbox::tests::a_status_outside_the_closed_set_is_unreadable ... ok
test bee::mailbox::tests::a_needs_you_entry_without_kind_parses ... ok
test bee::mailbox::tests::a_half_written_departure_is_unreadable ... ok
test bee::mailbox::tests::the_letter_bee_actually_composed_parses ... ok
test bee::mailbox::tests::reading_the_mailbox_opens_no_letter_for_writing ... ok
test bee::mailbox::tests::well_formed_letter_parses_every_frontmatter_field ... ok
test bee::mailbox::tests::read_snapshot_surfaces_the_mailbox ... ok
test bee::mailbox::tests::letters_come_back_sorted_and_a_broken_one_is_surfaced_beside_them ... ok
test bee::mailbox::tests::needs_human_decision_is_never_read_into_the_record ... ok
test bee::mailbox::tests::a_letter_missing_any_required_field_is_unreadable_and_names_it ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

**Verdict: no defect. The reader parsed the real letter on the first attempt, and
`crates/waggledance-core/src/bee/mailbox.rs` needed no change.** That is a genuine
result rather than a formality — it is the one outcome that could have sent this cell back
into `bi-1` — and it holds for a specific, checkable reason: `bi-1` chose `yaml-rust` over
a hand-rolled scanner precisely because bee's emitter says its block "must be valid YAML
for the consuming inbox's real parser", and a real YAML parser does not care whether a
sequence is nested or inline, nor whether a null is spelled or omitted. The shape is now
pinned by a test, so a later "simplification" of that parse cannot quietly break it.

---

## Leg (b) — what `GET /inbox` renders for that letter

Recorded here **beside leg (a)'s frontmatter**, deliberately: a render checked only
against itself proves nothing. Every visible string below traces to a field printed above.

```
$ cargo test -p waggledance --bin waggledance -- --exact --nocapture \
    server::bee_route_tests::the_inbox_renders_the_letter_bee_actually_composed
```

```html
<main class="fg-page">
  <header class="fg-pagehead">
    <div class="fg-pagehead__eyebrow">Hộp thư</div>
    <h1 class="fg-pagehead__title">Thư từ các lượt chạy</h1>
    
  </header>
  <div class="fg-card">
  <div class="fg-card__head"><a class="fg-card__title" href="/inbox/waggledance-server-bee-inbox-real-letter-558639-17/20260830T114615Z-bi5-scratch-run.md">Add the scratch reader file</a><span class="fg-chip fg-chip--neutral">Đã đọc</span></div>
  <div class="fg-card__sub">bi5-scratch · 2026-08-30T11:46:15.336Z</div>
  <form class="fg-card__foot" method="post" action="/inbox/waggledance-server-bee-inbox-real-letter-558639-17/20260830T114615Z-bi5-scratch-run.md/mark">
  <input type="hidden" name="status" value="unread">
  <input type="hidden" name="back" value="/inbox">
  <button type="submit" class="fg-btn fg-btn--secondary">Đánh dấu chưa đọc</button>
</form>
</div>
</main>
```

Held against the data, field by field:

| rendered | frontmatter field it came from |
| --- | --- |
| `Add the scratch reader file` (the link text) | `subject: "Add the scratch reader file"` |
| `bi5-scratch` | `project: "bi5-scratch"` |
| `2026-08-30T11:46:15.336Z` | `filed_at: "2026-08-30T11:46:15.336Z"` |
| `Đã đọc`, and **no** `Chưa đọc` chip | `status: "read"` |
| `…/20260830T114615Z-bi5-scratch-run.md` in both the href and the form action | the file name, which D11 makes the id |
| `value="unread"` on the flip | the only flip a read letter can offer |
| no unread-count chip in the page head | one letter, zero unread |
| no `Không đọc được` anywhere | the letter parsed |

The project segment (`waggledance-server-bee-inbox-real-letter-558639-17`) is the
harness's own registry id for the scratch root, not part of the letter.

---

## Leg (c) — the same letter with a required field removed

Not a new fixture: leg (a)'s bytes with the `filed_at:` line cut out — one of the five
human-mailbox D3 makes required at read. bee's own reason for showing it rather than
hiding it: *"a silently missing letter is worse than a noisy one: the human would read an
empty mailbox as a quiet night rather than as a broken store."*

```
$ cargo test -p waggledance --bin waggledance -- --exact --nocapture \
    server::bee_route_tests::a_real_letter_with_a_required_field_removed_renders_as_unreadable
```

```html
<main class="fg-page">
  <header class="fg-pagehead">
    <div class="fg-pagehead__eyebrow">Hộp thư</div>
    <h1 class="fg-pagehead__title">Thư từ các lượt chạy</h1>
    
  </header>
  <div class="fg-card fg-card--rule">
  <div class="fg-card__head"><div class="fg-card__title">20260830T114615Z-bi5-scratch-run.md</div><span class="fg-chip fg-chip--danger">Không đọc được</span></div>
  <div class="fg-card__sub">Scratch</div>
  <div class="fg-card__body">missing required field `filed_at`</div>
</div>
</main>
```

The file is on the page **by name**, marked `Không đọc được`, and the body names the field
that is missing rather than the parser. It is not clickable and offers no flip — there is
no letter behind it to open or to mark.

---

## Leg (d) — the mark command, and the page that reads its result back

Two halves. Both are here.

**The command half** (run 1, against the scratch store, using the project's own bee — the
one sanctioned mutation, human-mailbox D6):

```
$ cd target/bi5-scratch
$ bee mailbox mark --id 20260830T114615Z-bi5-scratch-run.md --status read --json
{
  "letter": "20260830T114615Z-bi5-scratch-run.md",
  "path": "/home/thanhsmind/Projects/goglbe/waggledance--wt--board-visibility/target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md",
  "status": "read",
  "previous_status": "unread",
  "changed": true
}
```

`previous_status: "unread"` → `status: "read"`, `changed: true`, and the field inside the
file flipped — which is why leg (a)'s `cat` above shows `status: "read"`.

**The render half** (run 2): the letter opened, reading that flipped field back.

```
$ cargo test -p waggledance --bin waggledance -- --exact --nocapture \
    server::bee_route_tests::opening_the_real_letter_shows_the_state_the_mark_command_set
```

```html
<main class="fg-page">
  <header class="fg-pagehead">
    <div class="fg-pagehead__eyebrow">bi5-scratch · 2026-08-30T11:46:15.336Z</div>
    <h1 class="fg-pagehead__title">Add the scratch reader file</h1>
    <div class="fg-pagehead__aside"><span class="fg-chip fg-chip--neutral">Đã đọc</span><form class="fg-card__foot" method="post" action="/inbox/waggledance-server-bee-inbox-real-letter-open-558639-22/20260830T114615Z-bi5-scratch-run.md/mark">
  <input type="hidden" name="status" value="unread">
  <input type="hidden" name="back" value="/inbox/waggledance-server-bee-inbox-real-letter-open-558639-22/20260830T114615Z-bi5-scratch-run.md">
  <button type="submit" class="fg-btn fg-btn--secondary">Đánh dấu chưa đọc</button>
</form></div>
  </header>
  <article class="fg-prose"><h2 data-sourcepos="1:1-1:7"><a href="#done" class="anchor" id="done" rel="noopener noreferrer"></a>Done</h2>
<ul data-sourcepos="3:1-4:0">
<li data-sourcepos="3:1-4:0">Add the scratch reader file</li>
</ul>
<h2 data-sourcepos="5:1-5:23"><a href="#broken-or-unfinished" class="anchor" id="broken-or-unfinished" rel="noopener noreferrer"></a>Broken or unfinished</h2>
<ul data-sourcepos="7:1-8:0">
<li data-sourcepos="7:1-8:0">the store path is not settled — the scratch reader needs a decision on where it reads from</li>
</ul>
<h2 data-sourcepos="9:1-9:18"><a href="#needs-your-call" class="anchor" id="needs-your-call" rel="noopener noreferrer"></a>Needs your call</h2>
<ul data-sourcepos="11:1-11:151">
<li data-sourcepos="11:1-11:151">[sx-2] the store path is not settled — the scratch reader needs a decision on where it reads from — blocks: Point the scratch reader at the store</li>
</ul>
</article>
  <p class="fg-card__sub">20260830T114615Z-bi5-scratch-run.md</p>
</main>
```

The two halves meet: the command wrote `read`, and the page shows `Đã đọc` and offers only
`value="unread"` — the flip back. The prose under the frontmatter is bee's own three
sections, through the existing markdown pipeline; the `needs_you` ask reaches the reader as
its own heading, id and all.

---

## Leg (e) — marking a letter to the status it already has

bee's `mailbox mark` is idempotent and its own source says a retry must not be punished.
Re-run fresh in run 2, against a letter already `read`:

```
$ cd target/bi5-scratch
$ bee mailbox mark --id 20260830T114615Z-bi5-scratch-run.md --status read --json
{
  "letter": "20260830T114615Z-bi5-scratch-run.md",
  "path": "/home/thanhsmind/Projects/goglbe/waggledance--wt--board-visibility/target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md",
  "status": "read",
  "previous_status": "read",
  "changed": false
}
[bee] mailbox mark 0ms
exit=0
```

`changed: false`, `previous_status == status`, **exit 0**. A double submit or a refresh is
success, not an error — which is what `re_marking_a_letter_to_the_status_it_already_has_is_success`
holds the route to.

And the bytes did not move:

```
$ sha256sum target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md
f48f7c17d453cb9ae2c18978c1d48a20dee909d0d96f22c7b3a1eb744ac15e00  .../20260830T114615Z-bi5-scratch-run.md
```

Same sha as leg (a).

---

## Leg (f) — zero letters, which is what every registered project actually has today

The empty state is a first-class deliverable of this slice (D6), not a placeholder: on
this machine it is the whole page. Quoted in full, unedited:

```
$ cargo test -p waggledance --bin waggledance -- --exact --nocapture \
    server::bee_route_tests::the_zero_letter_inbox_renders_its_whole_explanation
```

```html
<main class="fg-page">
  <header class="fg-pagehead">
    <div class="fg-pagehead__eyebrow">Hộp thư</div>
    <h1 class="fg-pagehead__title">Thư từ các lượt chạy</h1>
    
  </header>
  <div class="fg-card">
  <div class="fg-card__head"><div class="fg-card__title">Chưa có lá thư nào</div></div>
  <div class="fg-card__body fg-prose">
    <p>Thư ở đây không phải do bạn hay tôi ngồi viết: mỗi lượt chạy <strong>không có người ngồi trực</strong> sẽ tự để lại đúng một lá thư khi nó xong việc — kể lại đã làm gì, đi chệch kế hoạch ở đâu, hỏng chỗ nào, và có câu nào chỉ bạn mới trả lời được.</p>
    <p>Máy này chưa bật chế độ chạy không người trực, nên chưa lượt chạy nào tới lúc phải viết thư. Hộp thư trống vì vậy là <strong>bình thường</strong>, không phải hỏng hóc — khi nào bạn bật nó lên và một lượt chạy kết thúc, lá thư đầu tiên sẽ nằm ở đây.</p>
  </div>
</div>
</main>
```

It says what a
letter is, who writes one, **why there are none**, and that this is normal — never a bare
`0` and never an empty list frame, either of which would read as a broken store or a page
that has not finished loading.

---

## Leg (g) — nothing on the user's machine was armed or written

The scratch store in leg (a) is armed. Nothing else is. Measured again in run 2, after all
of the above:

```
$ ls -la .../waggledance/.bee/tmp/bee-herding.enable .../beehive/.bee/tmp/bee-herding.enable .../jarvis/.bee/tmp/bee-herding.enable
ls: cannot access '/home/thanhsmind/Projects/goglbe/waggledance/.bee/tmp/bee-herding.enable': No such file or directory
ls: cannot access '/home/thanhsmind/Projects/goglbe/beehive/.bee/tmp/bee-herding.enable': No such file or directory
ls: cannot access '/home/thanhsmind/Projects/goglbe/jarvis/.bee/tmp/bee-herding.enable': No such file or directory
```

```
$ ls -la .../waggledance/.bee/human-mailbox/ .../beehive/.bee/human-mailbox/ .../jarvis/.bee/human-mailbox/
ls: cannot access '/home/thanhsmind/Projects/goglbe/jarvis/.bee/human-mailbox/': No such file or directory
/home/thanhsmind/Projects/goglbe/beehive/.bee/human-mailbox/:
drwxr-xr-x 1 thanhsmind thanhsmind   14 Aug 25 22:55 .
drwxr-xr-x 1 thanhsmind thanhsmind  932 Aug 30 18:59 ..
drwxr-xr-x 1 thanhsmind thanhsmind 1764 Aug 30 18:37 entries

/home/thanhsmind/Projects/goglbe/waggledance/.bee/human-mailbox/:
drwxr-xr-x 1 thanhsmind thanhsmind   14 Aug 26 15:37 .
drwxr-xr-x 1 thanhsmind thanhsmind  580 Aug 30 18:53 ..
drwxr-xr-x 1 thanhsmind thanhsmind  504 Aug 30 17:27 entries
```

No arming marker in any of the three registered checkouts, and **zero `.md` letters** in
any of them — only the `entries/` streams that accumulate whether or not a run is armed.
This also re-establishes leg (f) as the true state of the real machine: the empty page
above is what the user's inbox shows right now, and correctly so.

No daemon was started. `~/.waggledance` was neither read nor written. `~/.cargo/bin/waggledance`
was not replaced. The user's running daemon was left alone.

---

## What the whole run rests on

```
$ cargo test -p waggledance --bin waggledance server::bee_route_tests
test result: ok. 452 passed; 0 failed; 0 ignored; 0 measured; 620 filtered out; finished in 5.72s

$ cargo test -p waggledance-core --lib bee::mailbox
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.00s
```

Five tests are new in this cell and are the proof's own residue — they stay in the tree so
the shapes cannot regress silently:

- `waggledance-core` · `bee::mailbox::tests::the_letter_bee_actually_composed_parses`
- `waggledance` · `server::bee_route_tests::the_inbox_renders_the_letter_bee_actually_composed`
- `waggledance` · `server::bee_route_tests::a_real_letter_with_a_required_field_removed_renders_as_unreadable`
- `waggledance` · `server::bee_route_tests::opening_the_real_letter_shows_the_state_the_mark_command_set`
- `waggledance` · `server::bee_route_tests::the_zero_letter_inbox_renders_its_whole_explanation`

### What this proof does not cover

- **The mark button end to end against a real bee binary.** `bi-3`'s tests drive the POST
  route with a fake `bee` on `PATH` and assert the exact argv; leg (d) drives the real
  `bee mailbox mark` from a shell. The two meet at an argv string that both sides pin
  (`mailbox mark --id <name> --status <s> --json`) rather than at a single traced call.
- **A browser.** Every render here is server-produced HTML, asserted as text. No CSS was
  applied and no click was made.
- **A letter from a real unattended run of a real project.** The letter is genuinely bee's,
  from bee's real verbs, but the run behind it was a two-cell scratch run, not a night's
  work on this repo. What that leaves untested is scale — a letter with many items — not
  shape.
