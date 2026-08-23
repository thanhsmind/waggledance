# Learnings — board-run-actions (2026-08-23)

## A knowledge record written before the code lands needs a post-landing pass

The docs cell ran in parallel with the code cells and wrote the Contract
from the plan. The review then found two real drifts (an extra `recorded`
field, a terminal-switch 404) and a locked decision reprinted without the
clause the plan had rejected. Either dispatch the docs cell after the code
caps, or give it a required "Settled in execution" pass before close.

## A plan that rejects part of a locked decision must log the supersede

plan.md said "Rejected: `bee herding run`" — the right call — but D1 still
carried that clause in the decision store until the review noticed. When
planning overrides any clause of a locked decision, log
`bee decisions log --relation supersedes:<id>` in the same turn.

## Board HTML assertions must target elements, not bare strings

`bee-hub__running` and the label `Run compound` both appear inside the
page's inline stylesheet (one in a CSS comment). Assert on
`<p class="bee-hub__running">` or a `data-*` attribute, never on a bare class
name or button label.

## Shared files across parallel workers: reserve, fall back, record

Two workers needed `server.rs`; the second was refused the reservation and
kept its half typed but unwired, leaving the two call sites to the dependent
cell. That pattern held: no clobbered commits, one follow-up cell, green base
throughout.
