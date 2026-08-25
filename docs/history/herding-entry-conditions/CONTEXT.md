# A herding Entry Is More Than An argv — Context

**Feature slug:** herding-entry-conditions
**Date:** 2026-08-25
**Shaping session:** complete
**Scope:** Standard
**Domain types:** CALL | RUN

## Feature Boundary

waggledance reads the **whole** entry shape bee declares in a project's
`herding.agents` — the bare argv array *and* the object form
`{"argv": […], "env": {…}, "workspace_trust": {file, key}}` — and honours the two
conditions bee applies at spawn: it seeds the foreign tool's trust store and exports
`env` before `agent.start`. It ends there: no new entry shape is invented, no other bee
behaviour is mirrored, and the target project's own `.bee/config.json` remains the single
source.

## What Was Found

The original PBI (`p-9212cae8`) was wrong, and correcting it is why this feature exists.

- **The object form is a documented, first-class bee shape**, not a malformed entry.
  bee's own knowledge (`bee-herding/agent-resolution-and-spawn-commands.md`,
  `defaults-and-agent-env` D4) declares two shapes: a bare argv array, and
  `{"argv": […], "env": {…}}`, with an optional `workspace_trust` field. So beehive's
  `agy-flash` is a valid declaration and waggledance is the incomplete reader.
- **Reading `.argv` alone would be worse than today's refusal.** bee does not merely read
  those fields, it *acts* on them: `env` is exported as one line before `agent start`, and
  `workspace_trust` is pre-seeded by `run::preflight_workspace_trust`, which exists
  precisely to stop the agent stalling on a trust prompt (`herding-prompt-stall` D5).
  waggledance does neither, so a naive fix trades an honest refusal for a silent hang.
- **The file is the whole source, and it is always fresh.** The user's call, and it holds:
  no cache, no bee verb needed. Checked: no `.bee/config.local.json` overlay exists on
  this machine for beehive, waggledance or collab-review, so the file waggledance reads is
  the effective config today.
- `~/.gemini/antigravity-cli/settings.json` exists and its `trustedWorkspaces` is a list
  with 3 entries — a real trust list, not a stub.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | An entry is read in both shapes: a bare argv array, or an object whose `argv` is the command, with optional `env` and `workspace_trust`. An object with no usable `argv` is still unresolvable, exactly as today. | The two shapes are bee's, not ours; reading one of them was the bug. |
| D2 | **waggledance seeds the trust store itself**, the way bee does. The user was shown the alternative — read it and refuse when the workspace is untrusted, adding no write power — and chose the write. | This is waggledance's first write *outside* a repo, into another tool's security settings, from a daemon with no authentication. It is recorded here as a deliberate choice, not an oversight. |
| D3 | The write only ever **adds the exact directory being spawned into**, and only after that directory has passed the project's own boundary check. It never removes an entry, never rewrites unrelated keys, and never adds a path the caller did not already prove it owns. | This is what keeps D2 narrow enough to audit: the daemon can only ever trust a folder it was already about to start an agent in. |
| D4 | The write is **idempotent**: an already-trusted workspace is left exactly as it is, byte for byte where the file's own formatting allows. | A spawn is not a config edit; running one twice must not churn the user's file. |
| D5 | A trust-store failure is **fail-open with a warning**, matching bee (`preflight_workspace_trust` returns a `Warning` the caller logs and proceeds past). It never blocks the spawn. | Diverging from bee here would make the same declaration behave differently depending on who started the agent. |
| D6 | `env` is exported as one line before `agent.start`, keys `[A-Za-z_][A-Za-z0-9_]*` and newline-free values; a violating entry is dropped and the rest proceed, and a failed send is a typed spawn failure. | Bee's own rules, verbatim — the registry's fail-open-per-entry rule and its typed send failure. |
| D7 | `ask_state`'s `resolvable` and the dispatch resolver keep sharing one predicate, and both now accept the object form. A label reported resolvable must dispatch. | The `dispatch-project-presets` D6 agreement, unchanged and now covering a shape it used to exclude. |
| D8 | bee's **built-in** entries (`claude-sonnet`, `agy-flash` resolve with no `herding` block at all) are **not** mirrored. A project with no block still reports none. | They live in bee's binary and in no file, so the file cannot know them. The fix for those projects is to declare a block, not to hardcode two names here and let them drift. |
| D9 | A failed trust seeding is **carried back in the answer to whoever asked for the spawn** — not only written to the daemon's log. The dispatch result says the agent started *and* that its trust could not be seeded. Where a surface genuinely cannot carry it, that surface's blind spot is written down as a gap rather than smoothed over. | Fail-open's real failure mode is a warning nobody reads: the spawn reports success, the pane then sits at a trust prompt, and the operator has a hang that is now *harder* to attribute than the one this feature removes. Proceeding is right; proceeding silently is not. |

### Agent's Discretion

Where the trust seeding lives; how the settings file is parsed and rewritten (preserving
unrelated content is required, exact formatting is not); the warning's wording; whether
`env` and trust share one pre-spawn step.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| entry | One value in `herding.agents` — an argv array, or an object carrying `argv` plus conditions. |
| condition | Something bee does *besides* running the argv: exporting `env`, seeding `workspace_trust`. |
| trust store | The foreign tool's own settings file, named by the entry's `workspace_trust.file` and `.key`. |

## Existing Code Context

- `crates/waggledance-core/src/bee.rs` — `herding_argv_for_label_from_config` accepts only
  `Value::Array`; this is the reader D1 widens. `BeeHerdingRegistry` publishes labels only
  and must keep publishing labels only.
- `crates/waggledance/src/herdr/mod.rs` — `start_agent_in_new_tab` is the one place every
  spawn passes through, so the conditions belong on its path.
- `crates/waggledance/src/orchestrate.rs` — carries the boundary-validated destination D3
  depends on.

## Canonical References

- beehive `docs/knowledge/areas/bee-herding/agent-resolution-and-spawn-commands.md` — the
  two entry shapes, the env rules, `preflight_workspace_trust` and its fail-open contract.
- `docs/knowledge/work/dispatch-project-presets/delivery.md` — the shared-predicate
  agreement D7 preserves.
- `README.md` — waggledance's read-only-by-design posture, which D2 knowingly qualifies.

## Outstanding Questions

### Deferred To Planning

- [ ] Whether the trust file's `trustedWorkspaces` entries are plain paths or objects.
      Answered by reading the live file before writing a parser against a guess.

## Deferred Ideas

- Mirroring any other bee spawn behaviour. Out of scope: this feature closes the two
  conditions a declaration can carry, not the gap between two spawners.
- Declaring `herding` blocks for `waggledance`, `jarvis`, `jarvis-mcp` and `memorypad` —
  config, and the real answer to D8.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
