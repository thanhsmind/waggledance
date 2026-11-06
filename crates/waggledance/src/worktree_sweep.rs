use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval_at, Instant, MissedTickBehavior};
use waggledance_core::worktree_sweep::{sweep, Outcome, ProjectSweep};
use waggledance_core::Engine;

const FIRST_SWEEP_AFTER: Duration = Duration::from_secs(300);
const PER_PROJECT_DEADLINE: Duration = Duration::from_secs(120);

pub fn every(engine: &Engine) -> Duration {
    Duration::from_secs(engine.config.worktree_sweep.interval_hours.max(1) * 3600)
}

pub async fn run(engine: Arc<Engine>) {
    let period = every(&engine);
    let mut ticks = interval_at(Instant::now() + FIRST_SWEEP_AFTER, period);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        let e = engine.clone();
        let days = engine.config.worktree_sweep.older_than_days;
        let report = tokio::task::spawn_blocking(move || {
            e.list_projects().map(|projects| sweep(&projects, days, PER_PROJECT_DEADLINE))
        })
        .await;
        match report {
            Ok(Ok(report)) => log(&report),
            Ok(Err(e)) => tracing::warn!("worktree sweep could not list projects: {e}"),
            Err(e) => tracing::warn!("worktree sweep task failed: {e}"),
        }
    }
}

fn log(report: &[ProjectSweep]) {
    let mut removed = 0;
    let mut kept = 0;
    let mut failed = 0;
    for project in report {
        match &project.outcome {
            Outcome::Swept { removed: r, kept: k } => {
                for v in r {
                    tracing::info!("worktree sweep removed {} in {}: {}", v.worktree, project.project_id, v.reason);
                }
                removed += r.len();
                kept += k.len();
            }
            Outcome::Failed(why) => {
                failed += 1;
                tracing::warn!("worktree sweep failed in {}: {why}", project.project_id);
            }
        }
    }
    tracing::info!(
        "worktree sweep: {} projects, {removed} removed, {kept} kept, {failed} failed",
        report.len()
    );
}
