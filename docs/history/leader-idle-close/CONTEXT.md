# leader-idle-close — context

The owner asked that a leader left idle for more than one day closes on its own.

## Locked decisions

- **D1** (`af012b0b`, touches leader-per-task D2): the daemon archives any
  waggledance leader (label `waggledance.role=leader`, wd leaders included)
  that is not `running` or `initializing` and whose Paseo `UpdatedAt` is more
  than 24 hours old. The sweep runs inside the delivery loop, which runs
  whenever Paseo is available, on the 5-minute discovery cadence.
- **D2** (`12506149`, touches leader-per-task D1): a follow-up run of a wd
  task (`request_id` `<corr>:<n>`, from `reply` or `redo`) is delivered to
  that task's recorded leader while the leader is not archived, waiting while
  it is `running` or `initializing`. When that leader is archived, a new
  leader starts, and its prompt carries the original task text before the
  follow-up. Every other run still starts its own new leader. This repairs a
  regression from leader-per-task: follow-ups had started landing on a fresh
  leader with no task context.
