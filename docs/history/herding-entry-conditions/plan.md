---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: A herding Entry Is More Than An argv

Mode: `standard` — 3 risk flags: external-systems, audit-security, covered-contract-change.
`audit-security` is a hard-gate flag, so the table's letter says high-risk. Recorded
deviation to standard: the user was shown the trade explicitly at shaping and chose the
write path (D2), the write mirrors an existing bee behaviour on the same file with the
same fail-open semantics, and the blast radius is one JSON key in one file. What keeps it
at standard rather than lower: waggledance gains its first write **outside** any repo,
into another tool's security settings, from a daemon with no authentication.

## Requirements (from CONTEXT.md)

- **D1** both entry shapes read; an object with no usable `argv` stays unresolvable.
- **D2** waggledance seeds the trust store itself.
- **D3** the write only adds the exact, boundary-validated directory being spawned into;
  never removes, never touches unrelated keys.
- **D4** idempotent — an already-trusted workspace is left alone.
- **D5** fail-open with a warning, matching bee; never blocks the spawn.
- **D6** `env` exported as one line before `agent.start`, bee's key/value rules, a
  violating entry dropped and a failed send a typed spawn failure.
- **D7** `resolvable` and the dispatch resolver keep one predicate, now covering objects.
- **D8** bee's built-ins are not mirrored.
- **D9** a failed seeding is reported in the spawn's own answer, not only the log.

## Discovery

- `~/.gemini/antigravity-cli/settings.json` read before writing a parser against a guess
  (CONTEXT.md's deferred question): the file has exactly one top-level key,
  `trustedWorkspaces`, and its entries are **plain absolute path strings** — e.g.
  `"/home/thanhsmind/Projects/goglbe/jarvis"`, 3 of them. So the seeding is: parse, test
  membership by path, append one string, write back. No object schema to model.
- `herding_argv_for_label_from_config` (`bee.rs`) matches `Value::Array` and returns
  `None` for anything else — one arm to widen, and `BeeHerdingRegistry`'s `resolvable`
  already asks that function, so D7 holds for free.
- `start_agent_in_new_tab` (`herdr/mod.rs`) is the single path every spawn takes, and it
  already receives the destination directory — which is the value D3 needs, already
  boundary-validated by its caller.

## Approach

Widen the reader to return the entry's conditions alongside its argv, and give
`start_agent_in_new_tab` one pre-spawn step that applies them.

*Rejected alternatives.*
- Read `.argv` and ignore the conditions — the original PBI's wording; it turns an honest
  refusal into a silent hang at the trust prompt.
- Read the trust store and refuse when untrusted — offered to the user, not chosen.
- Call bee to spawn conditioned entries — the user's point stands: the config file is the
  whole source and always fresh, so a bee round-trip buys nothing.

*Risk map.*

| Component | Risk | Proof needed |
|---|---|---|
| Trust-store write (D2/D3/D4) | **HIGH** — a write outside the repo into a security file | Adds only the spawn directory; idempotent; unrelated content survives; never removes |
| Boundary tie (D3) | **HIGH** — the whole audit story | The path written is the validated destination, asserted, never a caller-supplied string |
| Fail-open (D5/D9) | **HIGH** — its real failure mode is a warning nobody reads: the spawn reports success and the pane then hangs at a trust prompt, harder to attribute than the failure this feature removes | Unreadable/unwritable store proceeds, AND the dispatch answer carries the failure |
| `env` export (D6) | MEDIUM — shell injection surface | Key/value rules enforced; a violating entry dropped, the rest proceed |
| Reader widening (D1/D7) | LOW | The existing herding suite passes unedited; `agy-flash` becomes resolvable |

## Shape

One slice, three cells, sequential.

| Cell | What changes | Why its own cell |
|---|---|---|
| 1 | `bee.rs`: read the object form, return argv + conditions; `resolvable` follows for free | Pure, no I/O beyond the config already read |
| 2 | Trust seeding: parse, membership, append, write back — fail-open | The one risky write, provable in isolation against a temp file |
| 3 | `herdr/mod.rs`: one pre-spawn step applying trust then env, on the single spawn path | The wiring, last, on a proven base |

## Test matrix

*Happy path.* An object entry resolves to its argv and carries its conditions; a spawn
into an untrusted directory adds exactly that path and starts the agent.

*Edge cases.* An already-trusted workspace is left unchanged (D4). An entry with `argv`
but no conditions behaves exactly as a bare array does. The bare-array shape is untouched
— the existing herding suite passes **unedited**, which is what proves that. Unrelated
top-level keys in the settings file survive a write (the live file has none today, so
this is asserted against a fixture that does).

*Error paths.* A missing, unreadable, unparseable or unwritable trust store warns and the
spawn proceeds (D5) — asserted, because the tempting reading of "security" here is to
block, and bee does not. And the warning must **arrive**: the dispatch answer for that
spawn names the seeding failure (D9), asserted on the answer rather than on a log line,
because a warning only in the daemon's log is indistinguishable from no warning at the
moment the operator is looking at a pane that will not move. An `env` key or value breaking bee's rules drops that entry only.
An object entry with no usable `argv` is still unresolvable and still refuses in
`dispatch-project-presets` D4's terms.

*The audit assertion, and it is the one that matters:* the path written to the trust store
is the destination the caller already validated — never a string taken from a request,
never a parent, never a guess. Proved by driving a spawn and asserting the file gained
exactly that directory and nothing else.

*Live proof.* A real dispatch into beehive selecting `agy-flash` — the entry this whole
chain has been unable to start — on the installed herdr, with the opt-in turned on for
that one dispatch and off again after.

## Out of scope

- Mirroring any other bee spawn behaviour.
- bee's built-in entries (D8).
- Declaring `herding` blocks for the four projects that carry none.
