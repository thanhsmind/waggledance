//! herdr port — waggledance is a **client of the herdr server** (peer to the
//! TUI), talking `herdr.sock`'s JSON request/response API (DISCOVERY 2026-07-18).
//!
//! Ported from herdr-go (`docs/history/agent-terminal/CONTEXT.md` D1) into
//! the `waggledance` binary crate, not `waggledance-core`: `waggledance-core` is enforced
//! async-runtime-free (`bee::tests::no_web_framework_dependency_declared`
//! asserts no `tokio`/`axum`/`hyper` in its `Cargo.toml`), and this client
//! needs `tokio` for the socket I/O. `waggledance` already depends on both tokio
//! and axum, so it carries the extra weight this port needs no axum for.
//! `pane_scroller.rs` ports herdr-go's alt-screen scrollback nudger; it has
//! no route-level consumer yet (see its own module doc).
//!
//! The surface is request/response only (no live stream): snapshot the runtime,
//! read a pane's screen (polled), and send input as a reply. One trait, two
//! implementations — [`socket::SocketHerdr`] over the real socket and
//! [`fake::FakeHerdr`] for tests.

// `fake` backs every test's `Herdr` (see the module doc above); nothing
// outside `#[cfg(test)]` ever constructs it, so it is gated the same way.
#[cfg(test)]
pub mod fake;
pub mod pane_scroller;
pub mod socket;
pub mod wire;

use async_trait::async_trait;

pub use wire::{AgentStatus, ProtocolInfo, ScreenRead, Snapshot};
// `HERDR_PROTOCOL` reaches production code only through `super::wire::*`
// (see `socket.rs`) — this re-export exists for the `#[cfg(test)]` block
// in `main.rs`, so it is test-only too. `fake.rs` also imports `wire::*`
// directly, so `Agent` needs no re-export at all here.
#[cfg(test)]
pub use wire::HERDR_PROTOCOL;

#[derive(Debug, thiserror::Error)]
pub enum HerdrError {
    #[error("herdr runtime is unavailable: {0}")]
    Unavailable(String),
    #[error("protocol mismatch: gateway pins {expected}, server reports {actual}")]
    ProtocolMismatch { expected: u32, actual: u32 },
    #[error("herdr request failed: {0}")]
    Request(String),
    #[error("malformed herdr response: {0}")]
    Malformed(String),
    #[error("no such pane: {0}")]
    NoSuchPane(String),
    #[error("agent name already in use: {name} ({message})")]
    AgentNameTaken { name: String, message: String },
    #[error("workspace not found: {workspace_id} ({message})")]
    WorkspaceNotFound {
        workspace_id: String,
        message: String,
    },
    #[error("invalid agent argv: {0}")]
    InvalidAgentArgv(String),
    /// `tab.create` succeeded but no pane carrying that tab could be found
    /// in the snapshot that followed. Protocol 20 does not hand the pane back
    /// with the tab, so this is the one hop that can come up empty — and it
    /// is reported rather than papered over, because the tempting recovery
    /// (use some other pane) means starting an agent on top of work somebody
    /// else has open. The tab id rides along so a human can find and close
    /// what was left behind.
    #[error(
        "tab {tab_id} was created in workspace {workspace_id} but no pane for it appeared; \
         started nothing, and the tab is still open"
    )]
    TabPaneUnresolved {
        tab_id: String,
        workspace_id: String,
    },
    /// `agent.prompt` refused before sending anything -- the agent was
    /// already `Blocked` (dispatch-submit-and-reclaim plan, "the decisive
    /// finding"). Distinct from [`HerdrError::AgentPromptStalled`]: here
    /// nothing was submitted at all, so there is nothing to worry about
    /// re-sending.
    #[error("agent is blocked and refused the prompt: {0}")]
    AgentBlocked(String),
    /// `agent.prompt` submitted the text, but the agent never showed even
    /// one observed state change within the daemon's own ~5000ms window --
    /// the daemon's `agent_prompt_stalled`. The text already went in
    /// (dispatch-submit-and-reclaim P2-3: never retry a stall, it would
    /// re-type into a composer that may already hold it). Kept distinct
    /// from [`HerdrError::Timeout`] so a caller can branch on "no confirmed
    /// change" without also catching an ordinary deadline.
    #[error("agent prompt stalled: no confirmed state change observed ({0})")]
    AgentPromptStalled(String),
    /// `agent.prompt`'s own `timeout_ms` elapsed before the agent reached
    /// any of the requested `until` states. Unlike
    /// [`HerdrError::AgentPromptStalled`], a state change WAS observed
    /// first, so the text still went in -- the daemon's `timeout` code.
    #[error("agent prompt timed out waiting for a matching state: {0}")]
    Timeout(String),
    #[error("herdr refused the request ({code}): {message}")]
    Remote { code: String, message: String },
}

pub type Result<T> = std::result::Result<T, HerdrError>;

/// Which of herdr's own `pane.read` sources to request (matches herdr's own
/// `source` vocabulary, not a gateway invention). `Visible` is the current
/// on-screen rows; `Recent` is herdr's own scrollback buffer, hard-capped at
/// 1000 lines server-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSource {
    Visible,
    Recent,
}

