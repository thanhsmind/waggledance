//! Minimal MCP server over stdio (newline-delimited JSON-RPC 2.0).
//! Exposes the agent-facing query surface (PRD §5.5): the original write-side
//! `waggledance_view_file`, three read-only query tools —
//! `waggledance_search`, `waggledance_projects`, `waggledance_ask_state`
//! (mcp-query-surface D3) — and the orchestrator-dispatch write surface
//! (`docs/history/orchestrator-dispatch/plan.md` D2): `waggledance_dispatch`,
//! `waggledance_await`, `waggledance_runs`. Hand-rolled to avoid a heavy SDK
//! dependency; the protocol surface here is intentionally small.

use crate::herdr::{socket::SocketHerdr, Herdr};
use crate::orchestrate;
use crate::paseo;
use crate::runtime;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;
use waggledance_core::bee;
use waggledance_core::config::{registry_db_path, resolve_data_dir};
use waggledance_core::domain::{Project, Run};
use waggledance_core::indexer::now_rfc3339;
use waggledance_core::notify_store::NotifyStore;
use waggledance_core::{Config, Engine, Error, SqliteStore};

/// Default `waggledance_search` hit cap when the caller does not pass `limit`.
const DEFAULT_SEARCH_LIMIT: usize = 10;

/// Default `waggledance_runs` row cap per project when the caller does not
/// pass `limit` (no `limit` field is exposed on the tool at all — every
/// listed project is capped the same way).
const RUNS_LIST_LIMIT: usize = 100;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// The owned tokio runtime + herdr socket client the dispatch-family tools
/// need — `mcp.rs`'s stdio loop is otherwise sync with no herdr client at
/// all (module doc). Built lazily on the first `waggledance_dispatch`/
/// `waggledance_await` call (`orchestration_handle`) so a session that never
/// touches orchestration never pays for either; `waggledance_runs` needs
/// neither and never triggers this.
struct Orchestration {
    runtime: tokio::runtime::Runtime,
    herdr: SocketHerdr,
    notify_store: Option<NotifyStore>,
}

impl Orchestration {
    fn init(cfg: &waggledance_core::config::TerminalConfig) -> std::result::Result<Self, String> {
        Self::init_with_override(cfg, None)
    }

    fn init_with_override(
        cfg: &waggledance_core::config::TerminalConfig,
        override_dir: Option<&Path>,
    ) -> std::result::Result<Self, String> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("failed to start the orchestration runtime: {e}"))?;
        // Mirrors `server.rs`'s own best-effort fallback: an unresolvable
        // default socket path never blocks startup, it just means every
        // herdr call below fails with a named "unavailable" error instead
        // of a crash.
        let socket_path = crate::herdr::socket::default_socket_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("/nonexistent/herdr.sock"));
        let notify_store = open_notify_store(cfg, override_dir);
        Ok(Orchestration {
            runtime,
            herdr: SocketHerdr::new(socket_path),
            notify_store,
        })
    }
}

/// D7/D9 outbox for the MCP stdio process (dbn-4): Orchestration opens its own
/// NotifyStore against the SAME database file the server uses.
/// Per D6 the opt-in switch still governs: open the store and pass Some only
/// when the notify switch is enabled in the configuration, and None when it is off.
/// A store that fails to open falls back to None and logs one warning, matching
/// the degrade-rather-than-fail shape server.rs uses.
fn open_notify_store(
    cfg: &waggledance_core::config::TerminalConfig,
    override_dir: Option<&Path>,
) -> Option<NotifyStore> {
    if !cfg.notify_enabled {
        return None;
    }
    let notify_store_path = waggledance_core::config::notify_store_path_override(override_dir);
    NotifyStore::open(&notify_store_path)
        .map_err(|e| {
            tracing::warn!("notify outbox open failed ({e}); notifications disabled");
            e
        })
        .ok()
}

/// Lazily build (or reuse) the shared [`Orchestration`] handle.
fn orchestration_handle<'a>(
    slot: &'a mut Option<Orchestration>,
    cfg: &waggledance_core::config::TerminalConfig,
) -> std::result::Result<&'a Orchestration, String> {
    if slot.is_none() {
        *slot = Some(Orchestration::init(cfg)?);
    }
    Ok(slot.as_ref().expect("just initialized above"))
}

pub fn run() -> Result<()> {
    let engine = Engine::new(SqliteStore::open(&registry_db_path())?, Config::load());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut orchestration: Option<Orchestration> = None;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Notifications have no id and expect no response.
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => Some(ok(
                id,
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "waggledance", "version": env!("CARGO_PKG_VERSION") }
                }),
            )),
            "tools/list" => Some(ok(
                id,
                json!({
                    "tools": [
                        view_file_schema(),
                        search_schema(),
                        projects_schema(),
                        ask_state_schema(),
                        dispatch_schema(),
                        await_schema(),
                        runs_schema(),
                        task_schema()
                    ]
                }),
            )),
            "tools/call" => Some(handle_tool_call(id, &engine, &mut orchestration, &req)),
            "ping" => Some(ok(id, json!({}))),
            _ if id.is_some() => Some(err(id, -32601, "method not found")),
            _ => None, // notification
        };

        if let Some(resp) = response {
            writeln!(stdout, "{resp}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn view_file_schema() -> Value {
    json!({
        "name": "waggledance_view_file",
        "description": "Make a markdown file viewable in the browser and return its URL. \
    Auto-registers the project on first use and indexes the file immediately. \
    Pass the project root and the file path relative to that root.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "project_root": { "type": "string", "description": "Absolute path to the project root" },
                "relative_path": { "type": "string", "description": "Markdown file path relative to project_root" }
            },
            "required": ["project_root", "relative_path"]
        }
    })
}

fn search_schema() -> Value {
    json!({
        "name": "waggledance_search",
        "description": "Full-text search across every registered project's indexed markdown \
    (or one project, when `project` is given). Re-indexes changed files in the \
    searched project(s) before answering and reports any project whose refresh \
    failed in `structuredContent.refresh` — hits still return, but a failed \
    project's results may lag disk. Each hit carries a rich, <mark>-highlighted \
    excerpt — enough to answer without a follow-up read, never a bare path list \
    or a whole file.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Full-text query" },
                "project": { "type": "string", "description": "Optional project id to narrow the search to" },
                "limit": { "type": "integer", "description": "Max hits to return (default 10)" }
            },
            "required": ["query"]
        }
    })
}

fn projects_schema() -> Value {
    json!({
        "name": "waggledance_projects",
        "description": "List every registered project: id, name, root path, indexed file \
    count, and when it was last seen.",
        "inputSchema": {
            "type": "object",
            "properties": {}
        }
    })
}

fn ask_state_schema() -> Value {
    json!({
        "name": "waggledance_ask_state",
        "description": "Ask waggledance for a project's parsed bee state (active feature, \
    phase, open/blocked cells, recent decisions, sessions, handoff, attention) \
    without reading any .bee file yourself. Omit `project` to get a rollup across \
    every registered project. Two fields describe the fleet rather than the store: \
    `herding` names the agent kinds this project offers — {default, agents, resolvable} \
    as LABELS, never the argv behind them, and a label listed in `agents` but absent \
    from `resolvable` is one that will not start today; `panes` is the project's own \
    live pane inventory (pane_id, kind, status, bee_state, bee_feature, workspace, tab), \
    which is how you find an idle pane to reuse instead of spawning a new one. `panes` \
    is ABSENT when the terminal surface is switched off, `null` beside a `panes_error` \
    reason when herdr cannot be read — the bee state is complete either way — and an \
    array otherwise. Seeing a pane grants nothing: dispatching still requires the \
    project's own orchestration opt-in. `leaders` lists the Paseo agents labelled \
    waggledance.role=leader whose cwd sits in the project's root or one of its worktrees, \
    oldest first: {agent_id, status, cwd, created, via, labels, waiting_on, \
    last_daemon_run}, where `via` is `wd` for a leader carrying \
    waggledance.via=wd and `daemon` otherwise. `leaders` is ABSENT when \
    `[paseo] enabled = false`, `null` beside a `leaders_error` reason when Paseo cannot \
    be read, and an array (possibly empty) beside `leaders_skipped`, the count of \
    leaders under no registered project, otherwise.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "project": { "type": "string", "description": "Optional project id to narrow to a single project's full snapshot" }
            }
        }
    })
}

fn dispatch_schema() -> Value {
    json!({
        "name": "waggledance_dispatch",
        "description": "Dispatch a task to an agent pane in a project that has opted into \
    orchestrator dispatch (D6 — refused, naming the remedy, when the project or the terminal \
    surface has not been switched on). Either spawn a fresh agent via an operator-configured \
    preset label, target an already-running pane by id, or hand the task to the project's Paseo \
    leader with `leader: true` (queued and delivered by the waggledance daemon once the leader is \
    idle) — exactly one of `preset`/`pane_id`/`leader`, and never a raw command. Returns a \
    `run_id` immediately; poll completion with `waggledance_await`.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "project": { "type": "string", "description": "Registered project id" },
                "preset": { "type": "string", "description": "Operator-configured agent preset label to spawn a fresh pane — mutually exclusive with pane_id" },
                "pane_id": { "type": "string", "description": "An already-running pane id to target — mutually exclusive with preset" },
                "leader": { "type": "boolean", "description": "true sends the task to the project's Paseo leader — mutually exclusive with preset and pane_id" },
                "task": { "type": "string", "description": "The task text sent to the agent" },
                "request_id": { "type": "string", "description": "Caller-chosen id for this request — pass one on every call so a retry never sends twice" }
            },
            "required": ["project", "task"]
        }
    })
}

fn await_schema() -> Value {
    json!({
        "name": "waggledance_await",
        "description": "Block until a dispatched run completes, blocks on a human, or the \
    timeout elapses — whichever comes first, at most 60 seconds regardless of the \
    `timeout_seconds` requested (a longer request is silently clamped, never honored, never an \
    error). Returns `status` (working/done/blocked/timeout) and the transcript `delta` since \
    dispatch.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "run_id": { "type": "string", "description": "The run_id returned by waggledance_dispatch" },
                "timeout_seconds": { "type": "integer", "description": "Max seconds to wait, clamped to 60 server-side (default 60)" }
            },
            "required": ["run_id"]
        }
    })
}

fn runs_schema() -> Value {
    json!({
        "name": "waggledance_runs",
        "description": "List dispatched runs and their current status (read-only, D8), \
    optionally filtered to one project.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "project": { "type": "string", "description": "Optional project id to narrow to a single project's runs" }
            }
        }
    })
}

fn task_schema() -> Value {
    json!({
        "name": "waggledance_task",
        "description": "Follow up on a task wd handed to a project's Paseo leader, by its corr. \
    `reply` and `redo` queue the user's words for that leader; `accept` closes the task and \
    archives its leader when nothing else uses it; `allow` and `deny` answer the leader's \
    permission request `req_id` (the newest unanswered one when absent). Only ever pass the \
    user's own allow or deny.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "corr": { "type": "string", "description": "The task's correlation id" },
                "action": { "type": "string", "enum": ["reply", "redo", "accept", "allow", "deny"] },
                "text": { "type": "string", "description": "The user's words — required for reply and redo" },
                "req_id": { "type": "string", "description": "The permission request to answer — allow and deny only" }
            },
            "required": ["corr", "action"]
        }
    })
}

fn handle_tool_call(
    id: Option<Value>,
    engine: &Engine,
    orchestration: &mut Option<Orchestration>,
    req: &Value,
) -> Value {
    let args = req
        .get("params")
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(json!({}));
    let name = req
        .get("params")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("");
    match name {
        "waggledance_view_file" => handle_view_file(id, engine, &args),
        "waggledance_search" => handle_search(id, engine, &args),
        "waggledance_projects" => handle_projects(id, engine),
        "waggledance_ask_state" => handle_ask_state(id, engine, orchestration, &args),
        "waggledance_dispatch" => handle_dispatch(id, engine, orchestration, &args),
        "waggledance_await" => {
            handle_await(id, engine, orchestration, &args, &resolve_data_dir(None))
        }
        "waggledance_runs" => handle_runs(id, engine, &args),
        "waggledance_task" => handle_task(id, engine, orchestration, &args),
        _ => err(id, -32602, "unknown tool"),
    }
}

fn handle_view_file(id: Option<Value>, engine: &Engine, args: &Value) -> Value {
    let root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let rel = args
        .get("relative_path")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if root.is_empty() || rel.is_empty() {
        return tool_error(id, "project_root and relative_path are required");
    }

    match engine.view_file(Path::new(root), rel) {
        Ok(vf) => {
            // Ensure a daemon is up so the URL is actually viewable. When the
            // daemon binds a wildcard host with no host_name override, this is
            // one URL per reachable machine IP so the caller can pick a routable
            // address; otherwise it is a single URL.
            let bases = runtime::ensure_daemon_bases();
            let urls: Vec<String> = bases
                .iter()
                .map(|base| format!("{base}/s/{}", vf.code))
                .collect();
            let long_urls: Vec<String> = bases
                .iter()
                .map(|base| format!("{base}{}", vf.url))
                .collect();
            // Primary URL kept for back-compat with clients reading `url`.
            let primary = urls.first().cloned().unwrap_or_default();
            let text = viewable_text(&urls, &vf.rel_path, &vf.project_id);
            ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": {
                        "url": primary,
                        "urls": urls,
                        "long_url": long_urls.first().cloned().unwrap_or_default(),
                        "long_urls": long_urls,
                        "path": vf.url,
                        "code": vf.code,
                        "project_id": vf.project_id
                    }
                }),
            )
        }
        Err(e) => tool_error(id, &format!("view_file failed: {e}")),
    }
}

/// `waggledance_search`: FTS5 hits over one or every registered project.
///
/// D4 (never silently stale): re-indexes the searched project(s) before
/// querying — just the filtered project, or every registered project when
/// unfiltered (D1) — and reports which projects refreshed cleanly and which
/// failed (review P1-2: a refresh failure must surface, never masquerade as
/// a fresh result). D2 (rich, not bare): each hit carries `project_id`,
/// `rel_path`, `title`, a `<mark>`-highlighted `excerpt`, and `score` — no
/// whole-file content, no bare path list.
fn handle_search(id: Option<Value>, engine: &Engine, args: &Value) -> Value {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    if query.trim().is_empty() {
        return tool_error(id, "query is required");
    }
    let project = args
        .get("project")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_SEARCH_LIMIT);

    let refresh_results: Vec<(String, Result<usize, Error>)> = match project {
        Some(project_id) => {
            if engine.get_project(project_id).ok().flatten().is_none() {
                return tool_error(id, &format!("no such project: {project_id}"));
            }
            // The search itself still runs either way — a refresh failure is
            // reported, not fatal (results just stay as fresh as the last
            // successful index for that project).
            vec![(project_id.to_string(), engine.refresh_stale(project_id))]
        }
        None => {
            let Ok(projects) = engine.list_projects() else {
                return tool_error(id, "could not list registered projects");
            };
            projects
                .iter()
                .map(|p| (p.id.clone(), engine.refresh_stale(&p.id)))
                .collect()
        }
    };
    let refresh = summarize_refresh(refresh_results);

    match engine.search(query, project, limit) {
        Ok(hits) => {
            let mut text = if hits.is_empty() {
                format!("No hits for {query:?}.")
            } else {
                format!("{} hit(s) for {query:?}.", hits.len())
            };
            if let Some(warning) = &refresh.warning {
                text.push_str("; ");
                text.push_str(warning);
            }
            let structured_hits: Vec<Value> = hits
                .iter()
                .map(|h| {
                    json!({
                        "project_id": h.project_id,
                        "rel_path": h.rel_path,
                        "title": h.title,
                        "excerpt": h.excerpt,
                        "score": h.score
                    })
                })
                .collect();
            ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": { "hits": structured_hits, "refresh": refresh.structured }
                }),
            )
        }
        Err(e) => tool_error(id, &format!("search failed: {e}")),
    }
}

