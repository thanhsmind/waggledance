# dsr-5 — the whole path, proven against the real daemon

**Feature:** dispatch-submit-and-reclaim · **Cell:** `dsr-5` · **Run date:** 2026-08-30
**Decisions under proof:** D1 (close the pane on completion), D2 (completion is an explicit
declaration, never an inferred state), D4 (`agent.start` over the socket is asynchronous;
readiness is a separate `agent.wait` hop).

Every leg below carries the command that ran and the output it produced. Nothing here is a
summary of a run that happened somewhere else. The full unedited console log of the run is
`.bee/tmp/dispatch-submit-and-reclaim/run3.log` (untracked scratch); the harness is
`target/dsr5/proofrun.py` + `target/dsr5/mcpclient.py`.

## The binary under test

`CARGO_TARGET_DIR` on this host is redirected to `~/.cache/cargo-target`, so the in-repo
`target/fast/waggledance` path named by CLAUDE.md **does not exist**, and both the daemon
binary and the fresh one report `0.5.2` — the version string proves nothing. The path was
resolved from cargo's own `compiler-artifact` record:

```
$ cargo build --profile fast -p waggledance --message-format=json \
    | jq -r 'select(.reason=="compiler-artifact" and .executable) | .executable'
/home/thanhsmind/.cache/cargo-target/fast/waggledance

   Compiling waggledance v0.5.2 (…/waggledance--wt--dispatch-submit-and-reclaim/crates/waggledance)
    Finished `fast` profile [optimized] target(s) in 4.56s
```

```
$ stat -c '%n  size=%s  mtime=%y' /home/thanhsmind/.cache/cargo-target/fast/waggledance
/home/thanhsmind/.cache/cargo-target/fast/waggledance  size=22699064  mtime=2026-08-30 13:57:59.136067037 +0700

$ ls -la target/fast/waggledance
ls: cannot access 'target/fast/waggledance': No such file or directory
```

**Resolved binary:** `/home/thanhsmind/.cache/cargo-target/fast/waggledance`
**mtime:** `2026-08-30 13:57:59 +0700` — built from `6eaf80e` (dsr-7), the head of this branch.

### Nothing was installed, no daemon was restarted

A running MCP process keeps its old deleted-inode image, so the fresh binary was spawned as a
**child** `waggledance mcp` and spoken to over its stdin/stdout with MCP JSON-RPC, then torn
down. The user's installed daemon was read but never written:

```
$ stat -c '%n  size=%s  mtime=%y' ~/.cargo/bin/waggledance      # before the proof
/home/thanhsmind/.cargo/bin/waggledance  size=14365544  mtime=2026-08-30 07:13:29.303081691 +0700

$ stat -c '%n  size=%s  mtime=%y' ~/.cargo/bin/waggledance      # after the proof
/home/thanhsmind/.cargo/bin/waggledance  size=14365544  mtime=2026-08-30 07:13:29.303081691 +0700
```

```
child: /home/thanhsmind/.cache/cargo-target/fast/waggledance mcp  (pid 3007938)
…
child MCP process torn down (exit 0)
```

## The dispatch

One real spawn dispatch into project `beehive` (opted in), preset `claude-sonnet`, with a
trivial, self-contained, **non-mutating** task that ends by printing its done marker — so the
marker branch is what fires, not the stability fallback.

```
$ tools/call waggledance_dispatch {"project": "beehive", "preset": "claude-sonnet",
   "task": "This is an automated liveness probe for waggledance dispatch. Do not read, write,
    or modify any file. Do not run any command. Do not use any tool. Reply with exactly one
    line: DSR5-LIVE-PROOF-OK"}

{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [{"type": "text", "text": "dispatched run run-37b7972f700caa60"}],
    "structuredContent": {"run_id": "run-37b7972f700caa60", "warnings": []}
  }
}
dispatch took 4.5s
```

`dispatch` returned in **4.5 s** with a `run_id` — the `agent.wait` readiness hop added by
dsr-7 held: no `agent_not_ready`, which is exactly what killed attempt 1.

Row inserted by the dispatch:

```
{"id": "run-37b7972f700caa60", "project_id": "beehive", "pane_id": "w1:pN",
 "preset_label": "claude-sonnet", "status": "working",
 "marker": "HERDR_DONE_ca98f12aadb788e7", "baseline_len": 1212, "ft_len": null}
```

---

## Leg (a) — the task submitted with NO hand-sent Enter

