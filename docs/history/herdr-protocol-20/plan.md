---
artifact_contract: bee-plan/v1
mode: standard
---

# Plan: The Write Half Of The herdr Protocol Is Four Versions Behind

Mode: `standard` — 2 risk flags: covered-contract-change, external-systems.
`external-systems` is a hard-gate flag, so the table's letter says high-risk. Recorded
deviation to standard: this repairs an **existing** integration to the version already
installed, the wire contract is first-party evidence (`herdr api schema --json`) rather
than guesswork, no credential/billing/data-retention surface is involved, and no
validation is removed. What keeps it at standard rather than below: this is the only
path in the product that starts a process.

## Requirements (from CONTEXT.md)

- **D1** — `agent.start` sends `{name, kind, pane_id}` + `args`; `kind` = argv[0],
  `args` = argv[1..], the split bee's own `herding wave` uses.
- **D2** — spawn is `tab.create` then `agent.start` into that tab's pane, resolved by a
  `session.snapshot` lookup on `tab_id`.
- **D3** — a tab with no resolvable pane is a typed failure naming the orphaned tab,
  never a fallback to another pane.
- **D4** — `HERDR_PROTOCOL` → 20, justified by the seven-method audit and nothing else.
- **D5** — read calls and the three send calls are untouched.
- **D6** — no check relaxed: preflight, containment, opt-in, presets-only all stand.
- **D7** — `FakeHerdr` moves with the real client.

## Discovery

- All seven methods waggledance calls still exist in protocol 20; five validate as-is.
  Evidence: `herdr api schema --json`, `request.$defs` compared per method.
- `AgentStartParams` required `["name","kind","pane_id"]`; optional `args`,
  `timeout_ms`. Nothing named `argv`, `workspace_id`, `cwd` or `focus` — so the current
  call is not merely missing a field, it is the wrong shape.
- `tab_created` requires `["type","tab"]`, and `TabInfo` carries no pane id.
  `PaneListParams` filters by `workspace_id` only, so the tab→pane hop must go through
  a snapshot.
- `tab_create` has exactly one production caller (`server.rs:4325`); `agent_start` has
  one (`orchestrate.rs`'s `Spawn` branch, reached by both the board and MCP dispatch).

## Approach

Port the two drifted calls in `socket.rs`, restructure the `Spawn` branch into
tab-then-start, move `FakeHerdr` with them, bump the constant.

*Rejected alternatives.*
- Bump `HERDR_PROTOCOL` alone — the mismatch is not a version check, it is the payload;
  bumping would only hide the incompatibility behind a passing handshake.
- Pin waggledance to an older herdr — the user runs one herdr for everything; asking the
  machine to downgrade to suit one client is the wrong direction.
- Switch to `agent.prompt`/`agent.wait` while here — protocol 20 offers them and they
  would replace the marker protocol, but that supersedes `orchestrator-dispatch` D5 and
  belongs to the user, not to a port.

*Risk map.*

| Component | Risk | Proof needed |
|---|---|---|
| `agent_start` params (D1) | **HIGH** — decides which program starts | Unit: the argv split; live: a real spawn against the installed herdr |
| tab→pane lookup (D2/D3) | MEDIUM — a wrong pane means starting an agent on top of someone's work | Unit: match by tab_id, and the no-pane case fails typed rather than picking a neighbour |
| `tab_create` response (D2) | LOW — one caller, mechanical | The existing shell-create route still works |
| `FakeHerdr` (D7) | MEDIUM — if the double lags, every green test lies | The double's own suite, plus one test asserting the params it receives |
| Protocol bump (D4) | LOW — a constant, once the payloads match | The handshake stops reporting incompatible |

## Shape

One slice, three cells, sequential — cell 2 depends on cell 1's client surface, cell 3
on both.

| Cell | What changes | Why it is its own cell |
|---|---|---|
| 1 | `socket.rs` + `fake.rs`: `agent.start` params (D1), `tab.create` response (D2), the `Herdr` trait signature the new shape forces | The wire layer alone, provable against the double without touching the flow |
| 2 | `orchestrate.rs`: `Spawn` becomes tab-then-start with the D3 typed failure | The flow, once the client can express it |
| 3 | `wire.rs` constant → 20 (D4), `server.rs:4325` follows the new `tab_create` | The two mechanical followers, last so they land on a working base |

## Test matrix

*Happy path.* A spawn sends `agent.start` with `kind` = argv[0] and `args` = argv[1..],
into the pane of the tab it just created.

*Edge cases.* A single-token argv (`["claude"]`) sends that as `kind` with empty `args`.
Two panes in the workspace, only one in the new tab — the lookup takes the right one.
The board's shell-create route still returns a usable pane.

*Error paths.* A tab created with no pane findable in the snapshot fails typed, names the
orphaned tab, and **starts nothing** (D3). An empty argv is still refused before any call,
as today. Preflight, containment and the opt-in all still refuse exactly as they do now
(D6) — asserted, not assumed, because a port is where such rules get quietly dropped.

*Live proof, and it is required here.* The unit tests prove the shape against our own
double, which is the thing that was wrong for four protocol versions. The claim "spawn
works" is only earned by one real dispatch into an opted-in project on the installed
herdr — the same run that currently fails with `missing field 'kind'`.

## Out of scope

- `agent.prompt` / `agent.wait` / `events.subscribe`, and any change to the completion
  protocol.
- A field-by-field audit of read responses.
- Any relaxation of preflight, containment, opt-in or presets-only.
