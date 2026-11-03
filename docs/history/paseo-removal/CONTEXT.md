# Paseo Removal — Context

**Feature slug:** paseo-removal
**Date:** 2026-08-31
**Shaping session:** clear-ask fast path (gate bypass full)
**Scope:** Standard · flags: public-contracts, proof-weakening
**Domain types:** UI, HTTP, CORE

## Feature Boundary

Every paseo-related part leaves waggledance: the two `/paseo/...` route
families and their handlers, the two paseo modules, the paseo rows in the
agents feed, the paseo badges on the project list, the paseo counts on
project suggestions, the client-side paseo poller and composer, and the
`AppState` plumbing that fed them.

It ends at waggledance's own tree. The `paseo` CLI on this machine is
untouched and keeps running its own agents — this feature removes
waggledance's *view* of paseo, not paseo. Nothing about herdr panes, the
agents feed's shape for herdr rows, or the terminal surfaces changes.

Original ask (verbatim): «Tôi muốn loại bỏ các phần liên quan tới paseo ra
khỏi waggledance»

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The removal is total and leaves no compatibility surface: no `/paseo/*` route, no redirect, no deprecation shim, no feature flag. | A shim is a second thing to maintain for a feature the owner asked to be gone. |
| D2 | Shared surfaces keep their non-paseo behavior exactly. `AgentPaneRow` keeps its shape, so the herdr feed contract is untouched; `ProjectSuggestion` loses `paseo_count` because that field exists only to count paseo agents. | The feed is consumed by the drawer and the home page; changing its shape would be a second, unasked change. |
| D3 | The ~89 paseo-specific tests are deleted with the behavior they prove; the 3 mixed tests are edited, never deleted. Deleting proof is legitimate here precisely because the proven behavior is gone — that is what the `proof-weakening` flag records. | A mixed test still covers surviving behavior; deleting it would silently drop that cover. |
| D4 | `docs/history/**` is left exactly as it is — it is the record of what happened, not a claim about what exists. `docs/knowledge/**` entries that describe paseo as current behavior are corrected, because that layer IS the claim about what exists. | Rewriting history would erase why the code was ever there. |

### Agent's Discretion

Ordering within the removal, how the shared helpers are re-shaped once their
paseo arm is gone, and whether any now-single-use helper is inlined are
implementation choices — constrained only by D1–D4 and by the build staying
green.

## Existing Code Context

The full removal checklist, with file:line anchors for every item, is the
gather digest at `.bee/mailbox/job-1788156383816/report-1.md` (main checkout).
Headlines:

- Whole-file deletions: `crates/waggledance-core/src/paseo.rs` (368 lines),
  `crates/waggledance/src/paseo_cli.rs` (575 lines).
- Module wiring: `waggledance-core/src/lib.rs:21`, `waggledance/src/main.rs:10`.
- Server: 4 routes, ~15 production items, 3 test blocks (42 tests).
- Views: 2 structs/fields, ~17 view fns, 23 tests.
- Client: one IIFE at `assets/app.js:4515-4639` plus two stale comments.
- Entanglement to preserve: `/api/agents` herdr rows, `suggested_projects`,
  the home page and sidebar, `AppState`'s surviving fields.

## Out of Scope

- The live paseo agent `db58f33b` (a supervisor task from 2026-08-30 that
  finished, asked a closing question, and has been idle since). It belongs to
  paseo, not to waggledance; whether it is answered or deleted is the owner's
  call, and this feature neither touches nor depends on it.
- The two defects found in the paseo row while diagnosing this ask (the
  homepage link rewrite that made it unclickable, and the row labelled by
  model instead of task). Both die with the feature.
