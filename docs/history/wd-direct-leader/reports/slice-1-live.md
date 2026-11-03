# Slice 1 — the live walk

`green:live`. wd-bridge's `handoff` now goes through `waggledance_dispatch` with
`leader: true` and the corr as `request_id`, and a real leader's done report reached
wd's main session and its inbox row carries an `acked_at`.

Run on 2026-10-22 between 21:14 and 21:24 +07 on the machine that owns the store
(`/home/thanhsmind/.waggledance/registry.db`) and the OpenClaw gateway
(`openclaw-gateway`, pid 3828973).

## 1. What changed

`integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py`:

- `handoff` takes a required `--project`/`-p`.
- `hand_off()` calls `waggledance_dispatch` with `{project, task, leader: true,
  request_id: corr}` through the existing `call_mcp` helper.
- `call_mcp` takes an optional `extra_env`, and `hand_off` passes
  `WAGGLEDANCE_CALLER=wd`, so the daemon records `caller = "openclaw:wd"` and routes
  the run to a `waggledance.via=wd` leader (D1, slice-1 contract).
- It prints `corr` and `run_id` (plus `status`, `duplicate`, `warnings`), and exits
  non-zero with one line when waggledance refuses or returns no `run_id`.
- `--dry-run` prints the env-prefixed spawn and the dispatch request and runs nothing.
- `reply`, `redo` and `accept` still go through `send_to_supervisor` unchanged, as
  slice 1 requires.

`integrations/openclaw/waggledance-orchestrator/SKILL.md`:

- § 8 `handoff` says wd picks the project, asks the user when the words could mean
  more than one, and names the chosen project; documents the dispatch request and the
  printed `corr`/`run_id`; says a `queued` run id is a sent task, not finished work.
- The message shapes now describe the inbox frame
  `[waggledance#<corr> #<seq>] <project> — <kind label>: <text>`, keep the Vietnamese
  labels, add `xin quyền`, and say a repeated `#<seq>` for the same corr is a
  redelivery to skip.
- New step 5 under "Handling a report or a board" for `xin quyền`: name the project,
  tool and `req_id`, and never decide a permission on the user's behalf.
- "When `handoff` fails" now names waggledance only (not Paseo) and requires a printed
  `run_id` before saying a task was handed off. The old supervisor race note is
  replaced by the accepted per-project hold limit.

## 2. Build, and proving the binary by content

`CARGO_TARGET_DIR` on this host is `/home/thanhsmind/.cache/cargo-target`, shared with
the main checkout and any sibling session. The walk built into its own target dir
(`docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md`, disguise 4):

```
$ CARGO_TARGET_DIR=/home/thanhsmind/.cache/cargo-target-wdl5 \
    cargo build --profile fast -p waggledance --message-format=json
   Compiling waggledance-core v0.5.2 (/mnt/data/Projects/goglbe/waggledance--wt--wd-direct-leader/crates/waggledance-core)
   Compiling waggledance v0.5.2 (/mnt/data/Projects/goglbe/waggledance--wt--wd-direct-leader/crates/waggledance)
    Finished `fast` profile [optimized] target(s) in 52.43s

  compiler-artifact.executable ->
  /home/thanhsmind/.cache/cargo-target-wdl5/fast/waggledance
```

The literal had to be one only the new code contains — and it had to be checked
against the OLD artifact first, so the check itself was proven:

```
$ for lit in '--project beehive --task' 'def hand_off' 'WAGGLEDANCE_CALLER=wd'; do
    echo "== $lit"; echo -n "new: "; strings <new> | grep -c -- "$lit"
                           echo -n "old: "; strings <old> | grep -c -- "$lit"; done
== --project beehive --task
new: 4
old: 0
== def hand_off
new: 1
old: 0
== WAGGLEDANCE_CALLER=wd
new: 2
old: 0
```

The three literals come from the new `SKILL.md` and the new `hand_off` function; the
old binary scores `0` on every one, so the check discriminates.

Behaviour closed the loop as well — the old binary has no `mail` subcommand at all,
which is what "this daemon is running the slice-1 build" looks like from outside:

