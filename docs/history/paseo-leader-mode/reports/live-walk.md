# Paseo leader mode — the live walk

`green:live`. A daemon built from this branch started a real Paseo leader in
`bypassPermissions` with no mode configured, and repaired the same leader after it was
forced into `default`, before it sent the next task.

Run on 2026-10-23 between 01:56Z and 02:02Z on the host that runs the Paseo daemon
(`~/.paseo`, daemon 0.11.4).

## 1. Build, proven by content

`CARGO_TARGET_DIR` on this host is shared (`/home/thanhsmind/.cache/cargo-target`), so
the walk built into its own directory
(`docs/knowledge/patterns/the-binary-you-ran-is-not-the-one-you-built.md`):

```
$ CARGO_TARGET_DIR=/home/thanhsmind/.cache/cargo-target-plm \
    cargo build --profile fast -p waggledance --message-format=json
"executable":"/home/thanhsmind/.cache/cargo-target-plm/fast/waggledance"
$ strings <bin> | grep -c -- 'runs in mode'                            → 1
$ strings <bin> | grep -c -- 'runtimeInfo'                             → 1
$ strings <bin> | grep -c -- 'delivery leader runs in the wrong mode'  → 2
```

## 2. A separate daemon, not the owner's

The owner's daemon on port 7700 was left alone, because it was delivering the task that
started this work. The walk ran a second daemon with its own home and registry:

- `HOME=.bee/tmp/paseo-leader-mode/live/home` (config: port 7791, host 127.0.0.1,
  `[paseo] binary` = the npm `@getpaseo/cli`, no `mode` set anywhere).
- `PASEO_HOME=/home/thanhsmind/.paseo`, the real Paseo daemon home.
- A scratch project `scratch` with `.waggledance.toml` holding only
  `orchestration = true`.

```
waggledance serving on http://127.0.0.1:7791
{"project_id":"scratch","name":"plm-live",...}
```

## 3. A new leader starts in bypassPermissions

Dispatch through `waggledance mcp`, `waggledance_dispatch {project: scratch, leader: true}`:

```
queued run run-19e2a1ddc0ed3c06 for project scratch's Paseo leader
INFO waggledance::delivery: delivery starts the plm-live leader run=run-19e2a1ddc0ed3c06 workspace=wks_0ce4a560e9a4016f reason="created"
```

Paseo MCP `get_agent_status 781a71ed-…` after the turn:

```
"runtimeInfo":{…"modeId":"bypassPermissions"…}, "currentModeId":"bypassPermissions"
```

Run row: `status: done`, transcript
`[waggledance] leader started in Paseo workspace wks_0ce4a560e9a4016f (created)` then
`READY`. No mode warning line, as expected for a leader already in the asked mode.

## 4. An existing leader in the wrong mode is repaired before the send

```
$ paseo agent mode 781a71ed-… default --json
{"agentId": "781a71e", "mode": "default"}
$ grep '"modeId"' ~/.paseo/agents/<scratch-slug>/781a71ed-….json
"modeId": "default"
"modeId": "default"
"modeId": "bypassPermissions"
```

The second dispatch, `run-6c59e2e7d0a03eee`, asked the leader to run
`echo plm-two > two.txt`. Paseo's daemon log for the agent:

```
"time":1792720879224  "msg":"session: set_agent_mode_request"           ← the manual default, 02:01:19Z
"time":1792720908443  "msg":"session: set_agent_mode_request"           ← waggledance's repair, 02:01:48Z
"time":1792720908453  "msg":"session: set_agent_mode_request success"   ("modeId":"bypassPermissions")
```

After it: the agent record reads `"modeId": "bypassPermissions"` three times,
`scratch/two.txt` holds `plm-two` (the shell command ran with no permission prompt), and
the run row is `status: done`, transcript `DONE-TWO`, with no mode warning line.

## 5. Cleanup

The test daemon was stopped (`kill 897514`; port 7791 closed) and the scratch leader
archived (`paseo archive 781a71ed-… → "status": "archived"`).

## What the walk does not cover

- The `Mismatch` report line (mode still wrong after the set). Paseo honored the set, so
  this path is proven by `a_new_leader_still_in_the_wrong_mode_is_named_in_the_reply_and_the_run_ends_done`
  only.
- The npm CLI 0.10.3 was the binary in use; the AppImage CLI 0.12.0 was not driven.
- The beehive leader `ff24c29b` is still in runtime mode `default`. It is repaired on its
  next task once the owner's daemon runs this build.
