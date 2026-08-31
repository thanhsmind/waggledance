//! Board run reaper — the in-daemon sweep that finishes the runs nobody is
//! awaiting (feature `board-run-reaper`, D1 `eecfefeb` / D2 `4047ca75` /
//! D3 `c8847fb7`).
//!
//! `orchestrate::finish` is the only place a run is capped and its pane
//! closed, and `await_run` is its only road — called from exactly one place,
//! the `waggledance_await` MCP tool. A run dispatched from the board, or an
//! MCP dispatch whose caller walked away, therefore stayed `working`
//! forever with its pane open long after the agent printed its own
//! `HERDR_DONE_<nonce>`. This loop is the missing caller: every sweep it
//! asks the ledger for the waggledance-spawned `working` runs
//! (`SqliteStore::list_unattended_working_runs`) and, for each one old
//! enough to be past its owner's own await window, does exactly one of three
//! things:
//!
//! 1. **The pane is gone from the snapshot** → the row is capped `lost`
//!    through `Engine::update_run_status` alone (D2). Row-only, and
//!    deliberately so: a vanished pane has no process to protect and no
//!    screen to store, so nothing is read, nothing is closed, and this
//!    status never travels through `finish`.
//! 2. **The pane is there and shows a fresh marker** → `orchestrate::await_run`
//!    is called with a short budget, so `finish` stores the transcript,
//!    caps the run `done`, and closes the pane under its own three
//!    unchanged guards (D1). The reaper owns no completion logic of its
//!    own — it is a caller, never a second implementation.
//! 3. **Anything else** — quiet, blocked, unreadable, unparseable timestamp,
//!    marker not fresh — is left exactly as it was. The reaper never writes
//!    `blocked` or `timeout`: a blocked pane is the notify watcher's
//!    business, and a run still working is not the reaper's to judge.
//!
//! No notification is ever raised from here (D3): every `await_run` call
//! passes `notify_store: None`, so even the statuses `notify::is_run_notifiable`
//! would accept have no store to enqueue into. The board reflecting `done`
//! or `lost` is the whole surface.
//!
//! Wired in behind the terminal family switch AND `terminal.reaper_enabled`
//! by `crate::TerminalBackground` (`crates/waggledance/src/main.rs`), the
//! same slot/cancel-flag/tick-counter pattern the supervisor and notify
//! tasks already follow: `reconcile_reaper` is the only place a [`Reaper`]
//! is ever constructed, and either switch off drives it to spawn nothing.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use waggledance_core::domain::Run;
use waggledance_core::indexer::now_rfc3339;
use waggledance_core::Engine;

use crate::herdr::{AgentStatus, Herdr, ReadSource, Snapshot};
use crate::orchestrate::{self, RunStatus, RECENT_LINES_CAP};

/// How long a `working` row must have sat untouched before the reaper will
/// consider it unattended at all.
///
/// The reaper cannot see who is awaiting a run — an MCP caller's
/// `waggledance_await` leaves no mark on the row — so age is the proxy: an
/// interactive dispatch-then-await is clamped to
/// [`orchestrate::MAX_AWAIT_TIMEOUT`] (60s), and this window is that same
/// budget. A run still inside it is presumed to have an owner. The race
/// that slips through is benign in both directions: a run reaped a moment
/// before its own awaiter returns is answered from the ledger by
/// `await_run`'s finished-run short circuit, and a run missed this sweep is
/// picked up by the next one.
pub const GRACE_WINDOW: Duration = Duration::from_secs(60);

/// Production sweep cadence. Nothing here is latency-sensitive — every run
/// this loop touches has already been sitting for at least
/// [`GRACE_WINDOW`] — and each tick costs one herdr snapshot, so the
/// interval is set by politeness to herdr rather than by responsiveness.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// The budget the reaper hands `await_run`. Deliberately small: the reaper
/// only calls it once the marker is ALREADY visible in a read it just took,
/// so the very first poll inside `await_run` hits its declared-completion
/// branch and returns. The budget covers only the narrow race where the
/// marker scrolled out of the 1000-line window between those two reads —
/// and bounds how long that race can hold a sweep up.
const AWAIT_BUDGET: Duration = Duration::from_secs(5);

