---
type: bee.pattern
title: A crate-scoped formatter sweeps files your cell does not own
description: "Pitfall: cargo fmt -p <crate> is the obvious way to satisfy a format check, but it reflows every file in the crate — including a sibling worker's, and including pre-existing drift from a different rustfmt version — so the cell commits changes it never reasoned about."
timestamp: 2026-08-31
bee:
  id: a-crate-scoped-formatter-sweeps-files-you-do-not-own
  lifecycle: active
  areas: [workflow-state]
  sources: [.bee/cells/cds-6.json, .bee/cells/cds-7.json]
  polarity: pitfall
  signature: cargo fmt run at crate scope inside a cell
---

# A crate-scoped formatter sweeps files your cell does not own

## The trap

A cell touches four files, the format check complains, and the reflex is
`cargo fmt -p waggledance`. It reformats the **whole crate**.

`changes-diff-screen` hit this twice in consecutive cells. The sweep reflowed
`guide.rs`, `herdr/socket.rs` and `orchestrate.rs` — three files the cell did
not name and did not reserve. Worse, those files carried **pre-existing drift
from a different rustfmt version**, so the sweep was not even a no-op waiting to
happen: it was a real, unrelated diff that would have ridden into the cell's
commit under a subject line describing something else.

Both cells backed it out the same way, and it is the right way: revert the
collateral, then **fix your own hits by hand**.

## Why it is worse than untidy

The reservation and one-commit-per-cell rules exist so a diff can be read as one
decision. A formatter sweep breaks that quietly — nothing errors, the tests stay
green, and the commit now contains changes nobody chose. With a sibling worker
live in the same checkout it is the shared-index attribution loss by another
route: their in-flight file, reformatted and committed by you.

## The rule

Scope the formatter to the paths the cell owns, or fix the reported hits by
hand. Crate-wide formatting is its own piece of work — it needs its own cell,
its own reasoning about the drift it is about to normalize, and its own proof
that the sweep changed nothing that matters
([[prove-a-formatting-sweep-is-behavior-neutral]]).

## Related

- [[deferring-a-commit-on-a-contended-file]]
- [[a-private-index-commit-leaves-the-shared-one-lying]]