/// The per-project refresh outcome for a `waggledance_search` call, folded
/// into the shape the response carries and the one-line warning appended to
/// the human-readable text when at least one project's refresh failed
/// (review P1-2 — a stale-serving refresh failure must never be silent).
struct RefreshSummary {
    /// `structuredContent.refresh`: `{"refreshed": <ok count>, "failed": [...]}`.
    structured: Value,
    /// `Some` only when `failed` is non-empty.
    warning: Option<String>,
}

/// Pure so the failure shape is testable without a real store error: fold a
/// project id + its `refresh_stale` outcome into the response's `refresh`
/// field and, when any project failed, the warning appended to the text.
fn summarize_refresh(results: Vec<(String, Result<usize, Error>)>) -> RefreshSummary {
    let mut refreshed = 0usize;
    let mut failed: Vec<Value> = Vec::new();
    let mut failed_ids: Vec<String> = Vec::new();
    for (project_id, result) in results {
        match result {
            Ok(_) => refreshed += 1,
            Err(e) => {
                failed.push(json!({ "project_id": project_id, "error": e.to_string() }));
                failed_ids.push(project_id);
            }
        }
    }
    let warning = if failed_ids.is_empty() {
        None
    } else {
        Some(format!(
            "warning: refresh failed for {} — results may lag disk for those projects",
            failed_ids.join(", ")
        ))
    };
    RefreshSummary {
        structured: json!({ "refreshed": refreshed, "failed": failed }),
        warning,
    }
}

/// `waggledance_projects`: the registry, as-is. `file_count` reflects the
/// index as it stands and may lag until the next search touches a project
/// (recorded narrowing of D4 — plan.md Approach 3).
fn handle_projects(id: Option<Value>, engine: &Engine) -> Value {
    let projects = match engine.list_projects() {
        Ok(p) => p,
        Err(e) => return tool_error(id, &format!("could not list registered projects: {e}")),
    };
    let entries: Vec<Value> = projects
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "root_path": p.root_path.display().to_string(),
                "file_count": engine.file_count(&p.id).unwrap_or(0),
                "last_seen_at": p.last_seen_at
            })
        })
        .collect();
    let text = format!("{} registered project(s).", entries.len());
    ok(
        id,
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": { "projects": entries }
        }),
    )
}

/// `waggledance_ask_state`: parsed bee state, so the caller never opens a
/// `.bee/` file itself. With `project`: the full digest for that project
/// (`bee::read_snapshot`), including a project with no `.bee/` at all —
/// reported absent, never an error. Without `project`: a rollup across every
/// registered project (D1), via `bee::read_rollup`; `BeeProjectRollup` carries
/// no root/id of its own, so results are labeled by zipping the input roots'
/// projects back in by index (plan.md Approach 4).
fn handle_ask_state(
    id: Option<Value>,
    engine: &Engine,
    orchestration: &mut Option<Orchestration>,
    args: &Value,
) -> Value {
    let project = args
        .get("project")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());

    match project {
        Some(project_id) => {
            let Some(p) = engine.get_project(project_id).ok().flatten() else {
                return tool_error(id, &format!("no such project: {project_id}"));
            };
            // A rollup rather than a bare snapshot: the pane join reads the
            // project's worktree list off it, and `rollup.snapshot` is the
            // same read `read_snapshot` would have done.
            let mut rollups = bee::read_rollup(std::slice::from_ref(&p.root_path));
            let rollup = rollups.pop().expect("one root in, one rollup out");
            let snapshot = &rollup.snapshot;
            let fleet = read_fleet_panes(engine, orchestration);
            let leaders = read_leaders(&engine.config.paseo);
            let mut claims: Vec<LeaderClaim> = engine
                .list_projects()
                .unwrap_or_default()
                .iter()
                .map(|other| (canonical(&other.root_path), other.id.clone()))
                .collect();
            claims.extend(worktree_claims(&p, &rollup));
            let mut digest = ask_state_digest(&p.id, snapshot);
            attach_panes(&mut digest, &p, &rollup, &fleet);
            attach_leaders(&mut digest, engine, &p, &rollup, &claims, &leaders);
            let text = if !snapshot.present {
                format!("{}: no .bee/ directory (absent)", p.id)
            } else {
                let state = snapshot.state.as_ref();
                format!(
                    "{}: feature={:?} phase={:?} mode={:?} waiting_on_live={} \
                     doing={} waiting={} stuck={} done={}",
                    p.id,
                    state.and_then(|s| s.feature.as_deref()),
                    state.and_then(|s| s.phase.as_deref()),
                    state.and_then(|s| s.mode.as_deref()),
                    state.map(|s| s.waiting_on_live).unwrap_or(false),
                    snapshot.buckets.doing.len(),
                    snapshot.buckets.waiting.len(),
                    snapshot.buckets.stuck.len(),
                    snapshot.buckets.done.len(),
                )
            };
            ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": { "project": digest }
                }),
            )
        }
        None => {
            let Ok(projects) = engine.list_projects() else {
                return tool_error(id, "could not list registered projects");
            };
            let roots: Vec<std::path::PathBuf> =
                projects.iter().map(|p| p.root_path.clone()).collect();
            let rollups = bee::read_rollup(&roots);
            // D4: one fleet read for the whole rollup, filtered per project
            // below — never one herdr call per project.
            let fleet = read_fleet_panes(engine, orchestration);
            let leaders = read_leaders(&engine.config.paseo);
            let claims: Vec<LeaderClaim> = projects
                .iter()
                .map(|p| (canonical(&p.root_path), p.id.clone()))
                .chain(
                    projects
                        .iter()
                        .zip(rollups.iter())
                        .flat_map(|(p, rollup)| worktree_claims(p, rollup)),
                )
                .collect();
            let digests: Vec<Value> = projects
                .iter()
                .zip(rollups.iter())
                .map(|(p, rollup)| {
                    let mut digest = ask_state_digest(&p.id, &rollup.snapshot);
                    attach_panes(&mut digest, p, rollup, &fleet);
                    attach_leaders(&mut digest, engine, p, rollup, &claims, &leaders);
                    digest
                })
                .collect();
            let text = format!("bee state rollup across {} project(s).", digests.len());
            ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": { "projects": digests }
                }),
            )
        }
    }
}

/// One project's `waggledance_ask_state` answer, built from a
/// [`bee::BeeSnapshot`]: feature/phase/mode, whether a human is currently
/// being waited on, cell bucket counts with doing/stuck detail, recent
/// decisions, sessions, handoff, and attention items. A project whose
/// `.bee/` is absent still gets this shape — every field just reads empty.
fn ask_state_digest(project_id: &str, snapshot: &bee::BeeSnapshot) -> Value {
    let state = snapshot.state.as_ref();
    let cell_line = |c: &bee::BeeCell| json!({ "id": c.id, "title": c.title });
    json!({
        "project_id": project_id,
        "present": snapshot.present,
        "feature": state.and_then(|s| s.feature.clone()),
        "phase": state.and_then(|s| s.phase.clone()),
        "mode": state.and_then(|s| s.mode.clone()),
        "waiting_on_live": state.map(|s| s.waiting_on_live).unwrap_or(false),
        "active": snapshot.active,
        "cell_counts": {
            "doing": snapshot.buckets.doing.len(),
            "waiting": snapshot.buckets.waiting.len(),
            "stuck": snapshot.buckets.stuck.len(),
            "done": snapshot.buckets.done.len()
        },
        "doing": snapshot.buckets.doing.iter().map(cell_line).collect::<Vec<_>>(),
        "stuck": snapshot.buckets.stuck.iter().map(cell_line).collect::<Vec<_>>(),
        "recent_decisions": snapshot.decisions.recent.iter().map(|d| json!({
            "id": d.id,
            "date": d.date,
            "decision": d.decision
        })).collect::<Vec<_>>(),
        "sessions": snapshot.sessions.iter().map(|s| json!({
            "id": s.id,
            "live": s.live,
            "heartbeat_age_minutes": s.heartbeat_age_minutes
        })).collect::<Vec<_>>(),
        "handoff": snapshot.handoff.as_ref().map(|h| json!({
            "kind": h.kind,
            "written_at": h.written_at,
            "next_action": h.next_action
        })),
        "attention": snapshot.attention.iter().map(|a| json!({
            "severity": format!("{:?}", a.severity),
            "title": a.title,
            "detail": a.detail
        })).collect::<Vec<_>>(),
        // ask-state-fleet-read D1/D2: which agent kinds this project offers,
        // as labels. Serialized straight off `BeeHerdingRegistry` rather than
        // re-spelled field by field here, so the type's own no-argv guarantee
        // (and the test that proves it) covers these bytes too. `null` for a
        // project with no `.bee/` and for one whose config has no `herding`
        // block — one absence, one branch for the consumer.
        "herding": snapshot.config.as_ref().and_then(|c| c.herding.clone())
    })
}

/// What ONE `ask_state` call learned about herdr, read once and reused for
/// every project in the answer (ask-state-fleet-read D4).
///
/// herdr's snapshot is already the whole machine's flat pane list, so asking
/// it once per project would cost N times as much for the same bytes. The
/// three variants are the three things an answer can honestly say, and they
/// are deliberately not collapsed: "the switch is off" and "the transport
/// broke" are different facts and a consumer must be able to tell them apart.
enum FleetPanes {
    /// `terminal.enabled` is off, so nobody looked (D6). The key is omitted
    /// entirely — an empty array here would claim there are no panes.
    Off,
    /// herdr could not be read at all (D5). Every project reports `null`
    /// beside this reason; the bee half of the answer is unaffected.
    Unreachable(String),
    /// One snapshot, filtered per project on the way out (D4).
    Snapshot(Box<crate::herdr::Snapshot>),
}

/// Read the fleet once for this tool call.
///
/// Ordered so the cheap refusal comes first: with the terminal family off
/// there is no herdr client to ask, and `ask_state` must stay exactly the
/// filesystem-only tool it is today for every caller that never set that
/// switch (D6).
fn read_fleet_panes(engine: &Engine, orchestration: &mut Option<Orchestration>) -> FleetPanes {
    if !engine.config.terminal.enabled {
        return FleetPanes::Off;
    }
    // Through the LAZY builder, never `orchestration.as_ref()`: in a session
    // that only ever calls ask_state, nothing else builds the handle, so
    // reading the slot directly answers "no client" on every call and the
    // whole feature degrades to null forever. Caught by running the built
    // binary against the real registry — the unit tests could not see it,
    // because they hand this function the slot they want.
    let orch = match orchestration_handle(orchestration, &engine.config.terminal) {
        Ok(orch) => orch,
        // A handle that will not build is D5 like any other unreachable
        // transport, carrying the init error's own words.
        Err(e) => return FleetPanes::Unreachable(e),
    };
    match orch.runtime.block_on(orch.herdr.snapshot()) {
        Ok(snapshot) => FleetPanes::Snapshot(Box::new(snapshot)),
        // The transport's own words, never a generic failure — a caller that
        // loses the pane list still needs to know why.
        Err(e) => FleetPanes::Unreachable(format!("herdr snapshot failed: {e}")),
    }
}

/// One project's pane inventory as `ask_state` publishes it
/// (ask-state-fleet-read D3).
///
/// Pure on purpose, and this is the whole testability story for this
/// feature: `Orchestration` holds a concrete `SocketHerdr` rather than a
/// trait object, so the herdr-touching path cannot be faked at the handler —
/// the same reason `orchestrate.rs` is proved against `FakeHerdr` while
/// `mcp.rs`'s own dispatch tests only reach the pre-herdr refusals. Handed a
/// snapshot and a root, this function is provable with neither.
///
/// Containment is [`server::project_panes`]'s, not a second copy: the panes a
/// project may see are exactly the ones its own board surface shows.
fn project_panes_value(
    snapshot: &crate::herdr::Snapshot,
    project: &waggledance_core::domain::Project,
    rollup: &waggledance_core::bee::BeeProjectRollup,
    herdr_sessions: &std::collections::HashSet<String>,
) -> Value {
    let root = &project.root_path;
    let Ok(boundary) = waggledance_core::paths_boundary::Boundary::new(vec![root.to_path_buf()])
    else {
        // A root that will not resolve into a boundary can own no pane. That
        // is an empty inventory, not a failure of the whole answer.
        return json!([]);
    };
    let mut panes = crate::server::project_panes(snapshot, &boundary);
    // `project_panes` deliberately returns bee_state/bee_feature empty and
    // leaves the join to each caller. Applying it here is what makes the two
    // fields worth publishing at all: herdr's `status` is derived from the
    // screen, bee's is what its own hook runtime recorded, and an
    // orchestrator asking "is this pane really idle?" is asking for the
    // second one. Never a policy — the answer is reported, not acted on (D8).
    let activity = crate::server::project_bee_activity(project, rollup, herdr_sessions);
    crate::server::apply_bee_activity(&mut panes, &activity.pane);
    Value::Array(
        panes
            .iter()
            .map(|p| {
                json!({
                    "pane_id": p.pane_id,
                    "kind": p.kind,
                    "status": p.status,
                    "bee_state": p.bee_state.as_ref().map(|s| format!("{s:?}")),
                    "bee_feature": p.bee_feature,
                    "workspace": p.workspace,
                    "tab": p.tab
                })
            })
            .collect(),
    )
}

/// Fold this call's fleet read into one project's digest.
///
/// D6 omits, D5 nulls-with-a-reason, and only the third case carries an
/// inventory — see [`FleetPanes`] for why those stay three separate answers.
fn attach_panes(
    digest: &mut Value,
    project: &waggledance_core::domain::Project,
    rollup: &waggledance_core::bee::BeeProjectRollup,
    fleet: &FleetPanes,
) {
    let Some(object) = digest.as_object_mut() else {
        return;
    };
    match fleet {
        FleetPanes::Off => {}
        FleetPanes::Unreachable(reason) => {
            object.insert("panes".to_string(), Value::Null);
            object.insert("panes_error".to_string(), json!(reason));
        }
        FleetPanes::Snapshot(snapshot) => {
            // The session set is derived from this call's ONE snapshot (D4),
            // so it is built here rather than per project.
            let sessions = crate::server::herdr_session_ids(Some(snapshot));
            object.insert(
                "panes".to_string(),
                project_panes_value(snapshot, project, rollup, &sessions),
            );
        }
    }
}

enum LeadersRead {
    Off,
    Unreachable(String),
    Listed {
        all: Vec<paseo::ListedAgent>,
        wd: std::collections::HashSet<String>,
    },
}

type LeaderClaim = (std::path::PathBuf, String);

fn read_leaders(config: &waggledance_core::config::PaseoConfig) -> LeadersRead {
    if config.enabled == Some(false) {
        return LeadersRead::Off;
    }
    if let Some(reason) = paseo::paseo_unavailable(config) {
        return LeadersRead::Unreachable(reason);
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            return LeadersRead::Unreachable(format!("failed to start the Paseo read runtime: {e}"))
        }
    };
    let cli = paseo::PaseoCli::new(config);
    runtime
        .block_on(async {
            let all = cli.list_labelled(&paseo::role_leaders_args()).await?;
            let wd = paseo::labelled(paseo::role_leaders_args(), paseo::WD_LABEL);
            let wd = cli.list_labelled(&wd).await?;
            Ok::<_, paseo::PaseoError>(LeadersRead::Listed {
                all,
                wd: wd.into_iter().map(|a| a.id).collect(),
            })
        })
        .unwrap_or_else(|e| LeadersRead::Unreachable(e.to_string()))
}

fn canonical(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn agent_cwd(cwd: &str) -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    match cwd.strip_prefix('~') {
        Some(rest) => canonical(&home.join(rest.trim_start_matches('/'))),
        None => canonical(Path::new(cwd)),
    }
}

fn worktree_claims(
    project: &waggledance_core::domain::Project,
    rollup: &waggledance_core::bee::BeeProjectRollup,
) -> Vec<LeaderClaim> {
    let Some(parent) = project.root_path.parent() else {
        return Vec::new();
    };
    rollup
        .snapshot
        .worktrees
        .iter()
        .map(|w| (canonical(&parent.join(&w.id)), project.id.clone()))
        .collect()
}

fn leader_owner<'a>(cwd: &Path, claims: &'a [LeaderClaim]) -> Option<&'a str> {
    claims
        .iter()
        .filter(|(root, _)| cwd.starts_with(root))
        .max_by_key(|(root, _)| root.components().count())
        .map(|(_, id)| id.as_str())
}