impl ReadSource {
    /// The exact wire string herdr's `pane.read` expects for `source`.
    pub fn as_wire(&self) -> &'static str {
        match self {
            ReadSource::Visible => "visible",
            ReadSource::Recent => "recent",
        }
    }
}

/// Result of `tab.create` — the new tab's id and its root pane's id, both
/// opaque and read straight off the response, never constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabCreated {
    pub tab_id: String,
}

/// Result of `agent.start` — the new agent's pane/tab ids, plus the name
/// that actually succeeded (auto-generated, and only ever different from an
/// earlier attempt after that attempt collided).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStarted {
    pub tab_id: String,
    pub pane_id: String,
    pub name: String,
}

/// Auto-generate an agent name — the caller never types one.
fn generate_agent_name() -> String {
    format!("mobile-agent-{:06x}", rand::random::<u32>() & 0xff_ffff)
}

/// Owns the collision retry so callers never see `AgentNameTaken` themselves:
/// generate a name, try it, and on `AgentNameTaken` regenerate and try again.
/// Bounded at 5 attempts — a sixth consecutive collision means the name
/// generator itself is producing bad names, and looping harder would only
/// hide that, so the bound surfaces its own distinguishable `AgentNameTaken`
/// instead of silently giving up or looping forever.
///
/// Generic over both the name source and the "try once" call, so the retry
/// logic itself is testable against a deterministic name sequence,
/// independent of the real (random) generator `agent_start` uses in
/// production — the same pure-seam idea as `tab_create_params` /
/// `attach_workspace_id` in `socket.rs`.
async fn retry_on_name_collision<G, F, Fut>(
    mut generate_name: G,
    mut try_once: F,
) -> Result<AgentStarted>
where
    G: FnMut() -> String,
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<AgentStarted>>,
{
    const MAX_ATTEMPTS: u32 = 5;
    let mut last_collision: Option<(String, String)> = None;
    for _ in 0..MAX_ATTEMPTS {
        let name = generate_name();
        match try_once(name.clone()).await {
            Ok(started) => return Ok(started),
            Err(HerdrError::AgentNameTaken { name, message }) => {
                last_collision = Some((name, message));
            }
            Err(other) => return Err(other),
        }
    }
    let (name, message) = last_collision.unwrap_or_default();
    Err(HerdrError::AgentNameTaken {
        name,
        message: format!(
            "gave up after {MAX_ATTEMPTS} consecutive name collisions ({message}); the name generator may be broken"
        ),
    })
}

/// Everything waggledance needs from herdr — all request/response.
#[async_trait]
pub trait Herdr: Send + Sync {
    /// Snapshot the server's runtime (the flat agent list).
    async fn snapshot(&self) -> Result<Snapshot>;

    /// Health + protocol handshake; a mismatch is a typed error.
    async fn ping(&self) -> Result<ProtocolInfo>;

    /// Read one pane's rendered screen (polled for observation). `source`
    /// selects herdr's own `visible` (current on-screen rows) or `recent`
    /// (scrollback) read; `lines` is honored only for `Recent` (herdr ignores
    /// it for `Visible`) and is capped at herdr's own 1000-line server-side
    /// limit.
    async fn read_pane(
        &self,
        pane_id: &str,
        source: ReadSource,
        lines: usize,
    ) -> Result<ScreenRead>;

    /// Send a reply into a pane. `text` is typed in; `submit` then sends Enter
    /// as a second, separate request (handles herdr's send≠submit: text
    /// alone does not submit). When `text` is non-empty and `submit` is
    /// true, the Enter is held back until the pane's screen settles (two
    /// consecutive `Visible` reads report the same screen TEXT, not
    /// revision — see `SocketHerdr::wait_for_pane_to_settle`'s measurement
    /// note on why `revision` is a dead field — or a bounded cap elapses) —
    /// a slow composer redraw, e.g. an attachment path still
    /// resolving into an image chip, can otherwise swallow an Enter that
    /// lands mid-transition, leaving the whole reply sitting unsent in the
    /// composer (terminal-attach-submit-race). The settle wait never blocks
    /// the submit itself: on the cap, a read failure, or any error from the
    /// poll, the Enter is still sent and this still returns `Ok`. Both
    /// `submit: false` and an empty `text` with `submit: true` skip the
    /// settle wait entirely — see `SocketHerdr::wait_for_pane_to_settle`.
    async fn send_input(&self, pane_id: &str, text: &str, submit: bool) -> Result<()>;

