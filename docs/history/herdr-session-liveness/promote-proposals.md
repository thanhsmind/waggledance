promote proposal for work item "herdr-session-liveness" (.bee/logs/scribing-runs.jsonl + .bee/lanes/herdr-session-liveness.json) — 1 capped cell(s): hsl-1
anchor: ledger — .bee/logs/scribing-runs.jsonl, .bee/lanes/herdr-session-liveness.json
PROPOSAL ONLY — nothing was written. Applying any section below is a human or agent decision.

(a) DELIVERY DRAFT — save as docs/knowledge/work/herdr-session-liveness/delivery.md

---
type: bee.delivery
title: herdr-session-liveness — delivery
description: "Delivery record proposed by bee knowledge promote for work item herdr-session-liveness: 1 capped cell(s), 0 recorded deviation(s)."
timestamp: 2026-08-23
bee:
  id: herdr-session-liveness-delivery
  lifecycle: active
  areas: [bee-cockpit]
  required_context: [.bee/logs/scribing-runs.jsonl, .bee/lanes/herdr-session-liveness.json]
  sources: [.bee/logs/scribing-runs.jsonl, .bee/lanes/herdr-session-liveness.json, .bee/cells/hsl-1.json]
---

# herdr-session-liveness — Delivery

## What shipped

- **hsl-1** — heartbeat-stale bee session stays joined to its pane while herdr lists its session id (3 file(s) changed)

## Verify

Each cell below was capped only against a recorded passing verify result — bee refuses a cap without one.

- **hsl-1** — `cargo test -p waggledance green incl. a wire test parsing agent_session.value into Agent.session_id and a project_bee_activity test keeping a non-live session herdr lists; cargo clippy -p waggledance --all-targets -- -D warnings clean`

## Deviations

None recorded in the capped cell traces.

## Provenance

Proposed by `bee knowledge promote --work herdr-session-liveness` from 1 capped cell trace(s) in `.bee/cells/` and the anchor `.bee/logs/scribing-runs.jsonl`, `.bee/lanes/herdr-session-liveness.json`. Every line above is copied from a trace or from the work item; nothing here is curated truth until a human or agent accepts it.

(b) AREA UPDATES — candidate spec-sync bullets, each citing its cell

areas: from the scribing stamp for "herdr-session-liveness" — .bee/logs/scribing-runs.jsonl's most recent entry (2026-08-23T07:40:13.129Z), the work item declares no bee.areas.

area bee-cockpit:
  - [hsl-1] heartbeat-stale bee session stays joined to its pane while herdr lists its session id — feature-wide sync per the scribing stamp, 3 file(s) changed (trace .bee/cells/hsl-1.json)

(c) PATTERN CANDIDATES — candidate bee.pattern concepts, bee.polarity pitfall

None: no capped cell trace carries a deviation or a failure signature.

knowledge promote: 1 capped cell(s) mined, 1 delivery draft, 1 area bullet(s), 0 pattern candidate(s), 0 file(s) written.

---

<!-- bee:not-a-deferral: this section records a completed review with no outstanding action. -->

## Resolution — 2026-08-25, the spec already records this feature

Reviewed in the sweep of the unapplied-proposal backlog.

- **(a) Delivery draft** — not applied. `docs/knowledge/work/herdr-session-liveness/delivery.md`
  already exists as a curated record.
- **(b) Area updates** — not applied. `docs/specs/bee-cockpit.md` names `herdr-session-liveness` in its own
  `sources`, which is the receipt that this feature's behaviour was merged into
  the spec when it closed. The generated bullets restate the same work in
  implementation vocabulary — function and type names — which a spec carries only
  in its Pointers, so applying them would add nothing the spec does not say and
  would break its vocabulary rule. This resolution rests on that receipt and on
  the spec owning the area, not on a line-by-line re-reading of every bullet.
- **(c) Pattern candidates** — none promoted from here.

<!-- /bee:not-a-deferral -->
