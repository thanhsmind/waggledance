use anyhow::Result;

use waggledance_core::indexer::now_rfc3339;
use waggledance_core::Engine;

const LEADER_ENV: &str = "PASEO_AGENT_ID";

pub fn run(why: Option<String>, minutes: Option<u64>, release: bool) -> Result<()> {
    let Some(leader) = std::env::var(LEADER_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        eprintln!("error: {LEADER_ENV} is not set; run `waggledance hold` from inside the leader's own Paseo agent");
        std::process::exit(2);
    };
    let engine = crate::runtime::build_engine()?;
    println!(
        "{}",
        apply(
            &engine,
            &leader,
            why.as_deref(),
            minutes,
            release,
            &now_rfc3339()
        )?
    );
    Ok(())
}

fn apply(
    engine: &Engine,
    leader: &str,
    why: Option<&str>,
    minutes: Option<u64>,
    release: bool,
    now: &str,
) -> Result<serde_json::Value> {
    if release {
        let released = engine.release_hold(leader)?;
        return Ok(serde_json::json!({ "released": released }));
    }
    let hold = engine.set_hold(leader, why.unwrap_or_default(), minutes, now)?;
    Ok(serde_json::json!({ "hold": hold }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use waggledance_core::{Config, SqliteStore};

    const NOW: &str = "2026-11-11T10:00:00Z";

    fn engine() -> Engine {
        Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default())
    }

    #[test]
    fn hold_sets_replaces_and_releases_one_row_per_leader() {
        let e = engine();
        apply(&e, "l1", Some("first"), Some(10), false, NOW).unwrap();
        apply(&e, "l1", Some("second"), Some(20), false, NOW).unwrap();
        let held = e.live_hold("l1", NOW).unwrap().unwrap();
        assert_eq!(held.why, "second");
        assert_eq!(held.until, "2026-11-11T10:20:00Z");
        let out = apply(&e, "l1", None, None, true, NOW).unwrap();
        assert_eq!(out["released"], true);
        assert!(e.live_hold("l1", NOW).unwrap().is_none());
    }

    #[test]
    fn minutes_default_to_and_are_capped_at_hold_max_minutes() {
        let e = engine();
        let default = apply(&e, "l1", Some("w"), None, false, NOW).unwrap();
        assert_eq!(default["hold"]["until"], "2026-11-11T12:00:00Z");
        let capped = apply(&e, "l1", Some("w"), Some(100_000), false, NOW).unwrap();
        assert_eq!(capped["hold"]["until"], "2026-11-11T12:00:00Z");
    }

    #[test]
    fn a_hold_is_not_live_after_until() {
        let e = engine();
        apply(&e, "l1", Some("w"), Some(5), false, NOW).unwrap();
        assert!(e.live_hold("l1", "2026-11-11T10:04:59Z").unwrap().is_some());
        assert!(e.live_hold("l1", "2026-11-11T10:05:00Z").unwrap().is_none());
        assert!(e.live_hold("l1", "2026-11-11T11:00:00Z").unwrap().is_none());
    }
}