/// What one sweep decided about one run — returned so a caller (and this
/// module's tests) can see the decision itself rather than infer it from
/// the ledger afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Inside [`GRACE_WINDOW`]: presumed to still have an owner. Nothing
    /// was read and nothing was written — herdr is not even asked about it.
    TooYoung,
    /// D2: the pane is absent from the snapshot, so the row was capped
    /// `lost` from the ledger alone.
    Lost,
    /// The marker was fresh, so `await_run` ran and returned this status —
    /// `Done` in every ordinary case, with the transcript stored and the
    /// pane closed by `finish`'s own guards.
    Awaited(RunStatus),
    /// Everything else: a quiet pane, a blocked pane, an unreadable pane, a
    /// timestamp that would not parse, a store or herdr error. The row is
    /// exactly as it was.
    LeftAlone,
}

/// Whether `updated_at` is at least `grace` old as of `now`.
///
/// Fail-closed in both odd directions: a timestamp that will not parse, and
/// one stamped in the future (clock skew), both answer `false` — the reaper
/// would rather leave a row alone forever than cap one whose age it cannot
/// prove.
fn older_than(updated_at: &str, now: OffsetDateTime, grace: Duration) -> bool {
    let Ok(stamped) = OffsetDateTime::parse(updated_at, &Rfc3339) else {
        return false;
    };
    let grace = time::Duration::try_from(grace).unwrap_or(time::Duration::ZERO);
    now - stamped >= grace
}

/// The sweep loop itself. Construction is `TerminalBackground`'s alone.
pub struct Reaper {
    herdr: Arc<dyn Herdr>,
    engine: Arc<Engine>,
    interval: Duration,
    grace: Duration,
    /// Flipped by the owner (`TerminalBackground`) the moment either switch
    /// is turned off. Checked immediately before the one call in a sweep
    /// with an irreversible external side effect (`await_run`, which can
    /// close a pane) for the same reason the supervisor checks its own flag
    /// before spawning: cancelling the task only lands at its next `.await`
    /// point, which can be after the decision to act but before the act.
    cancelled: Arc<AtomicBool>,
}

impl Reaper {
    /// Production always shares a caller-owned cancellation flag via
    /// [`with_cancel_flag`](Self::with_cancel_flag); only this module's own
    /// tests, which have no such flag to share, use this constructor.
    #[cfg(test)]
    fn new(herdr: Arc<dyn Herdr>, engine: Arc<Engine>) -> Self {
        Self::with_cancel_flag(
            herdr,
            engine,
            SWEEP_INTERVAL,
            GRACE_WINDOW,
            Arc::new(AtomicBool::new(false)),
        )
    }

    pub fn with_cancel_flag(
        herdr: Arc<dyn Herdr>,
        engine: Arc<Engine>,
        interval: Duration,
        grace: Duration,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Reaper {
            herdr,
            engine,
            interval,
            grace,
            cancelled,
        }
    }