Nobody ran `herdr pane send-keys`. The proof harness contains no such call — the only textual
match is the label of the leg itself:

```
$ grep -rn "send-keys\|send_keys" target/dsr5/proofrun.py target/dsr5/mcpclient.py
target/dsr5/proofrun.py:102:    leg("LEG (a) PANE TEXT while working — nobody ran `herdr pane send-keys`")
```

Pane text read straight off the live pane while the run was in flight:

```
$ herdr pane read w1:pN --source recent --lines 40 --format text     # t+8s
     session can resume cleanly, or record a capture stub for what settled (bee capture add) and close
     cleanly.

✻ Brewed for 2s · done 1:58 PM
…
                                                                                    ● high · /effort
──────────────────────────────────────────────────────────────────────────────────────────────────────
❯
──────────────────────────────────────────────────────────────────────────────────────────────────────
  /home/thanhsmind/Projects/goglbe/beehive | main | Sonnet 5 [high] | ctx: 94% | 5h: 25% | 7d: 61%
  sonnet-5 58k new/0 cached
  sonnet-5 $0.23 billed
  ⏵⏵ bypass permissions on (shift+tab to cycle) · ← for agents

pane w1:pN present in herdr pane list: True
```

The agent had already accepted the prompt, run its turn and billed tokens by t+8s
(`✻ Brewed for 2s · done 1:58 PM`, `sonnet-5 $0.23 billed`) — with no keystroke sent by hand.
Under the old blind-Enter path this is precisely where the text sat unsubmitted in the
composer forever. The full submitted text, as the pane rendered it, is preserved in the stored
transcript quoted under leg (b).

## Leg (b) — `done` with a non-empty `final_transcript`, via the marker branch

```
$ waggledance_await {"run_id": "run-37b7972f700caa60", "timeout_seconds": 60}
status=done  delta_len=3021
await #1 final status: done
```

Stored row, read directly out of `~/.waggledance/registry.db`:

```
$ sqlite3 (read-only) SELECT id, project_id, pane_id, preset_label, status, marker,
                             length(baseline), length(final_transcript) FROM runs WHERE id=…
{
  "id": "run-37b7972f700caa60",
  "project_id": "beehive",
  "pane_id": "w1:pN",
  "preset_label": "claude-sonnet",
  "status": "done",
  "marker": "HERDR_DONE_ca98f12aadb788e7",
  "baseline_len": 1212,
  "ft_len": 3021
}
```

`final_transcript` is **3021 bytes, non-empty**. Completion was `Declared`, not inferred:

```
run.marker                        = 'HERDR_DONE_ca98f12aadb788e7'
marker IN final_transcript        = True
marker IN baseline (must be False)= False
baseline length                   = 1212
```

The marker appears **exactly once** in the transcript and **zero times** in the baseline —
and the one occurrence is on the agent's own output line, not in the echoed prompt. The prompt
deliberately spells the marker in two halves so the echo can never satisfy the match:

```
$ (ANSI-stripped stored transcript, ±700 bytes around the marker)
❯ This is an automated liveness probe for waggledance dispatch. Do not read, write, or modify any
  file. Do not run any command. Do not use any tool. Reply with exactly one line: DSR5-LIVE-PROOF-OK

  When (and only when) the task above is fully complete, print the string "HERDR_DONE_" immediately
  followed by "ca98f12aadb788e7" -- concatenate the two with no space, no punctuation, and no line
  break between them -- on a line by itself.

● DSR5-LIVE-PROOF-OK

  HERDR_DONE_ca98f12aadb788e7

--- count in transcript: 1  count in baseline: 0
```

That is `orchestrate.rs`'s marker branch (`Completion::Declared`), the only producer of `Done`
that guard 1 lets close a pane — the 1.5 s stability fallback never entered it.

`waggledance_runs` agrees, and gives the wall clock: created `06:58:12.737Z`, terminal
`06:58:26.777Z` — 14 s end to end.

```
$ waggledance_runs {"project": "beehive"}
{
  "id": "run-37b7972f700caa60",
  "project_id": "beehive",
  "pane_id": "w1:pN",
  "preset_label": "claude-sonnet",
  "task": "This is an automated liveness probe for waggledance dispatch. …",
  "status": "done",
  "created_at": "2026-08-30T06:58:12.737476969Z",
  "updated_at": "2026-08-30T06:58:26.776961852Z"
}
```

## Leg (c) — `herdr pane list`: the pane is present while working, absent after