    /// Submit text into a pane's REGISTERED agent and confirm the daemon
    /// itself observed the agent respond, rather than firing a blind Enter
    /// and hoping (`send_input`'s failure mode --
    /// dispatch-submit-and-reclaim defect A: a settle-race heuristic cannot
    /// know whether a keystroke was accepted; only the agent's own observed
    /// state can answer that). Wraps herdr's `agent.prompt`, whose own
    /// contract (`herdr agent prompt --help`, confirmed via
    /// `herdr api schema --json`) is:
    ///
    /// - an agent already `Blocked` refuses with [`HerdrError::AgentBlocked`]
    ///   BEFORE any input is sent -- nothing was submitted, nothing to
    ///   worry about re-sending;
    /// - otherwise the text is submitted, then the daemon waits up to
    ///   `timeout_ms` for the agent to reach one of `until`. Starting from a
    ///   non-`Working` state it first requires an observed state CHANGE
    ///   within its own internal ~5000ms window, or refuses with
    ///   [`HerdrError::AgentPromptStalled`] -- the text already went in, so
    ///   the caller must never retry a stall, only report it
    ///   (dispatch-submit-and-reclaim P2-3);
    /// - a `timeout_ms` shorter than that pending change instead refuses
    ///   with [`HerdrError::Timeout`] -- kept distinct from a stall because
    ///   a state change WAS observed, so the text still landed.
    ///
    /// `AgentBlocked`, `AgentPromptStalled` and `Timeout` are three
    /// deliberately distinct variants (never folded into the generic
    /// `Remote`) so a caller can branch on "definitely nothing sent" versus
    /// "sent but no confirmed change" without also catching an ordinary
    /// deadline that still means success.
    ///
    /// On success, returns the agent's own observed [`AgentStatus`] --
    /// whichever member of `until` it matched.
    async fn agent_prompt(
        &self,
        pane_id: &str,
        text: &str,
        until: &[AgentStatus],
        timeout_ms: u64,
    ) -> Result<AgentStatus>;

    /// Send raw bytes into a pane via herdr's `pane.send_text` channel — no
    /// bracketed-paste wrapping, no named-key translation, exactly the bytes
    /// given. Used to replay a VT escape sequence an alt-screen agent's own
    /// process interprets as a scroll gesture; never `send_keys`/`send_input`
    /// for this purpose.
    async fn send_text(&self, pane_id: &str, bytes: &str) -> Result<()>;

    /// Send raw key presses to a pane — e.g. arrow keys to drive a TUI option
    /// menu, or Enter/Escape/Tab. Key names are herdr's (`up`, `down`, `enter`,
    /// `escape`, `tab`, …).
    async fn send_keys(&self, pane_id: &str, keys: &[String]) -> Result<()>;

    /// Create a plain shell tab in `workspace_id`, never stealing the
    /// desktop's focus (`focus: false`). Returns the new tab's id.
    ///
    /// **It does not return a pane.** Protocol 20's `tab_created` carries
    /// `{type, tab}` and its `TabInfo` has no pane id of any kind, so the
    /// pane a caller wants must be found afterwards by matching this
    /// `tab_id` against a fresh snapshot (`pane.list` filters by workspace
    /// only). That hop is the caller's, deliberately: it is where the
    /// "no pane for this tab" failure has to be decided, and it must fail
    /// loudly rather than settle for a neighbouring pane.
    ///
    /// `cwd` is optional: `Some(path)` seeds that exact directory; `None`
    /// omits the key and lets herdr resolve the **workspace's own anchor**
    /// (its focused pane's folder), which is exactly what the desktop does.
    async fn tab_create(&self, workspace_id: &str, cwd: Option<&str>) -> Result<TabCreated>;

    /// Start a named agent **in an existing pane**. The name is
    /// auto-generated and a collision retried transparently (see
    /// `retry_on_name_collision`): callers never see `AgentNameTaken`
    /// themselves. Returns the pane's and tab's ids plus the name that
    /// actually succeeded.
    ///
    /// `argv` is split the way herdr's own protocol wants it and the way bee
    /// already writes it: **`argv[0]` is the agent `kind`** (`claude`, `pi`,
    /// `codex`, `agy`) and the rest are its `args`. An empty `argv` is
    /// refused before anything reaches the socket.
    ///
    /// There is no `cwd` here, and that is protocol 20's doing rather than a
    /// simplification: `agent.start` no longer creates anything, so the
    /// directory question is settled earlier, when the pane is made. The
    /// caller creates a tab at the destination it has already validated,
    /// resolves that tab's pane, and passes it here — which also removes the
    /// old hazard this doc used to warn about, where omitting `cwd` started
    /// an agent in herdr's own process directory.
    async fn agent_start(&self, pane_id: &str, argv: &[String]) -> Result<AgentStarted>;

    /// Close one pane — the teardown counterpart the spawn verbs above
    /// never had (dispatch-submit-and-reclaim defect B: every
    /// spawn-dispatch left its agent process alive). Wraps herdr's
    /// `pane.close`, whose params are a `PaneTarget { pane_id }` and whose
    /// reply is `pane_closed` (`herdr api schema --json`); a pane that is
    /// already gone answers `pane_not_found`, which arrives here as a
    /// generic [`HerdrError::Remote`] carrying that code.
    ///
    /// This kills a process. Nothing in this crate may call it from an
    /// INFERRED completion: `orchestrate::finish` closes only for a run
    /// whose agent printed its own done marker (D2 — completion is an
    /// explicit declaration, never an inferred state), because a pane's
    /// observed state cannot tell a finished agent from one working
    /// quietly in the background.
    async fn close_pane(&self, pane_id: &str) -> Result<()>;
}

/// Why a trust seeding did not happen. Never fatal — see
/// [`seed_workspace_trust`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustSeedWarning(pub String);

