//! Notification service — outbound alerts when an agent needs a human
//! (`blocked`) or finishes (`done`) (D1: ported from herdr-go's `notify`
//! module). [`Notifier`] is a hexagonal port: Telegram is one implementation,
//! never the only path, so a future channel drops in unchanged. [`RunOwnership`]
//! is a second port (D3) that lets this service suppress watcher alerts while
//! a dispatched run owns a pane, without taking a dependency on the engine.
//!
//! Delivery is **at-least-once**: the obligation is enqueued in the
//! [`NotifyStore`](waggledance_core::notify_store::NotifyStore) first and marked
//! delivered only after a successful send, so a crash between the two
//! resends rather than loses it.
//!
//! Wired in behind the D7 opt-in switch by `crate::TerminalBackground`
//! (`crates/waggledance/src/main.rs`): `reconcile` is the only place a
//! [`NotifyService`] is ever driven, and a switch left off drains nothing
//! and sends nothing.

pub mod telegram;

use std::sync::Arc;

use async_trait::async_trait;
use waggledance_core::bee::BeeActivityState;
use waggledance_core::notify_store::NotifyStore;

use crate::herdr::AgentStatus;
use crate::orchestrate::RunStatus;
use crate::watcher::StatusChange;

pub use telegram::TelegramNotifier;

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("notify send failed: {0}")]
    Send(String),
}

pub type Result<T> = std::result::Result<T, NotifyError>;

/// A channel that can deliver one alert. Implementations must not log
/// secrets.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, kind: &str, body: &str) -> Result<()>;
}

/// A notifier that only logs — the default until a real channel is
/// configured.
pub struct NullNotifier;

#[async_trait]
impl Notifier for NullNotifier {
    async fn send(&self, kind: &str, body: &str) -> Result<()> {
        tracing::info!(kind, body, "notify (null channel)");
        Ok(())
    }
}

/// A port for asking whether a pane currently has an owning run (D3).
/// Expressed as a hexagonal port so [`NotifyService`] does not take a hard
/// dependency on the engine.
pub trait RunOwnership: Send + Sync {
    fn is_pane_owned(&self, pane_id: &str) -> bool;
}

impl<F> RunOwnership for F
where
    F: Fn(&str) -> bool + Send + Sync,
{
    fn is_pane_owned(&self, pane_id: &str) -> bool {
        self(pane_id)
    }
}

/// Which status transitions are worth a human's attention.
pub fn is_notifiable(status: AgentStatus) -> bool {
    matches!(status, AgentStatus::Blocked | AgentStatus::Done)
}

/// Which run statuses are worth a human's attention (D1): `Blocked` waits on
/// a person, `Timeout` never got a trustworthy signal at all -- `Done` and
/// `Working` never notify. A separate answer from [`is_notifiable`] because
/// that one speaks [`AgentStatus`], this one [`RunStatus`] -- distinct
/// vocabularies, never overloaded onto one function.
pub fn is_run_notifiable(status: RunStatus) -> bool {
    matches!(status, RunStatus::Blocked | RunStatus::Timeout)
}

/// Bridges the watcher to a channel with durable, at-least-once delivery.
/// The store is plain, synchronous rusqlite (`waggledance-core` stays
/// async-runtime-free) held behind its own internal mutex, so calls here
/// block only as long as a single SQLite statement takes — the same shape
/// every other adapter this crate already uses.
pub struct NotifyService {
    store: Arc<NotifyStore>,
    notifier: Arc<dyn Notifier>,
    ownership: Option<Arc<dyn RunOwnership>>,
}

impl NotifyService {
    pub fn new(store: Arc<NotifyStore>, notifier: Arc<dyn Notifier>) -> Self {
        NotifyService {
            store,
            notifier,
            ownership: None,
        }
    }

    pub fn with_ownership(
        store: Arc<NotifyStore>,
        notifier: Arc<dyn Notifier>,
        ownership: Arc<dyn RunOwnership>,
    ) -> Self {
        NotifyService {
            store,
            notifier,
            ownership: Some(ownership),
        }
    }

    /// Record a status change as a pending obligation *if* it is notifiable
    /// and the pane is not currently owned by a dispatched run (D3).
    /// Returns true if it was enqueued.
    pub async fn record(&self, change: &StatusChange) -> bool {
        if !is_notifiable(change.status) {
            return false;
        }
        if let Some(ownership) = &self.ownership {
            if ownership.is_pane_owned(&change.pane_id) {
                return false;
            }
        }
        let body = format!(
            "{} agent {} is {}",
            change.kind,
            change.pane_id,
            change.status.as_str()
        );
        // Enqueue first (act → persist), before any send attempt.
        self.store
            .enqueue_notification(&change.pane_id, change.status.as_str(), &body)
            .is_ok()
    }

