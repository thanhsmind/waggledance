# Paseo Removal — Plan

**Lane:** standard · **Class:** feature · **Flags:** public-contracts, proof-weakening · **Product files:** 7
**Worktree:** `waggledance--wt--paseo-removal` (branch `wt/paseo-removal`)

## Shape

**One cell, deliberately not split.** The obvious split — backend first, then
views and client — does not survive contact: `home_page` and
`project_sidebar` take a `paseo_by_project` argument, `ProjectSuggestion`
carries `paseo_count`, and `build_state` names two `AppState` fields, so
every candidate seam runs through both files at once. Any split leaves the
tree not compiling between cells, and a red base is the one thing the cap
rule refuses. The removal is atomic because the coupling is.

Order inside the cell (the worker's own, recorded here as the expected path):

1. Delete the two whole-paseo modules and their `mod` lines.
2. Remove the four routes and every server handler, struct, const and enum
   that only they reached, plus the two `AppState` fields and their inits.
3. Re-shape the shared paths — `/api/agents` (drop the paseo lookup, the
   herdr-down paseo fallback and `push_paseo_rows`), `suggested_projects`
   (drop the paseo param and counts), the home page and sidebar (drop
   `paseo_by_project` and the badge calls).
4. Remove the views surface: `PaseoAgentBadge`, `paseo_count`, the ~17 view
   fns, the permit script const.
5. Remove the client IIFE at `assets/app.js:4515-4639` and the two stale
   comments that describe paseo.
6. Delete the 89 paseo tests; operate on the 3 mixed ones instead.
7. Sync `docs/knowledge/**` entries that describe paseo as current behavior
   (D4); leave `docs/history/**` untouched.

SMALLER PATH check: the cheaper shape would be deleting the routes and
leaving the plumbing dead behind them. Rejected — dead code that still
compiles is exactly what the next reader mistakes for a live feature, and D1
asks for total removal. There is no cheaper shape that honors D1. PASS.

Hat wave: SKIPPED — the ask is unambiguous and the gather digest already
carries every anchor a wave would rediscover.

## Load-bearing claims

| # | Claim | Anchor | Label | Evidence |
|---|-------|--------|-------|----------|
| 1 | Two files are 100% paseo and deletable outright | `waggledance-core/src/paseo.rs`, `waggledance/src/paseo_cli.rs` | read | gather digest §1, confirmed no non-paseo content |
| 2 | Four `/paseo/...` routes exist, no more | `server.rs:620,622,632,640` | read | route table grep in this session returned exactly these |
| 3 | The feed's row struct is shared with herdr and must not change shape | `server.rs` `AgentPaneRow` | read | digest §6: paseo only ADDS rows via `push_paseo_rows` |
| 4 | `home_page`/`project_sidebar` take a paseo argument — the seam that forbids a split | `views.rs:1269-1271, 1331-1333`; `server.rs:1031-1058` | read | digest §6 |
| 5 | 89 paseo tests exist; exactly 3 tests are mixed | digest §8 | read | ranges and names enumerated per file |
| 6 | Nothing outside waggledance consumes the paseo routes or rows | `.claude/skills/`, `.agents/skills/`, `mcp.rs` | read | digest §7: zero dependencies |
| 7 | The paseo CLI itself is independent of waggledance | `paseo agent ls` | ran | listed agent `db58f33` while waggledance was untouched |

## Discovery

Removal checklist: `.bee/mailbox/job-1788156383816/report-1.md` (main
checkout). No open questions.

## Proof

- Cell: `cargo test -p waggledance-core -p waggledance --no-fail-fast` — the
  full declared command, not a filtered subset. A removal's whole risk is what
  it breaks elsewhere, so the narrow-scope rule does not apply here.
- Plus a grep gate: `rg -i paseo crates/` returns nothing outside
  `docs/history/**`.
- Whole path after install: the app serves the home page, the project page and
  `/api/agents` with only herdr rows, and `/paseo/<anything>` 404s.

## Later slices

None. Follow-up headline only: the drawer's homepage link rewrite
(`app.js:4342`) assumes every row is a herdr pane — with paseo gone every row
is one, so the assumption is true again by accident rather than by design.
