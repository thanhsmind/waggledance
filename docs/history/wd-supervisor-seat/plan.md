# WD Supervisor Seat — Plan

**Feature:** wd-supervisor-seat · **Lane:** standard · **Class:** feature
**Flags:** external-systems, public-contracts · **Product files:** 2
**Context:** `docs/history/wd-supervisor-seat/CONTEXT.md` (Gate 1 approved 2026-08-30)
**Worktree:** `waggledance--wt--wd-supervisor-seat` (branch `wt/wd-supervisor-seat`)
**Revision:** rev 2 — rewritten after the plan review wave (findings P1-a, P1-b, P2-a…d, P3-a/b folded in)

## What is actually missing

The mechanical path is already built. Verified against source 2026-08-30:

| Accept criterion | State today | Gap |
|---|---|---|
| 1. Lead opens in beehive via the dispatch door | `waggledance_dispatch` resolves all four beehive preset labels (`bee.rs:2189` handles both the array and object entry forms) | `beehive.orchestration_enabled = 0` — proven live: a real dispatch call refused with *"project beehive has not opted into orchestrator dispatch"* |
| 2. Spec lands per beehive's spec-drop convention | The lead does it | Convention has **two** halves (decision `12deaa34`): write the spec into beehive's own docs tree **and** register `bee backlog pbi add --id <corr-id> --status proposed` |
| 3. Run visible in `waggledance_runs` | `orchestrate.rs:357` inserts the row; `mcp.rs:1128` returns `task` untruncated, so the correlation id in line 1 is readable | none |
| 4. Merge stays human | **Not true in beehive today** — see the standing risk below | not fixable from this repo |

## Standing risk — criterion 4 is not currently true, and this feature cannot make it true

beehive's `.bee/config.json` records `gate_bypass: "full"` with `uat_stop: "close"`
(and `staging_before_merge: false`). Per bee's own contract (`AGENTS.md:56-57`),
`"close"` means *the agent merges on green without asking*. So **any** lead the seat
opens in beehive may merge to beehive `main` on its own — not because of anything
waggledance does, but because that is how beehive is configured.

The seat itself is clean: no MCP tool merges, and the skill forbids the seat from
merging or asking for a merge. But "merge to main stays human-only" is a property of
the *target repo's* config, and only its owner can set it. This plan therefore:

- makes criterion 4 true **of the seat** by construction (no merge power, written rule);
- makes the proof run safe by construction (a file-and-stop task, below);
- **surfaces the residual to the owner** rather than claiming a guarantee it cannot keep.
  Setting beehive's `uat_stop` to `"merge"` would close it; that is the owner's call in
  the owner's repo, and is recorded here as the named remedy, not done from here.

## Shape — one slice, three cells

Two code cells, then one proof cell. Slice 1 is the whole feature and ends with a real
spec dropped into beehive by a real lead through the real door.

### `wds-1` — Write the supervisor skill template (D2, D4) · commits

New `docs/waggledance-supervisor-skill-template.md`, frontmatter-shaped like
`docs/waggledance-skill-template.md` but with its own `name:` (it installs to its own
skills directory). It documents the seat and only the seat:

1. Take the spec from the human; mint the correlation id.
2. Read `waggledance_ask_state` for the target — is a lead already live? Reuse-before-
   spawn is the caller's policy (`ask-state-fleet-read` D8), so the skill is where it belongs.