fn attach_leaders(
    digest: &mut Value,
    engine: &Engine,
    project: &waggledance_core::domain::Project,
    rollup: &waggledance_core::bee::BeeProjectRollup,
    claims: &[LeaderClaim],
    leaders: &LeadersRead,
) {
    let Some(object) = digest.as_object_mut() else {
        return;
    };
    let (all, wd) = match leaders {
        LeadersRead::Off => return,
        LeadersRead::Unreachable(reason) => {
            object.insert("leaders".to_string(), Value::Null);
            object.insert("leaders_error".to_string(), json!(reason));
            return;
        }
        LeadersRead::Listed { all, wd } => (all, wd),
    };
    let mut skipped = 0usize;
    let mut mine: Vec<(&paseo::ListedAgent, std::path::PathBuf)> = Vec::new();
    for agent in all {
        let cwd = agent_cwd(&agent.cwd);
        match leader_owner(&cwd, claims) {
            None => skipped += 1,
            Some(owner) if owner == project.id => mine.push((agent, cwd)),
            Some(_) => {}
        }
    }
    mine.sort_by_key(|(agent, _)| {
        std::cmp::Reverse(agent.created.as_deref().and_then(paseo::created_age_secs))
    });
    let last_daemon_run = if mine.is_empty() {
        Value::Null
    } else {
        engine
            .list_runs(&project.id, 20)
            .unwrap_or_default()
            .into_iter()
            .find(|r| r.pane_id.starts_with("paseo:"))
            .map_or(
                Value::Null,
                |r| json!({ "run_id": r.id, "status": r.status }),
            )
    };
    let waiting_on = rollup
        .snapshot
        .state
        .as_ref()
        .and_then(|s| s.waiting_on.as_ref())
        .map_or(
            Value::Null,
            |w| json!({ "kind": w.kind, "subject": w.subject }),
        );
    let entries: Vec<Value> = mine
        .into_iter()
        .map(|(agent, cwd)| {
            let via = wd.contains(&agent.id).then_some("wd");
            json!({
                "agent_id": agent.id,
                "status": agent.status,
                "cwd": cwd.display().to_string(),
                "created": agent.created,
                "via": via.unwrap_or("daemon"),
                "labels": {
                    "waggledance.role": "leader",
                    "waggledance.project": project.id,
                    "waggledance.via": via
                },
                "waiting_on": waiting_on,
                "last_daemon_run": last_daemon_run
            })
        })
        .collect();
    object.insert("leaders".to_string(), Value::Array(entries));
    object.insert("leaders_skipped".to_string(), json!(skipped));
}

/// The human-readable half of the tool result.
///
/// Pure on purpose: the caller resolves the daemon's base URLs (which starts a
/// daemon), so keeping the formatting separate is what makes this behaviour
/// testable at all.
///
/// The file's path rides along as ordinary text next to the short link, because
/// the link itself is opaque — without it, a transcript full of `/s/…` codes
/// tells a reader nothing about which document each one was.
/// Resolve a caller's `preset` label into the command it names, or the
/// refusal to hand back (dispatch-project-presets D1–D4).
///
/// Two sources, in this order and deliberately: the operator's **global**
/// preset list first, then the **target project's own** `herding.agents` —
/// the same registry the board's Start button spawns from. Global-first is
/// what makes this purely additive: an installation that configured global
/// presets keeps every label pointing exactly where it did, and only labels
/// that refuse today can begin resolving. Reversing the order would silently
/// re-aim an existing label at a different command.
///
/// The caller never supplies a command from either source — only a name
/// (`orchestrator-dispatch` D3, unweakened). Nor does this widen who may
/// dispatch: `terminal.enabled` and the per-project `orchestration.enabled`
/// are checked by the caller and remain the only gate (D5).
///
/// `pub(crate)` rather than private since observer-tick-trigger: the trigger
/// (`crates/waggledance/src/trigger.rs`) dispatches its own ticks and needs a
/// resolved entry to spawn one at all, so it calls this — the one resolver,
/// with its global-then-project order intact — rather than growing a second
/// copy that could disagree with it. Visibility only; no caller's answer
/// changes.
pub(crate) fn resolve_preset(
    engine: &Engine,
    project: &waggledance_core::domain::Project,
    label: &str,
) -> std::result::Result<
    (
        waggledance_core::config::AgentPreset,
        waggledance_core::bee::BeeHerdingEntry,
    ),
    String,
> {
    if let Some(global) = engine
        .config
        .terminal
        .agent_presets
        .iter()
        .find(|p| p.label == label)
    {
        // An operator's preset is a command and nothing else: the conditions
        // are a project's declaration, and a global list declares none.
        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: global.argv.clone(),
            env: Vec::new(),
            workspace_trust: None,
        };
        return Ok((global.clone(), entry));
    }
    if let Some(entry) = waggledance_core::bee::herding_entry_for_label(&project.root_path, label) {
        return Ok((
            waggledance_core::config::AgentPreset {
                label: label.to_string(),
                argv: entry.argv.clone(),
            },
            entry,
        ));
    }
    // D4: a label the project DECLARES but this reader cannot start is not
    // "unknown" — calling it that sends a caller hunting a typo that does not
    // exist. `ask_state`'s `resolvable` list already draws this same line, by
    // the same predicate (D6), so the two answers cannot disagree.
    let declared = waggledance_core::bee::read_snapshot(&project.root_path)
        .config
        .and_then(|c| c.herding)
        .is_some_and(|h| h.agents.iter().any(|a| a == label));
    if declared {
        return Err(format!(
            "agent preset {label:?} is declared by project {} but cannot be started: its \
             herding.agents entry is not an argv this reader understands",
            project.id
        ));
    }
    // D3: name the project too — with two registries searched, "unknown"
    // alone no longer says where to look.
    Err(format!(
        "unknown agent preset: {label} (searched the configured presets and project {}'s \
         herding.agents)",
        project.id
    ))
}

/// `waggledance_dispatch`: D3/D6 gated dispatch. Refuses before touching
/// herdr at all when the project or the terminal family is off, or when the
/// preset label is unknown — only a resolved preset/pane_id ever reaches
/// [`run_dispatch`]'s herdr calls.
fn handle_dispatch(
    id: Option<Value>,
    engine: &Engine,
    orchestration: &mut Option<Orchestration>,
    args: &Value,
) -> Value {
    let Some(project_id) = args
        .get("project")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        return tool_error(id, "\"project\" is required");
    };
    let Some(task) = args
        .get("task")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        return tool_error(id, "\"task\" is required");
    };
    let preset_label = args
        .get("preset")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let pane_id_arg = args
        .get("pane_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let leader = args.get("leader").and_then(Value::as_bool).unwrap_or(false);
    let request_id = args
        .get("request_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    match [preset_label.is_some(), pane_id_arg.is_some(), leader]
        .iter()
        .filter(|t| **t)
        .count()
    {
        0 => {
            return tool_error(
                id,
                "specify one of \"preset\", \"pane_id\" or \"leader\": true",
            )
        }
        1 => {}
        _ => {
            return tool_error(
                id,
                "specify exactly one of \"preset\", \"pane_id\" or \"leader\": true, not both",
            )
        }
    }

    let project = match engine.get_project(project_id) {
        Ok(Some(p)) => p,
        Ok(None) => return tool_error(id, &format!("project not found: {project_id}")),
        Err(e) => return tool_error(id, &format!("project lookup failed: {e}")),
    };
    if let Some(request_id) = request_id {
        match engine.run_by_request(&project.id, request_id) {
            Ok(Some(run)) => return duplicate(id, &run),
            Ok(None) => {}
            Err(e) => return tool_error(id, &format!("request lookup failed: {e}")),
        }
    }
    if leader {
        let caller = dispatch_caller(
            std::env::var("PASEO_AGENT_ID").ok(),
            std::env::var("WAGGLEDANCE_CALLER").ok(),
        );
        return dispatch_to_leader(
            id,
            engine,
            &project,
            task,
            request_id,
            caller.as_deref(),
            &resolve_data_dir(None),
        );
    }
    if !engine.config.terminal.enabled {
        return tool_error(
            id,
            "the terminal surface is disabled — turn on terminal.enabled from the settings \
             page before dispatching",
        );
    }
    if !engine.orchestration_allowed(&project) {
        return tool_error(id, &not_opted_in(engine, &project));
    }

    // Resolved before any herdr call so an unresolvable label never touches
    // the socket at all.
    let preset = match preset_label {
        Some(label) => match resolve_preset(engine, &project, label) {
            Ok(resolved) => Some(resolved),
            Err(refusal) => return tool_error(id, &refusal),
        },
        None => None,
    };

    let orch = match orchestration_handle(orchestration, &engine.config.terminal) {
        Ok(o) => o,
        Err(e) => return tool_error(id, &e),
    };
    let outcome = orch.runtime.block_on(run_dispatch(
        &orch.herdr,
        engine,
        &project,
        preset,
        pane_id_arg.map(str::to_string),
        task,
    ));
    match outcome {
        // D9: a fail-open step that went wrong is named HERE, in the answer
        // that says the agent started — not left in the daemon's log, where a
        // warning is indistinguishable from no warning at the moment the
        // operator is looking at a pane that will not move.
        Ok((run_id, mut warnings)) => {
            if let Some(request_id) = request_id {
                if let Err(e) = engine.set_run_request(&run_id, request_id) {
                    warnings.push(format!("request_id not recorded: {e}"));
                }
            }
            let text = if warnings.is_empty() {
                format!("dispatched run {run_id}")
            } else {
                format!("dispatched run {run_id}; {}", warnings.join("; "))
            };
            ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": { "run_id": run_id, "duplicate": false, "warnings": warnings }
                }),
            )
        }
        Err(msg) => tool_error(id, &msg),
    }
}

fn not_opted_in(engine: &Engine, project: &Project) -> String {
    let file = waggledance_core::project_settings::settings_path(&project.root_path);
    let mut text = format!(
        "project {} has not opted into orchestrator dispatch — enable it from the project's \
         settings page, or set `orchestration = true` in {}",
        project.id,
        file.display()
    );
    for warning in engine.project_settings(project).1 {
        text.push_str("; ");
        text.push_str(&warning);
    }
    text
}

const DELIVERY_PAUSED: &str =
    "paused — waggledance daemon not running (start it with `waggledance serve`)";

fn delivery_paused(data_dir: &Path) -> bool {
    match std::fs::metadata(paseo::delivery_heartbeat_path(data_dir)).and_then(|m| m.modified()) {
        Ok(beat) => beat
            .elapsed()
            .is_ok_and(|age| age > std::time::Duration::from_secs(30)),
        Err(_) => true,
    }
}

fn duplicate(id: Option<Value>, run: &Run) -> Value {
    ok(
        id,
        json!({
            "content": [{ "type": "text", "text": format!(
                "request already accepted as run {} ({}) — nothing sent again",
                run.id, run.status
            ) }],
            "structuredContent": { "run_id": run.id, "status": run.status, "duplicate": true }
        }),
    )
}

fn dispatch_caller(paseo_agent: Option<String>, waggledance: Option<String>) -> Option<String> {
    paseo_agent
        .filter(|s| !s.is_empty())
        .or_else(|| (waggledance.as_deref() == Some("wd")).then(|| paseo::WD_CALLER.to_string()))
}

fn dispatch_to_leader(
    id: Option<Value>,
    engine: &Engine,
    project: &Project,
    task: &str,
    request_id: Option<&str>,
    caller: Option<&str>,
    data_dir: &Path,
) -> Value {
    if !engine.orchestration_allowed(project) {
        return tool_error(id, &not_opted_in(engine, project));
    }
    if let Some(why) = paseo::paseo_unavailable(&engine.config.paseo) {
        return tool_error(id, &why);
    }
    let now = now_rfc3339();
    let run_id = format!("run-{:016x}", rand::random::<u64>());
    let task = match caller == Some(paseo::WD_CALLER) {
        true => {
            let corr = request_id.unwrap_or(&run_id);
            paseo::wd_task(corr.split_once(':').map_or(corr, |(c, _)| c), task)
        }
        false => task.to_string(),
    };
    let run = Run {
        id: run_id,
        project_id: project.id.clone(),
        pane_id: format!("{}pending", orchestrate::PASEO_PANE_PREFIX),
        preset_label: None,
        task,
        baseline: now.clone(),
        marker: String::new(),
        status: "queued".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    let stored = match engine.insert_run_with_request(&run, None, request_id, caller) {
        Ok(stored) => stored,
        Err(e) => return tool_error(id, &format!("run persistence failed: {e}")),
    };
    if stored != run.id {
        return match engine.get_run(&stored) {
            Ok(Some(existing)) => duplicate(id, &existing),
            Ok(None) => tool_error(id, &format!("run {stored} vanished")),
            Err(e) => tool_error(id, &format!("run lookup failed: {e}")),
        };
    }
    let mut warnings: Vec<String> = if delivery_paused(data_dir) {
        vec![DELIVERY_PAUSED.to_string()]
    } else {
        Vec::new()
    };
    let hand_off = request_id.filter(|r| caller == Some(paseo::WD_CALLER) && !r.contains(':'));
    if let Some(corr) = hand_off {
        if let Err(e) = engine.insert_task(corr, &project.id, &run.created_at) {
            warnings.push(format!("task {corr} was not recorded: {e}"));
        }
    }
    let mut text = format!(
        "queued run {} for project {}'s Paseo leader",
        run.id, project.id
    );
    for warning in &warnings {
        text.push_str("; ");
        text.push_str(warning);
    }
    ok(
        id,
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": {
                "run_id": run.id,
                "status": "queued",
                "duplicate": false,
                "warnings": warnings
            }
        }),
    )
}

fn handle_task(
    id: Option<Value>,
    engine: &Engine,
    orchestration: &mut Option<Orchestration>,
    args: &Value,
) -> Value {
    let orch = match orchestration_handle(orchestration, &engine.config.terminal) {
        Ok(o) => o,
        Err(e) => return tool_error(id, &e),
    };
    let cli = paseo::PaseoCli::new(&engine.config.paseo);
    orch.runtime
        .block_on(task_action(id, engine, &cli, args, &resolve_data_dir(None)))
}

async fn task_action(
    id: Option<Value>,
    engine: &Engine,
    paseo: &dyn paseo::Paseo,
    args: &Value,
    data_dir: &Path,
) -> Value {
    let arg = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    };
    let Some(corr) = arg("corr") else {
        return tool_error(id, "\"corr\" is required");
    };
    let action = arg("action").unwrap_or_default();
    let prefix = match action {
        "reply" => "Reply from the user:",
        "redo" => "Redo requested:",
        "accept" | "allow" | "deny" => "",
        _ => {
            return tool_error(
                id,
                &format!("unknown action {action}: use reply, redo, accept, allow or deny"),
            )
        }
    };
    let text = arg("text");
    if !prefix.is_empty() && text.is_none() {
        return tool_error(id, &format!("\"text\" is required for {action}"));
    }
    let task = match engine.get_task(corr) {
        Ok(Some(task)) => task,
        Ok(None) => return tool_error(id, &format!("unknown task {corr}")),
        Err(e) => return tool_error(id, &format!("task lookup failed: {e}")),
    };
    if task.state == "closed" {
        return match action {
            "accept" => ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": format!("task {corr} was already closed") }],
                    "structuredContent": { "corr": corr, "closed": true, "archived": false }
                }),
            ),
            _ => tool_error(id, &format!("task {corr} is closed")),
        };
    }
    let Some(leader) = task.leader_id.as_deref() else {
        return tool_error(
            id,
            &format!("task {corr} has no leader yet — it is still queued for delivery"),
        );
    };
    let project = match engine.get_project(&task.project_id) {
        Ok(Some(p)) => p,
        Ok(None) => return tool_error(id, &format!("project not found: {}", task.project_id)),
        Err(e) => return tool_error(id, &format!("project lookup failed: {e}")),
    };
    if !prefix.is_empty() {
        let body = format!("{prefix}\n{}", text.unwrap_or_default());
        for n in 1.. {
            let resp = dispatch_to_leader(
                id.clone(),
                engine,
                &project,
                &body,
                Some(&format!("{corr}:{n}")),
                Some(paseo::WD_CALLER),
                data_dir,
            );
            if resp["result"]["structuredContent"]["duplicate"] != true {
                return resp;
            }
        }
    }
    if let Some(why) = paseo::paseo_unavailable(&engine.config.paseo) {
        return tool_error(id, &why);
    }
    match action {
        "accept" => accept_task(id, engine, paseo, corr, leader, &project.id).await,
        _ => {
            permit_task(
                id,
                engine,
                paseo,
                corr,
                leader,
                action == "allow",
                arg("req_id"),
            )
            .await
        }
    }
}