    /// One sweep: every unattended `working` run, judged once. Returns the
    /// per-run verdicts in the order they were decided.
    ///
    /// The age filter runs BEFORE the snapshot, so a sweep whose every row
    /// is still young costs herdr nothing at all, and the whole sweep takes
    /// exactly one snapshot — never one per run.
    pub async fn sweep_once(&self) -> Vec<(String, Verdict)> {
        let runs = match self.engine.store.list_unattended_working_runs() {
            Ok(runs) => runs,
            Err(e) => {
                tracing::warn!("reaper could not list unattended runs: {e}");
                return Vec::new();
            }
        };
        let now = OffsetDateTime::now_utc();
        let mut verdicts: Vec<(String, Verdict)> = Vec::new();
        let mut due: Vec<Run> = Vec::new();
        for run in runs {
            if older_than(&run.updated_at, now, self.grace) {
                due.push(run);
            } else {
                verdicts.push((run.id, Verdict::TooYoung));
            }
        }
        if due.is_empty() {
            return verdicts;
        }
        let snapshot = match self.herdr.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(e) => {
                // Unverifiable, so nothing is concluded — the same
                // fail-closed posture `orchestrate::preflight` takes about a
                // snapshot it could not read. In particular a snapshot
                // failure must never read as "every pane is gone".
                tracing::warn!("reaper skipped a sweep -- herdr snapshot failed: {e}");
                verdicts.extend(due.into_iter().map(|r| (r.id, Verdict::LeftAlone)));
                return verdicts;
            }
        };
        for run in due {
            let verdict = self.reap_one(&run, &snapshot).await;
            verdicts.push((run.id.clone(), verdict));
        }
        verdicts
    }

    /// One run's verdict against one already-taken snapshot.
    async fn reap_one(&self, run: &Run, snapshot: &Snapshot) -> Verdict {
        // "Gone" means gone from BOTH lists: `panes` is the superset that
        // still holds a pane whose agent registration lapsed, so requiring
        // absence from both is what keeps a live-but-unregistered pane from
        // being written off as vanished.
        let pane_present = snapshot.panes.iter().any(|p| p.pane_id == run.pane_id)
            || snapshot.agents.iter().any(|a| a.pane_id == run.pane_id);
        if !pane_present {
            // D2: row-only. No `read_pane` (there is no pane to read), no
            // transcript, and never through `finish` — which would try to
            // close a pane that no longer exists.
            if let Err(e) = self.engine.update_run_status(
                &run.id,
                RunStatus::Lost.as_str(),
                &now_rfc3339(),
                None,
                None,
            ) {
                tracing::warn!("reaper could not cap run {} as lost: {e}", run.id);
                return Verdict::LeftAlone;
            }
            tracing::info!(
                run = %run.id,
                pane = %run.pane_id,
                "reaper capped a run whose pane is gone as lost"
            );
            return Verdict::Lost;
        }

        let status = snapshot
            .agents
            .iter()
            .find(|a| a.pane_id == run.pane_id)
            .map(|a| a.status);
        if status == Some(AgentStatus::Blocked) {
            // A blocked pane is waiting on a human and already belongs to
            // the notify watcher. Skipped before the read, not after: the
            // reaper has no business concluding anything about a pane whose
            // owner is mid-conversation with it, marker or no marker.
            return Verdict::LeftAlone;
        }

        let read = match self
            .herdr
            .read_pane(&run.pane_id, ReadSource::Recent, RECENT_LINES_CAP)
            .await
        {
            Ok(read) => read,
            Err(e) => {
                tracing::warn!("reaper could not read pane {}: {e}", run.pane_id);
                return Verdict::LeftAlone;
            }
        };
        if !orchestrate::marker_is_fresh(&run.baseline, &read.text, &run.marker) {
            // No declared completion on screen: still working as far as
            // anything here can honestly tell.
            return Verdict::LeftAlone;
        }

        if self.cancelled.load(Ordering::SeqCst) {
            // Last possible moment before the one call that can close a
            // pane (`crates/waggledance/src/supervisor.rs` takes the same
            // check for the same reason).
            return Verdict::LeftAlone;
        }
        // D1/D3: `finish`'s own guards do the capping, the transcript, and
        // the close; `None` is the notify store, so nothing is enqueued.
        match orchestrate::await_run(self.herdr.as_ref(), &self.engine, run, AWAIT_BUDGET, None)
            .await
        {
            Ok(outcome) => {
                tracing::info!(
                    run = %run.id,
                    pane = %run.pane_id,
                    status = outcome.status.as_str(),
                    "reaper finished an unattended run"
                );
                Verdict::Awaited(outcome.status)
            }
            Err(e) => {
                tracing::warn!("reaper could not finish run {}: {e}", run.id);
                Verdict::LeftAlone
            }
        }
    }

    /// Run the sweep loop. `ticks` counts every completed sweep — a real,
    /// externally observable side effect of the loop still running, which is
    /// what proves a switch-off actually stopped the task rather than merely
    /// emptying its owner's slot (same contract as the supervisor's own
    /// tick counter).
    pub async fn run(self, ticks: Arc<AtomicU64>) {
        loop {
            let _ = self.sweep_once().await;
            ticks.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(self.interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;
    use waggledance_core::domain::Project;
    use waggledance_core::{Config, SqliteStore};

    fn test_engine() -> Arc<Engine> {
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        engine
            .store
            .upsert_project(&Project {
                id: "proj-1".into(),
                name: "test-proj".into(),
                root_path: std::path::PathBuf::from("/tmp/test"),
                created_at: now_rfc3339(),
                last_seen_at: now_rfc3339(),
                orchestration_enabled: true,
            })
            .unwrap();
        Arc::new(engine)
    }

    /// A waggledance-spawned (`preset_label` present — `finish`'s second
    /// close guard) `working` run, stamped well outside the grace window
    /// unless `updated_at` says otherwise.
    fn seed_run(engine: &Engine, id: &str, pane_id: &str, baseline: &str, marker: &str) -> Run {
        seed_run_at(
            engine,
            id,
            pane_id,
            baseline,
            marker,
            "2026-01-01T00:00:00Z",
        )
    }

    fn seed_run_at(
        engine: &Engine,
        id: &str,
        pane_id: &str,
        baseline: &str,
        marker: &str,
        updated_at: &str,
    ) -> Run {
        let run = Run {
            id: id.into(),
            project_id: "proj-1".into(),
            pane_id: pane_id.into(),
            preset_label: Some("claude".into()),
            task: "do the thing".into(),
            baseline: baseline.into(),
            marker: marker.into(),
            status: "working".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated_at.into(),
        };
        engine.insert_run(&run, None).unwrap();
        run
    }

    fn status_of(engine: &Engine, id: &str) -> String {
        engine.get_run(id).unwrap().unwrap().status
    }

    /// D2, and the truth with the sharpest edge: a run whose pane is gone
    /// from the snapshot is capped `lost` from the ledger alone. No
    /// `read_pane` is attempted (there is nothing to read) and nothing is
    /// closed (there is nothing to close) — `read_pane_log` is what tells
    /// "never read" apart from "read and swallowed the error".
    #[tokio::test]
    async fn gone_pane_is_capped_lost_with_no_pane_call() {
        let herdr = Arc::new(FakeHerdr::new());
        let engine = test_engine();
        seed_run(&engine, "run-gone", "w9:p9", "before", "HERDR_DONE_gone");

        let reaper = Reaper::new(herdr.clone(), engine.clone());
        let verdicts = reaper.sweep_once().await;

        assert_eq!(verdicts, vec![("run-gone".to_string(), Verdict::Lost)]);
        assert_eq!(status_of(&engine, "run-gone"), "lost");
        assert!(
            herdr.read_pane_log().await.is_empty(),
            "a vanished pane must never be read: {:?}",
            herdr.read_pane_log().await
        );
        assert!(
            herdr.closed_panes().await.is_empty(),
            "`lost` is row-only -- nothing is ever closed by inference"
        );
        assert!(
            engine.run_final_transcript("run-gone").unwrap().is_none(),
            "a gone pane leaves no transcript to store"
        );
    }

    /// D1: a pane still there, showing a marker absent from the run's own
    /// baseline, is finished through `await_run` — transcript stored, run
    /// capped `done`, pane closed by `finish`'s three unchanged guards.
    #[tokio::test]
    async fn fresh_marker_pane_ends_done_with_its_transcript_and_pane_closed() {
        let herdr = Arc::new(FakeHerdr::new());
        let engine = test_engine();
        let marker = "HERDR_DONE_1234abcd";
        herdr.seed_scroll_pane(
            "w2:p3",
            "work in progress",
            &format!("work in progress\nall finished\n{marker}"),
            None,
        );
        seed_run(&engine, "run-fresh", "w2:p3", "work in progress", marker);

        let reaper = Reaper::new(herdr.clone(), engine.clone());
        let verdicts = reaper.sweep_once().await;

        assert_eq!(
            verdicts,
            vec![("run-fresh".to_string(), Verdict::Awaited(RunStatus::Done))]
        );
        assert_eq!(status_of(&engine, "run-fresh"), "done");
        let transcript = engine
            .run_final_transcript("run-fresh")
            .unwrap()
            .expect("a finished run stores its transcript");
        assert!(
            transcript.contains("all finished"),
            "the delta versus baseline is stored: {transcript:?}"
        );
        assert_eq!(
            herdr.closed_panes().await,
            vec!["w2:p3".to_string()],
            "a declared completion closes exactly the run's own pane"
        );
    }

    /// A pane that is alive and quiet — no marker anywhere — is not the
    /// reaper's to judge. It stays `working` for its next sweep, and
    /// nothing is closed.
    #[tokio::test]
    async fn quiet_working_pane_is_untouched() {
        let herdr = Arc::new(FakeHerdr::new());
        let engine = test_engine();
        seed_run(
            &engine,
            "run-quiet",
            "w1:p1",
            "still going",
            "HERDR_DONE_qq",
        );

        let reaper = Reaper::new(herdr.clone(), engine.clone());
        let verdicts = reaper.sweep_once().await;

        assert_eq!(
            verdicts,
            vec![("run-quiet".to_string(), Verdict::LeftAlone)]
        );
        assert_eq!(status_of(&engine, "run-quiet"), "working");
        assert!(herdr.closed_panes().await.is_empty());
        assert!(
            engine.run_final_transcript("run-quiet").unwrap().is_none(),
            "an open run has no final transcript"
        );
    }

    /// A blocked pane belongs to the notify watcher, and the reaper never
    /// writes `blocked`. Seeded deliberately WITH a fresh marker on screen:
    /// the skip must come from the status, before the read, not from
    /// failing to find anything afterwards.
    #[tokio::test]
    async fn blocked_pane_is_untouched_and_never_read() {
        let herdr = Arc::new(FakeHerdr::new());
        let engine = test_engine();
        let marker = "HERDR_DONE_blocked99";
        herdr.seed_scroll_pane("w1:p2", "waiting", &format!("waiting\n{marker}"), None);
        seed_run(&engine, "run-blocked", "w1:p2", "waiting", marker);

        let reaper = Reaper::new(herdr.clone(), engine.clone());
        let verdicts = reaper.sweep_once().await;

        assert_eq!(
            verdicts,
            vec![("run-blocked".to_string(), Verdict::LeftAlone)]
        );
        assert_eq!(
            status_of(&engine, "run-blocked"),
            "working",
            "the reaper never writes `blocked`"
        );
        assert!(
            herdr.read_pane_log().await.is_empty(),
            "a blocked pane is skipped before any read"
        );
        assert!(herdr.closed_panes().await.is_empty());
    }

    /// The grace window: a row touched moments ago is presumed to have an
    /// owner mid-await, so it is skipped even though its pane is gone and
    /// it would otherwise be capped `lost` on the spot.
    #[tokio::test]
    async fn young_row_inside_the_grace_window_is_untouched() {
        let herdr = Arc::new(FakeHerdr::new());
        let engine = test_engine();
        seed_run_at(
            &engine,
            "run-young",
            "w9:p9",
            "before",
            "HERDR_DONE_young",
            &now_rfc3339(),
        );

        let reaper = Reaper::new(herdr.clone(), engine.clone());
        let verdicts = reaper.sweep_once().await;

        assert_eq!(verdicts, vec![("run-young".to_string(), Verdict::TooYoung)]);
        assert_eq!(
            status_of(&engine, "run-young"),
            "working",
            "a run still inside its own await window is left to its owner"
        );
        assert!(
            herdr.read_pane_log().await.is_empty(),
            "an all-young sweep costs herdr nothing"
        );
    }

    /// A snapshot that could not be read is unverifiable, never "every pane
    /// is gone" — the whole sweep is skipped rather than capping live runs
    /// `lost` because herdr was momentarily down.
    #[tokio::test]
    async fn a_failed_snapshot_caps_nothing() {
        let herdr = Arc::new(FakeHerdr::new());
        herdr.set_available(false);
        let engine = test_engine();
        seed_run(&engine, "run-dark", "w1:p1", "before", "HERDR_DONE_dark");

        let verdicts = Reaper::new(herdr.clone(), engine.clone())
            .sweep_once()
            .await;

        assert_eq!(verdicts, vec![("run-dark".to_string(), Verdict::LeftAlone)]);
        assert_eq!(status_of(&engine, "run-dark"), "working");
    }

    #[test]
    fn grace_window_rejects_unparseable_and_future_stamps() {
        let now = OffsetDateTime::now_utc();
        assert!(older_than("2026-01-01T00:00:00Z", now, GRACE_WINDOW));
        assert!(!older_than(&now_rfc3339(), now, GRACE_WINDOW));
        assert!(
            !older_than("whenever", now, GRACE_WINDOW),
            "an age that cannot be proved is never old enough"
        );
        assert!(
            !older_than("2099-01-01T00:00:00Z", now, GRACE_WINDOW),
            "a future stamp (clock skew) is never old enough either"
        );
    }
}
