---
type: bee.pattern
title: Measure the send and the paint separately before you speed anything up
description: "Practice: a UI that feels slow invites a faster transport or a shorter interval, but splitting the measurement into the send and the paint usually shows both are already fast and the whole delay is the client waiting for its own next tick."
timestamp: 2026-08-31
bee:
  id: measure-the-send-and-the-paint-separately
  lifecycle: active
  areas: [agent-terminal]
  sources: [.bee/cells/pkr-1.json]
  polarity: practice
  signature: "felt lag, no measurement"
---

# Measure the send and the paint separately before you speed anything up

## The situation

"The key grid lags." The obvious suspects are the transport and the server, and
both invite expensive fixes: a faster channel, a shorter poll interval, a
connection kept open.

## The practice

Time the two legs apart before touching either.

On 2026-08-31 that measurement read: `POST /keys` returns in **14 ms**, and the
key is visible in `/screen` **28 ms** after the click. Both legs were already
fast. The felt lag was entirely the third thing nobody had timed — the client's
own wait for its next fixed tick. No sender kicked a refresh, so the UI sat out
`POLL_MS = 1500`: between 0 and 1500 ms of dead time, 750 ms on average.

The fix followed from the measurement and was neither of the obvious ones: a
**kick**, not a faster interval and not a faster transport. Senders mark the
pane dirty on `res.ok`; the pollers listen and poll immediately, with one-shot
follow-ups at 250 and 700 ms. `POLL_MS` stayed 1500.

## What the measurement also surfaces

Timing the legs separately puts a number on costs you were not looking for. The
same run found every key POST paying a full herdr snapshot for the boundary
check — two herdr connections per keystroke, about 10 of the 14 ms. That is a
real finding, and it is also *not* the felt lag; keeping the two apart is what
stops a 10 ms optimization being sold as the fix for a 1500 ms wait.

## The multiplier worth knowing

Through the cloudflared tunnel each leg adds roughly 365 ms, and the flow costs
two legs — the POST, then the next screen GET. A handset tap that feels
instant on localhost feels like 1.1–2.2 s remotely. Measure locally, then
multiply by the legs before deciding a number is acceptable.

## Pointers

- `crates/waggledance/assets/app.js` — `markPaneDirty` / `onPaneDirty`, `POLL_MS`
- `crates/waggledance/src/server.rs` — the per-POST boundary check
