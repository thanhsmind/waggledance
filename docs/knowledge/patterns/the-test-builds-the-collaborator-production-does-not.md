---
type: bee.pattern
title: A test that builds the collaborator itself cannot see that production builds it differently
description: "Pitfall: when a unit test hands the function the dependency it wants, the test proves the function and nothing about the wiring — so a lazily-built handle can be missing on every real call while the whole suite stays green."
timestamp: 2026-08-25
bee:
  id: test-builds-the-collaborator
  lifecycle: active
  areas: [orchestration, agent-terminal]
  required_context: [docs/knowledge/patterns/prove-the-whole-path.md]
  sources: [.bee/cells/archive/ask-state-fleet-read/asfr-2.json, .bee/cells/archive/ask-state-fleet-read/asfr-4.json, docs/knowledge/work/ask-state-fleet-read/delivery.md]
  polarity: pitfall
  signature: hand the function the slot it wants
---

# A test that builds the collaborator itself cannot see that production builds it differently

## The trap

A function reads something out of a shared slot — a connection, a handle, a client.
Its tests construct that slot and pass it in, fully populated, because that is the
only way to make the test deterministic. Every branch is covered. What no test can
see is *when* production fills the same slot: if it is built lazily, on first use,
by a different code path, then at the moment this function reads it the slot is
empty. The function returns its honest empty answer forever, and the suite has no
way to notice, because no test ever lets production do the filling.

## What it looked like here

A read that publishes a project's live pane inventory took the orchestration slot
directly. Its tests handed it a populated slot; 875 unit tests were green. Against a
running system the field degraded to null on every single call, because the handle is
built lazily and had not been built yet at read time. The defect was found only by
running the built binary against the real registry — never from a test — and cost a
fix-first cell opened against already-capped work.

## The tell

Look for a test whose *arrange* step constructs the very dependency the code under
test is supposed to find. Then ask a separate question the test cannot answer: who
puts that dependency there in production, and has it happened yet at this call site?
A second tell: a field or endpoint whose real-world value nobody has ever looked at,
only asserted on.

## What to do instead

- When a unit's answer depends on a slot it did not fill, one proof must let the real
  filler run — even if the ends are faked. This is the same law as
  [prove-the-whole-path](prove-the-whole-path.md), one level down: there the seams
  between cells went unproven, here the seam between a lazy constructor and its
  reader.
- Before capping a cell that publishes a new field, read the field once from the
  built binary. One real call is cheaper than the fix-first cell it replaces.
- Prefer passing the built dependency in over reaching into a slot for it; a
  parameter cannot be empty at the wrong moment.
