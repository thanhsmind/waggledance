# Board Visibility — slice 1 proof against the three real projects

**Cell:** `bv-5` · **Run 1:** 2026-08-30, 15:27–15:41 (+07) · **Worker:** `w-bv-5`
**Run 2:** 2026-08-30, 15:55–16:01 (+07) · **Worker:** `w-bv-5b` · after `bv-6` (`385fa87`)
**Status: COMPLETE as of run 2.** Legs (a), (b), (c) are run 1's, proven below with
their actual commands and actual output, and untouched by run 2. Run 1's leg (d) was a
measured negative against the *card's* sentence and is kept verbatim as
"Leg (d) — NOT PRODUCED"; `bv-6` then moved the subject onto the surface that actually
renders, and run 2's "Leg (d) — PRODUCED" below is the measurement of that surface.

Everything here is measured against the LIVE machine: the real registry
(`~/.waggledance/registry.db`) and the real `.bee` stores of the three registered
projects. Nothing is a fixture.

## Method — how a render was obtained without touching the user's daemon

The user's daemon (pid 2110441, `~/.cargo/bin/waggledance serve`, port 7700) was left
running and untouched; nothing was installed over `~/.cargo/bin/waggledance`.

A temporary probe harness — `crates/waggledance/tests/e2e_bv5_live_home.rs`, written for
this cell and **deleted before the commit**, so it is not in the tree — followed
`e2e_open.rs`'s own established idiom: spawn a compiled binary with `--port 0` and a
scratch `HOME` under `/tmp`, wait for that daemon's own `daemon.lock`, then raw
`GET /` over a socket. The scratch `HOME` receives a **copy** of
`~/.waggledance/registry.db`, so the three project rows and their absolute roots are the
live ones while every write the probe daemon makes lands on the copy.

