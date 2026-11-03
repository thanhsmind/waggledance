# Paseo Support — Context

**Feature slug:** paseo-support
**Date:** 2026-08-27
**Shaping session:** complete
**Scope:** Standard
**Domain types:** READ · SEE

## Feature Boundary

Waggledance detects agents that paseo is currently running by reading paseo's
on-disk agent store, maps each live agent to a waggledance project by its
working directory, and — for any agent whose project waggledance does not yet
track — surfaces an explicit invitation to register that project. It ends at
display and registration: waggledance never sends input to, opens, or otherwise
controls a paseo agent.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Display-only: detect paseo agents and map them to projects; never send input to or control them. Interactive control is out of scope. | Control would need the paseo daemon API and paseo's own terminal ownership; detection+mapping is a self-contained slice. |
| D2 | Read paseo agents from the on-disk per-agent JSON store at `~/.paseo/agents/<project-slug>/<uuid>.json`. Do not depend on the paseo daemon HTTP API (`127.0.0.1:6767`). | Disk store is always readable with no daemon/network/auth; a display view tolerates last-written status. |
| D3 | When a live paseo agent's `cwd` belongs to a project waggledance does not yet track, surface an explicit "Register" action for that project. Do not auto-register. | User keeps control over which projects enter the tracked set. |
| D4 | Show only LIVE paseo agents: `archivedAt` absent AND `lastStatus != "closed"`. Exclude closed/archived records. | Paseo stamps `archivedAt` and `lastStatus: "closed"` when an agent ends; those two fields are the liveness filter. |
| D5 | Map a paseo agent to a project by its `cwd`: the project whose `root_path` contains the agent's `cwd` (the existing path-boundary containment rule), matching waggledance's current pane→project mapping. | Reuses the established `paths_boundary::Boundary` containment used for herdr panes; no new matching concept. |

### Agent's Discretion

- The exact surface and layout of the paseo-agent display and the Register
  action within waggledance's browser UI (the index/board page where herdr
  agents already render) — constrained to the existing agent-display surface,
  not a new page.
- Whether to expose the mapped paseo agents through an MCP tool in addition to
  the browser view — planning decides based on effort; the browser view is the
  committed surface.
- Precise handling of a `lastStatus: "running"` record whose `lastActivityAt`
  is very old (a stale record from a crashed daemon): trust the record per D4,
  optionally annotate age. No extra liveness probe is required.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Paseo agent | One record in paseo's on-disk store (`~/.paseo/agents/<slug>/<uuid>.json`) representing an agent process paseo manages. |
| Live (paseo agent) | A paseo agent record with `archivedAt` absent and `lastStatus != "closed"`. |
| Tracked project | A project present in waggledance's `projects` table (`repository.rs`), keyed by `root_path`. |

## Specific Ideas And References

- Paseo agent record shape (verified from a live store):
  `id`, `provider` (`claude`/`codex`/…), `cwd`, `workspaceId`, `title`,
  `lastStatus` (`running`/`closed`), `lastActivityAt`, `archivedAt`,
  `requiresAttention`, `config.model`, `runtimeInfo.sessionId`.
- Paseo daemon identity (for reference only, not read by D2):
  `~/.paseo/paseo.pid` carries `listen: "127.0.0.1:6767"`.

## Existing Code Context

From the quick scout only.

### Reusable Assets

- `crates/waggledance-core/src/repository.rs` — `Project`, `upsert_project`,
  `find_project_by_root`, `list_projects`. Registration = `upsert_project`; the
  "is this project tracked?" check = `find_project_by_root`.
- `crates/waggledance-core/src/paths_boundary.rs` — `Boundary` containment used
  today to decide which project a pane's cwd belongs to; the join key for D5.
- `crates/waggledance/src/server.rs` — `index_page` / board rendering that today
  lists herdr agents per project; the integration surface for the paseo list.

### Established Patterns

- cwd→project mapping via `paths_boundary::Boundary` (`project_pane_cwd_in_boundary`
  in `server.rs`) — reuse for paseo agents.
- The viewer already auto-registers a project on first file view (CLAUDE.md);
  D3 deliberately chooses an explicit action here instead.

### Integration Points

- New reader module (planning names the path; candidate
  `crates/waggledance-core/src/paseo.rs`) that enumerates and filters the paseo
  store and returns live agents with their `cwd`.
- `server.rs` view assembly — join live paseo agents to `list_projects`, and
  compute the untracked-project set that gets a Register action.

## Canonical References

- `~/.paseo/agents/<project-slug>/<uuid>.json` — the paseo agent store D2 reads.
- `~/.paseo/config.json`, `~/.paseo/paseo.pid` — paseo daemon config/identity
  (context only).

## Outstanding Questions

### Resolve Before Planning

- None. The four forks are locked (D1–D4) and the mapping mechanism is D5.

### Deferred To Planning

- [ ] Reader module placement and public shape (core vs binary crate) — the
      store read is synchronous filesystem work with no async runtime, so it
      fits `waggledance-core`; planning confirms against the crate's
      no-async-runtime constraint.
- [ ] Exact browser surface for the paseo list + Register control, and whether
      an MCP tool is added — planning decides within the Agent's Discretion
      constraints above.
- [ ] Registration flow wiring: which route/handler the Register action posts
      to and how it calls `upsert_project`.

## Deferred Ideas

- Interactive control of paseo agents (send input / open a pane) — needs the
  paseo daemon API; explicitly out of scope per D1.
- Reading the paseo daemon API (`:6767`) for real-time status — deferred in
  favor of the disk store per D2.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs D1–D5 are stable. Planning reads
the locked decisions, the code context, and the deferred-to-planning questions,
routes the lane, creates the feature worktree, and presents Gate 2.