3. Dispatch with a task whose FIRST line is `spec-drop <corr-id> from waggledance@<sha>`
   (D4), then the spec, then the **file-and-stop contract**: write the spec into the
   target's own docs tree, register it with `bee backlog pbi add --id <corr-id> --status
   proposed`, report, and stop — no triage, no Lock, no routing, no merge. beehive's
   own convention says the drop is deliberately not ignitable until its Qualify triage
   locks `CONTEXT.md`, so filing-and-stopping *is* the convention, not a restriction on it.
4. Report the `run_id`; check with `waggledance_runs` / `waggledance_await`.
5. **Never merge, and never ask to.** Merging is the human's gesture in the target repo.

Refusal wordings the skill must carry, so a lead reads each correctly:
project not opted in · preset does not resolve · destination unresolved (below) · a
lead already working that repo · **duplicate PBI id**. That last one matters: beehive's
`--id` is a migration-only override and a duplicate **refuses**, so on a re-send after a
timeout the lead sees an error that actually means *the drop already landed* — without
this line it will read success as failure and work around it.

### `wds-2` — Install it through `doctor` (D2) · commits

This is a **refactor**, not an addition — `check_skill_at` (`doctor.rs:683-739`) is
single-skill throughout, and the review named every place:

- `doctor.rs:688,719` reference `SKILL_TEMPLATE` as a constant → becomes a parameter.
- `doctor.rs:692,710,729,734` return the literal check name `"skill"` in all four arms →
  becomes a parameter, or `--json` emits two rows keyed `skill` (`doctor.rs:63`).
- `doctor.rs:684-685,700-703,720-724` sweep the stale `mdview` directory
  unconditionally, and `old_skill_dir_for` derives it as `<skills>/mdview` — shared by
  *any* skill path. Left unconditional, the supervisor check reports `Manual` *"stale
  mdview present"* even when the supervisor skill is installed and current. → the sweep
  becomes viewer-only.
- `skill_path()` (`doctor.rs:656`) hardcodes one path → needs a sibling; `check_skill`
  (`doctor.rs:670`) and the push at `doctor.rs:59` gain a second call.

Signature: `check_skill_at(path, template, check_name, sweep_old, dry_run, fix)`. The six
existing test call sites (`doctor.rs:809-852`) update mechanically; their assertions do
not change, which is what keeps this a refactor. New tests mirror the existing three for
the second template, asserting the load-bearing content: the `spec-drop` line format, the
no-merge rule, the duplicate-id line, and that **no raw argv appears anywhere** (a caller
names a label, never a command — `orchestrator-dispatch` D3).

Also update `docs/specs/doctor.md:45,91`, which describe a single skill by name and have
no mechanical parity check to catch the drift.

### `wds-3` — Open the door and prove the whole path (all four criteria) · no commit

1. **Flip the opt-in** for `beehive`, `waggledance`, `jarvis` via
   `POST /api/projects/:id/orchestration` — the same endpoint the settings page posts, so
   the owner's documented gesture is exercised rather than side-stepped by writing
   SQLite. The owner delegated this click when they answered the opt-in question; D3
   records it. The refused-dispatch half of the proof is **already captured** (above), at
   zero spawns, because the flag is checked at `mcp.rs:937` before any herdr call.
2. **One real spec drop into beehive** through the seat: preset `claude-sonnet`, the
   file-and-stop task from `wds-1`.
3. **Verify all four**: the run row in `waggledance_runs` carrying the correlation id ·
   the spec file written into beehive's docs tree · the PBI in beehive's backlog,
   `proposed`, id = correlation id, provenance in its CoS · beehive `main` unmoved, by
   `git log` before and after.

Known refusal this cell must be ready for: `DestinationUnresolved`
(`orchestrate.rs:313`). The doc comment at `orchestrate.rs:419-423` records this exact
failure against this exact project on 2026-08-25 — beehive's own panes resolved while the
workspace *labelled* beehive held waggledance's. A second fallback pass has since landed
(`orchestrate.rs:451`), and `ask_state` currently lists five beehive panes in its own
workspace, so it should resolve — but if it refuses, the remedy is a focused pane inside
beehive, not a plan change.

This cell spawns a real agent in beehive with `bypassPermissions` argv and writes real
rows there. That is what `p-ba554933` was approved for. The file-and-stop task is what
bounds it; the standing risk above is what remains.

### At close — the three sibling PBIs (D1)

File the widened `ask_state` digest, the cockpit repository, and weekly reports as three
proposed PBIs carrying `from beehive@296e66c3` provenance, so D1's split is a record
rather than a promise.

## Smaller path check

*Is there a cheaper shape that honors every locked decision?*

- Drop `wds-3`, accept unit tests as proof. **FAIL** — this repo's critical pattern
  `prove-the-whole-path` says a cell promising a user-visible outcome owes one proof of
  the whole path, and criteria 1–2 are claims about a real lead in a real repo.
- Fold `wds-2` into `wds-1`. **FAIL** — different proofs (prose vs `cargo test`), and
  `wds-2` turned out to be a refactor touching six existing test call sites.
- Fold the opt-in flip into the proof cell. **ADOPTED** in rev 2 — it produces no repo
  diff, so as its own cell it would have been a cell with no commit.
- Dispatch a non-`bypassPermissions` preset (`pi-opencode-free`; `pi` is installed).
  **REJECTED, recorded** — it swaps a known runtime for an unproven one mid-proof, and
  the containment that actually matters is the file-and-stop task, not the permission
  mode. Available as a fallback if the run misbehaves.

## Test scope

`cargo test -p waggledance doctor` for `wds-2` (new template cases beside the existing
`SKILL_TEMPLATE` ones, plus the six updated call sites), then `cargo test --workspace`
once before merge — baseline is green as of 2026-08-30 (exit 0). `wds-3` is proven by its
own recorded command output, not unit tests, because it changes no code.

## Order and dependencies

`wds-1` → `wds-2` → `wds-3`. Serial, with a named reason: `wds-2` compiles the file
`wds-1` writes, and `wds-3` cannot run until the procedure exists to follow.
