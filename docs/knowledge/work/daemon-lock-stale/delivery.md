---
type: bee.delivery
title: daemon-lock-stale — delivery
description: "Delivery record for work item daemon-lock-stale: stopping the daemon returns in bounded time against a stale record, and a process is only ever signalled once it has answered as the daemon."
timestamp: 2026-08-06
bee:
  id: daemon-lock-stale-delivery
  lifecycle: active
  areas: [daemon]
  required_context: [docs/specs/daemon.md]
  sources: [.bee/logs/scribing-runs.jsonl]
---

# daemon-lock-stale — Delivery

## What shipped

Asking the daemon to stop could hang forever. The stop path first asks whether
the daemon is alive by opening a connection to the port its on-disk record
names, and that connection had no time limit. On a network that silently drops
traffic to a dead port rather than refusing it, the question never came back and
the command sat there — the operator's only way out was to kill it by hand.

The liveness question is now bounded: it waits half a second for an answer and
treats silence as "not alive". A stale record naming a port nobody is listening
on is answered in well under a second instead of never.

The same path carried a second, quieter fault. It signalled the process id in
the record unconditionally, on the assumption that the record's id still names
the daemon. Process ids are reused, so an old record could name a process that
had nothing to do with this program and stop it. A process is now signalled only
after the liveness check has confirmed that something at that record is actually
answering as the daemon; when nothing answers, the record is cleared and no
signal is sent.

## Verify

`cargo test --workspace` green. The proof is a reproduction written first and
watched fail: driving the stop path against a stale record naming both a dead
process id and a dead port, asserting it returns within a bounded time and
clears the record. Around it — a record naming a live process that is not this
daemon (nothing is signalled, the record is not cleared blindly), no record at
all (reports no daemon running), a genuinely live daemon (still stopped, record
cleared), and the existing guard that refuses to clear the record of a daemon
which failed to die but still answers.

## Deviations

None recorded.

## Pointers

`health_check` in `crates/waggledance-core/src/daemon.rs` bounds the connect with
`HEALTH_CHECK_CONNECT_TIMEOUT`; the stop path lives beside it and in
`crates/waggledance/src/cli.rs`.

## Provenance

Written at bundle cleanup from the capped trace of `dls-1`. The recycled-process-id
rule this work established is already carried by `docs/specs/daemon.md`.
