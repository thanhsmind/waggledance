---
type: bee.delivery
title: status-wildcard-host — delivery
description: "Delivery record for work item status-wildcard-host: 1 capped cell fixing the daemon health probe so `waggledance status` recognises a wildcard-bound (0.0.0.0) daemon as running."
timestamp: 2026-08-23
bee:
  id: status-wildcard-host-delivery
  lifecycle: active
  areas: [daemon, cli]
  required_context: []
  sources: [.bee/cells/archive/status-wildcard-host/status-wildcard-host-1.json]
---

# status-wildcard-host — Delivery

## What shipped

`waggledance status`, `stop` and `restart` read a daemon bound to `0.0.0.0`
as not running, even while it answered on the port. The health probe dialed
`127.0.0.1` correctly but still sent `Host: 0.0.0.0`, which the server's
loopback Host guard rejects with 421. The probe now presents the loopback
address it dials as the `Host` header (`127.0.0.1`, or `[::1]` bracketed
for an IPv6 wildcard, which also fixes the socket-address spelling).

- **status-wildcard-host-1** — `crates/waggledance-core/src/daemon.rs`
  `health_body`; the existing wildcard-host test now captures the request
  and asserts the header. Commit `00d4439`.

## Behaviour that settled

- The Host header always names the address actually dialed; the server's
  Host guard stays unchanged (recorded in the decision log, 2026-08-23).
- Consequence seen before the fix: `waggledance restart` against a
  systemd-managed `0.0.0.0:7700` daemon could not see it, so it started a
  second daemon on the next port. Reinstalling the CLI resolves that path.

## Verify

`cargo fmt --all --check && cargo clippy -p waggledance-core --all-targets
-- -D warnings && cargo test -p waggledance-core daemon` — green, 7 tests.
After reinstall from main `3d4bd44`: `waggledance status` →
`running: http://0.0.0.0:7700 (pid 52308)`.

## Deviations

None beyond the bracketed IPv6 loopback, recorded on the cell.
