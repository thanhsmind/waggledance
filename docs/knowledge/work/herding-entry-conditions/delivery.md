---
type: bee.delivery
title: herding-entry-conditions — delivery
description: "Delivery record for work item herding-entry-conditions: waggledance reads the whole entry shape bee declares and honours the conditions around the command — which is what finally let agy-flash start."
timestamp: 2026-08-25
bee:
  id: herding-entry-conditions-delivery
  lifecycle: active
  areas: [agent-terminal, orchestration]
  required_context: [docs/history/herding-entry-conditions/CONTEXT.md, docs/history/herding-entry-conditions/plan.md]
  sources: [docs/history/herding-entry-conditions/CONTEXT.md, docs/history/herding-entry-conditions/plan.md, docs/knowledge/work/herdr-protocol-20/delivery.md, docs/knowledge/work/dispatch-project-presets/delivery.md]
---

# herding-entry-conditions — Delivery

## What shipped

bee declares **two** entry shapes in `herding.agents`, not one: a bare argv array, and an
object `{"argv": […], "env": {…}}` with an optional `workspace_trust {file, key}`
(`bee-herding/agent-resolution-and-spawn-commands.md`, `defaults-and-agent-env` D4).
waggledance read only the first, so beehive's own `agy-flash` — a valid declaration —
reported unresolvable.

The fix is not "read `.argv`". bee does not merely read those fields, it **acts** on them:
`env` is exported before `agent start`, and `workspace_trust` is pre-seeded by
`preflight_workspace_trust`, which exists to stop the agent stalling on a trust prompt
(`herding-prompt-stall` D5). Reading the argv alone would have traded an honest refusal
for a silent hang — the original PBI (`p-9212cae8`) said exactly that, and was corrected
before this feature started.

A spawn now seeds the trust store, exports `env` into the pane, then starts.

## Locked decisions

| ID | Decision |
|----|----------|
| herding-entry-conditions D1 | Both entry shapes are read; an entry yielding no usable command is still unresolvable. |
| herding-entry-conditions D2 | **waggledance seeds the trust store itself.** The user was shown the alternative — read it and refuse when untrusted, adding no write power — and chose the write. |
| herding-entry-conditions D3 | The write adds only the exact directory being spawned into, after it passed the project's boundary check. Never removes, never rewrites another key. |
| herding-entry-conditions D4 | Idempotent: an already-trusted workspace is untouched, byte for byte. |
| herding-entry-conditions D5 | A trust failure is fail-open with a warning, matching bee. It never blocks the spawn. |
| herding-entry-conditions D6 | `env` is exported as one line before `agent.start`, bee's key/value rules; a violating entry drops alone, and a failed send is fatal. |
| herding-entry-conditions D7 | `resolvable` and the dispatch resolver keep one predicate, now covering objects. |
| herding-entry-conditions D8 | bee's built-in entries are **not** mirrored; the fix for a project with no block is to declare one. |
| herding-entry-conditions D9 | A failed seeding is carried back **in the answer to whoever asked for the spawn**, not only the daemon log. |

## The asymmetry, kept on purpose

Trust failing is a **warning**; `env` failing to send is **fatal**. That split is bee's,
not ours to tidy: an agent without its trust seeded merely meets a prompt, while an agent
without its declared environment is a different program than the one asked for.

## Verification

`cargo test -p waggledance` (899) and `-p waggledance-core` (440) on main after the merge,
green.

**Live:** a dispatch into beehive selecting `agy-flash` — the entry this whole chain had
never been able to start — returned `run-48e951cf2a67257a` with **no warnings**, and the
trust store went from three entries to four, gaining beehive and no other key. The opt-in
was on for that one dispatch and off again after.

## Learned

**A test that says "today" is telling you it is about an incompleteness, not a rule.**
Four tests pinned "the object form does not resolve", and three said so in those words.
Their premise was this reader being incomplete — so the honest handling was to update them
with the reason recorded where they stood, and re-assert the rules underneath: `resolvable`
means "this label can start", the registry and the resolver share one predicate, and a
declared-but-unstartable label is refused in its own terms and never as unknown. That last
one needed a **new example**, because `agy-flash` had stopped being one.

**Fail-open's real failure mode is fail-quiet.** Proceeding after a failed seeding is
right; proceeding silently would relocate the hang one step later and make it *harder* to
attribute, because the spawn reported success. D9 exists for that, and its test asserts on
the returned answer rather than on a log line — an assertion a later "just log it" cannot
satisfy.

**Narrowness came from the arguments, not from care.** The seeder is handed one absolute
directory by a caller that already validated it; it reaches for nothing else, so it
cannot trust a folder nobody was about to start an agent in.

## Open gaps

- The **board** still resolves argv-only (`herding_agent_argv`), so a board Start does not
  apply `env` or seed trust. Widening it to the full entry shape is its own change, noted
  at the call site.
- bee's built-in entries are still not mirrored (D8); `waggledance`, `jarvis`, `jarvis-mcp`
  and `memorypad` declare no `herding` block.
- No project has `orchestration.enabled` on: the capability works, nothing dispatches
  unattended.
