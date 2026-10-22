# Slice 2 — the live walk

`green:live`. wd's permission relay, `reply`, `accept` and the heartbeat were driven
against the running daemon on this host, with a real Paseo leader and a real wd session.

Run on 2026-10-22 between 21:54 and 22:37 +07 on the machine that owns the store
(`/home/thanhsmind/.waggledance/registry.db`) and the OpenClaw gateway
(`openclaw-gateway`, pid 3828973).

## 0. What changed in the repo this round

Nothing. This cell is a walk, not an edit: the only file it writes is this report.
The slices' code (`heartbeat.rs`, `waggledance_task`, migration 8) was already on the
branch (`4d95a8a`, `c4ab2cd`, `7f45e8e`) and this round proves it live.

## 1. Build, and proving the binary by content

`CARGO_TARGET_DIR` on this host is `/home/thanhsmind/.cache/cargo-target`, shared with
the main checkout and any sibling session, so the walk built into its own target dir:

```
$ CARGO_TARGET_DIR=/home/thanhsmind/.cache/cargo-target-wdl9 \
    cargo build --profile fast -p waggledance --message-format=json
   Compiling waggledance-core v0.5.2 (/mnt/data/Projects/goglbe/waggledance--wt--wd-direct-leader/crates/waggledance-core)
   Compiling waggledance v0.5.2 (/mnt/data/Projects/goglbe/waggledance--wt--wd-direct-leader/crates/waggledance)
    Finished `fast` profile [optimized] target(s) in 46.21s

  compiler-artifact.executable -> /home/thanhsmind/.cache/cargo-target-wdl9/fast/waggledance
```

The literals were chosen from slice-2-only code and scored against the OLD installed
binary first, so the check itself discriminates:

```
$ for lit in 'you are idle and wd has no report on this task' 'permission_requests' \
             'waggledance_task' 'progress_sent_at'; do
    echo -n "new: $(strings <new> | grep -c -- "$lit")  old: $(strings <old> | grep -c -- "$lit")  $lit"; done
new: 1   old: 0   you are idle and wd has no report on this task
new: 12  old: 0   permission_requests
new: 3   old: 0   waggledance_task
new: 4   old: 0   progress_sent_at
```

`you are idle and wd has no report on this task` is the heartbeat nudge body
(`heartbeat.rs::nudge_text`); `waggledance_task` is the slice-2 MCP tool;
`permission_requests` and `progress_sent_at` are migration-8 columns. The old binary
scores `0` on every one.

Shas, for the record (the version string `0.5.2` is identical on both and was not used
as evidence):

```
23543ac2b57785650f8dd31c29b17b2ef5098f7be6550a6f5a0ef6bb86a5eae1  new
93ecd05bb6aaca8f8bb58d629ccc68256daabe3d1ee90d6e5b8c245b5ba18fa2  old (slice-1 install)
```

## 2. Install and restart

```
$ cp <new> /home/thanhsmind/.cargo/bin/waggledance.wdl9.tmp \
  && chmod 755 … && mv … /home/thanhsmind/.cargo/bin/waggledance
installed
$ sha256sum <new> /home/thanhsmind/.cargo/bin/waggledance | awk '{print $1}' | uniq -c
      2 23543ac2b57785650f8dd31c29b17b2ef5098f7be6550a6f5a0ef6bb86a5eae1
$ ls -l /home/thanhsmind/.local/bin/waggledance
lrwxrwxrwx … /home/thanhsmind/.local/bin/waggledance -> /home/thanhsmind/.cargo/bin/waggledance
```

`~/.local/bin/waggledance` is a symlink to the installed copy, so neither path shadows
the other.

```
$ waggledance restart
Stopped daemon (pid 1554601).
Started daemon (pid 1891743) at https://waggle.gogl.be

$ sha256sum $(readlink -f /proc/1891743/exe)
23543ac2b57785650f8dd31c29b17b2ef5098f7be6550a6f5a0ef6bb86a5eae1  /home/thanhsmind/.cargo/bin/waggledance
```

Proven from `/proc/<pid>/exe`, not from the restart output. (The first `readlink` attempt
resolved the pid of the shell that ran the pipeline — `/usr/bin/bash` — a reminder that
`pgrep -f` matches your own command line; the pid was taken from the restart output
instead.)

The new daemon migrated the store from `user_version` 7 to 8 and created both tables —
the first proof that the daemon on the wire is the slice-2 build:

```
$ sqlite3 ~/.waggledance/registry.db "PRAGMA user_version; select name from sqlite_master where name in ('tasks','permission_requests');"
8
tasks
permission_requests
```

