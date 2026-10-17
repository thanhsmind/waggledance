# Hat wave synthesis — paseo-native-supervisor (plan step)

Five seats dispatched 2026-10-17 as `--kind advisor` (hat-facts-gaps, hat-alternatives,
hat-user-impact, hat-risks, hat-value). All five returned inside the budget; none dropped.

## PLAN CHECK
Work: slice 1 (pns-1 … pns-6)

### STRUCTURE
BLOCKERS (all applied to plan.md before the gate):
- claims rows 8, 14, 16 quoted trimmed bytes → re-quoted with their indentation.
- pns-1 and pns-3 verify strings carried two cargo filters → one filter per command, chained with `&&`.
- pns-1 rested on a `config get` verb that does not exist → dropped; the token moves to the owner-only `<data_dir>/hook.token` file (the `telegram.token` pattern), so nothing in `Config` needs redacting.
- `supervisor report` and the rollup `supervisor` field went past "two waggledance additions" → boundary restated in CONTEXT.md (D11); rollup `supervisor` and `unassigned_leaders` dropped from pns-3.
- the template's "Leader mode" section and the deployed `~/.agents/skills/waggledance-supervisor` had no owner → pns-5 rewrites the section; `waggledance setup openclaw` (pns-4) refreshes the deployed copy.
- `/hooks/agent` is a model turn, not a pipe → new claim row; D11 records the verbatim-forward instruction and the known limit.
- the supervisor's Paseo MCP access and the `run -d --json` shape were unproven → probed live (agent 57a65f50, archived): CLAUDE.md in cwd was read, 60+ `mcp__paseo__*` tools present; `run -d --json` captured in `.bee/tmp/pns/npm-run-detached.out`; the AppImage wrapper's `run` printed only log lines and created no agent (`.bee/tmp/pns/run-detached.out`), confirming paseo-auto-wiring D4.
WARNINGS (applied):
- pns-2 proof counted lines → a structure check plus pns-6 as the behaviour proof.
- D4 and D9 were never driven live → pns-6 drives question→advisor, escalation→reply, accept→archive, and asserts zero advisors.
- worktree cwd mapping → `leaders` maps with the rollup's worktree list.
- pns-5 deps gain pns-4. `ensure` passes `-g`, a prompt positional, and scrubs `PASEO_AGENT_*`.

### CELLS (reviewed: 6, leader's cold-pickup pass)
CRITICAL FLAGS: pns-4 wrote host files by hand (the write guard denies writes outside the worktree) → re-shaped as `waggledance setup openclaw`, the repo's own install pattern. pns-6 lacked the question and accept paths → added.
MINOR FLAGS: pns-3 `last_run` is daemon-path only → carried as `last_daemon_run`. pns-1 CLAUDE.md change under a live agent → `instructions_stale` flag.
CLEAN CELLS: pns-2, pns-5

### Findings adopted from seats
- value: via label `waggledance.via=supervisor` so the frozen daemon path and the supervisor never share a leader by accident (D10); pns-2's proof is pns-6.
- alternatives: keep `report` as a CLI for a deterministic body and bearer header (not "D6 forbids env"); drop rollup `supervisor` and `unassigned_leaders`; reuse the panes three-state pattern.
- user-impact: tagged, verbatim, Vietnamese reports; reply kinds reply/accept/redo with a corr tag (D12); acks for accepted, closed, blocked, error; failure rows for Paseo down and hook 401/5xx with ledger retry; permission requests escalate as blocked; heartbeat in slice 1.
- risks: graceful clear (pause handoff, then archive) (D13); append-only ledger, map is truth; token in an owner-only file; backup of `openclaw.json` chmod 600; write token first, flip gateway second, curl third; advisor label `waggledance.role=advisor`; `paseo ls` timeout row.

### Dismissed
- facts-gaps M4 "`paseo inspect` shows ParentAgentId but not labels" needs a row — not load-bearing once D10's via label replaced the parentage check.
- risks F1 option "a project is daemon-driven or supervisor-driven, never both" as a locked rule — the via label plus a documented limit covers it; the daemon path is frozen and used only by the plugin request box.
- risks F3 "re-create and poll" as a first-slice requirement — partly: the heartbeat lands in pns-2 (slice 1); `parent_agent_id` in `leaders` is not added (one inspect per leader per call).
- value "drop `last_run`" — D8 names it; carried as `last_daemon_run` and documented as daemon-path only.
- user-impact "Telegram length limits" — pns-2 prose only (report truncates at 3500 chars and points to the leader's Paseo chat); no code.

SUMMARY: the shape stays six cells. pns-4 changes from hand-deployed host files to a `setup openclaw` target. Three new decisions (D11 relay limit, D12 protocol, D13 operational rules) and one earlier one (D10) absorb the product findings. The claims table gains five rows and corrects three.
