# Slash Builtin Commands — Plan

**Lane:** standard · **Class:** feature · **Flags:** covered-contract-change · **Product files:** 4
**Worktree:** `waggledance--wt--slash-builtin-commands` (branch `wt/slash-builtin-commands`)

## Shape

One cell, one slice — the three edits are one story and share `slash.rs`:

1. **Table + tool.** `crates/waggledance/tools/extract-agent-builtins.py`
   (already written and proven against claude 2.1.251) is committed as the
   provenance and refresh recipe. Its JSON output is transcribed into
   `crates/waggledance/src/slash_builtins.rs` as a const table
   `(name, argument_hint, description)` per vendor, with a header comment
   naming the source version and the exact command to regenerate.
2. **Resolution.** `slash.rs` gains `builtin_entries(vendor)` and a
   `vendor_for_kind(kind)` mirroring `bee_hub_agent_logo`'s substring
   matching; `slash_entries` grows an optional vendor argument and appends
   built-ins last, so D3's project → user → builtin shadow order falls out of
   the existing first-seen-wins loop.
3. **Wiring.** `server.rs` reads `?pane=<id>` on both `_slash` routes, joins
   the herdr snapshot's `agents[]` for that pane's kind, and passes the vendor
   down; `app.js` appends `?pane=<paneId>` at the two pane composer sites
   (the paseo composer has no herdr pane and stays as it is).

SMALLER PATH check: cheaper shapes considered and rejected on the D1/D2
records — a hardcoded list (wrong the day the vendor ships a command, and
already proven wrong from memory) and an unconditional claude list (lies on
shell panes). A runtime grep of the agent binary per request is neither
cheaper nor safer than a generated table. PASS.

Hat wave: SKIPPED — clear-ask fast path, same recorded exception the two
preceding features took.

## Load-bearing claims

| # | Claim | Anchor | Label | Evidence |
|---|-------|--------|-------|----------|
| 1 | Claude Code registers its slash commands as `type:"local"\|"local-jsx"\|"prompt"` objects carrying name/description | `~/.local/share/claude/versions/2.1.251` | ran | extraction tool run this session: 103 commands, 92 with descriptions |
| 2 | A memory-written built-in list is wrong | same binary | ran | `/cost`, `/review`, `/doctor`, `/todos`, `/rewind`, `/vim` are NOT registered in 2.1.251 |
| 3 | An unbounded read window mis-pairs descriptions | same binary | ran | first pass described `/login` as "Sign out from your Anthropic account" (that is `/logout`'s); bounding the window at the next `name:`/`type:` marker fixed it |
| 4 | Pane kind is reachable only through `agents[]` | `crates/waggledance/src/herdr/wire.rs:46-52`, `:151` | read | `Agent` carries `kind`; `Pane` does not |
| 5 | A shell pane never appears in `agents[]` | `crates/waggledance/src/herdr/wire.rs:145-149` | read | the struct's own doc says so |
| 6 | A kind→vendor substring classification already exists | `crates/waggledance/src/views.rs:792-812` | read | `bee_hub_agent_logo` |
| 7 | The menu badges entries by `kind`, so a third value needs no JS change | `crates/waggledance/assets/app.js:148+` | read | `.slash-item__kind` built from the entry's own `kind` |

## Discovery

No open questions. The extracted dataset lives at
`/tmp/.../scratchpad/builtins.json` for this session; the committed tool
regenerates it from any installed agent bundle.

## Proof

- Cell: `cargo test -p waggledance slash` — module fixtures plus the route
  tests, extended for the vendor join and the builtin shadow order.
- Merge: `cargo test -p waggledance-core -p waggledance --no-fail-fast`.
- Whole path: after install, `curl '/p/<id>/_slash?pane=<agent pane>'` must
  carry `/model` with `kind: "builtin"`, and the same call for a shell pane
  must carry none.

## Later slices

None planned. Headlines only: a `codex` table once that CLI is installed here;
rendering the argument hint in the menu row.