## 3. A deviation, named: the slice-1 scratch project was wedged

The cell names `/tmp/wdl5-scratch`. It could not be used as handed, and the reason is
worth recording.

The slice-1 walk left a leader on `wdl5-scratch` (`4712bb6e-…`) stopped on an unanswered
permission. The permission was never answered (answering it is slice 2), so Paseo still
reported the agent as `running`, and `Delivery::leader_idle` therefore never released the
project's hold:

```
$ sqlite3 ~/.waggledance/registry.db "select id,status,pane_id from runs where project_id='wdl5-scratch';"
run-12e19c4a69100160|lost|paseo:4712bb6e-7ef5-4f8d-accb-fa8a536f012f
run-20c954f7e11ddb89|queued|paseo:pending            # <- a fresh hand-off, stuck queued
$ ask_state wdl5-scratch → leaders[0].status = "running", created 47 minutes ago
```

A hand-off to `wdl5-scratch` at 14:59 stayed `queued` for five minutes because the
project was held by that leader. Two responses were possible: answer the stale
permission first, or walk on a fresh scratch project. The first would mean answering a
permission without the recorded `wd-bridge permit` command the cell's prohibition
forbids — the daemon never answers one — so the walk used a fresh scratch project,
`/tmp/wdl9-scratch` (same shape: `orchestration = true`, no `bypassPermissions`).
The stale task was closed later with a real `accept` (§4.5), which archived its leader
and released the project.

That is a named deviation from "using the scratch project /tmp/wdl5-scratch", not a
silent one: the substituted project is identical in every property the cell's task
depends on (orchestration opt-in, no bypass), and every prohibition still holds.

## 4. The permission relay, walked end to end

### 4.1 The hand-off

```
$ python3 integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py \
    handoff --project wdl9-scratch \
    --task "Create PERM.md in this project root with one line: slice-2 permission walk ok. \
            Use a shell command to write it. Then report back to wd with \
            \`waggledance mail send --to wd --corr <your corr> --kind result\` …"
{"corr": "t-e5f0db", "run_id": "run-cb3965b471f9d7bc", "project": "wdl9-scratch",
 "status": "queued", "duplicate": false, "warnings": []}
```

The task row was inserted by the hand-off writer:

```
$ sqlite3 ~/.waggledance/registry.db "select corr,project_id,leader_id,state from tasks;"
t-e5f0db|wdl9-scratch||open
```

### 4.2 Delivery blocks, and the watcher mails wd exactly once

The leader reached the permission within seconds; delivery marked the run `blocked` and
the watcher (`paseo permit ls --json`, every 30 s) wrote one `permission` message:

```
$ sqlite3 ~/.waggledance/registry.db \
   "select seq,corr,kind,created_at,acked_at,body from inbox where corr='t-e5f0db';"
5|t-e5f0db|permission|2026-10-22T15:09:35…|2026-10-22T15:10:44…|wdl9-scratch leader
  90f7def2-ed1b-481a-bd6b-2af5e3912d0e asks to use Bash: - — req_id
  permission-c84463ed-1eed-4ab2-ba86-27d82d18180b; answer with permit allow|deny
  t-e5f0db permission-c84463ed-1eed-4ab2-ba86-27d82d18180b

$ sqlite3 ~/.waggledance/registry.db "select req_id,corr,tool,seen_at,answered_at from permission_requests;"
permission-c84463ed-1eed-4ab2-ba86-27d82d18180b|t-e5f0db|Bash|2026-10-22T15:09:35…|
```

One message, one request row. Nothing else appeared for that request across the beats
until it was answered — `blocked_then_done` no longer writes a second copy (slice 2
removed `mail_permissions`), so the watcher really is the one writer.

The frame as it landed in wd's own session
(`…/agents/wd/sessions/fb87fe74-…jsonl`):

```
[waggledance#t-e5f0db #5] wdl9-scratch — xin quyền: wdl9-scratch leader 90f7def2-… asks to
use Bash: - — req_id permission-c84463ed-…; answer with permit allow|deny t-e5f0db permission-c84463ed-…
```

`acked_at` at 15:10:44 is the wake's own ack, not a manual step.

### 4.3 wd answers with a recorded `permit` command

