# verify-app drive 20261112-154223-full-1523078
scope: full; features: daemon-lifecycle registry agent-cli mcp-server leader-mail-and-dispatch
checkout: /mnt/data/Projects/goglbe/waggledance @ 601f92d
binary: /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance
binary sha256: 5f4255cac6eaf27efc9dfd046ad08cdcaf0c506726a85dcd58ed9a5b4dd4f3e1  mtime: 2026-11-11T23:57:32+07:00
binary version: waggledance 0.5.5

doctor: ok pid=1523129 version=0.5.5 home=/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-154223-full-1523078/home
== daemon-lifecycle
### 01 lifecycle-status-json  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance status --json 
{"running":true,"pid":1523129,"version":"0.5.5","project_count":0}
```
### 02 lifecycle-second-serve  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance serve 
A waggledance daemon is already running.
```
### 03 lifecycle-restart  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance restart 
Stopped daemon (pid 1523129).
Started daemon (pid 1523236).
```
### 04 lifecycle-stop  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance stop 
Stopped daemon (pid 1523236).
```
### 05 lifecycle-status-after-stop  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance status 
running: no
projects: 0
```
### 06 lifecycle-serve-again  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance status --json 
{"running":true,"pid":1523438,"version":"0.5.5","project_count":0}
```
== registry
### 07 registry-list-empty  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance list 
No projects registered.
```
### 08 registry-register  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance register /mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-154223-full-1523078/repo --name verify-scratch --json 
{"project_id":"repo","name":"verify-scratch","root_path":"/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-154223-full-1523078/repo"}
```
### 09 registry-list  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance list 
repo                 /mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-154223-full-1523078/repo
```
### 10 registry-projects-json  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance projects 
{"projects":[{"id":"repo","name":"verify-scratch","root_path":"/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-154223-full-1523078/repo"}]}
```
### 11 registry-status  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance status 
running: pid 1523438
projects: 1
```
### 12 registry-unregister  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance unregister repo 
Unregistered 'repo'.
```
== agent-cli
### 13 agentcli-search  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance search list projects 
{"matches":[{"verb":"projects","score":7,"help":"List registered projects as JSON.","usage":"waggledance projects [OPTIONS]"},{"verb":"runs","score":2,"help":"List dispatched runs with a one-line task summary; `runs show` prints one run in full.","usage":"waggledance runs [OPTIONS] [COMMAND]"}]}
```
### 14 agentcli-schema  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance schema state 
{"name":"state","help":"Show bee state: one summary row per project, or one project's digest.","flags":[{"name":"project","positional":true,"required":false,"takes_value":true,"help":""},{"name":"--full","positional":false,"required":false,"takes_value":false,"help":"Print the tool's full structuredContent unchanged"},{"name":"--fields","positional":false,"required":false,"takes_value":true,"help":"Keep only these comma-separated keys on each row"},{"name":"--limit","positional":false,"required":false,"takes_value":true,"help":"Cap the number of rows"}],"subcommands":[]}
```
### 15 agentcli-state  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance state 
{"projects":[{"project_id":"repo","feature":null,"phase":null,"mode":null,"waiting_on_live":false,"cell_counts":{"doing":0,"waiting":0,"stuck":0,"done":0},"leader_count":0}]}
```
### 16 agentcli-state-fields  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance state --fields project_id\,phase 
{"projects":[{"project_id":"repo","phase":null}]}
```
### 17 agentcli-runs  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance runs 
{"runs":[]}
```
### 18 agentcli-unknown-verb  (exit 2)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance sttae 
--- stderr
error: unrecognized subcommand 'sttae'

  tip: some similar subcommands exist: 'status', 'state'

Usage: waggledance <COMMAND>

For more information, try '--help'.
```
== mcp-server
### 19 mcp-stdio-session  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance mcp 
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"waggledance","version":"0.5.5"}}}
{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"waggledance_projects","description":"List every registered project: id, name, root path, and when it was last seen.","inputSchema":{"type":"object","properties":{}}},{"name":"waggledance_bee_upgrade","description":"Upgrade bee in the registered projects that already use bee. Previews by default. With `apply: true` it runs `bee onboard --apply` and commits only bee's files. `apply` requires a non-empty `projects` list the human has approved.","inputSchema":{"type":"object","properties":{"apply":{"type":"boolean","default":false,"description":"Run the upgrade and commit; false previews"},"projects":{"type":"array","items":{"type":"string"},"description":"Project ids to narrow the run to; required with apply"}}}},{"name":"waggledance_ask_state","description":"Ask waggledance for a project's parsed bee state (active feature, phase, open/blocked cells, recent decisions, sessions, handoff, attention) without reading any .bee file yourself. Omit `project` to get a rollup across every registered project; each rollup digest drops `sessions` and `recent_decisions` and carries `session_count` instead, while a one-project answer keeps every field. Two fields describe the fleet rather than the store: `herding` names the agent kinds this project offers — {default, agents, resolvable} as LABELS, never the argv behind them, and a label listed in `agents` but absent from `resolvable` is one that will not start today; `panes` is the project's own live pane inventory (pane_id, kind, status, bee_state, bee_feature, workspace, tab), which is how you find an idle pane to reuse instead of spawning a new one. `panes` is ABSENT when the terminal surface is switched off, `null` beside a `panes_error` reason when herdr cannot be read — the bee state is complete either way — and an array otherwise. Seeing a pane grants nothing: dispatch is refused in a project whose .waggledance.toml says `orchestration = false`. `leaders` lists the Paseo agents labelled waggledance.role=leader whose cwd sits in the project's root or one of its worktrees, oldest first: {agent_id, status, cwd, created, via, labels, waiting_on, last_daemon_run, runs}, where `via` is `wd` for a leader carrying waggledance.via=wd and `daemon` otherwise, and `runs` is the work that leader carries, newest first, at most 5: {run_id, request_id, status, follow_up_of, steer, task_summary}, `task_summary` being the task's first line that is not a spec-drop or wd provenance line or the steer marker, cut to 200 characters, and `steer` true for a context note sent with `steer: true` rather than work. Read `runs` before every leader dispatch, leaders included, skipping steer rows, and pick one of three tiers: (1) same work — same feature, same files, same output — dispatch with `follow_up_of` set to that run_id; (2) same cluster — same area or module or overlapping paths, different feature — open the new leader, then send the running leader a `steer: true` note naming the new request, its area, the new run_id and what to watch for; (3) unrelated — new leader, no steer. When a summary is not enough to judge, read the run's full `task` from `waggledance_runs` with its `run_id` argument. Each leader also carries `family_root` (the run id every run it carries descends from, null when it carries none) and `current` (true when it is the family's newest leader, the one a follow-up reaches). `leaders` is ABSENT when `[paseo] enabled = false`, `null` beside a `leaders_error` reason when Paseo cannot be read, and an array (possibly empty) beside `leaders_skipped`, the count of leaders under no registered project, otherwise.","inputSchema":{"type":"object","properties":{"project":{"type":"string","description":"Optional project id to narrow to a single project's full snapshot"}}}},{"name":"waggledan```
== leader-mail-and-dispatch (refusal paths only; live agents are not driven)
### 20 dispatch-leader-paseo-off  (exit 1)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance dispatch --project repo --leader --task verify-app\ probe --request-id verify-app-1 
--- stderr
error: Paseo leader dispatch is off — `enabled = false` under `[paseo]` in ~/.waggledance/config.toml; remove that line to let waggledance detect the Paseo CLI
```
### 21 mail-send-unknown-corr  (exit 2)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance mail send --to wd --corr no-such-corr --kind result 
--- stderr
error: no run found for corr `no-such-corr`
error: the result body is missing the done-report parts: Proof, Left, Where
```
### 22 mail-send-bad-recipient  (exit 2)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance mail send --to someone --corr no-such-corr --kind gossip 
--- stderr
error: recipient `someone` is not accepted; the only recipient is wd
error: kind `gossip` is not one a leader may send; accepted kinds: result, question, progress, error
error: no run found for corr `no-such-corr`
```
doctor: ok pid=1523438 version=0.5.5 home=/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-154223-full-1523078/home
