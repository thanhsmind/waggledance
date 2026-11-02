# leader-per-task — context

The owner asked that a task handed to a project's leader always goes to a new
leader, with no search for a free one, and that a leader closes once it has
finished all of its work.

## Locked decisions

- **D1** (`a29369df`, supersedes leader-delivery D3): every leader dispatch
  starts a fresh Paseo leader for that run. Delivery never looks up, reuses or
  waits on an existing leader. Runs of one project are delivered in parallel,
  one leader each.
- **D2** (`0398ca94`): the daemon archives a non-wd run's leader when the run
  ends `done`. A wd run keeps its leader for `reply` and `redo`, and `accept`
  archives it as before. The leader of a `lost` or `blocked` run is not
  archived.
