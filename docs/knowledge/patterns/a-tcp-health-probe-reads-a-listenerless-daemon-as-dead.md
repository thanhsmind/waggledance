---
type: bee.pattern
title: A TCP health probe reads a listenerless daemon as dead — status, stop and restart must key off the lock pid
description: "Pitfall: waggledance-core's running_daemon dials the lock's host:port for GET /health, so a headless daemon that binds no socket never answers and status, stop and restart treat it as stale."
timestamp: 2026-11-11
bee:
  id: a-tcp-health-probe-reads-a-listenerless-daemon-as-dead
  lifecycle: active
  areas: [orchestration]
  anchors: [crates/waggledance/src/runtime.rs, crates/waggledance/src/daemon.rs, crates/waggledance/src/cli.rs, crates/waggledance/tests/e2e_daemon.rs]
  sources: [docs/history/headless-supervisor/CONTEXT.md, docs/history/headless-supervisor/plan.md]
  polarity: pitfall
  critical: false
  signature: daemon not running but the process is alive
---

# A TCP health probe reads a listenerless daemon as dead

Read when: `waggledance status` says "running: no" while the daemon process is alive, `stop` clears a live daemon's lock without killing it, or someone reuses `waggledance_core::daemon::running_daemon` from the binary.

## The trap

The core `running_daemon` answers "alive" only when the lock's host:port replies to `GET /health`. D2 removed every listener, so that probe can never succeed: status reports not running, stop deletes the lock and leaves the process, restart spawns a second daemon.

## What holds

The binary crate owns its own `DaemonInfo` (pid, started_at, version) and `running_daemon` in `runtime.rs`: the lock is trusted only while its pid names a live process whose command is `waggledance`. The command check is the guard against a recycled pid: `stop` must never signal an unrelated process the lock once named. The core probe stays in core until the core cell removes it, and the binary never calls it.

## Proof

`crates/waggledance/tests/e2e_daemon.rs` runs the real binary under a scratch HOME: `serve` writes the lock, holds no listening TCP socket, `status` reports it running, `stop` ends it; a lock naming an unrelated live pid is cleared without killing that process.