```
$ /home/thanhsmind/.cargo/bin/waggledance mail --help      # OLD
error: unrecognized subcommand 'mail'

$ /home/thanhsmind/.cache/cargo-target-wdl5/fast/waggledance mail --help   # NEW
Send one message to wd's inbox (leaders use this to report back).
Usage: waggledance mail <COMMAND>
Commands:
  send  Queue one message for wd; the body is read from --body-file or stdin.
```

Note that `--version` reports `0.5.2` for both, which is exactly why it was not used
as evidence.

## 3. Install and restart

```
$ cp <new> /home/thanhsmind/.cargo/bin/waggledance.wdl5.tmp \
  && chmod 755 … && mv … /home/thanhsmind/.cargo/bin/waggledance
installed
$ sha256sum <new> /home/thanhsmind/.cargo/bin/waggledance | awk '{print $1}' | uniq -c
      2 93ecd05bb6aaca8f8bb58d629ccc68256daabe3d1ee90d6e5b8c245b5ba18fa2
```

The copy is byte-identical, and `~/.local/bin/waggledance` is a symlink to
`~/.cargo/bin/waggledance`, so neither install path shadows the other.

```
$ waggledance restart
Stopped daemon (pid 140879).
Started daemon (pid 1554601) at https://waggle.gogl.be

$ sha256sum $(readlink -f /proc/1554601/exe)
93ecd05bb6aaca8f8bb58d629ccc68256daabe3d1ee90d6e5b8c245b5ba18fa2  /home/thanhsmind/.cargo/bin/waggledance
```

The running process resolves to the artifact that was built and checked — proven from
`/proc/<pid>/exe`, not from the restart command's own output.

Starting the new daemon migrated the store to the slice-1 schema and created the
inbox table, which is the first proof that the daemon on the wire really is the new one:

```
$ sqlite3 ~/.waggledance/registry.db "PRAGMA user_version; select name from sqlite_master where name='inbox';"
7
inbox
```

## 4. A scratch project and a hand-off

```
$ waggledance register /tmp/wdl5-live --name wdl5-live --json
{"project_id":"wdl5-live","name":"wdl5-live","root_path":"/tmp/wdl5-live","file_count":1,...}
```

`/tmp/wdl5-live/.waggledance.toml` holds `orchestration = true` (the opt-in the
dispatch path requires) and `[paseo] mode = "bypassPermissions"`, so the leader can
finish the trivial task without stopping for a permission — slice 2 is where wd answers
permissions, and the permission path was already walked separately in § 6.

The hand-off itself, from the worktree, through the edited bridge:

```
$ python3 integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py \
    handoff --project wdl5-live \
    --task "Create NOTES.md in this project root with one line: slice-1 live walk ok. \
            Then mail wd the done report with Done:, Proof:, Left:, Where: parts."
{"corr": "t-69838f", "run_id": "run-195e88cfa5951fcc", "project": "wdl5-live",
 "status": "queued", "duplicate": false, "warnings": []}
```

Truth 1 holds: `handoff` returns a corr and a run id. The whole call took 0.16 s — it
queues, it does not wait.

The daemon stored the run against the wd caller, with the corr as the request id and the
leader prompt prefix the slice-1 contract gives:

```
$ sqlite3 ~/.waggledance/registry.db \
    "select id,project_id,status,caller,request_id,substr(task,1,50) from runs \
     where id='run-195e88cfa5951fcc';"
run-195e88cfa5951fcc|wdl5-live|working|openclaw:wd|t-69838f|Task waggledance#t-69838f from wd.
```

(`status` was `working` at the moment of that read; it reached `done` at
`2026-10-22T14:23:18Z`.)

## 5. The leader's result reaches wd, and the row is acked

```
$ cat /tmp/wdl5-live/NOTES.md
slice-1 live walk ok

$ sqlite3 ~/.waggledance/registry.db "select body from inbox where seq=2;"
Done: Created NOTES.md in project root with the single line "slice-1 live walk ok".
Proof: `cat /tmp/wdl5-live/NOTES.md` prints exactly "slice-1 live walk ok" (1 line).
Left: Nothing.
Where: /tmp/wdl5-live/NOTES.md
```

The leader wrote the file, validated its own four-part report through
`waggledance mail send` (a `result` missing a part exits 2 — the row exists, so the
grammar passed), and the daemon woke wd's main session with it. The frame as it arrived:

```
[waggledance#t-69838f #2] wdl5-live — kết quả: Done: Created NOTES.md in project root with the
single line "slice-1 live walk ok".
```

That line is in wd's own session transcript,
`/home/thanhsmind/aicoworker/openclaw/agents/wd/sessions/fb87fe74-2bc0-4899-aef9-49469d8185f7.jsonl`.

The inbox row, with its `acked_at`:

```
$ sqlite3 ~/.waggledance/registry.db \
    "select seq,corr,kind,substr(created_at,12,8) cr,substr(acked_at,12,8) ack,attempts,dead_at from inbox order by seq;"
1|t-fd12f9|permission|14:20:36|14:21:47|0|
2|t-69838f|result    |14:23:13|14:24:26|0|
```

Truth 2 holds: the row for the result (seq 2, corr `t-69838f`) has
`acked_at = 2026-10-22T14:24:26.302324334Z`, `attempts = 0`, `dead_at` null. The ack
landed ~73 s after the mail and ~68 s after the leader's run finished, which is the
wake loop's own pacing, not a manual step.

## 6. The permission row, walked on the way

The first scratch project (`wdl5-scratch`, no `bypassPermissions`) produced the
`permission` message the slice-1 contract requires from `delivery.rs` on a blocked wd
run — a live sighting of wdl-2's daemon-written message, not a unit test:

```
1|t-fd12f9|permission|wdl5-scratch leader 4712bb6e-7ef5-4f8d-accb-fa8a536f012f is waiting for a
  permission on run run-12e19c4a69100160: permission-9bb191a7-c8f6-4d23-9a1b-70c8e4c38bfb (Bash)||0|
```

It was framed with the new `xin quyền` label and acked at 14:21:47:

```
[waggledance#t-fd12f9 #1] wdl5-scratch — xin quyền: wdl5-scratch leader 4712bb6e-… is waiting
for a permission on run run-12e19c4a69100160: permission-9bb191a7-… (Bash)
```

Answering that permission is wd's `permit` path, which is slice 2 (D9). For the result
walk the scratch project was therefore given `bypassPermissions`, so the leader would
not block on a tool the walk cannot yet allow. That is a limit of slice 1, not a defect
in it.

## 7. Tests

`setup.rs` parity is the guarding test for both edited files — `SKILL.md` and
`bin/wd-bridge.py` are `include_str!`-embedded, so a mismatch between the repo files and
the embedded copies reds the suite:

```
$ CARGO_TARGET_DIR=/home/thanhsmind/.cache/cargo-target-wdl5 cargo test --profile fast -p waggledance setup::
test setup::tests::embedded_tables_match_the_integration_sources ... ok
test setup::tests::embedded_openclaw_files_equal_the_repo_files ... ok
test setup::tests::openclaw_install_writes_the_skill_under_the_resolved_workspace ... ok
test result: ok. 19 passed; 0 failed
```

The whole crate, so nothing beside the edited pair moved:

```
$ CARGO_TARGET_DIR=/home/thanhsmind/.cache/cargo-target-wdl5 cargo test --profile fast -p waggledance
test result: ok. 1273 passed; 0 failed; 2 ignored
test result: ok. 3 passed; 0 failed
test result: ok. 5 passed; 0 failed
```

`python3 -m py_compile bin/wd-bridge.py` compiles clean, and
`handoff --project wdl5-live --task probe --dry-run` prints the env-prefixed spawn and
the dispatch request without running anything.

## What the walk does not cover

- `reply`, `redo`, `accept` are untouched and still reach the Paseo supervisor — by
  design, slice 2 owns them (D10).
- Answering a permission through wd is slice 2 (D9). The permission message itself is
  live (seq 1), the answer path is not.
- The 10-minute heartbeat and the board are slice 2 (D8).
- The per-project hold is shared: a leader busy with another caller's task parks a wd
  task on the same project until it is idle. Accepted in the plan's out-of-scope list.
- The scratch projects (`/tmp/wdl5-live`, `/tmp/wdl5-scratch`) are still registered in
  the real store. They are harmless but are not part of the feature; unregister them
  when the slice is closed.