async fn accept_task(
    id: Option<Value>,
    engine: &Engine,
    paseo: &dyn paseo::Paseo,
    corr: &str,
    leader: &str,
    project_id: &str,
) -> Value {
    if let Err(e) = engine.close_task(corr, &now_rfc3339()) {
        return tool_error(id, &format!("task {corr} was not closed: {e}"));
    }
    let pane = format!("{}{leader}", orchestrate::PASEO_PANE_PREFIX);
    let pending = format!("{}pending", orchestrate::PASEO_PANE_PREFIX);
    let queued = engine
        .list_queued_paseo_runs(project_id)
        .unwrap_or_default();
    let inflight = engine.list_inflight_paseo_runs().unwrap_or_default();
    let live = queued
        .iter()
        .filter(|r| {
            r.pane_id == pane || (r.pane_id == pending && paseo::wd_corr(&r.task).is_some())
        })
        .chain(inflight.iter().filter(|r| r.pane_id == pane))
        .next();
    let sharing = engine
        .list_open_tasks()
        .unwrap_or_default()
        .into_iter()
        .find(|t| t.leader_id.as_deref() == Some(leader));
    let kept = match (live, sharing) {
        (Some(run), _) => Some(format!("run {} is still queued or working on it", run.id)),
        (None, Some(other)) => Some(format!("open task {} shares it", other.corr)),
        (None, None) => None,
    };
    let text = match &kept {
        Some(why) => format!("task {corr} closed; leader {leader} left running: {why}"),
        None => match paseo.archive(leader).await {
            Ok(()) => format!("task {corr} closed; leader {leader} archived"),
            Err(e) => {
                return tool_error(
                    id,
                    &format!("task {corr} closed, but archiving leader {leader} failed: {e}"),
                )
            }
        },
    };
    ok(
        id,
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": { "corr": corr, "closed": true, "archived": kept.is_none() }
        }),
    )
}

async fn permit_task(
    id: Option<Value>,
    engine: &Engine,
    paseo: &dyn paseo::Paseo,
    corr: &str,
    leader: &str,
    allow: bool,
    req_id: Option<&str>,
) -> Value {
    let req_id = match req_id {
        Some(r) => r.to_string(),
        None => match engine.unanswered_permission_requests(corr) {
            Ok(requests) => match requests.into_iter().next() {
                Some(r) => r.req_id,
                None => {
                    return tool_error(
                        id,
                        &format!("task {corr} has no unanswered permission request"),
                    )
                }
            },
            Err(e) => return tool_error(id, &format!("permission lookup failed: {e}")),
        },
    };
    let verb = if allow { "allow" } else { "deny" };
    if let Err(e) = paseo.permit(leader, &req_id, allow).await {
        return tool_error(
            id,
            &format!("paseo permit {verb} {leader} {req_id} failed: {e}"),
        );
    }
    let mut warnings = Vec::new();
    if let Err(e) = engine.answer_permission_request(&req_id, &now_rfc3339()) {
        warnings.push(format!("request {req_id} not marked answered: {e}"));
    }
    let mut text = format!("{verb} sent for request {req_id} of leader {leader} (task {corr})");
    for warning in &warnings {
        text.push_str("; ");
        text.push_str(warning);
    }
    ok(
        id,
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": { "corr": corr, "req_id": req_id, "answer": verb, "warnings": warnings }
        }),
    )
}

/// The dispatch protocol sequence itself (D1/D3/D5): resolve the target pane
/// (spawn via `preset`'s argv, or preflight an existing `pane_id`), capture
/// the baseline, mint a marker, send the task, and persist the run row (D7).
/// String-errored — every failure here is already a named, human-readable
/// refusal by the time it reaches [`handle_dispatch`].
async fn run_dispatch(
    herdr: &dyn Herdr,
    engine: &Engine,
    project: &Project,
    preset: Option<(
        waggledance_core::config::AgentPreset,
        waggledance_core::bee::BeeHerdingEntry,
    )>,
    pane_id_arg: Option<String>,
    task: &str,
) -> std::result::Result<(String, Vec<String>), String> {
    // This tool's own half: which destination the two argument shapes mean,
    // and how a run started here is labelled. Everything after it -- the
    // preflight, baseline, marker, send and the persisted run -- is the one
    // shared dispatch path (`orchestrate::dispatch_run`), the same one the
    // board's run actions use.
    let (target, preset_label) = match (preset, pane_id_arg) {
        (Some((preset, entry)), None) => (
            orchestrate::DispatchTarget::Spawn { entry, cwd: None },
            Some(preset.label),
        ),
        (None, Some(pane_id)) => {
            // D6 containment: a caller-supplied pane_id must belong to THIS
            // project's own root before any send. Without this, an opted-in
            // project could dispatch into any pane on the host (pane ids are
            // enumerable off GET /api/agents) -- the same boundary every
            // sibling pane-scoped write route enforces via
            // `project_and_verify_pane_in_boundary`. It stays HERE rather
            // than inside the shared path because the board contains its
            // panes differently: a feature's granted worktree is a sibling
            // directory outside this very boundary.
            let snapshot = herdr
                .snapshot()
                .await
                .map_err(|e| format!("herdr snapshot failed: {e}"))?;
            let boundary =
                waggledance_core::paths_boundary::Boundary::new(vec![project.root_path.clone()])
                    .map_err(|e| format!("project {} destination unresolved: {e}", project.id))?;
            orchestrate::verify_pane_in_boundary(&snapshot, &boundary, &pane_id, &project.id)
                .map_err(|e| e.to_string())?;
            (orchestrate::DispatchTarget::Pane(pane_id), None)
        }
        (Some(_), Some(_)) | (None, None) => {
            unreachable!("handle_dispatch already validated exactly one of preset/pane_id")
        }
    };

    // No feature: a run started from this tool belongs to whoever called it,
    // not to a board card, and must never hold a card's per-feature lock.
    let dispatched =
        orchestrate::dispatch_run(herdr, engine, project, target, task, None, preset_label)
            .await
            .map_err(|e| e.to_string())?;
    Ok((dispatched.run.id, dispatched.warnings))
}

/// `waggledance_await`: bounded poll for a run's completion (D4/D5).
/// `run_id` is resolved before the orchestration runtime is ever built, so
/// an unknown id never touches herdr.
fn handle_await(
    id: Option<Value>,
    engine: &Engine,
    orchestration: &mut Option<Orchestration>,
    args: &Value,
    data_dir: &Path,
) -> Value {
    let Some(run_id) = args
        .get("run_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        return tool_error(id, "\"run_id\" is required");
    };
    let run = match engine.get_run(run_id) {
        Ok(Some(r)) => r,
        Ok(None) => return tool_error(id, &format!("unknown run_id: {run_id}")),
        Err(e) => return tool_error(id, &format!("run lookup failed: {e}")),
    };
    let timeout = args
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .map(std::time::Duration::from_secs)
        .unwrap_or(orchestrate::MAX_AWAIT_TIMEOUT);

    let orch = match orchestration_handle(orchestration, &engine.config.terminal) {
        Ok(o) => o,
        Err(e) => return tool_error(id, &e),
    };
    match orch.runtime.block_on(orchestrate::await_run(
        &orch.herdr,
        engine,
        &run,
        timeout,
        orch.notify_store.as_ref(),
    )) {
        Ok(mut outcome) => {
            let mut text = format!("run {run_id}: {}", outcome.status.as_str());
            let mut report = json!({
                "run_id": run_id,
                "status": outcome.status.as_str(),
                "delta": outcome.delta,
            });
            if outcome.status == orchestrate::RunStatus::Queued {
                let age = time::OffsetDateTime::parse(
                    &run.created_at,
                    &time::format_description::well_known::Rfc3339,
                )
                .map(|at| {
                    (time::OffsetDateTime::now_utc() - at)
                        .whole_seconds()
                        .max(0)
                })
                .unwrap_or(0);
                text.push_str(&format!(" for {age}s"));
                report["age_secs"] = json!(age);
            }
            if run.pane_id.starts_with(orchestrate::PASEO_PANE_PREFIX)
                && !outcome.status.is_terminal()
                && delivery_paused(data_dir)
            {
                outcome.warnings.push(DELIVERY_PAUSED.to_string());
            }
            for warning in &outcome.warnings {
                text.push_str("; ");
                text.push_str(warning);
            }
            report["warnings"] = json!(outcome.warnings);
            ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": report
                }),
            )
        }
        Err(e) => tool_error(id, &format!("await failed: {e}")),
    }
}

/// `waggledance_runs`: the run store, read-only (D8), optionally narrowed to
/// one project.
fn handle_runs(id: Option<Value>, engine: &Engine, args: &Value) -> Value {
    let project_filter = args
        .get("project")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let project_ids: Vec<String> = match project_filter {
        Some(p) => vec![p.to_string()],
        None => match engine.list_projects() {
            Ok(ps) => ps.into_iter().map(|p| p.id).collect(),
            Err(e) => return tool_error(id, &format!("could not list registered projects: {e}")),
        },
    };
    let mut runs: Vec<Run> = Vec::new();
    for pid in &project_ids {
        match engine.list_runs(pid, RUNS_LIST_LIMIT) {
            Ok(rs) => runs.extend(rs),
            Err(e) => return tool_error(id, &format!("run listing failed: {e}")),
        }
    }
    let rows: Vec<Value> = runs
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "project_id": r.project_id,
                "pane_id": r.pane_id,
                "preset_label": r.preset_label,
                "task": r.task,
                "status": r.status,
                "created_at": r.created_at,
                "updated_at": r.updated_at
            })
        })
        .collect();
    ok(
        id,
        json!({
            "content": [{ "type": "text", "text": format!("{} run(s)", rows.len()) }],
            "structuredContent": { "runs": rows }
        }),
    )
}