impl std::fmt::Display for TrustSeedWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Add ONE directory to a foreign tool's own per-workspace trust list, so an
/// agent that gates on it does not stop at a trust prompt the operator never
/// sees (herding-entry-conditions D2).
///
/// This is waggledance's only write outside a repository, and the only one
/// into another program's settings. What keeps that narrow enough to audit is
/// not this function's care but its arguments: it is handed **one absolute
/// directory**, by a caller that has already validated it against the
/// project's own boundary, and it invents no path and reads none from
/// anywhere else (D3). It cannot trust a folder nobody was about to start an
/// agent in, because it is never told about one.
///
/// - **Adds only.** An entry is never removed, and no other key is rewritten.
///   The file round-trips through `Value`, not through a typed struct that
///   would silently drop whatever it does not model.
/// - **Idempotent.** An already-trusted directory is a no-op that does not
///   touch the file at all (D4).
/// - **Fail-open.** Every failure — no file, unreadable, unparseable, the key
///   absent or not an array, a write denied — returns a warning and leaves the
///   file exactly as it was. It never blocks a spawn (D5). bee's own
///   `preflight_workspace_trust` is fail-open, and diverging here would make
///   one declaration behave differently depending on which spawner ran it.
///   The tempting reading of "security" is to refuse; bee does not, and a
///   refusal would strand every agent that declares a trust store.
pub fn seed_workspace_trust(
    trust: &waggledance_core::bee::BeeWorkspaceTrust,
    directory: &str,
) -> std::result::Result<(), TrustSeedWarning> {
    let path = expand_home(&trust.file);
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        TrustSeedWarning(format!(
            "could not read trust store {}: {e}",
            path.display()
        ))
    })?;
    let mut doc: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        TrustSeedWarning(format!(
            "could not parse trust store {}: {e}",
            path.display()
        ))
    })?;

    let Some(list) = doc.get_mut(&trust.key).and_then(|v| v.as_array_mut()) else {
        return Err(TrustSeedWarning(format!(
            "trust store {} has no array at key {:?}",
            path.display(),
            trust.key
        )));
    };
    if list.iter().any(|e| e.as_str() == Some(directory)) {
        // Already trusted: not a write. A spawn is not a config edit, and
        // running one twice must not churn the operator's file.
        return Ok(());
    }
    list.push(serde_json::Value::String(directory.to_string()));

    let rendered = serde_json::to_string_pretty(&doc)
        .map_err(|e| TrustSeedWarning(format!("could not render trust store: {e}")))?;
    std::fs::write(&path, rendered + "\n").map_err(|e| {
        TrustSeedWarning(format!(
            "could not write trust store {}: {e}",
            path.display()
        ))
    })?;
    Ok(())
}

/// Expand a leading `~` against the current user's home. Any other path is
/// returned as given — this resolves a prefix, it does not search.
fn expand_home(raw: &str) -> std::path::PathBuf {
    match raw.strip_prefix("~/") {
        Some(rest) => match dirs::home_dir() {
            Some(home) => home.join(rest),
            None => std::path::PathBuf::from(raw),
        },
        None => std::path::PathBuf::from(raw),
    }
}

/// Start an agent in a pane of its own: create a tab at `cwd`, find the pane
/// that tab brought with it, and start the agent there.
///
/// This is what protocol 20 turned `agent.start` into. Before it, one call
/// both made a pane and launched into it; now `agent.start` only ever
/// attaches to an existing pane, and `tab_created` does not say which pane it
/// just made — `TabInfo` carries no pane id, and `pane.list` filters by
/// workspace, not tab. So the hop goes through a fresh snapshot, matching on
/// `tab_id`.
///
/// **Matched on `tab_id` alone.** Not the newest pane, not the focused one,
/// not the last in the list: each of those picks the wrong pane on a busy
/// machine, and the cost of picking wrong is an agent typing into somebody
/// else's session. When no pane matches, this returns
/// [`HerdrError::TabPaneUnresolved`] and starts nothing — the tab is left
/// standing and named in the error rather than quietly reused.
///
/// One implementation for every caller (the board's spawn, the board's
/// shell-create, and MCP dispatch) on purpose: a second copy of this hop
/// would be free to drift into a friendlier fallback, and friendlier is
/// exactly wrong here.
pub async fn start_agent_in_new_tab(
    herdr: &dyn Herdr,
    workspace_id: &str,
    cwd: Option<&str>,
    argv: &[String],
) -> Result<AgentStarted> {
    let entry = waggledance_core::bee::BeeHerdingEntry {
        argv: argv.to_vec(),
        env: Vec::new(),
        workspace_trust: None,
    };
    Ok(start_declared_agent(herdr, workspace_id, cwd, &entry)
        .await?
        .started)
}

/// What a spawn produced: the agent, and anything that went wrong on the way
/// which did not stop it (herding-entry-conditions D9).
///
/// `warnings` is the reason this type exists. A trust seeding that fails is
/// fail-open by design — but a warning that reaches only the daemon's log is
/// indistinguishable from no warning at the moment an operator is staring at
/// a pane that will not move. So it travels back to whoever asked for the
/// spawn, in the same answer that says the agent started.
#[derive(Debug, Clone)]
pub struct SpawnOutcome {
    pub started: AgentStarted,
    pub warnings: Vec<String>,
}