```
$ python3 integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py permit allow t-e5f0db
{"corr": "t-e5f0db", "action": "allow",
 "text": "allow sent for request permission-c84463ed-1eed-4ab2-ba86-27d82d18180b of leader
          90f7def2-ed1b-481a-bd6b-2af5e3912d0e (task t-e5f0db)",
 "req_id": "permission-c84463ed-1eed-4ab2-ba86-27d82d18180b", "answer": "allow", "warnings": []}

$ sqlite3 ~/.waggledance/registry.db "select req_id,answered_at from permission_requests;"
permission-c84463ed-1eed-4ab2-ba86-27d82d18180b|2026-10-22T15:11:08…
```

The bridge called `waggledance_task {corr, action: allow}` under
`WAGGLEDANCE_CALLER=wd`, which ran `paseo permit allow <leader> <req_id>` — the cell's
key link. The daemon itself decided nothing: it relayed the user's answer.

### 4.4 The leader continues, and its result reaches wd

Truth 1 ("a permission answered through wd unblocks the leader") holds:

```
$ cat /tmp/wdl9-scratch/PERM.md
slice-2 permission walk ok

$ sqlite3 ~/.waggledance/registry.db "select id,status from runs where request_id='t-e5f0db';"
run-cb3965b471f9d7bc|done

$ sqlite3 ~/.waggledance/registry.db "select seq,kind,created_at,body from inbox where seq=7;"
7|result|2026-10-22T15:12:02…|Done: Created PERM.md in project root with one line "slice-2
  permission walk ok", written via shell (echo redirect).
  Proof: `cat PERM.md` -> "slice-2 permission walk ok"; `ls -la PERM.md` -> -rw-r--r-- 27
  bytes, Oct 22 22:11.
  Left: Nothing.
  Where: /tmp/wdl9-scratch/PERM.md
```

The wake carried it as `[waggledance#t-e5f0db #7] wdl9-scratch — kết quả: Done: Created
PERM.md …`.

A second permission (`permission-0784b96d-…`, seq 6) appeared a minute later and vanished
unanswered; the watcher closed it and mailed one `closed` message (seq 8) at 15:12:05 —
the "vanished unanswered" rule, seen live rather than only in a test. A third, seq 10,
belonged to the held `wdl5-scratch` task and was closed with it.

### 4.5 `reply` on the same corr, and the `corr:1` run

```
$ python3 …/wd-bridge.py reply --corr t-e5f0db --text "Cảm ơn. Hãy thêm một dòng thứ hai vào PERM.md: reply walk ok."
{"corr": "t-e5f0db", "action": "reply",
 "text": "queued run run-15baff2ed5c9c7e4 for project wdl9-scratch's Paseo leader",
 "run_id": "run-15baff2ed5c9c7e4", "status": "queued", "duplicate": false, "warnings": []}

$ sqlite3 ~/.waggledance/registry.db \
   "select id,status,request_id,substr(task,1,40) from runs where request_id like 't-e5f0db%';"
run-cb3965b471f9d7bc|done|t-e5f0db   |Task waggledance#t-e5f0db from wd.
run-15baff2ed5c9c7e4|queued|t-e5f0db:1|Task waggledance#t-e5f0db from wd.
```

`request_id` is `t-e5f0db:1` and the run carries the same `wd_task` prefix, so
`wd_corr` still resolves the corr. That run blocked on a permission of its own, which was
answered the same way:

```
$ python3 …/wd-bridge.py permit allow t-e5f0db
{"corr": "t-e5f0db", "action": "allow", "text": "allow sent for request
 permission-4890c4db-aff9-4648-a63f-eb0dde22de99 of leader 90f7def2-… (task t-e5f0db)", …}
```

and finished:

```
$ sqlite3 ~/.waggledance/registry.db "select id,status from runs where request_id='t-e5f0db:1';"
run-15baff2ed5c9c7e4|done

$ cat /tmp/wdl9-scratch/PERM.md
slice-2 permission walk ok
reply walk ok

$ sqlite3 ~/.waggledance/registry.db "select seq,kind,created_at,body from inbox where seq=18;"
18|result|2026-10-22T15:16:06…|Done: Appended second line "reply walk ok" to PERM.md via shell (echo >>).
  Proof: `cat -n PERM.md` -> 1 "slice-2 permission walk ok", 2 "reply walk ok".
  Left: Nothing.  Where: /tmp/wdl9-scratch/PERM.md
```

The reply run's `mail send` — a leader-side write on a `corr:1` run — resolved the corr
to the task, which is the corr-lookup rule holding on the follow-up path.

### 4.6 `accept` closes the task and archives the leader