fn viewable_text(urls: &[String], rel_path: &str, project_id: &str) -> String {
    let viewable = if urls.len() > 1 {
        let lines = urls
            .iter()
            .map(|u| format!("  {rel_path} → {u}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Viewable at (pick a reachable IP):\n{lines}")
    } else {
        let primary = urls.first().map(String::as_str).unwrap_or_default();
        format!("Viewable at: {rel_path} → {primary}")
    };
    format!("{viewable}\nproject_id: {project_id}")
}

fn ok(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}
fn err(id: Option<Value>, code: i64, msg: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } })
}
/// Tool-level error: reported inside a successful result with isError=true (MCP convention).
fn tool_error(id: Option<Value>, msg: &str) -> Value {
    ok(
        id,
        json!({ "content": [{ "type": "text", "text": msg }], "isError": true }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_base_renders_a_single_line() {
        let text = viewable_text(
            &["http://design-lap:7700/s/a3f9c1d20b74".into()],
            "docs/history/short-link/DISCUSSION.md",
            "waggledance",
        );
        assert_eq!(
            text,
            "Viewable at: docs/history/short-link/DISCUSSION.md → \
             http://design-lap:7700/s/a3f9c1d20b74\nproject_id: waggledance"
        );
        assert_eq!(text.lines().count(), 2);
    }

    #[test]
    fn several_bases_render_one_line_each() {
        let text = viewable_text(
            &[
                "http://192.168.1.10:7700/s/a3f9c1d20b74".into(),
                "http://10.0.0.5:7700/s/a3f9c1d20b74".into(),
            ],
            "docs/a.md",
            "waggledance",
        );
        assert!(text.contains("pick a reachable IP"));
        assert!(text.contains("  docs/a.md → http://192.168.1.10:7700/s/a3f9c1d20b74"));
        assert!(text.contains("  docs/a.md → http://10.0.0.5:7700/s/a3f9c1d20b74"));
    }

    /// The whole point of the feature: the emitted line has to stay inside a
    /// terminal width, which the full path did not.
    #[test]
    fn the_short_line_fits_in_a_terminal() {
        let deep = "docs/history/short-link-for-file-urls/DISCUSSION.md";
        let text = viewable_text(
            &["http://design-lap:7700/s/a3f9c1d20b74".into()],
            deep,
            "waggledance",
        );
        let url_line = text.lines().next().unwrap();
        let url = url_line.split(" → ").nth(1).unwrap();
        assert!(url.len() <= 40, "short url grew to {}: {url}", url.len());
    }

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    fn call_tool(engine: &Engine, name: &str, args: Value) -> Value {
        call_tool_with_orchestration(engine, &mut None, name, args)
    }

    fn call_tool_with_orchestration(
        engine: &Engine,
        orchestration: &mut Option<Orchestration>,
        name: &str,
        args: Value,
    ) -> Value {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": args }
        });
        handle_tool_call(Some(json!(1)), engine, orchestration, &req)
    }

    /// Two registered projects, each with one markdown file sharing a word
    /// ("grapefruit") that appears nowhere else — a search unfiltered must
    /// span both (D1); a filtered search must narrow to one.
    fn two_project_engine(
        tag: &str,
    ) -> (
        Engine,
        waggledance_core::domain::Project,
        waggledance_core::domain::Project,
    ) {
        let dir_a =
            std::env::temp_dir().join(format!("waggledance-mcp-{tag}-a-{}", std::process::id()));
        let dir_b =
            std::env::temp_dir().join(format!("waggledance-mcp-{tag}-b-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
        write(
            &dir_a,
            "docs/a.md",
            "# Project A\nThe grapefruit orchard thrives in spring.",
        );
        write(
            &dir_b,
            "docs/b.md",
            "# Project B\nA grapefruit smoothie recipe for summer.",
        );

        let mut config = Config::default();
        config.paseo.enabled = Some(false);
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        let pa = engine.register(&dir_a, None).unwrap();
        let pb = engine.register(&dir_b, None).unwrap();
        (engine, pa, pb)
    }

    /// A published field nobody can discover is not published: an agent
    /// learns what `ask_state` answers with from this description and
    /// nowhere else. Pinned so a later trim cannot quietly drop the two
    /// fleet fields back out of the contract while the payload still
    /// carries them.
    #[test]
    fn ask_state_description_names_both_fleet_fields_and_the_absent_panes_case() {
        let description = ask_state_schema()["description"]
            .as_str()
            .expect("the tool carries a description")
            .to_string();

        for promised in ["herding", "resolvable", "panes", "panes_error"] {
            assert!(
                description.contains(promised),
                "{promised:?} is answered but never described: {description}"
            );
        }
        assert!(
            description.contains("ABSENT"),
            "the terminal-off case is the one a caller gets wrong by guessing: {description}"
        );
    }

    #[test]
    fn tools_list_has_eight_schemas() {
        let tools = [
            view_file_schema(),
            search_schema(),
            projects_schema(),
            ask_state_schema(),
            dispatch_schema(),
            await_schema(),
            runs_schema(),
            task_schema(),
        ];
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            vec![
                "waggledance_view_file",
                "waggledance_search",
                "waggledance_projects",
                "waggledance_ask_state",
                "waggledance_dispatch",
                "waggledance_await",
                "waggledance_runs",
                "waggledance_task"
            ]
        );
    }

    #[test]
    fn search_unfiltered_spans_multiple_projects_with_marked_excerpts() {
        let (engine, pa, pb) = two_project_engine("search-multi");
        let resp = call_tool(
            &engine,
            "waggledance_search",
            json!({ "query": "grapefruit" }),
        );
        let hits = resp["result"]["structuredContent"]["hits"]
            .as_array()
            .unwrap();
        assert_eq!(hits.len(), 2, "expected a hit from each project: {resp}");
        let ids: Vec<&str> = hits
            .iter()
            .map(|h| h["project_id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&pa.id.as_str()));
        assert!(ids.contains(&pb.id.as_str()));
        for h in hits {
            assert!(h["excerpt"].as_str().unwrap().contains("<mark>"));
        }

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn search_project_filter_narrows_to_one() {
        let (engine, pa, pb) = two_project_engine("search-filter");
        let resp = call_tool(
            &engine,
            "waggledance_search",
            json!({ "query": "grapefruit", "project": pa.id }),
        );
        let hits = resp["result"]["structuredContent"]["hits"]
            .as_array()
            .unwrap();
        assert_eq!(hits.len(), 1, "expected only project a's hit: {resp}");
        assert_eq!(hits[0]["project_id"], pa.id);

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn search_reflects_a_file_edited_on_disk_since_last_index() {
        let (engine, pa, pb) = two_project_engine("search-stale");
        // Edit project a's file after registration (which already indexed
        // it once) — D4: the next search must see the new content without a
        // separate refresh call.
        write(
            &pa.root_path,
            "docs/a.md",
            "# Project A\nNow mentions pineapple instead.",
        );
        let resp = call_tool(
            &engine,
            "waggledance_search",
            json!({ "query": "pineapple" }),
        );
        let hits = resp["result"]["structuredContent"]["hits"]
            .as_array()
            .unwrap();
        assert_eq!(hits.len(), 1, "edited content must be searchable: {resp}");
        assert_eq!(hits[0]["project_id"], pa.id);

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn fts_hostile_query_returns_empty_not_error() {
        let (engine, pa, pb) = two_project_engine("search-hostile");
        let resp = call_tool(&engine, "waggledance_search", json!({ "query": "*)(" }));
        assert!(
            resp["result"]["isError"].is_null(),
            "unexpected error: {resp}"
        );
        let hits = resp["result"]["structuredContent"]["hits"]
            .as_array()
            .unwrap();
        assert!(
            hits.is_empty(),
            "hostile query must not error, and must not match: {resp}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn search_missing_query_is_a_tool_error() {
        let (engine, pa, pb) = two_project_engine("search-missing-query");
        let resp = call_tool(&engine, "waggledance_search", json!({}));
        assert_eq!(resp["result"]["isError"], true, "{resp}");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn search_nonexistent_project_is_a_tool_error_naming_it() {
        let (engine, pa, pb) = two_project_engine("search-no-project");
        let resp = call_tool(
            &engine,
            "waggledance_search",
            json!({ "query": "grapefruit", "project": "does-not-exist" }),
        );
        assert_eq!(resp["result"]["isError"], true, "{resp}");
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("does-not-exist"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// Happy path (review P1-2): a clean unfiltered search reports both
    /// registered projects refreshed and names none as failed.
    #[test]
    fn search_reports_a_clean_refresh_outcome() {
        let (engine, pa, pb) = two_project_engine("search-refresh-happy");
        let resp = call_tool(
            &engine,
            "waggledance_search",
            json!({ "query": "grapefruit" }),
        );
        let refresh = &resp["result"]["structuredContent"]["refresh"];
        assert!(
            refresh["refreshed"].as_u64().unwrap() >= 1,
            "expected at least one project refreshed: {resp}"
        );
        assert_eq!(refresh["failed"].as_array().unwrap().len(), 0, "{resp}");
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("warning"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// Failure path (review P1-2): `summarize_refresh` is the seam a real
    /// `refresh_stale` error (DB locked past busy_timeout, store error
    /// mid-walk) folds through — asserted directly since inducing a real
    /// store failure from this test would be unreliable.
    #[test]
    fn summarize_refresh_surfaces_a_failed_project_and_a_warning() {
        let summary = summarize_refresh(vec![
            ("ok-project".to_string(), Ok(3)),
            (
                "broken-project".to_string(),
                Err(Error::Other("db locked".to_string())),
            ),
        ]);
        assert_eq!(summary.structured["refreshed"], 1);
        let failed = summary.structured["failed"].as_array().unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0]["project_id"], "broken-project");
        assert_eq!(failed[0]["error"], "db locked");
        let warning = summary.warning.expect("a failed project must warn");
        assert!(warning.contains("broken-project"), "{warning}");
        assert!(warning.contains("may lag disk"), "{warning}");
    }

    /// A search response with no failed projects carries no warning.
    #[test]
    fn summarize_refresh_is_silent_when_nothing_failed() {
        let summary = summarize_refresh(vec![("a".to_string(), Ok(1)), ("b".to_string(), Ok(0))]);
        assert_eq!(summary.structured["refreshed"], 2);
        assert_eq!(summary.structured["failed"].as_array().unwrap().len(), 0);
        assert!(summary.warning.is_none());
    }

    #[test]
    fn projects_lists_both_with_counts_and_root() {
        let (engine, pa, pb) = two_project_engine("projects-list");
        let resp = call_tool(&engine, "waggledance_projects", json!({}));
        let entries = resp["result"]["structuredContent"]["projects"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 2, "{resp}");
        let a = entries.iter().find(|e| e["id"] == pa.id).unwrap();
        assert_eq!(a["file_count"], 1);
        assert_eq!(a["root_path"], pa.root_path.display().to_string());
        assert!(a["last_seen_at"].as_str().is_some());

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn ask_state_filtered_reads_feature_phase_and_buckets_without_a_direct_read() {
        let (engine, pa, pb) = two_project_engine("ask-state-filtered");
        write(
            &pa.root_path,
            ".bee/state.json",
            r#"{"feature": "widget-polish", "phase": "execution", "mode": "standard"}"#,
        );
        let resp = call_tool(
            &engine,
            "waggledance_ask_state",
            json!({ "project": pa.id }),
        );
        let digest = &resp["result"]["structuredContent"]["project"];
        assert_eq!(digest["present"], true, "{resp}");
        assert_eq!(digest["feature"], "widget-polish");
        assert_eq!(digest["phase"], "execution");
        assert_eq!(digest["mode"], "standard");
        assert_eq!(digest["cell_counts"]["doing"], 0);

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// ask-state-fleet-read D1: the herding labels ride on BOTH answer
    /// shapes, because the rollup is the cross-project view an orchestrator
    /// leans on hardest — and D2: what rides is labels, never argv.
    #[test]
    fn ask_state_carries_the_projects_herding_labels_in_both_forms() {
        let (engine, pa, pb) = two_project_engine("ask-state-herding");
        write(
            &pa.root_path,
            ".bee/config.json",
            r#"{
                "herding": {
                    "agent_command": "claude-sonnet",
                    "agents": {
                        "claude-sonnet": ["claude", "--model", "sonnet"],
                        "agy-flash": {"argv": ["agy", "--dangerously-skip-permissions"]},
                        "broken": {"env": {"K": "v"}}
                    }
                }
            }"#,
        );

        let filtered = call_tool(
            &engine,
            "waggledance_ask_state",
            json!({ "project": pa.id }),
        );
        let herding = &filtered["result"]["structuredContent"]["project"]["herding"];
        assert_eq!(herding["default"], "claude-sonnet", "{filtered}");
        assert_eq!(
            herding["agents"],
            json!(["claude-sonnet", "agy-flash", "broken"])
        );
        // herding-entry-conditions D1: the object form is a bee shape, so
        // `agy-flash` resolves. What still cannot start is an entry yielding no
        // command at all — and it is listed, because a label a consumer cannot
        // see is a label it cannot ask about.
        assert_eq!(
            herding["resolvable"],
            json!(["claude-sonnet", "agy-flash"]),
            "an object entry with a usable argv resolves; one without does not"
        );

        let rollup = call_tool(&engine, "waggledance_ask_state", json!({}));
        let entries = rollup["result"]["structuredContent"]["projects"]
            .as_array()
            .unwrap();
        let a = entries.iter().find(|e| e["project_id"] == pa.id).unwrap();
        assert_eq!(a["herding"]["default"], "claude-sonnet", "{rollup}");
        let b = entries.iter().find(|e| e["project_id"] == pb.id).unwrap();
        assert!(
            b["herding"].is_null(),
            "a project with no .bee/ reports no registry, not an empty one: {b}"
        );

        // D2 at the wire: no argv token may appear in either answer. `sonnet`
        // and `agy` are deliberately not probed — both are substrings of a
        // label, so they would pass on the label and prove nothing.
        for body in [filtered.to_string(), rollup.to_string()] {
            for token in ["--model", "--dangerously-skip-permissions"] {
                assert!(
                    !body.contains(token),
                    "argv token {token:?} reached an ask_state answer"
                );
            }
        }

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// ask-state-fleet-read D3, proved at the pure seam because that is the
    /// only place it CAN be proved: `Orchestration` holds a concrete
    /// `SocketHerdr`, so a live-transport answer cannot be faked at the
    /// handler. Handed a snapshot and a root, the projection stands on its
    /// own — a pane inside the project is kept, one outside is dropped, and
    /// neither the agent's name nor its terminal title comes along.
    #[test]
    fn panes_keep_what_the_project_owns_and_publish_no_name_or_title() {
        let inside = std::env::temp_dir().join(format!("wd-panes-in-{}", std::process::id()));
        let outside = std::env::temp_dir().join(format!("wd-panes-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&inside);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).unwrap();
        write(&inside, "docs/a.md", "# Project");
        // A live bee session claiming w1:p1 — the join's whole input. Its
        // heartbeat is stamped now so the session reads live.
        let now = waggledance_core::indexer::now_rfc3339();
        write(
            &inside,
            ".bee/sessions/s1.json",
            &format!(
                r#"{{"id": "s1", "started_at": "{now}", "last_heartbeat": "{now}",
                     "activity": {{"state": "blocked", "event": "PermissionRequest",
                                   "tool_name": "Bash", "at": "{now}", "pane": "w1:p1",
                                   "cwd": "{cwd}", "feature": "widget-polish"}}}}"#,
                cwd = inside.display()
            ),
        );

        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&inside, None).unwrap();
        let mut rollups = bee::read_rollup(std::slice::from_ref(&project.root_path));
        let rollup = rollups.pop().unwrap();

        let agent = |pane_id: &str| crate::herdr::wire::Agent {
            pane_id: pane_id.to_string(),
            workspace_id: "w1".to_string(),
            tab_id: "t1".to_string(),
            kind: "claude".to_string(),
            name: "surly-otter".to_string(),
            status: crate::herdr::wire::AgentStatus::Idle,
            title: "editing bee.rs".to_string(),
            session_id: None,
        };
        let pane = |pane_id: &str, cwd: &std::path::Path| crate::herdr::wire::Pane {
            pane_id: pane_id.to_string(),
            workspace_id: "w1".to_string(),
            tab_id: "t1".to_string(),
            cwd: Some(cwd.display().to_string()),
            foreground_cwd: None,
        };
        let snapshot = crate::herdr::wire::Snapshot {
            agents: vec![agent("w1:p1"), agent("w1:p2"), agent("w1:p9")],
            panes: vec![
                pane("w1:p1", &inside),
                pane("w1:p2", &inside),
                pane("w1:p9", &outside),
            ],
            ..Default::default()
        };

        let sessions = crate::server::herdr_session_ids(Some(&snapshot));
        let value = project_panes_value(&snapshot, &project, &rollup, &sessions);
        let rows = value.as_array().expect("panes is an array");
        assert_eq!(rows.len(), 2, "only the in-boundary panes survive: {value}");

        let fields: Vec<&str> = rows[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            vec![
                "pane_id",
                "kind",
                "status",
                "bee_state",
                "bee_feature",
                "workspace",
                "tab"
            ],
            "D3 pins the field set exactly"
        );

        // The pane bee claims carries bee's own reading, which is the half an
        // orchestrator needs: herdr says `idle` for both panes, bee says this
        // one is blocked on a human.
        let claimed = rows.iter().find(|r| r["pane_id"] == "w1:p1").unwrap();
        assert_eq!(claimed["status"], "idle", "herdr's screen-derived reading");
        assert_eq!(claimed["bee_state"], "Blocked", "{claimed}");
        assert_eq!(claimed["bee_feature"], "widget-polish", "{claimed}");

        // A pane no session claims reports nothing rather than borrowing its
        // neighbour's state.
        let unclaimed = rows.iter().find(|r| r["pane_id"] == "w1:p2").unwrap();
        assert!(unclaimed["bee_state"].is_null(), "{unclaimed}");
        assert!(unclaimed["bee_feature"].is_null(), "{unclaimed}");

        let published = value.to_string();
        for leaked in ["surly-otter", "editing bee.rs"] {
            assert!(
                !published.contains(leaked),
                "{leaked:?} is the agent's own name or title and must not be published: {published}"
            );
        }

        std::fs::remove_dir_all(&inside).ok();
        std::fs::remove_dir_all(&outside).ok();
    }

    /// D6: with the terminal family off nobody looked, so the key is absent
    /// — not null, not an empty array — and the answer is what every caller
    /// that never set that switch already gets today.
    #[test]
    fn ask_state_omits_panes_entirely_when_the_terminal_family_is_off() {
        let (engine, pa, pb) = two_project_engine("ask-state-panes-off");
        assert!(
            !engine.config.terminal.enabled,
            "this test's premise is the default-off switch"
        );

        let resp = call_tool(
            &engine,
            "waggledance_ask_state",
            json!({ "project": pa.id }),
        );
        let digest = &resp["result"]["structuredContent"]["project"];
        assert!(
            digest.get("panes").is_none(),
            "an empty array would claim there are no panes: {digest}"
        );
        assert!(digest.get("panes_error").is_none(), "{digest}");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// D5: an unreadable transport nulls the pane list and names why, and
    /// leaves the bee half of the answer whole. Losing herdr must never cost
    /// a caller the state it actually asked for.
    ///
    /// Proved on `attach_panes` rather than through the handler on purpose.
    /// An earlier version drove the handler with an empty orchestration slot
    /// and passed — but only because the code was reading that slot directly
    /// instead of building the handle, which is exactly the bug asfr-4 fixed.
    /// Now that the handler builds a real client, the same test would pass or
    /// fail depending on whether a herdr socket happens to be alive on the
    /// machine running it. The shape belongs here, where it depends on nothing.
    #[test]
    fn unreachable_herdr_nulls_panes_with_a_reason_and_leaves_bee_state_whole() {
        let mut digest = json!({
            "project_id": "widgets",
            "feature": "widget-polish",
            "phase": "execution"
        });
        // The project and rollup are inert on this branch — an unreachable
        // transport is answered without ever looking at either — but the
        // function takes them, so a real pair is cheaper than a fake one.
        let dir = std::env::temp_dir().join(format!("wd-panes-down-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# Project");
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        let mut rollups = bee::read_rollup(std::slice::from_ref(&project.root_path));
        let rollup = rollups.pop().unwrap();

        attach_panes(
            &mut digest,
            &project,
            &rollup,
            &FleetPanes::Unreachable("herdr snapshot failed: connection refused".to_string()),
        );

        assert!(digest["panes"].is_null(), "{digest}");
        assert_eq!(
            digest["panes_error"], "herdr snapshot failed: connection refused",
            "the transport's own words, never a bare null: {digest}"
        );
        assert_eq!(
            digest["feature"], "widget-polish",
            "the bee half of the answer is untouched: {digest}"
        );
        assert_eq!(digest["phase"], "execution");

        std::fs::remove_dir_all(&dir).ok();
    }

    fn listed(id: &str, cwd: &str, created: &str) -> paseo::ListedAgent {
        paseo::ListedAgent {
            id: id.to_string(),
            cwd: cwd.to_string(),
            status: "idle".to_string(),
            created: Some(created.to_string()),
        }
    }

    fn leaders_project(tag: &str) -> (Engine, Project, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!("wd-leaders-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("widgets");
        let worktree = base.join("widgets--wt--polish");
        write(&root, "docs/a.md", "# Project");
        write(&worktree, "src/lib.rs", "");
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&root, None).unwrap();
        (engine, project, base)
    }

    fn with_worktree(
        mut rollup: waggledance_core::bee::BeeProjectRollup,
        id: &str,
    ) -> waggledance_core::bee::BeeProjectRollup {
        rollup.snapshot.worktrees.push(
            serde_json::from_value(json!({
                "id": id, "resolved": false, "unresolved_reason": null, "feature": null,
                "phase": null, "mode": null, "branch": null, "created_at": null,
                "live": false, "heartbeat_age_minutes": null, "merged_pending": false
            }))
            .unwrap(),
        );
        rollup
    }

    #[test]
    fn leaders_map_root_and_worktree_cwds_oldest_first_and_count_the_homeless() {
        let (engine, project, base) = leaders_project("map");
        write(
            &project.root_path,
            ".bee/state.json",
            r#"{"feature": "widget-polish", "waiting_on": {"kind": "gate", "subject": "approve plan"}}"#,
        );
        let mut daemon_run = seeded_run(&project.id, "run-paseo");
        daemon_run.pane_id = "paseo:abc".to_string();
        daemon_run.status = "done".to_string();
        engine.insert_run(&daemon_run, None).unwrap();
        let mut rollups = bee::read_rollup(std::slice::from_ref(&project.root_path));
        let rollup = with_worktree(rollups.pop().unwrap(), "widgets--wt--polish");
        let mut claims = vec![(canonical(&project.root_path), project.id.clone())];
        claims.extend(worktree_claims(&project, &rollup));
        let root = project.root_path.display().to_string();
        let worktree = base.join("widgets--wt--polish/src").display().to_string();
        let read = LeadersRead::Listed {
            all: vec![
                listed("young", &root, "2 hours ago"),
                listed("old", &worktree, "3 days ago"),
                listed(
                    "homeless",
                    "/nonexistent/wd-leaders/elsewhere",
                    "1 days ago",
                ),
            ],
            wd: Default::default(),
        };
        let mut digest = ask_state_digest(&project.id, &rollup.snapshot);
        attach_leaders(&mut digest, &engine, &project, &rollup, &claims, &read);

        let leaders = digest["leaders"]
            .as_array()
            .expect("a list when Paseo answered");
        let ids: Vec<&str> = leaders
            .iter()
            .map(|l| l["agent_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["old", "young"], "{digest}");
        assert_eq!(digest["leaders_skipped"], 1, "{digest}");
        assert_eq!(leaders[0]["via"], "daemon");
        assert!(leaders[0]["labels"]["waggledance.via"].is_null());
        assert_eq!(leaders[1]["via"], "daemon");
        assert!(leaders[1]["labels"]["waggledance.via"].is_null());
        assert_eq!(leaders[1]["labels"]["waggledance.role"], "leader");
        assert_eq!(
            leaders[1]["labels"]["waggledance.project"],
            json!(project.id)
        );
        assert_eq!(
            leaders[1]["cwd"],
            json!(canonical(&project.root_path).display().to_string())
        );
        assert_eq!(
            leaders[1]["waiting_on"],
            json!({ "kind": "gate", "subject": "approve plan" })
        );
        assert_eq!(
            leaders[0]["last_daemon_run"],
            json!({ "run_id": "run-paseo", "status": "done" })
        );
        assert_eq!(leaders[0]["created"], "3 days ago");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn leaders_go_to_the_deeper_root_and_tilde_cwds_expand_to_home() {
        let claims = vec![
            (std::path::PathBuf::from("/srv/outer"), "outer".to_string()),
            (
                std::path::PathBuf::from("/srv/outer/inner"),
                "inner".to_string(),
            ),
        ];
        assert_eq!(
            leader_owner(Path::new("/srv/outer/inner/src"), &claims),
            Some("inner")
        );
        assert_eq!(
            leader_owner(Path::new("/srv/outer/docs"), &claims),
            Some("outer")
        );
        assert_eq!(leader_owner(Path::new("/srv/outerwear"), &claims), None);
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            agent_cwd("~/wd-leaders-no-such-dir"),
            home.join("wd-leaders-no-such-dir")
        );
    }

    #[test]
    fn leaders_of_another_project_are_neither_listed_nor_skipped() {
        let (engine, project, base) = leaders_project("other");
        let mut rollups = bee::read_rollup(std::slice::from_ref(&project.root_path));
        let rollup = rollups.pop().unwrap();
        let claims = vec![
            (canonical(&project.root_path), project.id.clone()),
            (
                std::path::PathBuf::from("/srv/elsewhere"),
                "elsewhere".to_string(),
            ),
        ];
        let read = LeadersRead::Listed {
            all: vec![listed("theirs", "/srv/elsewhere", "1 hours ago")],
            wd: Default::default(),
        };
        let mut digest = ask_state_digest(&project.id, &rollup.snapshot);
        attach_leaders(&mut digest, &engine, &project, &rollup, &claims, &read);
        assert_eq!(digest["leaders"], json!([]), "{digest}");
        assert_eq!(digest["leaders_skipped"], 0, "{digest}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn unreachable_paseo_nulls_leaders_with_a_reason_and_keeps_every_key() {
        let (engine, project, base) = leaders_project("down");
        let mut rollups = bee::read_rollup(std::slice::from_ref(&project.root_path));
        let rollup = rollups.pop().unwrap();
        let before = ask_state_digest(&project.id, &rollup.snapshot);
        let mut digest = before.clone();
        attach_leaders(
            &mut digest,
            &engine,
            &project,
            &rollup,
            &[],
            &LeadersRead::Unreachable("`paseo ls` timed out after 15s".to_string()),
        );
        assert!(digest["leaders"].is_null(), "{digest}");
        assert_eq!(digest["leaders_error"], "`paseo ls` timed out after 15s");
        for (key, value) in before.as_object().unwrap() {
            assert_eq!(&digest[key], value, "{key} changed");
        }
        assert!(digest.get("leaders_skipped").is_none(), "{digest}");

        let mut off = before.clone();
        attach_leaders(&mut off, &engine, &project, &rollup, &[], &LeadersRead::Off);
        assert_eq!(off, before);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn paseo_switched_off_leaves_no_leaders_key_in_either_form() {
        let (engine, pa, pb) = two_project_engine("ask-state-leaders-off");
        assert_eq!(engine.config.paseo.enabled, Some(false));
        assert!(matches!(
            read_leaders(&engine.config.paseo),
            LeadersRead::Off
        ));
        let one = call_tool(
            &engine,
            "waggledance_ask_state",
            json!({ "project": pa.id }),
        );
        let digest = &one["result"]["structuredContent"]["project"];
        assert!(digest.get("leaders").is_none(), "{digest}");
        assert!(digest.get("leaders_error").is_none(), "{digest}");
        let all = call_tool(&engine, "waggledance_ask_state", json!({}));
        for entry in all["result"]["structuredContent"]["projects"]
            .as_array()
            .unwrap()
        {
            assert!(entry.get("leaders").is_none(), "{entry}");
        }

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn ask_state_description_names_leaders_via_and_the_three_absence_states() {
        let description = ask_state_schema()["description"]
            .as_str()
            .unwrap()
            .to_string();
        for promised in [
            "leaders",
            "via",
            "leaders_error",
            "leaders_skipped",
            "enabled = false",
            "`null`",
            "possibly empty",
        ] {
            assert!(
                description.contains(promised),
                "{promised:?}: {description}"
            );
        }
    }

    #[test]
    fn ask_state_unfiltered_rolls_up_every_project_including_one_with_no_bee_dir() {
        let (engine, pa, pb) = two_project_engine("ask-state-rollup");
        write(
            &pa.root_path,
            ".bee/state.json",
            r#"{"feature": "widget-polish", "phase": "execution", "mode": "standard"}"#,
        );
        // project b deliberately has no .bee/ at all.
        let resp = call_tool(&engine, "waggledance_ask_state", json!({}));
        let entries = resp["result"]["structuredContent"]["projects"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 2, "{resp}");
        let a = entries.iter().find(|e| e["project_id"] == pa.id).unwrap();
        assert_eq!(a["present"], true);
        assert_eq!(a["feature"], "widget-polish");
        let b = entries.iter().find(|e| e["project_id"] == pb.id).unwrap();
        assert_eq!(
            b["present"], false,
            "absent .bee/ must report absent, not error: {b}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn ask_state_nonexistent_project_is_a_tool_error_naming_it() {
        let (engine, pa, pb) = two_project_engine("ask-state-no-project");
        let resp = call_tool(
            &engine,
            "waggledance_ask_state",
            json!({ "project": "does-not-exist" }),
        );
        assert_eq!(resp["result"]["isError"], true, "{resp}");
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("does-not-exist"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn unknown_tool_stays_on_the_json_rpc_error_path() {
        let (engine, pa, pb) = two_project_engine("unknown-tool");
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "not_a_real_tool", "arguments": {} }
        });
        let resp = handle_tool_call(Some(json!(1)), &engine, &mut None, &req);
        assert_eq!(resp["error"]["code"], -32602, "{resp}");
        assert!(
            resp["result"].is_null(),
            "unknown tool must not be a tool_error: {resp}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// A single-project engine with the terminal family already on
    /// (`Config::default()` leaves it off) — the dispatch-family refusal
    /// tests below build on this and flip the per-project D6 switch
    /// themselves where a test needs it on too.
    fn dispatch_engine(tag: &str) -> (Engine, waggledance_core::domain::Project) {
        let dir =
            std::env::temp_dir().join(format!("waggledance-mcp-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# Project\nnothing interesting.");
        let mut config = Config::default();
        config.terminal.enabled = true;
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        let project = engine.register(&dir, None).unwrap();
        (engine, project)
    }

    #[test]
    fn dispatch_requires_project_and_task() {
        let (engine, pa) = dispatch_engine("dispatch-required-fields");
        let missing_project = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "task": "do the thing", "pane_id": "w1:p1" }),
        );
        assert_eq!(
            missing_project["result"]["isError"], true,
            "{missing_project}"
        );
        let missing_task = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "pane_id": "w1:p1" }),
        );
        assert_eq!(missing_task["result"]["isError"], true, "{missing_task}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn dispatch_requires_exactly_one_of_preset_or_pane_id() {
        let (engine, pa) = dispatch_engine("dispatch-preset-xor-pane");
        let neither = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go" }),
        );
        let neither_text = neither["result"]["content"][0]["text"].as_str().unwrap();
        assert!(neither_text.contains("specify one of"), "{neither_text}");

        let both = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "preset": "p", "pane_id": "w1:p1" }),
        );
        let both_text = both["result"]["content"][0]["text"].as_str().unwrap();
        assert!(both_text.contains("not both"), "{both_text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn dispatch_refuses_an_unknown_project() {
        let (engine, pa) = dispatch_engine("dispatch-unknown-project");
        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": "does-not-exist", "task": "go", "pane_id": "w1:p1" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("does-not-exist"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// D6: the terminal family is off by default even on an
    /// orchestration-enabled project — the refusal must name the remedy.
    #[test]
    fn dispatch_refuses_when_terminal_family_is_off() {
        let dir = std::env::temp_dir().join(format!(
            "waggledance-mcp-dispatch-terminal-off-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# Project\n");
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        let project = engine.register(&dir, None).unwrap();
        engine.set_orchestration_enabled(&project.id, true).unwrap();

        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": project.id, "task": "go", "pane_id": "w1:p1" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("terminal.enabled"), "{text}");

        std::fs::remove_dir_all(&project.root_path).ok();
    }

    /// D6: terminal family on, but this project never opted in — the
    /// refusal must name the remedy (the project's own settings page).
    #[test]
    fn dispatch_refuses_a_project_that_has_not_opted_in() {
        let (engine, pa) = dispatch_engine("dispatch-not-opted-in");
        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "pane_id": "w1:p1" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("has not opted into orchestrator dispatch"),
            "{text}"
        );
        assert!(text.contains("settings page"), "{text}");
        let file = pa.root_path.join(".waggledance.toml");
        assert!(file.is_absolute());
        assert!(
            text.contains(&format!("`orchestration = true` in {}", file.display())),
            "{text}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn dispatch_refusal_reports_a_project_file_that_fails_to_parse() {
        let (engine, pa) = dispatch_engine("dispatch-bad-settings-file");
        write(&pa.root_path, ".waggledance.toml", "orchestration = tru\n");
        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "pane_id": "w1:p1" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("has not opted into"), "{text}");
        assert!(text.contains("failed to parse"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn dispatch_accepts_the_project_file_opt_in() {
        let (engine, pa) = dispatch_engine("dispatch-file-opt-in");
        write(&pa.root_path, ".waggledance.toml", "orchestration = true\n");
        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "preset": "nope" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("has not opted into"), "{text}");
        assert!(text.contains("unknown agent preset: nope"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn dispatch_refuses_an_unknown_preset_label_before_touching_herdr() {
        let (engine, pa) = dispatch_engine("dispatch-unknown-preset");
        engine.set_orchestration_enabled(&pa.id, true).unwrap();

        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "preset": "nope" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown agent preset: nope"), "{text}");
        // D3: with two registries searched, "unknown" alone no longer tells a
        // caller where to look, so the refusal names the project too.
        assert!(
            text.contains(&pa.id),
            "the refusal must name the project whose registry was searched: {text}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// D1/D2: a label only the TARGET PROJECT declares now resolves, using
    /// that project's own argv. This is the asymmetry the feature closes —
    /// a human clicking Start could already spawn this; an agent could not.
    ///
    /// The dispatch is driven all the way to its herdr outcome rather than
    /// stopping at resolution, because "it resolved" is not the claim: the
    /// claim is that a project-declared label stops being refused. Whatever
    /// herdr does next in the sandbox, the answer must no longer be
    /// `unknown agent preset`.
    #[test]
    fn dispatch_resolves_a_label_only_the_target_project_declares() {
        let (engine, pa) = dispatch_engine("dispatch-project-label");
        engine.set_orchestration_enabled(&pa.id, true).unwrap();
        assert!(
            engine.config.terminal.agent_presets.is_empty(),
            "this test's premise is an empty global preset list"
        );
        write(
            &pa.root_path,
            ".bee/config.json",
            r#"{"herding": {"agents": {"pi-agy-flash-3.7": ["pi", "-a", "--model", "x"]}}}"#,
        );

        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "preset": "pi-agy-flash-3.7" }),
        );
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            !text.contains("unknown agent preset"),
            "a label the project declares must not be refused as unknown: {text}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// D1's collision rule, pinned because the collision IS the decision:
    /// the global list is searched first, so an installation that configured
    /// a label keeps it pointing exactly where it did. Proved by resolution,
    /// not by dispatch, because the two argvs differ only in what would be
    /// spawned.
    #[test]
    fn a_label_defined_in_both_sources_resolves_to_the_global_one() {
        let (mut engine, pa) = dispatch_engine("dispatch-preset-collision");
        write(
            &pa.root_path,
            ".bee/config.json",
            r#"{"herding": {"agents": {"shared": ["from-project"]}}}"#,
        );
        engine
            .config
            .terminal
            .agent_presets
            .push(waggledance_core::config::AgentPreset {
                label: "shared".to_string(),
                argv: vec!["from-global".to_string()],
            });

        let resolved = resolve_preset(&engine, &pa, "shared").expect("both sources hold it");
        assert_eq!(
            resolved.0.argv,
            vec!["from-global".to_string()],
            "the global list is searched first, so an existing label never re-aims"
        );

        // And with no global entry, the same label falls through to the
        // project — the fallback only ever fills a gap.
        engine.config.terminal.agent_presets.clear();
        let resolved = resolve_preset(&engine, &pa, "shared").expect("the project holds it");
        assert_eq!(resolved.0.argv, vec!["from-project".to_string()]);

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// D4/D6: a label the project DECLARES but that cannot be started is
    /// refused in those terms, never as unknown — and the line it is refused
    /// on is exactly the line `ask_state` publishes as `resolvable`, so the
    /// tool cannot advertise a label and then reject it.
    ///
    /// *Updated by herding-entry-conditions D1:* the example moved. `agy-flash`
    /// used to be the declared-but-unstartable case purely because this reader
    /// did not understand its shape; it starts now. The rule is unchanged and
    /// still needs a case, so the example is an entry that genuinely yields no
    /// command — declared, and still unable to run.
    #[test]
    fn a_declared_but_unstartable_label_refuses_in_its_own_terms_and_matches_ask_state() {
        let (engine, pa) = dispatch_engine("dispatch-object-form");
        engine.set_orchestration_enabled(&pa.id, true).unwrap();
        write(
            &pa.root_path,
            ".bee/config.json",
            r#"{
                "herding": {
                    "agents": {
                        "claude-sonnet": ["claude", "--model", "sonnet"],
                        "agy-flash": {
                            "argv": ["agy", "--dangerously-skip-permissions"],
                            "workspace_trust": {
                                "file": "~/.gemini/settings.json",
                                "key": "trustedWorkspaces"
                            }
                        },
                        "no-command": {"env": {"K": "v"}}
                    }
                }
            }"#,
        );

        // The object form now starts, which is this feature's whole point.
        resolve_preset(&engine, &pa, "agy-flash")
            .expect("an object entry with a usable argv resolves");

        let refusal = resolve_preset(&engine, &pa, "no-command")
            .expect_err("an entry that yields no command cannot start");
        assert!(refusal.contains("no-command"), "{refusal}");
        assert!(
            !refusal.contains("unknown"),
            "the label exists — calling it unknown sends a caller hunting a typo: {refusal}"
        );

        // The agreement itself: every label ask_state reports resolvable must
        // resolve here, and every one it omits must refuse. This is the case
        // that fails if the two readers ever stop sharing a predicate.
        let state = call_tool(
            &engine,
            "waggledance_ask_state",
            json!({ "project": pa.id }),
        );
        let herding = &state["result"]["structuredContent"]["project"]["herding"];
        for label in herding["agents"].as_array().unwrap() {
            let label = label.as_str().unwrap();
            let published = herding["resolvable"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == label);
            assert_eq!(
                resolve_preset(&engine, &pa, label).is_ok(),
                published,
                "ask_state and dispatch disagree about {label:?}: {herding}"
            );
        }

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// A project that declares no registry at all behaves exactly as it does
    /// today: the fallback finds nothing and the refusal stands.
    #[test]
    fn a_project_with_no_herding_block_still_refuses_every_unconfigured_label() {
        let (engine, pa) = dispatch_engine("dispatch-no-herding");
        assert!(
            resolve_preset(&engine, &pa, "claude-sonnet").is_err(),
            "no global entry and no registry means no command"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// Targeting a pane_id goes through `orchestrate::preflight` before any
    /// send — this proves that wiring end to end against whatever herdr
    /// state the test sandbox actually has: no socket at all reports
    /// unverifiable/unavailable, a live socket with no such pane reports
    /// `no such pane` — either way a named error, never a silent hang and
    /// never a fabricated completion (D5 fail-closed).
    #[test]
    fn dispatch_to_an_unreachable_pane_refuses_fail_closed_never_a_completion() {
        let (engine, pa) = dispatch_engine("dispatch-herdr-unavailable");
        engine.set_orchestration_enabled(&pa.id, true).unwrap();

        let mut orchestration: Option<Orchestration> = None;
        let resp = call_tool_with_orchestration(
            &engine,
            &mut orchestration,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "pane_id": "w1:p1" }),
        );
        assert_eq!(resp["result"]["isError"], true, "{resp}");
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let lower = text.to_lowercase();
        assert!(
            lower.contains("unverifiable")
                || lower.contains("herdr")
                || lower.contains("no such pane"),
            "expected a named fail-closed refusal, got: {text}"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn await_requires_run_id() {
        let (engine, pa) = dispatch_engine("await-required-field");
        let resp = call_tool(&engine, "waggledance_await", json!({}));
        assert_eq!(resp["result"]["isError"], true, "{resp}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    /// An unknown run_id never touches herdr — resolved before the
    /// orchestration runtime is built.
    #[test]
    fn await_refuses_an_unknown_run_id_without_touching_herdr() {
        let (engine, pa) = dispatch_engine("await-unknown-run");
        let resp = call_tool(
            &engine,
            "waggledance_await",
            json!({ "run_id": "does-not-exist" }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown run_id: does-not-exist"), "{text}");

        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    fn seeded_run(project_id: &str, id: &str) -> Run {
        let now = waggledance_core::indexer::now_rfc3339();
        Run {
            id: id.to_string(),
            project_id: project_id.to_string(),
            pane_id: "w1:p1".to_string(),
            preset_label: None,
            task: "do the thing".to_string(),
            baseline: String::new(),
            marker: "HERDR_DONE_deadbeef".to_string(),
            status: "working".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn runs_filtered_lists_only_that_projects_rows() {
        let (engine, pa, pb) = two_project_engine("runs-filtered");
        engine
            .insert_run(&seeded_run(&pa.id, "run-a"), None)
            .unwrap();
        engine
            .insert_run(&seeded_run(&pb.id, "run-b"), None)
            .unwrap();

        let resp = call_tool(&engine, "waggledance_runs", json!({ "project": pa.id }));
        let rows = resp["result"]["structuredContent"]["runs"]
            .as_array()
            .unwrap();
        assert_eq!(rows.len(), 1, "{resp}");
        assert_eq!(rows[0]["id"], "run-a");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    #[test]
    fn runs_unfiltered_spans_every_project() {
        let (engine, pa, pb) = two_project_engine("runs-unfiltered");
        engine
            .insert_run(&seeded_run(&pa.id, "run-a"), None)
            .unwrap();
        engine
            .insert_run(&seeded_run(&pb.id, "run-b"), None)
            .unwrap();

        let resp = call_tool(&engine, "waggledance_runs", json!({}));
        let rows = resp["result"]["structuredContent"]["runs"]
            .as_array()
            .unwrap();
        assert_eq!(rows.len(), 2, "{resp}");

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// D6 / dbn-4: in MCP stdio process, Orchestration opens its own NotifyStore
    /// against the server's database when notifications are enabled, and None
    /// when disabled so the await path raises no alerts.
    #[test]
    fn await_path_receives_store_when_notify_enabled_and_none_when_disabled() {
        // 1. Switch off: orchestration initialized on await path has no notify store
        let (engine_off, pa) = dispatch_engine("await-notify-off");
        assert!(!engine_off.config.terminal.notify_enabled);
        let mut orch_off: Option<Orchestration> = None;
        let orch = orchestration_handle(&mut orch_off, &engine_off.config.terminal).unwrap();
        assert!(
            orch.notify_store.is_none(),
            "D6: notify switch off must not open a notify store on the await path"
        );

        // 2. Switch on: orchestration initialized on await path receives a live notify store
        let mut config_on = Config::default();
        config_on.terminal.enabled = true;
        config_on.terminal.notify_enabled = true;
        let dir_on = std::env::temp_dir().join(format!(
            "waggledance-mcp-await-notify-on-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir_on);
        write(&dir_on, "docs/a.md", "# Project\n");
        let engine_on = Engine::new(SqliteStore::open_in_memory().unwrap(), config_on);
        let pb = engine_on.register(&dir_on, None).unwrap();
        let mut orch_on: Option<Orchestration> = None;
        let orch = orchestration_handle(&mut orch_on, &engine_on.config.terminal).unwrap();
        assert!(
            orch.notify_store.is_some(),
            "notify switch on must open a notify store for the await path"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    /// D6 / dbn-4: `open_notify_store` opens lazily only when `notify_enabled` is true,
    /// and leaves no database file when it is false.
    #[test]
    fn notify_store_in_mcp_opens_only_when_notify_switch_is_on() {
        let dir = std::env::temp_dir().join(format!(
            "waggledance-mcp-notify-store-lazy-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = waggledance_core::config::notify_store_path_override(Some(&dir));

        let off = waggledance_core::config::TerminalConfig::default();
        assert!(!off.notify_enabled);
        let store_off = open_notify_store(&off, Some(&dir));
        assert!(store_off.is_none(), "switch off must return None");
        assert!(
            !path.exists(),
            "opening store with notify switch off must not create database file"
        );

        let on = waggledance_core::config::TerminalConfig {
            notify_enabled: true,
            ..Default::default()
        };
        let store_on = open_notify_store(&on, Some(&dir));
        assert!(store_on.is_some(), "switch on must return Some(store)");
        assert!(
            path.exists(),
            "opening store with notify switch on must create database file"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// D6 / dbn-4: calling `waggledance_await` with notify switch on arms Orchestration
    /// with a notify store, and with it off leaves notify_store None.
    #[test]
    fn await_tool_call_arms_notify_store_under_opt_in_switch() {
        let (engine_off, pa) = dispatch_engine("await-tool-off");
        engine_off
            .insert_run(&seeded_run(&pa.id, "run-tool-off"), None)
            .unwrap();
        let mut orch_off: Option<Orchestration> = None;
        let _ = call_tool_with_orchestration(
            &engine_off,
            &mut orch_off,
            "waggledance_await",
            json!({ "run_id": "run-tool-off", "timeout_seconds": 0 }),
        );
        let orch = orch_off
            .as_ref()
            .expect("orchestration initialized by await");
        assert!(
            orch.notify_store.is_none(),
            "disabled notify switch -> no store"
        );

        let dir = std::env::temp_dir().join(format!(
            "waggledance-mcp-await-tool-on-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# Project\n");
        let mut config_on = Config::default();
        config_on.terminal.enabled = true;
        config_on.terminal.notify_enabled = true;
        let engine_on = Engine::new(SqliteStore::open_in_memory().unwrap(), config_on);
        let pb = engine_on.register(&dir, None).unwrap();
        engine_on
            .insert_run(&seeded_run(&pb.id, "run-tool-on"), None)
            .unwrap();

        let mut orch_on: Option<Orchestration> = None;
        let _ = call_tool_with_orchestration(
            &engine_on,
            &mut orch_on,
            "waggledance_await",
            json!({ "run_id": "run-tool-on", "timeout_seconds": 0 }),
        );
        let orch = orch_on
            .as_ref()
            .expect("orchestration initialized by await");
        assert!(
            orch.notify_store.is_some(),
            "enabled notify switch -> Some(store)"
        );

        std::fs::remove_dir_all(&pa.root_path).ok();
        std::fs::remove_dir_all(&pb.root_path).ok();
    }

    fn leader_engine(
        tag: &str,
        terminal: bool,
        paseo_on: bool,
        opted_in: bool,
    ) -> (Engine, Project, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "waggledance-mcp-leader-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "docs/a.md", "# Project\n");
        let mut config = Config::default();
        config.terminal.enabled = terminal;
        config.paseo.enabled = Some(paseo_on);
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        let project = engine.register(&dir, None).unwrap();
        engine
            .set_orchestration_enabled(&project.id, opted_in)
            .unwrap();
        let project = engine.get_project(&project.id).unwrap().unwrap();
        let data_dir = dir.join(".data");
        (engine, project, data_dir)
    }

    fn leader_call(
        engine: &Engine,
        project: &Project,
        data_dir: &Path,
        task: &str,
        request_id: Option<&str>,
    ) -> Value {
        dispatch_to_leader(
            Some(json!(1)),
            engine,
            project,
            task,
            request_id,
            Some("agent-caller"),
            data_dir,
        )
    }

    fn text_of(resp: &Value) -> String {
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn beat(data_dir: &Path, age_secs: u64) {
        std::fs::create_dir_all(data_dir).unwrap();
        let path = crate::paseo::delivery_heartbeat_path(data_dir);
        let file = std::fs::File::create(&path).unwrap();
        file.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(age_secs))
            .unwrap();
    }

    #[test]
    fn dispatch_refuses_leader_beside_another_target() {
        let (engine, pa) = dispatch_engine("dispatch-leader-xor");
        let resp = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "preset": "p", "leader": true }),
        );
        let text = text_of(&resp);
        assert!(text.contains("exactly one of"), "{text}");
        assert!(text.contains("leader"), "{text}");
        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    #[test]
    fn dispatch_schema_offers_a_request_id_for_every_target() {
        let schema = dispatch_schema();
        let desc = schema["inputSchema"]["properties"]["request_id"]["description"]
            .as_str()
            .unwrap();
        assert!(desc.contains("retry never sends twice"), "{desc}");
    }

    #[test]
    fn leader_dispatch_with_orchestration_off_is_refused_and_stores_nothing() {
        let (engine, project, data_dir) = leader_engine("orch-off", true, true, false);
        let resp = leader_call(&engine, &project, &data_dir, "go", None);
        assert_eq!(resp["result"]["isError"], true);
        let text = text_of(&resp);
        assert!(
            text.contains(&project.id) && text.contains("has not opted into"),
            "{text}"
        );
        assert!(engine.list_runs(&project.id, 10).unwrap().is_empty());
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn leader_dispatch_with_paseo_off_is_refused_naming_the_config_line() {
        let (engine, project, data_dir) = leader_engine("paseo-off", true, false, true);
        let text = text_of(&leader_call(&engine, &project, &data_dir, "go", None));
        assert!(text.contains("~/.waggledance/config.toml"), "{text}");
        assert!(
            text.contains("`[paseo]`") && text.contains("`enabled = false`"),
            "{text}"
        );
        assert!(engine.list_runs(&project.id, 10).unwrap().is_empty());

        let through_the_tool = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": project.id, "task": "go", "leader": true }),
        );
        assert!(text_of(&through_the_tool).contains("enabled = false"));
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn leader_dispatch_is_not_gated_by_the_terminal_surface() {
        let (engine, project, _) = leader_engine("terminal-off", false, false, true);
        let through_the_tool = text_of(&call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": project.id, "task": "go", "leader": true }),
        ));
        assert!(
            !through_the_tool.contains("terminal.enabled") && through_the_tool.contains("[paseo]"),
            "{through_the_tool}"
        );
        std::fs::remove_dir_all(&project.root_path).ok();

        let (engine, project, data_dir) = leader_engine("terminal-off-on", false, true, true);
        let resp = leader_call(&engine, &project, &data_dir, "go", None);
        assert_ne!(resp["result"]["isError"], true, "{resp}");
        let run_id = resp["result"]["structuredContent"]["run_id"]
            .as_str()
            .unwrap();
        assert!(engine.get_run(run_id).unwrap().is_some());
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn a_leader_dispatch_stores_a_queued_run_and_sends_nothing() {
        let (engine, project, data_dir) = leader_engine("queued", true, true, true);
        beat(&data_dir, 0);
        let task = "--help me\n  ship the thing  ";
        let resp = leader_call(&engine, &project, &data_dir, task, Some("req-1"));
        let sc = &resp["result"]["structuredContent"];
        assert_eq!(sc["status"], "queued", "{resp}");
        assert_eq!(sc["duplicate"], false);
        assert_eq!(sc["warnings"], json!([]));
        let run = engine
            .get_run(sc["run_id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(run.pane_id, "paseo:pending");
        assert_eq!(run.preset_label, None);
        assert_eq!(run.task, task);
        assert_eq!(run.marker, "");
        assert_eq!(run.status, "queued");
        assert_eq!(
            engine.run_caller(&run.id).unwrap().as_deref(),
            Some("agent-caller")
        );
        assert_eq!(
            engine
                .run_by_request(&project.id, "req-1")
                .unwrap()
                .unwrap()
                .id,
            run.id
        );
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn a_wd_caller_comes_from_waggledance_caller_only_without_a_paseo_agent() {
        let some = |v: &str| Some(v.to_string());
        assert_eq!(
            dispatch_caller(None, some("wd")).as_deref(),
            Some("openclaw:wd")
        );
        assert_eq!(
            dispatch_caller(some(""), some("wd")).as_deref(),
            Some("openclaw:wd")
        );
        assert_eq!(
            dispatch_caller(some("agent"), some("wd")).as_deref(),
            Some("agent")
        );
        assert_eq!(dispatch_caller(None, some("other")), None);
        assert_eq!(dispatch_caller(None, None), None);
        let schema = dispatch_schema();
        assert!(schema["inputSchema"]["properties"].get("caller").is_none());
        assert!(schema["inputSchema"]["properties"].get("via").is_none());
    }

    #[test]
    fn a_wd_leader_dispatch_stores_the_task_behind_the_wd_prefix() {
        let (engine, project, data_dir) = leader_engine("wd-prefix", true, true, true);
        beat(&data_dir, 0);
        let resp = dispatch_to_leader(
            Some(json!(1)),
            &engine,
            &project,
            "ship it",
            Some("corr-9"),
            Some(paseo::WD_CALLER),
            &data_dir,
        );
        let run_id = resp["result"]["structuredContent"]["run_id"]
            .as_str()
            .unwrap();
        let run = engine.get_run(run_id).unwrap().unwrap();
        assert_eq!(run.task, paseo::wd_task("corr-9", "ship it"));
        assert_eq!(paseo::wd_corr(&run.task), Some("corr-9"));
        assert_eq!(
            engine.run_caller(run_id).unwrap().as_deref(),
            Some("openclaw:wd")
        );
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn a_wd_hand_off_opens_one_task_and_follow_ups_and_other_callers_open_none() {
        let (engine, project, data_dir) = leader_engine("wd-task", true, true, true);
        beat(&data_dir, 0);
        let send = |task: &str, request_id: &str, caller: Option<&str>| {
            dispatch_to_leader(
                Some(json!(1)),
                &engine,
                &project,
                task,
                Some(request_id),
                caller,
                &data_dir,
            )
        };
        send("ship it", "t-abc123", Some(paseo::WD_CALLER));
        let task = engine.get_task("t-abc123").unwrap().unwrap();
        assert_eq!(
            (task.project_id.as_str(), task.state.as_str()),
            (project.id.as_str(), "open")
        );
        assert_eq!(task.changed_at, task.created_at);
        assert_eq!(task.leader_id, None);

        let dup = send("ship it", "t-abc123", Some(paseo::WD_CALLER));
        assert_eq!(dup["result"]["structuredContent"]["duplicate"], true);
        send("more", "t-abc123:1", Some(paseo::WD_CALLER));
        send("plain", "t-def456", Some("agent-caller"));
        send("plain", "t-fed654", None);
        let open: Vec<String> = engine
            .list_open_tasks()
            .unwrap()
            .into_iter()
            .map(|t| t.corr)
            .collect();
        assert_eq!(open, vec!["t-abc123"]);
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn a_via_wd_leader_shows_via_wd_in_ask_state() {
        let (engine, project, base) = leaders_project("wd-via");
        let rollup = bee::read_rollup(std::slice::from_ref(&project.root_path))
            .pop()
            .unwrap();
        let claims = vec![(canonical(&project.root_path), project.id.clone())];
        let root = project.root_path.display().to_string();
        let read = LeadersRead::Listed {
            all: vec![listed("wd-lead", &root, "2 hours ago")],
            wd: ["wd-lead".to_string()].into_iter().collect(),
        };
        let mut digest = ask_state_digest(&project.id, &rollup.snapshot);
        attach_leaders(&mut digest, &engine, &project, &rollup, &claims, &read);
        assert_eq!(digest["leaders"][0]["via"], "wd", "{digest}");
        assert_eq!(digest["leaders"][0]["labels"]["waggledance.via"], "wd");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_leader_dispatch_is_accepted_while_an_earlier_one_is_still_held() {
        let (engine, project, data_dir) = leader_engine("busy", true, true, true);
        beat(&data_dir, 0);
        let first = leader_call(&engine, &project, &data_dir, "one", None);
        let second = leader_call(&engine, &project, &data_dir, "two", None);
        assert_ne!(second["result"]["isError"], true, "{second}");
        assert_eq!(second["result"]["structuredContent"]["status"], "queued");
        assert_ne!(
            first["result"]["structuredContent"]["run_id"],
            second["result"]["structuredContent"]["run_id"]
        );
        let queued = engine.list_queued_paseo_runs(&project.id).unwrap();
        let tasks: Vec<&str> = queued.iter().map(|r| r.task.as_str()).collect();
        assert_eq!(tasks, ["one", "two"]);
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn dispatch_warns_delivery_is_paused_on_a_missing_or_stale_heartbeat() {
        let (engine, project, data_dir) = leader_engine("heartbeat", true, true, true);
        let missing = leader_call(&engine, &project, &data_dir, "a", None);
        assert_eq!(
            missing["result"]["structuredContent"]["warnings"],
            json!([DELIVERY_PAUSED])
        );
        assert!(text_of(&missing).contains("paused — waggledance daemon not running"));

        beat(&data_dir, 60);
        let stale = leader_call(&engine, &project, &data_dir, "b", None);
        assert_eq!(
            stale["result"]["structuredContent"]["warnings"],
            json!([DELIVERY_PAUSED])
        );

        beat(&data_dir, 5);
        let fresh = leader_call(&engine, &project, &data_dir, "c", None);
        assert_eq!(fresh["result"]["structuredContent"]["warnings"], json!([]));
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn await_on_a_queued_leader_run_reports_its_age_and_a_paused_daemon() {
        let (engine, project, data_dir) = leader_engine("await-queued", false, true, true);
        let sent = leader_call(&engine, &project, &data_dir, "go", None);
        let run_id = sent["result"]["structuredContent"]["run_id"].clone();
        let args = json!({ "run_id": run_id, "timeout_seconds": 0 });
        let mut orchestration = None;

        let paused = handle_await(
            Some(json!(2)),
            &engine,
            &mut orchestration,
            &args,
            &data_dir,
        );
        let report = &paused["result"]["structuredContent"];
        assert_eq!(report["status"], "queued");
        assert!(
            report["age_secs"].as_i64().is_some_and(|a| a >= 0),
            "{report}"
        );
        assert_eq!(report["warnings"], json!([DELIVERY_PAUSED]));
        assert!(text_of(&paused).contains("queued for"));

        beat(&data_dir, 0);
        let live = handle_await(
            Some(json!(3)),
            &engine,
            &mut orchestration,
            &args,
            &data_dir,
        );
        assert_eq!(live["result"]["structuredContent"]["warnings"], json!([]));
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn dispatch_with_a_repeated_leader_request_id_returns_the_first_run() {
        let (engine, project, data_dir) = leader_engine("repeat", true, true, true);
        let first = leader_call(&engine, &project, &data_dir, "go", Some("req-7"));
        let again = leader_call(&engine, &project, &data_dir, "go", Some("req-7"));
        assert_eq!(
            again["result"]["structuredContent"]["run_id"],
            first["result"]["structuredContent"]["run_id"]
        );
        assert_eq!(again["result"]["structuredContent"]["duplicate"], true);
        assert_eq!(engine.list_runs(&project.id, 10).unwrap().len(), 1);

        let through_the_tool = call_tool(
            &engine,
            "waggledance_dispatch",
            json!({ "project": project.id, "task": "go", "leader": true, "request_id": "req-7" }),
        );
        let sc = &through_the_tool["result"]["structuredContent"];
        assert_eq!(sc["run_id"], first["result"]["structuredContent"]["run_id"]);
        assert_eq!(sc["status"], "queued");
        assert_eq!(sc["duplicate"], true);
        assert_eq!(engine.list_runs(&project.id, 10).unwrap().len(), 1);
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn dispatch_with_a_repeated_herdr_request_id_returns_the_stored_run_without_herdr() {
        let (engine, pa) = dispatch_engine("dispatch-herdr-repeat");
        engine.set_orchestration_enabled(&pa.id, true).unwrap();
        let stored = engine
            .insert_run_with_request(&seeded_run(&pa.id, "run-h"), None, Some("req-h"), None)
            .unwrap();
        let mut orchestration: Option<Orchestration> = None;
        let resp = call_tool_with_orchestration(
            &engine,
            &mut orchestration,
            "waggledance_dispatch",
            json!({ "project": pa.id, "task": "go", "pane_id": "w9:p9", "request_id": "req-h" }),
        );
        let sc = &resp["result"]["structuredContent"];
        assert_eq!(sc["run_id"], stored, "{resp}");
        assert_eq!(sc["status"], "working");
        assert_eq!(sc["duplicate"], true);
        assert!(orchestration.is_none());
        std::fs::remove_dir_all(&pa.root_path).ok();
    }

    fn task_rig(tag: &str) -> (Engine, Project, std::path::PathBuf, paseo::fake::FakePaseo) {
        let (engine, project, data_dir) = leader_engine(tag, true, true, true);
        beat(&data_dir, 0);
        engine
            .insert_task("t-1", &project.id, "2026-10-22T10:00:00Z")
            .unwrap();
        engine
            .mark_task_delivered("t-1", "lead-1", "2026-10-22T10:01:00Z")
            .unwrap();
        (engine, project, data_dir, paseo::fake::FakePaseo::default())
    }

    async fn task_call(
        engine: &Engine,
        fake: &paseo::fake::FakePaseo,
        data_dir: &Path,
        args: Value,
    ) -> Value {
        task_action(Some(json!(1)), engine, fake, &args, data_dir).await
    }

    fn permission(req_id: &str, seen_at: &str) -> waggledance_core::domain::PermissionRequest {
        waggledance_core::domain::PermissionRequest {
            req_id: req_id.into(),
            corr: "t-1".into(),
            leader_id: "lead-1".into(),
            tool: "Bash".into(),
            description: "rm -rf build".into(),
            seen_at: seen_at.into(),
            answered_at: None,
            closed_at: None,
        }
    }

    #[tokio::test]
    async fn reply_and_redo_queue_numbered_wd_runs_that_resolve_to_the_corr() {
        let (engine, project, data_dir, fake) = task_rig("task-reply");
        let reply = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-1", "action": "reply", "text": "use dd/mm" }),
        )
        .await;
        assert_ne!(reply["result"]["isError"], true, "{reply}");
        let first = engine
            .run_by_request(&project.id, "t-1:1")
            .unwrap()
            .unwrap();
        assert_eq!(reply["result"]["structuredContent"]["run_id"], first.id);
        assert_eq!(
            first.task,
            paseo::wd_task("t-1", "Reply from the user:\nuse dd/mm")
        );
        assert_eq!(paseo::wd_corr(&first.task), Some("t-1"));
        assert_eq!(first.status, "queued");
        assert_eq!(
            engine.run_caller(&first.id).unwrap().as_deref(),
            Some(paseo::WD_CALLER)
        );

        let redo = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-1", "action": "redo", "text": "button on the right" }),
        )
        .await;
        assert_eq!(
            redo["result"]["structuredContent"]["duplicate"], false,
            "{redo}"
        );
        let second = engine
            .run_by_request(&project.id, "t-1:2")
            .unwrap()
            .unwrap();
        assert_eq!(
            second.task,
            paseo::wd_task("t-1", "Redo requested:\nbutton on the right")
        );
        assert!(fake.calls().is_empty());
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[tokio::test]
    async fn accept_closes_the_task_archives_an_unused_leader_and_repeats_quietly() {
        let (engine, project, data_dir, fake) = task_rig("task-accept");
        let args = json!({ "corr": "t-1", "action": "accept" });
        let resp = task_call(&engine, &fake, &data_dir, args.clone()).await;
        assert_eq!(
            resp["result"]["structuredContent"]["archived"], true,
            "{resp}"
        );
        assert_eq!(engine.get_task("t-1").unwrap().unwrap().state, "closed");
        assert_eq!(
            fake.calls(),
            vec![paseo::fake::Call::Archive("lead-1".into())]
        );

        let again = task_call(&engine, &fake, &data_dir, args).await;
        assert_ne!(again["result"]["isError"], true, "{again}");
        assert_eq!(again["result"]["structuredContent"]["archived"], false);
        assert_eq!(fake.calls().len(), 1);
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[tokio::test]
    async fn accept_leaves_a_leader_another_task_or_a_live_run_still_uses() {
        let (engine, project, data_dir, fake) = task_rig("task-accept-shared");
        engine
            .insert_task("t-2", &project.id, "2026-10-22T10:02:00Z")
            .unwrap();
        engine
            .mark_task_delivered("t-2", "lead-1", "2026-10-22T10:03:00Z")
            .unwrap();
        let resp = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-1", "action": "accept" }),
        )
        .await;
        assert_eq!(
            resp["result"]["structuredContent"]["archived"], false,
            "{resp}"
        );
        assert!(text_of(&resp).contains("t-2"), "{resp}");
        assert_eq!(engine.get_task("t-1").unwrap().unwrap().state, "closed");

        task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-2", "action": "reply", "text": "one more" }),
        )
        .await;
        let resp = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-2", "action": "accept" }),
        )
        .await;
        assert_eq!(
            resp["result"]["structuredContent"]["archived"], false,
            "{resp}"
        );
        assert!(text_of(&resp).contains("queued or working"), "{resp}");
        assert!(fake.calls().is_empty());
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[tokio::test]
    async fn allow_and_deny_permit_the_leader_and_mark_the_request_answered() {
        let (engine, project, data_dir, fake) = task_rig("task-permit");
        engine
            .insert_permission_request(&permission("req-old", "2026-10-22T10:05:00Z"))
            .unwrap();
        engine
            .insert_permission_request(&permission("req-new", "2026-10-22T10:06:00Z"))
            .unwrap();
        let allow = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-1", "action": "allow" }),
        )
        .await;
        assert_ne!(allow["result"]["isError"], true, "{allow}");
        let deny = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-1", "action": "deny", "req_id": "req-old" }),
        )
        .await;
        assert_ne!(deny["result"]["isError"], true, "{deny}");
        assert_eq!(
            fake.calls(),
            vec![
                paseo::fake::Call::Permit("lead-1".into(), "req-new".into(), true),
                paseo::fake::Call::Permit("lead-1".into(), "req-old".into(), false),
            ]
        );
        assert!(engine
            .unanswered_permission_requests("t-1")
            .unwrap()
            .is_empty());

        let none_left = task_call(
            &engine,
            &fake,
            &data_dir,
            json!({ "corr": "t-1", "action": "allow" }),
        )
        .await;
        assert_eq!(none_left["result"]["isError"], true);
        assert!(text_of(&none_left).contains("no unanswered permission request"));
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[tokio::test]
    async fn task_refusals_name_their_cause_and_touch_nothing() {
        let (engine, project, data_dir, fake) = task_rig("task-refuse");
        engine
            .insert_task("t-new", &project.id, "2026-10-22T10:00:00Z")
            .unwrap();
        engine
            .insert_task("t-done", &project.id, "2026-10-22T10:00:00Z")
            .unwrap();
        engine
            .mark_task_delivered("t-done", "lead-9", "2026-10-22T10:01:00Z")
            .unwrap();
        engine.close_task("t-done", "2026-10-22T10:02:00Z").unwrap();
        let cases = [
            (
                json!({ "action": "reply", "text": "x" }),
                "\"corr\" is required",
            ),
            (
                json!({ "corr": "t-1", "action": "merge" }),
                "unknown action merge",
            ),
            (
                json!({ "corr": "t-1", "action": "reply" }),
                "\"text\" is required",
            ),
            (
                json!({ "corr": "t-zz", "action": "reply", "text": "x" }),
                "unknown task t-zz",
            ),
            (
                json!({ "corr": "t-done", "action": "reply", "text": "x" }),
                "task t-done is closed",
            ),
            (
                json!({ "corr": "t-done", "action": "allow" }),
                "task t-done is closed",
            ),
            (
                json!({ "corr": "t-new", "action": "redo", "text": "x" }),
                "task t-new has no leader yet",
            ),
            (
                json!({ "corr": "t-new", "action": "deny" }),
                "task t-new has no leader yet",
            ),
        ];
        for (args, cause) in cases {
            let resp = task_call(&engine, &fake, &data_dir, args.clone()).await;
            assert_eq!(resp["result"]["isError"], true, "{args} {resp}");
            assert!(text_of(&resp).contains(cause), "{args} {resp}");
        }
        assert!(fake.calls().is_empty());
        assert!(engine.list_runs(&project.id, 10).unwrap().is_empty());
        std::fs::remove_dir_all(&project.root_path).ok();
    }

    #[test]
    fn the_task_tool_is_listed_and_routed() {
        let schema = task_schema();
        assert_eq!(schema["name"], "waggledance_task");
        assert_eq!(schema["inputSchema"]["required"], json!(["corr", "action"]));
        let (engine, pa) = dispatch_engine("task-routed");
        let resp = call_tool(
            &engine,
            "waggledance_task",
            json!({ "corr": "t-x", "action": "reply", "text": "x" }),
        );
        assert!(text_of(&resp).contains("unknown task t-x"), "{resp}");
        std::fs::remove_dir_all(&pa.root_path).ok();
    }
}