    /// The machine token a bee activity state is filed under in the outbox
    /// -- the record's own `state` string, not the human word. Local to the
    /// notifier because the outbox's `kind` column is its vocabulary, the
    /// same way `record` files a herdr change under `status.as_str()`.
    fn activity_kind(state: &BeeActivityState) -> &str {
        match state {
            BeeActivityState::Working => "working",
            BeeActivityState::WaitingInput => "waiting_input",
            BeeActivityState::Blocked => "blocked",
            BeeActivityState::Idle => "idle",
            BeeActivityState::Exited => "exited",
            BeeActivityState::Unknown(raw) => raw,
        }
    }

    /// Record a bee agent-activity transition as a pending obligation (A5).
    /// The caller (`watcher::ActivityCursor`) has already decided the
    /// transition is worth saying -- entry into the need-you family, or
    /// `exited` -- so there is no second notifiability test here; what this
    /// adds is the same run-ownership suppression `record` applies, on the
    /// session's pane when one is known. A session with no pane cannot be
    /// owned by a dispatched run, so it is never suppressed. Returns true
    /// if it was enqueued.
    pub async fn record_activity(
        &self,
        session_id: &str,
        pane: Option<&str>,
        to: &BeeActivityState,
    ) -> bool {
        if let (Some(ownership), Some(pane)) = (&self.ownership, pane) {
            if ownership.is_pane_owned(pane) {
                return false;
            }
        }
        // The pane is what a human recognizes; the session id is the
        // fallback for an agent running outside herdr.
        let subject = pane.unwrap_or(session_id);
        let body = format!("agent {} {}", subject, to.word());
        self.store
            .enqueue_notification(subject, Self::activity_kind(to), &body)
            .is_ok()
    }