```
$ python3 …/wd-bridge.py accept --corr t-e5f0db
{"corr": "t-e5f0db", "action": "accept", "text": "task t-e5f0db closed; leader
 90f7def2-ed1b-481a-bd6b-2af5e3912d0e archived", "closed": true, "archived": true}

$ sqlite3 ~/.waggledance/registry.db "select corr,state,changed_at from tasks where corr='t-e5f0db';"
t-e5f0db|closed|2026-10-22T15:17:39…
```

Truth 2 ("accept archives the leader") holds, proven from Paseo's own agent record and
from the process table, not only from the tool's own words:

```
$ python3 -c "print(json.load(open('…/agents/tmp-wdl9-scratch/90f7def2-….json'))['archivedAt'])"
2026-10-22T15:17:40.228Z
$ ask_state wdl9-scratch → leaders count 0
$ ps … (no claude process with cwd /tmp/wdl9-scratch)
```

The stale slice-1 task was closed the same way, which is what released `wdl5-scratch`:

```
$ python3 …/wd-bridge.py accept --corr t-c99048
{"corr": "t-c99048", "action": "accept", "text": "task t-c99048 closed; leader
 4712bb6e-7ef5-4f8d-accb-fa8a536f012f archived", "closed": true, "archived": true}
```

## 5. The heartbeat: one nudge, then one progress

`[openclaw] heartbeat_secs` was set to `60` temporarily and the daemon restarted.

A hand-off whose leader would finish without mailing a result is the case the heartbeat
exists for:

```
$ python3 …/wd-bridge.py handoff --project wdl9-hb --task "… Reply with the single word
  DONE and stop. For this task only, do NOT run waggledance mail send."
{"corr": "t-d71865", "run_id": "run-7211512e04a568cb", "project": "wdl9-hb",
 "status": "queued", "duplicate": false, "warnings": []}
```

The leader finished and stayed silent. One beat later:

```
$ sqlite3 ~/.waggledance/registry.db \
   "select corr,leader_id,last_delivered_at,nudged_at,progress_sent_at,board_sent_at from tasks;"
t-d71865|33a0c12a-c723-4c80-b8b7-8f2f0e5226bf|2026-10-22T15:29:25.667…|
  2026-10-22T15:29:53.384…|2026-10-22T15:30:52.204…|2026-10-22T15:29:53.475…
```