/// Start an agent from a project's own declaration, honouring the conditions
/// it carries: seed the tool's trust store, export its `env`, then start.
///
/// Order matters and is bee's: trust is seeded before the pane exists at all
/// (nothing about it depends on the pane), `env` is exported into the pane
/// **before** `agent.start` so the agent inherits it, and only then does the
/// agent run.
///
/// The two failures are deliberately asymmetric, and that asymmetry is bee's
/// rather than ours to tidy: a trust seeding that fails **warns and proceeds**
/// (D5), while an `env` line that fails to send is **fatal** — the pane has
/// already been made and the agent would start without the environment it was
/// declared to have, which is a different program than the one asked for.
pub async fn start_declared_agent(
    herdr: &dyn Herdr,
    workspace_id: &str,
    cwd: Option<&str>,
    entry: &waggledance_core::bee::BeeHerdingEntry,
) -> Result<SpawnOutcome> {
    let mut warnings = Vec::new();

    // D3: the directory seeded is the one the caller already validated — this
    // reaches for nothing else, and when the caller named no directory there
    // is nothing here to trust.
    if let (Some(trust), Some(dir)) = (entry.workspace_trust.as_ref(), cwd) {
        if let Err(w) = seed_workspace_trust(trust, dir) {
            warnings.push(w.0);
        }
    }

    let created = herdr.tab_create(workspace_id, cwd).await?;
    let pane_id = pane_of_tab(herdr, &created.tab_id).await?.ok_or_else(|| {
        HerdrError::TabPaneUnresolved {
            tab_id: created.tab_id.clone(),
            workspace_id: workspace_id.to_string(),
        }
    })?;

    // A pane exists the moment the tab does, but it is not immediately able
    // to host an agent — herdr answers `agent_pane_busy: … is not an
    // available shell` for the first fraction of a second while the shell
    // comes up. Observed live on 2026-08-25, one step past the protocol port.
    // So: try, and on THAT refusal only, wait and try again, a small fixed
    // number of times.
    //
    // Only that code is retried. A retry loop that swallowed other failures
    // would be worse than the race it fixes — a name collision, an
    // unreachable socket or a refusal from the agent itself must surface at
    // once. And when the attempts run out, herdr's own last words come back:
    // never a summary, and never another pane.
    // D6, bee's own rules: one export line before the agent starts, keys
    // `[A-Za-z_][A-Za-z0-9_]*` and newline-free values, a violating entry
    // dropped while the rest still go (the registry's fail-open-per-entry
    // rule). A failed SEND is fatal, unlike a dropped entry.
    let exports: Vec<String> = entry
        .env
        .iter()
        .filter(|(k, v)| is_env_key(k) && !v.contains('\n'))
        .map(|(k, v)| format!("export {k}='{}'", v.replace('\'', "'\\''")))
        .collect();
    if !exports.is_empty() {
        herdr
            .send_input(&pane_id, &exports.join("; "), true)
            .await?;
    }

    let argv = &entry.argv;
    let mut attempt = 0;
    loop {
        match herdr.agent_start(&pane_id, argv).await {
            Err(e) if attempt + 1 < PANE_READY_ATTEMPTS && is_pane_not_ready(&e) => {
                attempt += 1;
                tokio::time::sleep(PANE_READY_INTERVAL).await;
            }
            Err(e) => return Err(e),
            Ok(started) => return Ok(SpawnOutcome { started, warnings }),
        }
    }
}