    /// Drain the outbox: send each pending notification, marking it
    /// delivered only on success. A send failure leaves it pending for the
    /// next drain (at-least-once). Returns how many were delivered this
    /// pass.
    pub async fn drain(&self) -> usize {
        let pending = match self.store.undelivered() {
            Ok(p) => p,
            Err(_) => return 0,
        };
        let mut delivered = 0;
        for n in pending {
            match self.notifier.send(&n.kind, &n.body).await {
                Ok(()) => {
                    // Send succeeded → now mark delivered. Order matters: a
                    // crash before this line resends; it never silently
                    // drops.
                    if self.store.mark_delivered(n.id).is_ok() {
                        delivered += 1;
                    }
                }
                Err(_) => { /* leave pending for the next drain */ }
            }
        }
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingNotifier {
        sent: Mutex<Vec<String>>,
        fail_next: AtomicUsize,
    }
    #[async_trait]
    impl Notifier for RecordingNotifier {
        async fn send(&self, _kind: &str, body: &str) -> Result<()> {
            if self.fail_next.load(Ordering::SeqCst) > 0 {
                self.fail_next.fetch_sub(1, Ordering::SeqCst);
                return Err(NotifyError::Send("simulated".into()));
            }
            self.sent.lock().unwrap().push(body.to_string());
            Ok(())
        }
    }

    fn store() -> Arc<NotifyStore> {
        Arc::new(NotifyStore::open_in_memory().unwrap())
    }

    fn change(pane: &str, status: AgentStatus) -> StatusChange {
        StatusChange {
            pane_id: pane.into(),
            kind: "claude".into(),
            status,
        }
    }

    #[tokio::test]
    async fn only_blocked_and_done_are_recorded() {
        let store = store();
        let svc = NotifyService::new(store.clone(), Arc::new(NullNotifier));
        assert!(svc.record(&change("p", AgentStatus::Blocked)).await);
        assert!(svc.record(&change("p", AgentStatus::Done)).await);
        assert!(!svc.record(&change("p", AgentStatus::Working)).await);
        assert!(!svc.record(&change("p", AgentStatus::Idle)).await);
        assert_eq!(store.undelivered().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn blocked_pane_with_owning_run_enqueues_nothing() {
        let store = store();
        let ownership = Arc::new(|pane: &str| pane == "p1");
        let svc = NotifyService::with_ownership(store.clone(), Arc::new(NullNotifier), ownership);
        assert!(!svc.record(&change("p1", AgentStatus::Blocked)).await);
        assert!(store.undelivered().unwrap().is_empty());
    }

    #[tokio::test]
    async fn blocked_pane_without_owning_run_enqueues() {
        let store = store();
        let ownership = Arc::new(|pane: &str| pane == "other");
        let svc = NotifyService::with_ownership(store.clone(), Arc::new(NullNotifier), ownership);
        assert!(svc.record(&change("p1", AgentStatus::Blocked)).await);
        assert_eq!(store.undelivered().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn done_pane_with_owning_run_stays_suppressed() {
        let store = store();
        let ownership = Arc::new(|pane: &str| pane == "p1");
        let svc = NotifyService::with_ownership(store.clone(), Arc::new(NullNotifier), ownership);
        assert!(!svc.record(&change("p1", AgentStatus::Done)).await);
        assert!(store.undelivered().unwrap().is_empty());
    }

    #[tokio::test]
    async fn service_without_ownership_port_behaves_as_before() {
        let store = store();
        let svc = NotifyService::new(store.clone(), Arc::new(NullNotifier));
        assert!(svc.record(&change("p1", AgentStatus::Blocked)).await);
        assert!(svc.record(&change("p1", AgentStatus::Done)).await);
        assert_eq!(store.undelivered().unwrap().len(), 2);
    }

    /// Run-ownership suppression reaches activity notifications exactly as
    /// it reaches herdr ones (A5): a pane a dispatched run already owns
    /// stays silent.
    #[tokio::test]
    async fn activity_entry_on_an_owned_pane_enqueues_nothing() {
        let store = store();
        let ownership = Arc::new(|pane: &str| pane == "w1:p1");
        let svc = NotifyService::with_ownership(store.clone(), Arc::new(NullNotifier), ownership);
        assert!(
            !svc.record_activity("sess-1", Some("w1:p1"), &BeeActivityState::Blocked)
                .await
        );
        assert!(store.undelivered().unwrap().is_empty());
    }

    #[tokio::test]
    async fn activity_entry_on_an_unowned_pane_enqueues_one_row() {
        let store = store();
        let ownership = Arc::new(|pane: &str| pane == "other");
        let svc = NotifyService::with_ownership(store.clone(), Arc::new(NullNotifier), ownership);
        assert!(
            svc.record_activity("sess-1", Some("w1:p1"), &BeeActivityState::Blocked)
                .await
        );
        let pending = store.undelivered().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].body, "agent w1:p1 needs approval");
        assert_eq!(pending[0].kind, "blocked");
    }

    /// No pane -- an agent outside herdr -- names the session and can never
    /// be suppressed by ownership, which only speaks panes.
    #[tokio::test]
    async fn activity_without_a_pane_names_the_session() {
        let store = store();
        let ownership = Arc::new(|_pane: &str| true);
        let svc = NotifyService::with_ownership(store.clone(), Arc::new(NullNotifier), ownership);
        assert!(
            svc.record_activity("sess-1", None, &BeeActivityState::WaitingInput)
                .await
        );
        let pending = store.undelivered().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].body, "agent sess-1 needs an answer");
        assert_eq!(pending[0].kind, "waiting_input");
    }

    #[tokio::test]
    async fn exited_activity_is_enqueued_with_its_word() {
        let store = store();
        let svc = NotifyService::new(store.clone(), Arc::new(NullNotifier));
        assert!(
            svc.record_activity("sess-1", Some("w2:p3"), &BeeActivityState::Exited)
                .await
        );
        let pending = store.undelivered().unwrap();
        assert_eq!(pending[0].body, "agent w2:p3 exited");
        assert_eq!(pending[0].kind, "exited");
    }

    #[tokio::test]
    async fn drain_delivers_and_marks() {
        let store = store();
        let notifier = Arc::new(RecordingNotifier::default());
        let svc = NotifyService::new(store.clone(), notifier.clone());
        svc.record(&change("p1", AgentStatus::Blocked)).await;
        assert_eq!(svc.drain().await, 1);
        assert_eq!(notifier.sent.lock().unwrap().len(), 1);
        assert!(store.undelivered().unwrap().is_empty());
        // Draining again sends nothing (already delivered).
        assert_eq!(svc.drain().await, 0);
    }

    #[tokio::test]
    async fn failed_send_stays_pending_then_redelivers() {
        let store = store();
        let notifier = Arc::new(RecordingNotifier::default());
        notifier.fail_next.store(1, Ordering::SeqCst);
        let svc = NotifyService::new(store.clone(), notifier.clone());
        svc.record(&change("p1", AgentStatus::Done)).await;
        // First drain: the send fails, so nothing is marked delivered.
        assert_eq!(svc.drain().await, 0);
        assert_eq!(store.undelivered().unwrap().len(), 1);
        // Second drain: send succeeds now — at-least-once holds. Restarting
        // the drain never resends a notification that already succeeded and
        // never resends indefinitely once it does.
        assert_eq!(svc.drain().await, 1);
        assert!(store.undelivered().unwrap().is_empty());
        assert_eq!(svc.drain().await, 0);
    }
}
