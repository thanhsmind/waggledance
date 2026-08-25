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
    let created = herdr.tab_create(workspace_id, cwd).await?;
    let pane_id = pane_of_tab(herdr, &created.tab_id).await?.ok_or_else(|| {
        HerdrError::TabPaneUnresolved {
            tab_id: created.tab_id.clone(),
            workspace_id: workspace_id.to_string(),
        }
    })?;
    herdr.agent_start(&pane_id, argv).await
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