Both binaries were rendered **inside one test process, back to back**, with all three
`.bee/state.json` files read immediately before the first render and immediately after
the second, and asserted byte-identical across the pair — so no before/after difference
below can be an artefact of a store that moved. (It moves often: `waggledance`'s
`waiting_on` changed three times during this session's own measurements.)

## Trap 1 — the binary, resolved from cargo, not from a path

```
$ cargo build --profile fast -p waggledance --message-format=json | jq -r 'select(.reason=="compiler-artifact" and .executable!=null and .target.name=="waggledance") | .executable'
/home/thanhsmind/.cache/cargo-target/fast/waggledance
```

```
$ stat -c '%n mtime=%y size=%s' /home/thanhsmind/.cache/cargo-target/fast/waggledance
/home/thanhsmind/.cache/cargo-target/fast/waggledance  mtime=2026-08-30 15:39:23.787856418 +0700  size=22712416
$ sha256sum /home/thanhsmind/.cache/cargo-target/fast/waggledance
a91a325c001edfb983228bc4ae438e67c6fce4d9608b866c8d06103169eea99e  .../fast/waggledance
```

`CARGO_TARGET_DIR=/home/thanhsmind/.cache/cargo-target` is set in the environment, so the
in-repo path is stale and says nothing about it:

```
$ stat -c '%n mtime=%y' /home/thanhsmind/Projects/goglbe/waggledance/target/fast/waggledance
/home/thanhsmind/Projects/goglbe/waggledance/target/fast/waggledance mtime=2026-08-26 15:39:12.005420652 +0700
$ /home/thanhsmind/.cache/cargo-target/fast/waggledance --version   # fresh
waggledance 0.5.2
$ /home/thanhsmind/Projects/goglbe/waggledance/target/fast/waggledance --version   # 4 days old
waggledance 0.5.2
```

Four days apart, same version string. The version is not evidence; the resolved path and
its mtime are.

### Trap 1 has a second half, hit during this cell

The pre-bv-1 comparison binary was first built from a scratch source tree
(`git archive HEAD` + one reverted line) **while sharing the same `CARGO_TARGET_DIR`**.
Cargo uplifts both trees' binaries to the *same* `fast/waggledance` path and reuses the
same `deps/` slot for `waggledance-core`, so the next "rebuild HEAD" reported
`Finished in 0.11s` and left a binary that was byte-identical to the patched one:

```
$ cmp /home/thanhsmind/.cache/cargo-target/fast/waggledance <patched-tree build>
(no output — identical)
$ sha256sum both
6aca9cf0304258b8e2e92cebf939a3959b0cf6265cb61e58067430fca6ed7d00  (both files)
```

Two source trees, one target dir, one silently wrong binary — and cargo's own
`--message-format=json` reported the same `executable` path for both, so the JSON
resolution alone does not save you. The fix used here: a separate `CARGO_TARGET_DIR` for
the comparison tree. After that the two binaries differ, as they must:

| binary | source | sha256 | mtime |
| --- | --- | --- | --- |
| AFTER | HEAD `951f86b` | `a91a325c001edfb9…` | 2026-08-30 15:39:23 |
| BEFORE | HEAD with only bv-1's `kind != "turn-end"` reverted to `true` | `6aca9cf0304258b8…` | 2026-08-30 15:37:22 |

The BEFORE binary isolates bv-1 alone: it carries bv-2, bv-3 and bv-4 unchanged, so every
difference in the render below is bv-1's and nothing else's.

## Trap 2 — the configured test command is wrong for this repo

`.bee/config.json` records:

```json
"commands": { "test": "PATH=\"${CARGO_HOME:-$HOME/.cargo}/bin:$PATH\" cargo test --release --no-fail-fast --manifest-path packages/bee-rs/Cargo.toml" }
```

`packages/bee-rs/` is **beehive's** layout and does not exist in this repo:

```
$ cargo test --release --no-fail-fast --manifest-path packages/bee-rs/Cargo.toml
error: manifest path `/home/thanhsmind/Projects/goglbe/waggledance--wt--board-visibility/packages/bee-rs/Cargo.toml` does not exist
```

Filed as `p-c67e8eb5`. The command that actually proves this workspace, and the one this
cell ran:

```
$ cargo test -p waggledance -p waggledance-core
     Running unittests src/main.rs
test result: ok. 1041 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.90s
     Running tests/e2e_open.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/e2e_stop_stale_lock.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running unittests src/lib.rs (waggledance_core)
test result: ok. 463 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
   Doc-tests waggledance_core
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## The store, as the two renders saw it

Read by the probe process itself, immediately before and immediately after the pair, and
asserted unchanged across both:

```
BV5 store stable across BOTH renders [/home/thanhsmind/Projects/goglbe/waggledance] = true
BV5 store stable across BOTH renders [/home/thanhsmind/Projects/goglbe/beehive]     = true
BV5 store stable across BOTH renders [/home/thanhsmind/Projects/goglbe/jarvis]      = true
```

| project | `.bee/state.json` `feature` | `phase` | `waiting_on.kind` | `waiting_on.subject` |
| --- | --- | --- | --- | --- |
| waggledance | `todo-column-collapse` | `compounding` | `turn-end` | `Giờ đo chi phí thật mỗi vòng poll và viết vào báo cáo.` |
| beehive | `slp-dissent-stop-and-ask` | `compounding` | `turn-end` | `All six surfaces are quiet — no open/claimed/blocked cells, no reservations, no danger ops, no undecided big decisions. One new thing since` |
| jarvis | `harness-install-landing` | `exploring` | `question` | `Chon muc do tich hop Jarvis vao Super+Space tren Omarchy` |

## Leg (a) — projects reading as "waiting on you", BEFORE and AFTER bv-1

Command (one process, both binaries, the store snapshot above):

```
$ BV5_BIN_AFTER=.../waggledance-after \
  BV5_BIN_BEFORE=.../target-before/fast/waggledance \
  BV5_OUT_DIR=.../render \
  cargo test --profile fast -p waggledance --test e2e_bv5_live_home -- --nocapture
```

Actual output:

```
BV5 [after] binary=.../waggledance-after port=33609 bytes=211629
BV5 [before] binary=.../target-before/fast/waggledance port=40477 bytes=211783
BV5 [after]  rail wait pills (proj-row__badge--bee-wait) = 1
BV5 [after]  card reason lines (bee-hub__reason">)       = 0
BV5 [after]  'Waiting on you' occurrences                = 1
BV5 [after]  'need you' stat tile                        = 0
BV5 [before] rail wait pills (proj-row__badge--bee-wait) = 3
BV5 [before] card reason lines (bee-hub__reason">)       = 0
BV5 [before] 'Waiting on you' occurrences                = 3
BV5 [before] 'need you' stat tile                        = 0
```

**Projects reading as "waiting on you" on the home page: 3 of 3 BEFORE → 1 of 3 AFTER.**
The number goes down, and it goes down on the only project rows where it was false.

The entire difference between the two rendered pages is those two pills. Diff of the two
HTML bodies, tag-per-line, zero context:

```
--- before
+++ after
@@ -882 +881,0 @@
-<span class="proj-row__badge proj-row__badge--bee-wait">Waiting on you</span>
@@ -913 +911,0 @@
-<span class="proj-row__badge proj-row__badge--bee-wait">Waiting on you</span>
```

Six diff lines total. Nothing else on the page moved — no column count, no stat tile, no
card, no badge. bv-1 removed exactly two false claims and touched nothing else.

The `need you` stat tile reads `0` in **both** renders, because that tile counts the In
Progress group's waiting chip, not project marks — see Leg (d).

## Leg (b) — every mark whose classification changed, with its kind and subject

On the page, exactly two classifications changed, and both are the `state.json` marks of
the two projects that lost their pill:

| project | mark | recorded `kind` | recorded `subject` | BEFORE | AFTER |
| --- | --- | --- | --- | --- | --- |
| waggledance | `.bee/state.json` | `turn-end` | `Giờ đo chi phí thật mỗi vòng poll và viết vào báo cáo.` | live | not live |
| beehive | `.bee/state.json` | `turn-end` | `All six surfaces are quiet — no open/claimed/blocked cells, no reservations, no danger ops, no undecided big decisions. One new thing since` | live | not live |
| jarvis | `.bee/state.json` | `question` | `Chon muc do tich hop Jarvis vao Super+Space tren Omarchy` | live | live (unchanged) |

Both changed marks are `turn-end`. The one `question` mark survived. No `gate` and no
`question` mark anywhere lost its liveness.

Widening the check past the page to **every** `waiting_on` mark in the three stores
(`state.json` plus every `.bee/lanes/*.json`), under both predicates — note that
`waiting_on_live` is a field of `BeeState` only, so lane marks are carried here for
completeness and are not read by any board surface:

```
$ python3  # applies waiting_on_fields(), then the two predicates, ran 2026-08-30T15:40:30
waggledance: marks live BEFORE=20  live AFTER=10  changed=10
beehive:     marks live BEFORE=60  live AFTER=7   changed=53
jarvis:      marks live BEFORE=7   live AFTER=2   changed=5
TOTAL live BEFORE=87  live AFTER=19  changed=68
```

Every one of those 68 changed marks printed `kind=turn-end`; the enumeration prints the
kind on each line and no other value appeared. The 19 that stay live are `gate` and
`question` marks. Sample of the changed lines, verbatim from that run:

```
    state.json                             kind=turn-end  subject='Giờ đo chi phí thật mỗi vòng poll và viết vào báo cáo.'
    board-live-morph.json                  kind=turn-end  subject='Không còn gì chờ bạn.'
    herding-worker-standalone.json         kind=turn-end  subject='(turn ended)'
    tmux-herding-transport.json            kind=turn-end  subject='Pass received. I approve uat, close the feature, and clean up.'
    release-version-stamp.json             kind=turn-end  subject='Giờ chạy thử hai đoạn bash đó tại máy, không tin suông:'
```

`board-live-morph`'s subject is literally *"nothing is waiting on you"* — the mark the
board used to count as a demand, and the case D4 was written from.

## Leg (c) — the per-project rail, all three projects, beside the store

Rendered rail rows, extracted verbatim from `home-after.html`:

```
rail waggledance => <span class="proj-row__badge proj-row__badge--bee">todo-column-collapse · compounding</span>
rail beehive     => <span class="proj-row__badge proj-row__badge--bee">slp-dissent-stop-and-ask · compounding</span>
rail jarvis      => <span class="proj-row__badge proj-row__badge--bee">harness-install-landing · exploring</span><span class="proj-row__badge proj-row__badge--bee-wait">Waiting on you</span>
```

Each row's text against the store value it came from:

| project | rendered work pill | `state.json` `feature` | `lanes/<feature>.json` `phase` | rendered wait pill | `waiting_on.kind` |
| --- | --- | --- | --- | --- | --- |
| waggledance | `todo-column-collapse · compounding` | `todo-column-collapse` | `compounding` | *(none)* | `turn-end` → not live |
| beehive | `slp-dissent-stop-and-ask · compounding` | `slp-dissent-stop-and-ask` | `compounding` | *(none)* | `turn-end` → not live |
| jarvis | `harness-install-landing · exploring` | `harness-install-landing` | `exploring` | `Waiting on you` | `question` → live |

Every rendered token has its source in the store, and no row carries a token the store
does not hold. `state.json`'s own `phase` agrees with each lane's `phase` here, so this
run does not discriminate which of the two `proj_row_bee` read; it read the lane's first,
falling back to `state.phase`.

All three registered projects render a row — no project is silent, which was the ask.

## Leg (d) — NOT PRODUCED. The subject-vs-derived sentence has zero live instances.

The leg asked for one project whose waiting sentence uses a recorded subject and one that
falls back. **Neither exists**: the home page rendered **zero** waiting sentences, in
both binaries.

```
BV5 [after]  card reason lines (bee-hub__reason">) = 0
BV5 [before] card reason lines (bee-hub__reason">) = 0
$ grep -c 'bee-hub__reason' home-after.html
1        # the CSS rule `.bee-hub__reason { font-style: italic; }` — no rendered line
```

The three per-project pages were checked too (same probe, HEAD binary, 15:34), and are
also empty of it:

```
BV5 /p/waggledance/ status=200 bytes=34157 bee-hub__reason=0 'Waiting on you'=0
BV5 /p/beehive/     status=200 bytes=34121 bee-hub__reason=0 'Waiting on you'=0
BV5 /p/jarvis/      status=200 bytes=34112 bee-hub__reason=0 'Waiting on you'=0
```

Why, per project, from the rendered page and the store:

- **waggledance** — In Progress holds `board-visibility`, but `state.json`'s `feature` is
  `todo-column-collapse`, so `is_active` is false for that card; and this session is the
  one working it, so `working_now` suppresses the line regardless.
- **beehive** — no beehive feature is In Progress at all (In Progress count is 2:
  `board-visibility` and `harness-install-landing`), and its `state.json` mark is
  `turn-end`, so `live_wait` is `None` by bv-3's own `waiting_on_live` gate.
- **jarvis** — the one card that passes every gate bv-3 cares about: it IS the active
  feature, it IS In Progress, and its mark IS live. It still renders no sentence, because
  `reason` additionally needs `gate_stop.is_some()`, and this lane's gates are
  `{context: false, shape: true, execution: true, review: false, uat: false}` —
  `bee_gate_current_stop` starts scanning after the last approved gate (`execution`) and
  returns `("review", …)`, which the call site filters out by name. So `gate_stop` is
  `None` and the sentence is never built. The card shows only the danger chip
  `Awaiting approval`.

Two findings this leaves for the owner, both measured, neither mine to act on:

1. **bv-3's rule cannot be exercised by the data that motivated it.** The four
   `"AskUserQuestion"` subjects the plan cites live in `.bee/lanes/*.json` records
   (`dispatch-submit-and-reclaim`, `herdr-protocol-20`, `paseo-control` in waggledance;
   `slp-contract-original-request` in beehive), and `views.rs` reads the recorded subject
   **only** from `state.json`. The site can never see them.
2. **The only live subject that reaches the site would win, and is being withheld by a
   different gate.** jarvis's `"Chon muc do tich hop Jarvis vao Super+Space tren Omarchy"`
   is eight words and is not contained in any derived sentence, so
   `bee_hub_subject_beats_derived` would return `true` — but the `gate_stop` requirement
   upstream means the comparison is never reached. On this store, at this moment, bv-3 is
   dead code on every surface.

That is a measurement, not a defect verdict: whether a live `question` mark with no
non-review gate pending *should* produce a waiting sentence is a product question the
`review`-exclusion comment at `views.rs` already flags as deliberate.

## Leg (d) — PRODUCED (run 2, after bv-6). The rail pill names what the project waits for.

Run 1 measured the *card's* sentence and found zero instances of it on this store; that
section stands as written. `bv-6` (`385fa87`) moved the judgment onto the rail pill —
the element run 1 had just measured as rendering exactly once — calling `bv-3`'s
`bee_hub_subject_beats_derived` with the pill's own bare label as the derived side. This
leg measures that pill against the live registry.

### The probe binary — resolved from cargo, then proved to be the bv-6 one

```
$ cargo build --profile fast -p waggledance --message-format=json > .bee/tmp/bv5-legd/build.json
   Compiling waggledance v0.5.2 (/home/thanhsmind/Projects/goglbe/waggledance--wt--board-visibility/crates/waggledance)
    Finished `fast` profile [optimized] target(s) in 4.60s
$ jq -r 'select(.reason=="compiler-artifact" and .executable!=null and .target.name=="waggledance") | .executable' .bee/tmp/bv5-legd/build.json
/home/thanhsmind/.cache/cargo-target/fast/waggledance
$ stat -c '%n mtime=%y size=%s' /home/thanhsmind/.cache/cargo-target/fast/waggledance
/home/thanhsmind/.cache/cargo-target/fast/waggledance mtime=2026-08-30 15:55:35.814777374 +0700 size=22707272
$ sha256sum /home/thanhsmind/.cache/cargo-target/fast/waggledance
f2726fd9c8faefcf141151c0f8bc952aa2fb42895329f7c1b4d716b8e3a99d89  /home/thanhsmind/.cache/cargo-target/fast/waggledance
```

Trap 1's second half, restated because it applies here too: the uplift path
`fast/waggledance` is shared by every tree that uses this `CARGO_TARGET_DIR`, so neither
the path nor the sha says *which source* produced it. The decisive check is a string that
exists only after `bv-6` — the pill's own format literal — so the binary was copied out of
the target dir immediately after the build and grepped:

```
$ cp /home/thanhsmind/.cache/cargo-target/fast/waggledance .bee/tmp/bv5-legd/wd-head
$ sha256sum .bee/tmp/bv5-legd/wd-head
f2726fd9c8faefcf141151c0f8bc952aa2fb42895329f7c1b4d716b8e3a99d89  .bee/tmp/bv5-legd/wd-head
$ strings -n 8 .bee/tmp/bv5-legd/wd-head | grep -c 'proj-row__badge--bee-wait" title='
1
$ strings -n 8 /home/thanhsmind/.cargo/bin/waggledance | grep -c 'proj-row__badge--bee-wait" title='
0
```

| binary | sha256 | carries bv-6's pill literal |
| --- | --- | --- |
| probe (`.bee/tmp/bv5-legd/wd-head`) | `f2726fd9c8faefcf…` | yes — `1` |
| the user's running daemon `~/.cargo/bin/waggledance` | `ae770d4182587f7f…` | no — `0` |
| in-repo `target/fast/waggledance` (stale, unused) | `420d8646b4cea0fd…` | not used |

Three different shas, and the one binary that renders below is the only one carrying the
code under test. The user's daemon (pid 2110441, port 7700) was left running and
untouched; nothing was installed over `~/.cargo/bin/waggledance`.

### How the page was rendered

`.bee/spikes/board-visibility/bv5_legd.rs` — a std-only probe compiled with plain `rustc`,
no repo test file added. It sets `HOME` on the **child** process (the idiom
`crates/waggledance/tests/e2e_open.rs` already uses), spawns `serve --port 0`, polls that
daemon's own `daemon.lock` for the real bound port, then does a raw `GET /`. The scratch
`HOME` holds a `sqlite3 .backup` snapshot of the **live** registry, so the project rows and
their absolute roots are the real ones while every write lands on the copy:

```
$ sqlite3 file:/home/thanhsmind/.waggledance/registry.db?mode=ro ".backup '.../bv5-legd/home/.waggledance/registry.db'"
exit=0
$ sqlite3 .bee/tmp/bv5-legd/home/.waggledance/registry.db "select id,name,root_path from projects;"
waggledance|waggledance|/home/thanhsmind/Projects/goglbe/waggledance
jarvis|jarvis|/home/thanhsmind/Projects/goglbe/jarvis
beehive|beehive|/home/thanhsmind/Projects/goglbe/beehive
```

The three real stores were fingerprinted before the first render and again after the last,
and did not move across this whole leg:

```
$ md5sum .../waggledance/.bee/state.json .../beehive/.bee/state.json .../jarvis/.bee/state.json   # before
df9f7f2d861b5d14ea07d881f7bcd3d0  /home/thanhsmind/Projects/goglbe/waggledance/.bee/state.json
89be4fb942c13bd4645142aeaaca0872  /home/thanhsmind/Projects/goglbe/beehive/.bee/state.json
211eb85cd23dc2560a4e6fec11eb64fa  /home/thanhsmind/Projects/goglbe/jarvis/.bee/state.json
$ md5sum <same three>                                                                             # after
df9f7f2d861b5d14ea07d881f7bcd3d0  /home/thanhsmind/Projects/goglbe/waggledance/.bee/state.json
89be4fb942c13bd4645142aeaaca0872  /home/thanhsmind/Projects/goglbe/beehive/.bee/state.json
211eb85cd23dc2560a4e6fec11eb64fa  /home/thanhsmind/Projects/goglbe/jarvis/.bee/state.json
```

The store as this leg read it — note beehive's subject differs from run 1's, which is the
store moving between the two runs, not a transcription:

```
$ jq -c '{feature, phase, waiting_on}' /home/thanhsmind/Projects/goglbe/waggledance/.bee/state.json
{"feature":"todo-column-collapse","phase":"compounding","waiting_on":{"kind":"turn-end","subject":"Giờ đo chi phí thật mỗi vòng poll và viết vào báo cáo.","asked_at":"2026-08-30T08:33:31.324Z","session":"9b03b358-41a6-4720-a2d9-2e2bef89adb6"}}
$ jq -c '{feature, phase, waiting_on}' /home/thanhsmind/Projects/goglbe/beehive/.bee/state.json
{"feature":"slp-dissent-stop-and-ask","phase":"compounding","waiting_on":{"kind":"turn-end","subject":"All six surfaces are quiet again — nothing new that rises to a signal. Writing the silence record.","asked_at":"2026-08-30T08:45:03.060Z","session":"660592ec-adc9-4a17-93b2-c79e8fd280e5"}}
$ jq -c '{feature, phase, waiting_on}' /home/thanhsmind/Projects/goglbe/jarvis/.bee/state.json
{"feature":"harness-install-landing","phase":"exploring","waiting_on":{"kind":"question","subject":"Chon muc do tich hop Jarvis vao Super+Space tren Omarchy","asked_at":"2026-08-28T14:13:13.080Z","session":"d9842a15-bc20-4afe-bc40-e9cd7b2f4ced"}}
```

### Render 1 of this leg — the three live projects

```
$ .bee/tmp/bv5-legd/bv5_legd .bee/tmp/bv5-legd/wd-head <scratch-home> .bee/tmp/bv5-legd/home-live.html
BV5D binary=.bee/tmp/bv5-legd/wd-head port=41071 status=HTTP/1.1 200 OK bytes=211940 out=.bee/tmp/bv5-legd/home-live.html
```

Every wait pill on that page, verbatim:

```
$ grep -o '<span class="proj-row__badge proj-row__badge--bee-wait"[^§]\{0,220\}' .bee/tmp/bv5-legd/home-live.html
<span class="proj-row__badge proj-row__badge--bee-wait" title="Waiting on you — Chon muc do tich hop Jarvis vao Super+Space tren Omarchy">Waiting on you<span class="proj-row__badge-title">— Chon muc do tich hop Jarvis vao Super+Space…</span></span></div>
$ grep -o 'proj-row__badge--bee-wait' .bee/tmp/bv5-legd/home-live.html | wc -l
1
$ grep -o 'proj-row__badges--bee' .bee/tmp/bv5-legd/home-live.html | wc -l
3
```

Three rail rows, one wait pill — and that pill now says what it wants.

Each rail row, extracted by its project's own `/p/<slug>/` link:

```
rail waggledance => <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">todo-column-collapse · compounding</span></div>
rail beehive     => <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">slp-dissent-stop-and-ask · compounding</span></div>
rail jarvis      => <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">harness-install-landing · exploring</span><span class="proj-row__badge proj-row__badge--bee-wait" title="Waiting on you — Chon muc do tich hop Jarvis vao Super+Space tren Omarchy">Waiting on you<span class="proj-row__badge-title">— Chon muc do tich hop Jarvis vao Super+Space…</span></span></div>
```

**The winner, live: `jarvis`.** Its `state.json` records
`kind="question"`, `subject="Chon muc do tich hop Jarvis vao Super+Space tren Omarchy"`;
the pill's hover title carries that subject whole
(`title="Waiting on you — Chon muc do tich hop Jarvis vao Super+Space tren Omarchy"`) and
the visible text carries it clipped on a word boundary:

```
$ python3 -c "print(len('Chon muc do tich hop Jarvis vao Super+Space tren Omarchy'), len('Chon muc do tich hop Jarvis vao Super+Space'))"
56 43
```

56 characters recorded, budget 48, cut back to the last word boundary at 43 plus `…`, with
`tren Omarchy` recoverable from the title. Before `bv-6` this same row read `Waiting on
you` and nothing else.

### The loser: **no live project supplies one**, said plainly

`waggledance` and `beehive` both record `kind: "turn-end"`, which `bv-1` classifies as not
live, so their rows carry **no wait pill at all** — not a plain-label one. On this store, at
this moment, **there is no live project whose rail pill falls back to the bare label**, and
this proof claims none.

The subjects that *would* lose are real and they are in this store — they simply sit where
the site cannot read them (`.bee/lanes/*.json`, never `state.json`), which is run 1's
finding 1, unchanged:

```
$ grep -l 'AskUserQuestion' /home/thanhsmind/Projects/goglbe/waggledance/.bee/lanes/*.json
/home/thanhsmind/Projects/goglbe/waggledance/.bee/lanes/herdr-protocol-20.json
/home/thanhsmind/Projects/goglbe/waggledance/.bee/lanes/dispatch-submit-and-reclaim.json
/home/thanhsmind/Projects/goglbe/waggledance/.bee/lanes/paseo-control.json
$ jq -c '.waiting_on' /home/thanhsmind/Projects/goglbe/waggledance/.bee/lanes/paseo-control.json
{"kind":"gate","subject":"AskUserQuestion","asked_at":"2026-08-30T00:40:18.888Z","session":"4fca0305-e2f3-45e2-bf83-ade1a1e08ad8"}
```

So the losing branch is demonstrated the honest way instead of claimed: **that exact
recorded object**, copied verbatim, is placed where the site *does* read — a project's own
`state.json` — on a **synthetic project that is not a real project of this machine**
(`.bee/tmp/bv5-legd/losing-demo`, whose README says so), and rendered by the same binary,
in the same page, beside the live winner.

### Render 2 of this leg — the same page with the synthetic project registered

```
$ .bee/tmp/bv5-legd/bv5_legd .bee/tmp/bv5-legd/wd-head <scratch-home> .bee/tmp/bv5-legd/home-with-demo.html ".../bv5-legd/losing-demo=bv5-losing-demo"
BV5D register bv5-losing-demo status=exit status: 0 out=Registered 'bv5-losing-demo' (losing-demo) — 1 markdown files
  /home/thanhsmind/Projects/goglbe/waggledance--wt--board-visibility/.bee/tmp/bv5-legd/losing-demo
BV5D binary=.bee/tmp/bv5-legd/wd-head port=43631 status=HTTP/1.1 200 OK bytes=214687 out=.bee/tmp/bv5-legd/home-with-demo.html
```

Every rail row on that page, verbatim:

```
rail losing-demo =>
  <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">losing-subject-demo · executing</span><span class="proj-row__badge proj-row__badge--bee-wait">Waiting on you</span></div>

rail waggledance =>
  <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">todo-column-collapse · compounding</span></div>

rail beehive =>
  <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">slp-dissent-stop-and-ask · compounding</span></div>

rail jarvis =>
  <div class="proj-row__badges proj-row__badges--bee"><span class="proj-row__badge proj-row__badge--bee">harness-install-landing · exploring</span><span class="proj-row__badge proj-row__badge--bee-wait" title="Waiting on you — Chon muc do tich hop Jarvis vao Super+Space tren Omarchy">Waiting on you<span class="proj-row__badge-title">— Chon muc do tich hop Jarvis vao Super+Space…</span></span></div>

wait pills = 2
bee blocks = 4
```

Both branches, one render, one binary:

| project | live? | recorded `waiting_on` | rendered wait pill | branch |
| --- | --- | --- | --- | --- |
| `jarvis` | **live** | `question` · `"Chon muc do tich hop Jarvis vao Super+Space tren Omarchy"` | `Waiting on you — Chon muc do tich hop Jarvis vao Super+Space…` (full text in `title`) | subject **wins** |
| `bv5-losing-demo` | **synthetic, not a real project** | `gate` · `"AskUserQuestion"` (copied verbatim from waggledance's `lanes/paseo-control.json`) | `Waiting on you` — plain label, no `title`, no subject span | subject **refused** |
| `waggledance` | live | `turn-end` · `"Giờ đo chi phí thật mỗi vòng poll…"` | *(no pill at all)* | not live (`bv-1`) |
| `beehive` | live | `turn-end` · `"All six surfaces are quiet again…"` | *(no pill at all)* | not live (`bv-1`) |

The two pills differ in exactly the way the rule says they should: a sentence a human wrote
is named, a bare tool name is not, and neither is a finished turn.

### The same rule at the fixture level

```
$ cargo test -p waggledance a_rail_wait_pill_names_the_subject_only_when_it_beats_the_bare_label -- --nocapture
test views::tests::a_rail_wait_pill_names_the_subject_only_when_it_beats_the_bare_label ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1043 filtered out; finished in 0.00s
```

### The workspace scope, green at the end of this leg

```
$ cargo test -p waggledance -p waggledance-core
     Running unittests src/main.rs
test result: ok. 1043 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.90s
     Running tests/e2e_open.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
     Running tests/e2e_stop_stale_lock.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running unittests src/lib.rs (waggledance_core)
test result: ok. 463 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
   Doc-tests waggledance_core
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## What this proof does and does not establish

- Established: bv-1 lowers the count of projects reading as "waiting on you" from 3 to 1
  against the live store, changes only `turn-end` marks, and changes nothing else on the
  page; bv-4's rail renders correctly for all three registered projects against their
  store values.
- Not established: any behaviour of bv-3, which produced no rendered output on this store.
- Not covered: bv-2's field is exercised only through bv-3's site, so it is likewise
  unproven at the render layer here (its reader-level tests are green in the suite above).

**Amended by run 2** — the two "not established" lines above were true of run 1's tree and
are superseded, not deleted:

- Now established: bv-3's rule renders. Called from bv-6's rail pill, it names jarvis's
  recorded question on the live home page and refuses waggledance's real
  `"AskUserQuestion"` subject, both in one render from one binary.
- Now established: bv-2's `waiting_on` field reaches the render layer — the pill's text and
  hover title are read from it.
- Still not established: bv-3's *original* call site, the card's waiting sentence, which
  still renders zero times on this store for the `gate_stop` reason run 1 diagnosed. That
  condition is untouched by bv-6 and remains the owner's question.
- Still synthetic: the losing branch has no live instance on this store. Its demonstration
  uses a real recorded subject on a project that is not real, and is labelled as such
  everywhere it appears.