`nudged_at` 15:29:53, then `progress_sent_at` 15:30:52 — 59 s later, one beat, exactly as
D8 orders it. The nudge really reached the leader (a `user`-role message carrying the
nudge text in that leader's transcript, counted once):

```
$ # d2c1876b-…jsonl (the t-d71865 leader's session)
nudge user messages: 1
```

The two messages the heartbeat wrote:

```
$ sqlite3 ~/.waggledance/registry.db "select seq,corr,kind,created_at,acked_at,body from inbox where seq>=21;"
21|board   |board   |2026-10-22T15:29:53.411…|2026-10-22T15:30:56.072…|t-d71865 wdl9-hb idle
22|t-d71865|progress|2026-10-22T15:30:52.143…|2026-10-22T15:32:03.461…|wdl9-hb leader
   33a0c12a-… was nudged at 2026-10-22T15:29:53.384… and has still sent no report; its
   status is idle
```

Both frames reached wd's session, the board with its own instruction line ("Danh sách
các task đang mở…") rather than as a report on one task:

```
[waggledance#board #21] all — bảng việc: t-d71865 wdl9-hb idle
[waggledance#t-d71865 #22] wdl9-hb — tiến độ: wdl9-hb leader 33a0c12a-… was nudged at
  2026-10-22T15:29:53.384… and has still sent no report; its status is idle
```

Truth 3 ("the heartbeat nudges then reports progress") holds.

The "once" half matters as much: after the progress, four more beats ran with no new row
and no stamp movement —

```
$ sqlite3 … "select seq,kind,created_at from inbox where seq>=21 order by seq;"   # at 15:33
21|board|2026-10-22T15:29:53…
22|progress|2026-10-22T15:30:52…
$ sqlite3 … "select nudged_at,progress_sent_at from tasks where corr='t-d71865';"
2026-10-22T15:29:53.384…|2026-10-22T15:30:52.204…
```

— and the nudge appeared exactly once in the leader's transcript. No board went out on
those beats, because nothing had changed since `board_sent_at`.

### An earlier pass of the same walk

A first heartbeat pass ran on `/tmp/wdl9-scratch` (corr `t-652f56`) before the project was
unregistered and reached the same state — `nudged_at` 15:19:20, one `progress` at
15:20:19 (seq 21), one board (seq 20), and no repeat across three further beats. It is
recorded as corroboration, but the walk above is the one quoted: its frames were read in
wd's session *before* the project was unregistered, so nothing about it depends on a
later reconstruction.

## 6. Restore, and cleanup

`heartbeat_secs` restored to its pre-walk value, and the whole config file proven
byte-identical to the backup taken before the walk (the temporary `heartbeat_secs = 60`
line is gone, not left in place):

```
$ cp /tmp/wdl9-config.bak ~/.waggledance/config.toml
$ diff /tmp/wdl9-config.bak ~/.waggledance/config.toml && echo 'config byte-identical to pre-walk'
config byte-identical to pre-walk

$ waggledance restart
Stopped daemon (pid 2222085).
Started daemon (pid 2279698) at https://waggle.gogl.be
$ sha256sum $(readlink -f /proc/2279698/exe)
23543ac2b57785650f8dd31c29b17b2ef5098f7be6550a6f5a0ef6bb86a5eae1  /home/thanhsmind/.cargo/bin/waggledance

$ curl -s http://127.0.0.1:7700/api/config | …
heartbeat_secs = 600
```

The running daemon reports `600` — the restore is in force, not just written to a file.
(The config has no `heartbeat_secs` key at all now; `600` is the `OpenClawConfig::default`
the loader falls back to, which is the same value the file carried before the walk.)

All four scratch projects unregistered — the two the cell names and the two this walk
created:

```
$ waggledance unregister wdl9-hb
Unregistered 'wdl9-hb'.
$ waggledance unregister wdl5-live
Unregistered 'wdl5-live'.
$ waggledance unregister wdl5-scratch
Unregistered 'wdl5-scratch'.
$ waggledance unregister wdl9-scratch
Unregistered 'wdl9-scratch'.

$ sqlite3 ~/.waggledance/registry.db "select id from projects where id like 'wdl%';"
(no rows)
```

(`unregister` intermittently returned `database is locked` — the store is written by the
daemon and every live MCP session — so each call was retried until it took; all four
succeeded.)

Their task and permission rows went with them through `delete_project`, which is the
contract's own rule, so the tables are back to empty and the store is clean:
`tasks` 0 rows, `permission_requests` 0 rows, `user_version` 8.

## 7. Tests

Nothing in the repo was edited this round, so these are a regression check on the branch
the walk ran, not a proof of new tests:

```
$ CARGO_TARGET_DIR=…-wdl9 cargo test --release --no-fail-fast --manifest-path crates/waggledance/Cargo.toml
test result: ok. 1293 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 5.54s
test result: ok.    3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
test result: ok.    5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.78s
```

## Two things the walk found

**A permission frame cannot say which command.** Every `permission` message in this walk
reads `asks to use Bash: -`. Paseo's `permit ls --json` row carries `name` (the tool) and
`description`, and for these requests the description is the literal `-`:

```
$ sqlite3 … "select req_id,tool,description from permission_requests;"
permission-c84463ed-…|Bash|-
```

So wd can tell the user *that* a leader wants `Bash` but not *what* it wants to run, and
the user is asked to allow a shell command they cannot see. That is a gap in the relay's
usefulness, not in its plumbing — D9 says the message names "the tool or command", and
the command half is empty on this Paseo version. Worth its own cell before the slice
ships; this walk does not fix it.

**A board can wait a long time behind a wake turn.** Seq 19 and 20 (`project = all`) sat
unacked for minutes at a time while a wake turn was in flight — one turn carried the
backlog in `seq` order, which is D6 working as designed, but it means a board written
during a long wd turn is read late. Both were acked (15:27:34 and 15:28:40) once the turn
ended.

## What the walk does not cover

- `redo` was not driven live. It shares `task_action`'s prefix branch with `reply`
  (`"redo" => "Redo requested:"`), and `reply` proves that branch end to end; the only
  untested difference is the prefix string.
- `permit deny` was not driven live. It is the same `permit_task` call with
  `allow = false`; `allow` proves the path, and denying is what the daemon is expressly
  forbidden to do on its own.
- The 10-minute beat was never observed at its real interval — the walk drove 60 s
  beats. The interval is one config read (`OpenClawConfig::heartbeat_secs`), and `600` is
  what the restarted daemon reports.
- The sliding `heartbeat_secs = 60` window left `board` messages in the store for other
  projects' live tasks (seq 19 `t-39cfa3`-era boards pre-dated this walk; seq 19/20 here
  listed the scratch tasks). They are acked and harmless.
- The board's "only when a task changed" half was observed as the *absence* of boards on
  four unchanged beats; a board caused by a single task's state change was seen at
  seq 19/20 (the reply run moved a task to `done`), not isolated to one task by design.
