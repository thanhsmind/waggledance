promote proposal for work item "status-wildcard-host" (.bee/logs/scribing-runs.jsonl + .bee/lanes/status-wildcard-host.json) — 1 capped cell(s): status-wildcard-host-1
anchor: ledger — .bee/logs/scribing-runs.jsonl, .bee/lanes/status-wildcard-host.json
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/status-wildcard-host/delivery.md

---
type: bee.delivery
title: status-wildcard-host — delivery
description: "Delivery record proposed by bee knowledge promote for work item status-wildcard-host: 1 capped cell(s), 1 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: status-wildcard-host-delivery
  lifecycle: active
  areas: [daemon, cli]
  required_context: [.bee/logs/scribing-runs.jsonl, .bee/lanes/status-wildcard-host.json]
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/status-wildcard-host.json, .bee/cells/status-wildcard-host-1.json]
---

# status-wildcard-host — Delivery

## What shipped

- **status-wildcard-host-1** — Health probe sends the dialed loopback as Host so a wildcard-bound daemon reads as running (1 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **status-wildcard-host-1** — `cargo test -p waggledance-core daemon`

## Deviations

- **status-wildcard-host-1** — IPv6 wildcard connect host changed from ::1 to [::1] so the socket address parses; same fix, one line further

## Provenance

Proposed by `bee knowledge promote --work status-wildcard-host` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/status-wildcard-host.json`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "status-wildcard-host" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-23T01:38:43.515Z), the work item declares no bee.areas.

area daemon:
  - [status-wildcard-host-1] Health probe sends the dialed loopback as Host so a wildcard-bound daemon reads as running — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/status-wildcard-host-1.json)

area cli:
  - [status-wildcard-host-1] Health probe sends the dialed loopback as Host so a wildcard-bound daemon reads as running — feature-wide sync per the scribing stamp, 1 file(s) changed (trace .bee/cells/status-wildcard-host-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

from cell status-wildcard-host-1 — save as docs/knowledge/patterns/status-wildcard-host-status-wildcard-host-1-pitfall.md

---
type: bee.pattern
title: status-wildcard-host cell status-wildcard-host-1 — pitfall candidate
description: "Pitfall candidate mined from cell status-wildcard-host-1's capped trace: IPv6 wildcard connect host changed from ::1 to [::1] so the socket address parses; same fix, one line further"
timestamp: 2026-08-23
bee:
  id: status-wildcard-host-status-wildcard-host-1-pitfall
  lifecycle: draft
  areas: [daemon, cli]
  sources: [.bee/cells/status-wildcard-host-1.json]
  polarity: pitfall
---

# status-wildcard-host cell status-wildcard-host-1 — pitfall candidate

## What the cell did

Health probe sends the dialed loopback as Host so a wildcard-bound daemon reads as running

## Recorded evidence (verbatim from .bee/cells/status-wildcard-host-1.json)

- **deviation** — IPv6 wildcard connect host changed from ::1 to [::1] so the socket address parses; same fix, one line further

## Status

Candidate only. `bee knowledge promote` proposes; naming the pattern, generalizing it beyond this cell, and moving `bee.lifecycle` to `active` are a human or agent decision.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 2 area bullet(s), 1 pattern candidate(s), 0 file(s) written.

---

<!-- bee:not-a-deferral: this section records a completed review with no outstanding action. -->

## Resolution — 2026-08-25, reviewed against the spec

Reviewed in the sweep of the unapplied-proposal backlog. The generated bullets
are each cell's outcome in implementation vocabulary, which a spec never carries
outside its Pointers, so each was checked as behaviour rather than pasted in.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/status-wildcard-host/delivery.md`
  already exists as a curated record.
- **(b) Area updates** — already stated in `docs/specs/daemon.md` under the all-interfaces liveness check — the probe dials loopback and names the address it dialed as its host, so a daemon that checks the host it is asked for recognises its own probe.
- **(c) Pattern candidates** — none promoted from here.

<!-- /bee:not-a-deferral -->