/// bee's environment-key rule: `[A-Za-z_][A-Za-z0-9_]*`. A key outside it
/// drops its own entry and nothing else.
fn is_env_key(k: &str) -> bool {
    let mut chars = k.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// How many times [`start_agent_in_new_tab`] will re-offer a freshly created
/// pane that herdr says is not ready yet, and how long it waits between
/// offers. Deliberately small: a pane that is genuinely unusable should fail
/// in about a second, not hold a caller for minutes.
const PANE_READY_ATTEMPTS: u32 = 6;
const PANE_READY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

/// herdr's "that pane cannot host an agent yet" refusal — the startup race,
/// and the only error [`start_agent_in_new_tab`] retries.
fn is_pane_not_ready(e: &HerdrError) -> bool {
    matches!(e, HerdrError::Remote { code, .. } if code == "agent_pane_busy")
}

/// The pane belonging to `tab_id`, read from a fresh snapshot. `None` when
/// the tab has no pane yet — the caller decides what that means.
pub async fn pane_of_tab(herdr: &dyn Herdr, tab_id: &str) -> Result<Option<String>> {
    let snapshot = herdr.snapshot().await?;
    Ok(snapshot
        .panes
        .iter()
        .find(|p| p.tab_id == tab_id)
        .map(|p| p.pane_id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D9, and it is the load-bearing one: fail-open must not mean fail-quiet.
    /// The seeding fails, the agent still starts — and the answer that says so
    /// also says the trust could not be seeded. Asserted on the returned
    /// value, never on a log line, because a warning only in the daemon's log
    /// is indistinguishable from no warning at the moment an operator is
    /// looking at a pane that will not move.
    #[tokio::test]
    async fn a_failed_seeding_does_not_stop_the_spawn_and_does_not_go_quiet() {
        let h = FlakyPane::refusing(0);
        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: vec!["agy".to_string()],
            env: Vec::new(),
            workspace_trust: Some(waggledance_core::bee::BeeWorkspaceTrust {
                file: "/definitely/not/here/settings.json".to_string(),
                key: "trustedWorkspaces".to_string(),
            }),
        };

        let outcome = start_declared_agent(&h, "w1", Some("/projects/beehive"), &entry)
            .await
            .expect("a trust failure must not stop the spawn");

        assert_eq!(outcome.started.pane_id, "w1:new");
        assert_eq!(
            outcome.warnings.len(),
            1,
            "the failure must travel back with the answer: {:?}",
            outcome.warnings
        );
        assert!(
            outcome.warnings[0].contains("trust store"),
            "the warning must say what could not be done: {}",
            outcome.warnings[0]
        );
    }

    /// D3's audit tie: the directory offered to the trust store is the one the
    /// caller passed — the destination it already validated — and nothing
    /// else. Proved by seeding a real temp store and reading back exactly what
    /// arrived in it.
    #[tokio::test]
    async fn the_directory_seeded_is_the_one_the_caller_validated() {
        let dir = std::env::temp_dir().join(format!("wd-tie-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = dir.join("settings.json");
        std::fs::write(&store, r#"{"trustedWorkspaces": []}"#).unwrap();

        let h = FlakyPane::refusing(0);
        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: vec!["agy".to_string()],
            env: Vec::new(),
            workspace_trust: Some(waggledance_core::bee::BeeWorkspaceTrust {
                file: store.to_string_lossy().into_owned(),
                key: "trustedWorkspaces".to_string(),
            }),
        };

        let outcome = start_declared_agent(&h, "w1", Some("/projects/beehive"), &entry)
            .await
            .unwrap();
        assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);

        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&store).unwrap()).unwrap();
        assert_eq!(
            doc["trustedWorkspaces"],
            serde_json::json!(["/projects/beehive"]),
            "exactly the caller's directory, and nothing else"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// D6: env is exported before the agent starts, and bee's key rule drops a
    /// violating entry while the rest still go — fail-open per entry, not per
    /// declaration.
    #[tokio::test]
    async fn env_is_exported_before_the_agent_starts_and_a_bad_key_drops_only_itself() {
        let h = FlakyPane::refusing(0);
        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: vec!["agy".to_string()],
            env: vec![
                ("GOOD_KEY".to_string(), "yes".to_string()),
                ("bad key".to_string(), "dropped".to_string()),
                ("ALSO_GOOD".to_string(), "kept".to_string()),
            ],
            workspace_trust: None,
        };

        start_declared_agent(&h, "w1", None, &entry).await.unwrap();

        let sent = h.inputs.lock().unwrap().clone();
        assert_eq!(sent.len(), 1, "one export line, before the agent: {sent:?}");
        assert!(sent[0].contains("export GOOD_KEY='yes'"), "{}", sent[0]);
        assert!(sent[0].contains("export ALSO_GOOD='kept'"), "{}", sent[0]);
        assert!(
            !sent[0].contains("bad key"),
            "a key outside bee's rule drops its own entry only: {}",
            sent[0]
        );
    }

    /// An entry with no conditions costs nothing extra — no store touched, no
    /// line sent. The common case must not pay for the rare one.
    #[tokio::test]
    async fn an_entry_with_no_conditions_sends_nothing_extra() {
        let h = FlakyPane::refusing(0);
        let entry = waggledance_core::bee::BeeHerdingEntry {
            argv: vec!["claude".to_string()],
            env: Vec::new(),
            workspace_trust: None,
        };

        start_declared_agent(&h, "w1", Some("/projects/beehive"), &entry)
            .await
            .unwrap();

        assert!(h.inputs.lock().unwrap().is_empty(), "no export line");
    }

    fn trust_file(tag: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("wd-trust-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, body).unwrap();
        path
    }

    fn trust_decl(path: &std::path::Path) -> waggledance_core::bee::BeeWorkspaceTrust {
        waggledance_core::bee::BeeWorkspaceTrust {
            file: path.to_string_lossy().into_owned(),
            key: "trustedWorkspaces".to_string(),
        }
    }

    /// D3: the write adds EXACTLY the directory it was handed, and touches
    /// nothing else — not another entry, not another key. This is the whole
    /// audit story for waggledance's only write outside a repository.
    #[test]
    fn seeding_adds_exactly_the_one_directory_and_disturbs_nothing_else() {
        let path = trust_file(
            "add",
            r#"{"trustedWorkspaces": ["/already/here"], "otherSetting": {"keep": true}}"#,
        );
        let decl = trust_decl(&path);

        seed_workspace_trust(&decl, "/projects/beehive").expect("an untrusted path is added");

        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            doc["trustedWorkspaces"],
            serde_json::json!(["/already/here", "/projects/beehive"]),
            "the existing entry survives and exactly one is appended"
        );
        assert_eq!(
            doc["otherSetting"],
            serde_json::json!({"keep": true}),
            "an unrelated key must survive a write untouched"
        );

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    /// D4: a spawn is not a config edit. Seeding an already-trusted directory
    /// leaves the file alone — not rewritten, not reformatted.
    #[test]
    fn seeding_an_already_trusted_directory_does_not_touch_the_file() {
        let body = r#"{"trustedWorkspaces":["/projects/beehive"],"style":"preserved"}"#;
        let path = trust_file("idem", body);
        let decl = trust_decl(&path);

        seed_workspace_trust(&decl, "/projects/beehive").expect("already trusted is fine");

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            body,
            "an already-trusted path is a no-op, byte for byte"
        );

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    /// D5: every failure warns and leaves the file exactly as it was. The
    /// tempting reading of "security" here is to refuse the spawn; bee does
    /// not, and a refusal would strand every agent that declares a trust
    /// store. Nothing here returns an error the caller must treat as fatal.
    #[test]
    fn every_failure_warns_and_leaves_the_store_untouched() {
        // No file at all.
        let missing = waggledance_core::bee::BeeWorkspaceTrust {
            file: "/definitely/not/here/settings.json".to_string(),
            key: "trustedWorkspaces".to_string(),
        };
        assert!(seed_workspace_trust(&missing, "/projects/beehive").is_err());

        for (tag, body, why) in [
            ("badjson", "{not json", "unparseable"),
            ("nokey", r#"{"somethingElse": []}"#, "the key is absent"),
            (
                "notarray",
                r#"{"trustedWorkspaces": "nope"}"#,
                "the key is not a list",
            ),
        ] {
            let path = trust_file(tag, body);
            let decl = trust_decl(&path);
            let before = std::fs::read_to_string(&path).unwrap();

            let warning = seed_workspace_trust(&decl, "/projects/beehive")
                .expect_err(&format!("{why} must warn"));
            assert!(!warning.0.is_empty(), "a warning must say what happened");
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                before,
                "{why}: the store must be left exactly as it was, never half-written"
            );

            std::fs::remove_dir_all(path.parent().unwrap()).ok();
        }
    }

    /// A herdr whose `agent_start` refuses `agent_pane_busy` a fixed number
    /// of times before succeeding — the startup race, made deterministic.
    /// Every other method answers the minimum `start_agent_in_new_tab` needs.
    struct FlakyPane {
        refusals: std::sync::Mutex<u32>,
        other_error: Option<&'static str>,
        starts: std::sync::Mutex<u32>,
        snapshots: std::sync::Mutex<u32>,
        inputs: std::sync::Mutex<Vec<String>>,
    }

    impl FlakyPane {
        fn refusing(n: u32) -> Self {
            Self {
                refusals: std::sync::Mutex::new(n),
                other_error: None,
                starts: std::sync::Mutex::new(0),
                snapshots: std::sync::Mutex::new(0),
                inputs: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn failing_with(code: &'static str) -> Self {
            Self {
                refusals: std::sync::Mutex::new(u32::MAX),
                other_error: Some(code),
                starts: std::sync::Mutex::new(0),
                snapshots: std::sync::Mutex::new(0),
                inputs: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl Herdr for FlakyPane {
        async fn snapshot(&self) -> Result<Snapshot> {
            *self.snapshots.lock().unwrap() += 1;
            Ok(Snapshot {
                panes: vec![wire::Pane {
                    pane_id: "w1:new".into(),
                    workspace_id: "w1".into(),
                    tab_id: "w1:new-tab".into(),
                    cwd: None,
                    foreground_cwd: None,
                }],
                ..Default::default()
            })
        }
        async fn ping(&self) -> Result<ProtocolInfo> {
            unreachable!()
        }
        async fn read_pane(&self, _: &str, _: ReadSource, _: usize) -> Result<ScreenRead> {
            unreachable!()
        }
        async fn send_input(&self, _: &str, text: &str, _: bool) -> Result<()> {
            self.inputs.lock().unwrap().push(text.to_string());
            Ok(())
        }
        async fn agent_prompt(
            &self,
            _: &str,
            _: &str,
            _: &[AgentStatus],
            _: u64,
        ) -> Result<AgentStatus> {
            unreachable!()
        }
        async fn send_text(&self, _: &str, _: &str) -> Result<()> {
            unreachable!()
        }
        async fn send_keys(&self, _: &str, _: &[String]) -> Result<()> {
            unreachable!()
        }
        async fn tab_create(&self, _: &str, _: Option<&str>) -> Result<TabCreated> {
            Ok(TabCreated {
                tab_id: "w1:new-tab".into(),
            })
        }
        async fn close_pane(&self, _: &str) -> Result<()> {
            unreachable!()
        }
        async fn agent_start(&self, pane_id: &str, _: &[String]) -> Result<AgentStarted> {
            *self.starts.lock().unwrap() += 1;
            let mut left = self.refusals.lock().unwrap();
            if *left > 0 {
                *left -= 1;
                return Err(HerdrError::Remote {
                    code: self.other_error.unwrap_or("agent_pane_busy").into(),
                    message: format!("agent target pane {pane_id} is not an available shell"),
                });
            }
            Ok(AgentStarted {
                tab_id: "w1:new-tab".into(),
                pane_id: pane_id.to_string(),
                name: "started".into(),
            })
        }
    }

    /// The race this exists for: herdr refuses the brand-new pane twice while
    /// its shell comes up, and the third offer succeeds.
    #[tokio::test]
    async fn a_pane_that_is_not_ready_yet_is_offered_again_until_it_is() {
        let h = FlakyPane::refusing(2);
        let started = start_agent_in_new_tab(&h, "w1", None, &["claude".to_string()])
            .await
            .expect("a pane that becomes ready must be started, not refused");

        assert_eq!(started.pane_id, "w1:new");
        assert_eq!(*h.starts.lock().unwrap(), 3, "two refusals, then success");
    }

    /// The bound is real, and giving up returns herdr's own last words rather
    /// than a summary — and never another pane.
    #[tokio::test]
    async fn a_pane_that_never_becomes_ready_gives_up_with_herdrs_own_error() {
        let h = FlakyPane::refusing(u32::MAX);
        let err = start_agent_in_new_tab(&h, "w1", None, &["claude".to_string()])
            .await
            .expect_err("a pane that never comes up must fail");

        match err {
            HerdrError::Remote { code, message } => {
                assert_eq!(code, "agent_pane_busy");
                assert!(message.contains("w1:new"), "{message}");
            }
            other => panic!("expected herdr's own refusal, got {other:?}"),
        }
        assert_eq!(
            *h.starts.lock().unwrap(),
            PANE_READY_ATTEMPTS,
            "the bound is finite and is the one the constant names"
        );
    }

    /// Only the not-ready refusal is retried. Anything else is a real failure
    /// and must surface on the first attempt — a loop that swallowed those
    /// would be worse than the race it fixes.
    #[tokio::test]
    async fn any_other_refusal_is_not_retried() {
        let h = FlakyPane::failing_with("agent_name_taken");
        let err = start_agent_in_new_tab(&h, "w1", None, &["claude".to_string()])
            .await
            .expect_err("an unrelated refusal must not be retried");

        assert!(matches!(err, HerdrError::Remote { ref code, .. } if code == "agent_name_taken"));
        assert_eq!(*h.starts.lock().unwrap(), 1, "tried exactly once");
    }

    /// The common case pays nothing: a pane ready on the first offer costs one
    /// start and the single snapshot the tab-to-pane hop already needs.
    #[tokio::test]
    async fn a_ready_pane_costs_no_extra_attempt() {
        let h = FlakyPane::refusing(0);
        start_agent_in_new_tab(&h, "w1", None, &["claude".to_string()])
            .await
            .unwrap();

        assert_eq!(*h.starts.lock().unwrap(), 1);
        assert_eq!(*h.snapshots.lock().unwrap(), 1);
    }

    /// A synthetic "already used" set, standing in for a real snapshot's
    /// agents[] just for this pure retry logic -- proves the loop itself
    /// (regenerate on collision, stop on success or on the bound) without
    /// depending on FakeHerdr's randomness or state.
    async fn try_against(taken: &[&str], name: String) -> Result<AgentStarted> {
        if taken.contains(&name.as_str()) {
            Err(HerdrError::AgentNameTaken {
                name: name.clone(),
                message: format!("{name} is already used"),
            })
        } else {
            Ok(AgentStarted {
                tab_id: "w1:t1".into(),
                pane_id: "w1:p9".into(),
                name,
            })
        }
    }

    #[tokio::test]
    async fn agentstart_retries_once_then_succeeds() {
        // The first generated name collides, the second does not -- the
        // caller must end up with the second name and never see the
        // collision itself.
        let names = ["taken-1".to_string(), "free-1".to_string()];
        let mut next = names.into_iter();
        let taken = ["taken-1"];
        let result = retry_on_name_collision(
            || next.next().expect("only 2 attempts expected"),
            |name| try_against(&taken, name),
        )
        .await
        .unwrap();
        assert_eq!(result.name, "free-1");
    }

    #[tokio::test]
    async fn agentstart_gives_up_after_five_collisions() {
        // All 5 generated names collide -- the bound must stop the loop
        // with a terminal, distinguishable error rather than looping
        // forever or silently reporting success.
        let names = [
            "taken-1".to_string(),
            "taken-2".to_string(),
            "taken-3".to_string(),
            "taken-4".to_string(),
            "taken-5".to_string(),
        ];
        let mut next = names.clone().into_iter();
        let taken = names.iter().map(String::as_str).collect::<Vec<_>>();
        let err = retry_on_name_collision(
            || next.next().expect("exactly 5 attempts expected"),
            |name| try_against(&taken, name),
        )
        .await
        .unwrap_err();
        match err {
            HerdrError::AgentNameTaken { name, message } => {
                assert_eq!(name, "taken-5", "carries the last attempted name");
                assert!(
                    message.contains("gave up after 5"),
                    "message must distinguish a bound-exhausted failure from a single collision: {message}"
                );
            }
            other => panic!("expected AgentNameTaken, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn agentstart_succeeds_immediately_with_no_collision() {
        let mut next = std::iter::once("free-0".to_string());
        let err_free: [&str; 0] = [];
        let result =
            retry_on_name_collision(|| next.next().unwrap(), |name| try_against(&err_free, name))
                .await
                .unwrap();
        assert_eq!(result.name, "free-0");
    }
}