`herdr pane list` was run against the real daemon before the dispatch, twice during the run,
and again after completion. `w1:pN` is the pane the dispatch spawned, so it is correctly
absent from the "before" list.

```
$ herdr pane list                                        # before dispatch
pane ids before: ['w1:pE', 'w2:p1', 'w2:p2', 'w2:p4', 'w2:p5', 'w2:p6', 'w2:p7', 'w2:p8',
                  'w3:p1', 'w3:p2', 'w3:p3', 'w4:p1', 'w5:p1']

$ herdr pane list                                        # t+8s, run working
pane w1:pN present in herdr pane list: True

$ herdr pane list                                        # t+18s, run working
pane w1:pN present in herdr pane list: True

$ herdr pane list                                        # after status=done
pane ids after:  ['w1:pE', 'w2:p1', 'w2:p2', 'w2:p4', 'w2:p5', 'w2:p6', 'w2:p7', 'w2:p8',
                  'w3:p1', 'w3:p2', 'w3:p3', 'w4:p1', 'w5:p1']
run pane w1:pN present after completion: False
```

The before and after lists are **identical** — the pane waggledance created is the only pane
that appeared, and it is gone. That is `Herdr::close_pane` firing `pane.close` against the
real daemon: the wire shape is accepted, and the user's thirteen pre-existing panes, none of
which waggledance created, were untouched. Defect B closes on the live path.

## Leg (d) — a second await answers from the store, not from the closed pane

```
$ waggledance_await {"run_id": "run-37b7972f700caa60", "timeout_seconds": 10}
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [{"type": "text", "text": "run run-37b7972f700caa60: done"}],
    "structuredContent": {
      "run_id": "run-37b7972f700caa60",
      "status": "done",
      "delta": "\r\n… ▐ ▛███▛█   Claude Code v2.1.251\r\n ▝▜ █████ █▀  Sonnet 5 · Claude Max\r\n
                 ▝▝ ▝▝    ~/Projects/goglbe/beehive\r\n\r\n\r\n
                 ❯ This is an automated liveness probe for waggledance dispatch. …"
    }
  }
}
```

`w1:pN` no longer exists, and the await still answered `done` with the stored transcript
instead of propagating a `pane_not_found` — the terminal-status short-circuit from `dsr-3`
(P2-4) holds against a genuinely closed pane.

## Cleanup

The only pane this proof created is `w1:pN`, and the feature under test closed it (leg c). A
final `herdr pane list` after the run confirms the pane set is back to the thirteen panes that
existed before:

```
$ herdr pane list
['w1:pE', 'w2:p1', 'w2:p2', 'w2:p4', 'w2:p5', 'w2:p6', 'w2:p7', 'w2:p8',
 'w3:p1', 'w3:p2', 'w3:p3', 'w4:p1', 'w5:p1']
```

The child MCP process exited 0 when its stdin closed.

## Unit suite, same tree

```
$ cargo test -p waggledance -p waggledance-core
test result: ok. 1036 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.86s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 455 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Carried forward from the two blocked attempts

Both earlier attempts returned `[BLOCKED]` correctly and each bought a real fix — they are
part of this record, not noise:

- **Attempt 1** found that `agent.start` returns before the agent is promptable, so every
  spawn dispatch died with `agent_not_ready`.
- **Attempt 2** proved at the wire that `timeout_ms` was not the fix: `agent.start` over the
  socket is asynchronous (`launch_pending: true`, `agent_status: unknown`, returning in
  0.00 s), and `timeout_ms` is a startup timeout, not a readiness wait — recorded as D4.
- **dsr-7** (`6eaf80e`) added the measured remedy: an `agent.wait` hop in
  `start_declared_agent`, `until: [idle, working, done]`, `timeout_ms: 30000`. This run is
  that remedy working: dispatch returned a `run_id` in 4.5 s.

Also re-confirmed live by this run, having first been shown in the earlier attempts:
`agent.prompt`'s parameter shape is accepted by the real daemon and submits with no hand-sent
Enter; `pane.close`'s shape is accepted; the transcript stores; a re-await answers from the
store against a closed pane.

## One follow-up noticed, not fixed here

`cargo test` emits `warning: unused variable: 'before'` at
`crates/waggledance/src/herdr/fake.rs:1126` (dsr-4's test code). Harmless, outside this cell's
declared files, and left for the feature's owner.

`cargo fmt` was **not** run: it is not idempotent against this tree and reformats pre-existing
code in `guide.rs`, `views.rs`, `server.rs` and `orchestrate.rs`.
