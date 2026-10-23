use crate::agent_usage::AgentRun;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use time::{Date, Duration, OffsetDateTime, UtcOffset};

const DAY_MS: i64 = 86_400_000;
const SCHEMA: &str = "bee-usage/v1";

static LOCAL_OFFSET_SECS: OnceLock<i32> = OnceLock::new();

pub fn set_local_offset_secs(secs: i32) {
    let _ = LOCAL_OFFSET_SECS.set(secs);
}

pub fn local_offset_secs() -> i32 {
    LOCAL_OFFSET_SECS.get().copied().unwrap_or(0)
}

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tokens {
    pub new: u64,
    pub cached: u64,
}

impl Tokens {
    pub fn total(&self) -> u64 {
        self.new.saturating_add(self.cached)
    }

    fn add(&mut self, other: &Tokens) {
        self.new = self.new.saturating_add(other.new);
        self.cached = self.cached.saturating_add(other.cached);
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SessionUsage {
    pub started_ms: Option<i64>,
    pub ended_ms: Option<i64>,
    pub models: BTreeMap<String, Tokens>,
    pub subagent_models: BTreeMap<String, Tokens>,
}

impl SessionUsage {
    fn active_ms(&self) -> i64 {
        match (self.started_ms, self.ended_ms) {
            (Some(s), Some(e)) if e > s => e - s,
            _ => 0,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct FeatureUsage {
    pub feature: String,
    pub closed_at_ms: Option<i64>,
    pub cells_capped: usize,
    pub sessions: Vec<SessionUsage>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProjectUsageInput {
    pub id: String,
    pub name: String,
    pub features: Vec<FeatureUsage>,
}

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpendSplit {
    pub main: Tokens,
    pub subagents: Tokens,
    pub all: Tokens,
}

impl SpendSplit {
    fn add_tokens(&mut self, subagent: bool, t: &Tokens) {
        if subagent {
            self.subagents.add(t);
        } else {
            self.main.add(t);
        }
        self.all.add(t);
    }

    fn add_session(&mut self, s: &SessionUsage) {
        for t in s.models.values() {
            self.add_tokens(false, t);
        }
        for t in s.subagent_models.values() {
            self.add_tokens(true, t);
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ProjectSpend {
    pub id: String,
    pub name: String,
    pub spend: SpendSplit,
    pub features_done: usize,
    pub active_ms: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ModelSpend {
    pub model: String,
    pub main: Tokens,
    pub subagents: Tokens,
}

#[derive(Serialize, Clone, Debug)]
pub struct DaySpend {
    pub date: String,
    pub spend: Tokens,
}

#[derive(Serialize, Clone, Debug)]
pub struct FeatureRow {
    pub project_id: String,
    pub project_name: String,
    pub feature: String,
    pub closed_at_ms: Option<i64>,
    pub cells_capped: usize,
    pub active_ms: i64,
    pub span_ms: Option<i64>,
    pub spend: SpendSplit,
    pub models: Vec<ModelSpend>,
}

#[derive(Serialize, Clone, Debug)]
pub struct RunRow {
    pub key: String,
    pub project_id: String,
    pub project_name: String,
    pub provider: String,
    pub title: String,
    pub model: String,
    pub parent_key: Option<String>,
    pub subagent: bool,
    pub started_ms: i64,
    pub duration_ms: i64,
    pub spend: Tokens,
}

#[derive(Serialize, Clone, Debug)]
pub struct UsageSummary {
    pub days: u32,
    pub since_ms: i64,
    pub now_ms: i64,
    pub totals: SpendSplit,
    pub projects: Vec<ProjectSpend>,
    pub models: Vec<ModelSpend>,
    pub per_day: Vec<DaySpend>,
    pub features: Vec<FeatureRow>,
    pub runs: Vec<RunRow>,
}

pub fn read_usage(root: &Path) -> (Vec<FeatureUsage>, usize) {
    let Ok(rd) = fs::read_dir(root.join(".bee").join("usage")) else {
        return (Vec::new(), 0);
    };
    let mut paths: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    let mut features = Vec::new();
    let mut skipped = 0;
    for path in paths {
        match fs::read_to_string(&path)
            .ok()
            .and_then(|raw| parse_usage(root, &raw))
        {
            Some(f) => features.push(f),
            None => skipped += 1,
        }
    }
    (features, skipped)
}

fn parse_usage(root: &Path, raw: &str) -> Option<FeatureUsage> {
    let v: Value = serde_json::from_str(raw).ok()?;
    if v.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return None;
    }
    let feature = v.get("feature").and_then(Value::as_str)?.to_string();
    let closed_at_ms = v
        .get("closed_at")
        .and_then(Value::as_str)
        .and_then(|s| OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok())
        .map(|dt| (dt.unix_timestamp_nanos() / 1_000_000) as i64);
    let sessions = v
        .get("sessions")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(parse_session).collect())
        .unwrap_or_default();
    let cells_capped = crate::bee::read_archived_cells(root, &feature)
        .iter()
        .filter(|c| c.status == "capped")
        .count();
    Some(FeatureUsage {
        feature,
        closed_at_ms,
        cells_capped,
        sessions,
    })
}

fn parse_session(v: &Value) -> SessionUsage {
    SessionUsage {
        started_ms: v.get("started_ms").and_then(Value::as_i64),
        ended_ms: v.get("ended_ms").and_then(Value::as_i64),
        models: parse_models(v.get("models")),
        subagent_models: parse_models(v.get("subagent_models")),
    }
}

fn parse_models(v: Option<&Value>) -> BTreeMap<String, Tokens> {
    let Some(map) = v.and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    map.iter()
        .map(|(model, t)| {
            let field = |k: &str| t.get(k).and_then(Value::as_u64).unwrap_or(0);
            (
                model.clone(),
                Tokens {
                    new: field("new"),
                    cached: field("cached"),
                },
            )
        })
        .collect()
}

fn local_date(ms: i64, offset: UtcOffset) -> Option<Date> {
    OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000)
        .ok()
        .map(|dt| dt.to_offset(offset).date())
}

fn add_models(acc: &mut BTreeMap<String, (Tokens, Tokens)>, s: &SessionUsage) {
    for (m, t) in &s.models {
        acc.entry(m.clone()).or_default().0.add(t);
    }
    for (m, t) in &s.subagent_models {
        acc.entry(m.clone()).or_default().1.add(t);
    }
}

fn model_list(acc: BTreeMap<String, (Tokens, Tokens)>) -> Vec<ModelSpend> {
    let mut list: Vec<ModelSpend> = acc
        .into_iter()
        .map(|(model, (main, subagents))| ModelSpend {
            model,
            main,
            subagents,
        })
        .collect();
    list.sort_by(|a, b| {
        let total = |m: &ModelSpend| m.main.total().saturating_add(m.subagents.total());
        total(b).cmp(&total(a)).then_with(|| a.model.cmp(&b.model))
    });
    list
}

pub fn summarize(
    projects: &[ProjectUsageInput],
    now_ms: i64,
    days: u32,
    offset_secs: i32,
) -> UsageSummary {
    summarize_with(projects, None, now_ms, days, offset_secs)
}

pub fn summarize_runs(
    projects: &[ProjectUsageInput],
    runs: &[(Option<String>, AgentRun)],
    now_ms: i64,
    days: u32,
    offset_secs: i32,
) -> UsageSummary {
    summarize_with(projects, Some(runs), now_ms, days, offset_secs)
}

fn summarize_with(
    projects: &[ProjectUsageInput],
    runs: Option<&[(Option<String>, AgentRun)]>,
    now_ms: i64,
    days: u32,
    offset_secs: i32,
) -> UsageSummary {
    let offset = UtcOffset::from_whole_seconds(offset_secs).unwrap_or(UtcOffset::UTC);

    let mut per_day: Vec<DaySpend> = Vec::new();
    let mut day_index: BTreeMap<Date, usize> = BTreeMap::new();
    if let Some(today) = local_date(now_ms, offset) {
        for back in (0..i64::from(days)).rev() {
            if let Some(date) = today.checked_sub(Duration::days(back)) {
                day_index.insert(date, per_day.len());
                per_day.push(DaySpend {
                    date: format!(
                        "{:04}-{:02}-{:02}",
                        date.year(),
                        u8::from(date.month()),
                        date.day()
                    ),
                    spend: Tokens::default(),
                });
            }
        }
    }
    let since_ms = day_index
        .keys()
        .next()
        .map(|d| (d.midnight().assume_offset(offset).unix_timestamp_nanos() / 1_000_000) as i64)
        .unwrap_or(now_ms - i64::from(days) * DAY_MS);
    let in_window = |t: i64| t >= since_ms && t <= now_ms;
    let day_of = |t: i64| {
        local_date(t, offset)
            .and_then(|d| day_index.get(&d))
            .copied()
    };

    let mut totals = SpendSplit::default();
    let mut models: BTreeMap<String, (Tokens, Tokens)> = BTreeMap::new();
    let mut project_rows: Vec<ProjectSpend> = projects
        .iter()
        .map(|p| ProjectSpend {
            id: p.id.clone(),
            name: p.name.clone(),
            spend: SpendSplit::default(),
            features_done: 0,
            active_ms: 0,
        })
        .collect();
    let mut features = Vec::new();

    for (p, row) in projects.iter().zip(project_rows.iter_mut()) {
        for f in &p.features {
            for s in f.sessions.iter().filter(|_| runs.is_none()) {
                let Some(at) = s.ended_ms.or(f.closed_at_ms) else {
                    continue;
                };
                if !in_window(at) {
                    continue;
                }
                let mut one = SpendSplit::default();
                one.add_session(s);
                row.spend.add_session(s);
                totals.add_session(s);
                add_models(&mut models, s);
                row.active_ms += s.active_ms();
                if let Some(i) = day_of(at) {
                    per_day[i].spend.add(&one.all);
                }
            }
            if f.closed_at_ms.is_some_and(in_window) {
                row.features_done += 1;
                let mut row_spend = SpendSplit::default();
                let mut row_models = BTreeMap::new();
                for s in &f.sessions {
                    row_spend.add_session(s);
                    add_models(&mut row_models, s);
                }
                let earliest = f.sessions.iter().filter_map(|s| s.started_ms).min();
                features.push(FeatureRow {
                    project_id: p.id.clone(),
                    project_name: p.name.clone(),
                    feature: f.feature.clone(),
                    closed_at_ms: f.closed_at_ms,
                    cells_capped: f.cells_capped,
                    active_ms: f.sessions.iter().map(SessionUsage::active_ms).sum(),
                    span_ms: f.closed_at_ms.zip(earliest).map(|(c, s)| c - s),
                    spend: row_spend,
                    models: model_list(row_models),
                });
            }
        }
    }

    let mut run_rows = Vec::new();
    for (project_id, run) in runs.unwrap_or_default() {
        let events: Vec<_> = run.events.iter().filter(|e| in_window(e.ts_ms)).collect();
        if events.is_empty() {
            continue;
        }
        let at = project_id
            .as_deref()
            .and_then(|id| project_rows.iter().position(|r| r.id == id))
            .or_else(|| project_rows.iter().position(|r| r.id.is_empty()))
            .unwrap_or_else(|| {
                project_rows.push(ProjectSpend {
                    id: String::new(),
                    name: "Other".into(),
                    spend: SpendSplit::default(),
                    features_done: 0,
                    active_ms: 0,
                });
                project_rows.len() - 1
            });
        let row = &mut project_rows[at];
        let mut spend = Tokens::default();
        let mut by_model: BTreeMap<&str, u64> = BTreeMap::new();
        for e in events {
            let t = Tokens {
                new: e.new,
                cached: e.cached,
            };
            spend.add(&t);
            *by_model.entry(&e.model).or_default() += t.total();
            row.spend.add_tokens(run.subagent, &t);
            totals.add_tokens(run.subagent, &t);
            let m = models.entry(e.model.clone()).or_default();
            if run.subagent { &mut m.1 } else { &mut m.0 }.add(&t);
            if let Some(i) = day_of(e.ts_ms) {
                per_day[i].spend.add(&t);
            }
        }
        let duration_ms = (run.ended_ms - run.started_ms).max(0);
        if !run.subagent {
            row.active_ms += duration_ms;
        }
        run_rows.push(RunRow {
            key: run.key.clone(),
            project_id: row.id.clone(),
            project_name: row.name.clone(),
            provider: run.provider.clone(),
            title: run.title.clone(),
            model: by_model
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(m, _)| m.to_string())
                .unwrap_or_default(),
            parent_key: run.parent_key.clone(),
            subagent: run.subagent,
            started_ms: run.started_ms,
            duration_ms,
            spend,
        });
    }
    run_rows.sort_by(|a, b| {
        b.started_ms
            .cmp(&a.started_ms)
            .then_with(|| a.key.cmp(&b.key))
    });

    project_rows.sort_by(|a, b| {
        b.spend
            .all
            .total()
            .cmp(&a.spend.all.total())
            .then_with(|| a.name.cmp(&b.name))
    });
    features.sort_by(|a, b| {
        b.closed_at_ms
            .cmp(&a.closed_at_ms)
            .then_with(|| a.feature.cmp(&b.feature))
    });

    UsageSummary {
        days,
        since_ms,
        now_ms,
        totals,
        projects: project_rows,
        models: model_list(models),
        per_day,
        features,
        runs: run_rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(s: &str) -> i64 {
        let dt = OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).unwrap();
        (dt.unix_timestamp_nanos() / 1_000_000) as i64
    }

    fn tok(new: u64, cached: u64) -> Tokens {
        Tokens { new, cached }
    }

    fn session(ended: Option<i64>, main: u64, sub: u64) -> SessionUsage {
        let mut s = SessionUsage {
            started_ms: ended.map(|e| e - 60_000),
            ended_ms: ended,
            ..Default::default()
        };
        s.models.insert("opus".into(), tok(main, 0));
        s.subagent_models.insert("sonnet".into(), tok(sub, 0));
        s
    }

    fn feature(name: &str, closed: Option<i64>, sessions: Vec<SessionUsage>) -> FeatureUsage {
        FeatureUsage {
            feature: name.into(),
            closed_at_ms: closed,
            cells_capped: 0,
            sessions,
        }
    }

    fn project(features: Vec<FeatureUsage>) -> Vec<ProjectUsageInput> {
        vec![ProjectUsageInput {
            id: "p1".into(),
            name: "one".into(),
            features,
        }]
    }

    const REAL: &str = r#"{
      "schema": "bee-usage/v1",
      "feature": "paseo-leader-dispatch",
      "closed_at": "2026-10-14T16:26:27.352Z",
      "sessions": [
        {
          "session_id": "dd55",
          "models": {"claude-opus-5-5": {"input": 610, "output": 220583, "cache_write": 504415, "cache_read": 81916346, "new": 725608, "cached": 81916346, "total": 82641954}},
          "subagent_models": {
            "claude-sonnet-5-5": {"new": 349458, "cached": 2814166, "total": 3163624},
            "claude-fable-5-1": {"new": 180317}
          },
          "subagent_count": 27,
          "started_ms": 1791987286848,
          "ended_ms": 1791995186603
        },
        {"session_id": "ee", "models": {"claude-opus-5-5": {"new": 1, "cached": 2}}}
      ],
      "skipped": 0
    }"#;

    #[test]
    fn read_usage_parses_models_and_subagent_models() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join(".bee/usage");
        fs::create_dir_all(&usage).unwrap();
        fs::write(usage.join("paseo-leader-dispatch.json"), REAL).unwrap();
        let archive = dir.path().join(".bee/cells/archive/paseo-leader-dispatch");
        fs::create_dir_all(&archive).unwrap();
        fs::write(archive.join("a.json"), r#"{"id":"a","status":"capped"}"#).unwrap();
        fs::write(archive.join("b.json"), r#"{"id":"b","status":"dropped"}"#).unwrap();

        let (features, skipped) = read_usage(dir.path());
        assert_eq!(skipped, 0);
        assert_eq!(features.len(), 1);
        let f = &features[0];
        assert_eq!(f.feature, "paseo-leader-dispatch");
        assert_eq!(f.closed_at_ms, Some(ms("2026-10-14T16:26:27.352Z")));
        assert_eq!(f.cells_capped, 1);
        assert_eq!(f.sessions.len(), 2);
        let s = &f.sessions[0];
        assert_eq!(s.started_ms, Some(1791987286848));
        assert_eq!(s.ended_ms, Some(1791995186603));
        assert_eq!(s.models["claude-opus-5-5"], tok(725608, 81916346));
        assert_eq!(s.subagent_models["claude-sonnet-5-5"], tok(349458, 2814166));
        assert_eq!(s.subagent_models["claude-fable-5-1"], tok(180317, 0));
        assert!(f.sessions[1].subagent_models.is_empty());
        assert_eq!(f.sessions[1].ended_ms, None);
    }

    #[test]
    fn malformed_and_foreign_files_are_skipped_and_counted() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join(".bee/usage");
        fs::create_dir_all(&usage).unwrap();
        fs::write(usage.join("good.json"), REAL).unwrap();
        fs::write(usage.join("bad.json"), "{not json").unwrap();
        fs::write(
            usage.join("other.json"),
            r#"{"schema":"bee-usage/v9","feature":"x"}"#,
        )
        .unwrap();
        fs::write(usage.join("notes.txt"), "ignored").unwrap();

        let (features, skipped) = read_usage(dir.path());
        assert_eq!(features.len(), 1);
        assert_eq!(skipped, 2);
        assert_eq!(read_usage(&dir.path().join("missing")).1, 0);
    }

    #[test]
    fn window_edge_counts_inside_in_full_and_outside_zero() {
        let now = ms("2026-10-23T12:00:00Z");
        let since = ms("2026-10-17T00:00:00Z");
        let input = project(vec![feature(
            "f",
            Some(now),
            vec![
                session(Some(since), 10, 5),
                session(Some(since - 1), 100, 50),
                session(Some(now), 1000, 0),
                session(Some(now + 1), 7, 7),
            ],
        )]);
        let sum = summarize(&input, now, 7, 0);
        assert_eq!(sum.since_ms, since);
        assert_eq!(sum.totals.main, tok(1010, 0));
        assert_eq!(sum.totals.subagents, tok(5, 0));
        assert_eq!(sum.totals.all, tok(1015, 0));
        assert_eq!(sum.projects[0].active_ms, 2 * 60_000);
    }

    #[test]
    fn main_and_subagent_tokens_stay_separate_per_model() {
        let now = ms("2026-10-23T12:00:00Z");
        let mut s = session(Some(now - 1000), 10, 20);
        s.subagent_models.insert("opus".into(), tok(3, 4));
        let sum = summarize(&project(vec![feature("f", Some(now), vec![s])]), now, 1, 0);
        assert_eq!(
            sum.models,
            vec![
                ModelSpend {
                    model: "sonnet".into(),
                    main: Tokens::default(),
                    subagents: tok(20, 0),
                },
                ModelSpend {
                    model: "opus".into(),
                    main: tok(10, 0),
                    subagents: tok(3, 4),
                },
            ]
        );
    }

    #[test]
    fn session_without_ended_ms_falls_back_to_closed_at() {
        let now = ms("2026-10-23T12:00:00Z");
        let inside = project(vec![feature(
            "f",
            Some(now - DAY_MS),
            vec![session(None, 9, 1)],
        )]);
        assert_eq!(summarize(&inside, now, 7, 0).totals.all, tok(10, 0));
        let outside = project(vec![feature(
            "f",
            Some(now - 8 * DAY_MS),
            vec![session(None, 9, 1)],
        )]);
        assert_eq!(summarize(&outside, now, 7, 0).totals.all, Tokens::default());
        let unknown = project(vec![feature("f", None, vec![session(None, 9, 1)])]);
        assert_eq!(summarize(&unknown, now, 7, 0).totals.all, Tokens::default());
    }

    #[test]
    fn local_offset_moves_a_session_across_the_day_boundary() {
        let now = ms("2026-10-23T03:00:00Z");
        let input = project(vec![feature(
            "f",
            Some(now),
            vec![session(Some(ms("2026-10-22T20:00:00Z")), 5, 0)],
        )]);
        let local = summarize(&input, now, 2, 25200);
        assert_eq!(local.per_day[0].date, "2026-10-22");
        assert_eq!(local.per_day[1].date, "2026-10-23");
        assert_eq!(local.per_day[0].spend, Tokens::default());
        assert_eq!(local.per_day[1].spend, tok(5, 0));
        let utc = summarize(&input, now, 2, 0);
        assert_eq!(utc.per_day[0].spend, tok(5, 0));
        assert_eq!(utc.per_day[1].spend, Tokens::default());
    }

    #[test]
    fn per_day_has_one_entry_per_window_day_including_zero_days() {
        let now = ms("2026-10-23T12:00:00Z");
        for days in [1, 7, 14, 30] {
            let sum = summarize(&[], now, days, 25200);
            assert_eq!(sum.per_day.len(), days as usize);
            assert_eq!(sum.per_day.last().unwrap().date, "2026-10-23");
        }
        assert_eq!(summarize(&[], now, 30, 0).per_day[0].date, "2026-09-24");
    }

    #[test]
    fn feature_row_only_for_closed_at_inside_window_with_whole_spend() {
        let now = ms("2026-10-23T12:00:00Z");
        let mut fresh = feature(
            "fresh",
            Some(now - DAY_MS),
            vec![
                session(Some(now - 10 * DAY_MS), 100, 0),
                session(Some(now - DAY_MS - 1000), 1, 2),
            ],
        );
        fresh.cells_capped = 3;
        let newer = feature(
            "newer",
            Some(now - 1000),
            vec![session(Some(now - 2000), 1, 0)],
        );
        let old = feature(
            "old",
            Some(now - 8 * DAY_MS),
            vec![session(Some(now - 1000), 4, 0)],
        );
        let sum = summarize(&project(vec![fresh, old, newer]), now, 7, 0);

        let names: Vec<&str> = sum.features.iter().map(|f| f.feature.as_str()).collect();
        assert_eq!(names, vec!["newer", "fresh"]);
        let row = &sum.features[1];
        assert_eq!(row.cells_capped, 3);
        assert_eq!(row.spend.all, tok(103, 0));
        assert_eq!(row.active_ms, 2 * 60_000);
        assert_eq!(row.span_ms, Some(9 * DAY_MS + 60_000));
        assert_eq!(sum.projects[0].features_done, 2);
        assert_eq!(sum.totals.all, tok(8, 0));
    }

    fn per_day_sum(sum: &UsageSummary) -> Tokens {
        let mut acc = Tokens::default();
        for d in &sum.per_day {
            acc.add(&d.spend);
        }
        acc
    }

    #[test]
    fn window_starts_at_local_midnight_of_the_oldest_day() {
        let now = ms("2026-10-23T05:00:00Z");
        let input = project(vec![feature(
            "f",
            Some(now),
            vec![
                session(Some(ms("2026-10-16T18:00:00Z")), 3, 0),
                session(Some(ms("2026-10-16T16:00:00Z")), 40, 0),
                session(Some(ms("2026-10-16T17:00:00Z")), 500, 0),
            ],
        )]);
        let sum = summarize(&input, now, 7, 25200);
        assert_eq!(sum.since_ms, ms("2026-10-16T17:00:00Z"));
        assert_eq!(sum.per_day[0].date, "2026-10-17");
        assert_eq!(sum.per_day[0].spend, tok(503, 0));
        assert_eq!(sum.totals.all, tok(503, 0));
        assert_eq!(per_day_sum(&sum), sum.totals.all);
    }

    #[test]
    fn one_day_window_covers_only_today_local() {
        let now = ms("2026-10-23T05:00:00Z");
        let input = project(vec![feature(
            "f",
            Some(now),
            vec![
                session(Some(ms("2026-10-22T17:00:00Z")), 2, 0),
                session(Some(ms("2026-10-22T16:59:59Z")), 30, 0),
            ],
        )]);
        let sum = summarize(&input, now, 1, 25200);
        assert_eq!(sum.since_ms, ms("2026-10-22T17:00:00Z"));
        assert_eq!(sum.per_day.len(), 1);
        assert_eq!(sum.totals.all, tok(2, 0));
        assert_eq!(per_day_sum(&sum), sum.totals.all);
    }

    #[test]
    fn per_day_bars_sum_to_totals() {
        let now = ms("2026-10-23T05:00:00Z");
        let ends = [
            "2026-09-23T16:59:59Z",
            "2026-09-23T17:00:00Z",
            "2026-10-01T00:00:00Z",
            "2026-10-16T16:00:00Z",
            "2026-10-16T18:00:00Z",
            "2026-10-22T23:59:59Z",
            "2026-10-23T05:00:00Z",
            "2026-10-23T05:00:01Z",
        ];
        let sessions = ends.iter().map(|e| session(Some(ms(e)), 7, 3)).collect();
        let input = project(vec![feature("f", Some(now), sessions)]);
        for days in [1, 7, 14, 30] {
            for offset in [0, 25200, -18000] {
                let sum = summarize(&input, now, days, offset);
                assert_eq!(
                    per_day_sum(&sum),
                    sum.totals.all,
                    "days {days} offset {offset}"
                );
            }
        }
    }

    fn run(
        key: &str,
        provider: &str,
        subagent: bool,
        events: &[(&str, &str, u64, u64)],
    ) -> AgentRun {
        let events: Vec<crate::agent_usage::UsageEvent> = events
            .iter()
            .map(|(ts, model, new, cached)| crate::agent_usage::UsageEvent {
                ts_ms: ms(ts),
                model: (*model).into(),
                new: *new,
                cached: *cached,
            })
            .collect();
        AgentRun {
            key: key.into(),
            provider: provider.into(),
            title: format!("{key} task"),
            parent_key: subagent.then(|| "claude:main".to_string()),
            subagent,
            started_ms: events.iter().map(|e| e.ts_ms).min().unwrap(),
            ended_ms: events.iter().map(|e| e.ts_ms).max().unwrap(),
            events,
            ..Default::default()
        }
    }

    #[test]
    fn runs_feed_totals_projects_models_and_days_while_features_stay_from_bee_files() {
        let now = ms("2026-10-23T12:00:00Z");
        let input = project(vec![feature(
            "f",
            Some(now - 1000),
            vec![session(Some(now - 2000), 500, 500)],
        )]);
        let runs = vec![
            (
                Some("p1".to_string()),
                run(
                    "claude:main",
                    "claude",
                    false,
                    &[
                        ("2026-10-22T10:00:00Z", "opus", 10, 1),
                        ("2026-10-23T10:00:00Z", "opus", 20, 2),
                        ("2026-10-23T10:30:00Z", "sonnet", 1, 0),
                    ],
                ),
            ),
            (
                Some("p1".to_string()),
                run(
                    "pi:child",
                    "pi",
                    true,
                    &[("2026-10-23T11:00:00Z", "hive/flash", 7, 3)],
                ),
            ),
            (
                None,
                run(
                    "codex:x",
                    "codex",
                    false,
                    &[("2026-10-23T09:00:00Z", "gpt", 100, 0)],
                ),
            ),
            (
                Some("unregistered".to_string()),
                run(
                    "claude:old",
                    "claude",
                    false,
                    &[("2026-10-10T09:00:00Z", "opus", 9, 9)],
                ),
            ),
        ];
        let sum = summarize_runs(&input, &runs, now, 2, 0);

        assert_eq!(sum.totals.main, tok(131, 3));
        assert_eq!(sum.totals.subagents, tok(7, 3));
        assert_eq!(per_day_sum(&sum), sum.totals.all);
        assert_eq!(sum.per_day[0].spend, tok(10, 1));

        let ids: Vec<(&str, &str)> = sum
            .projects
            .iter()
            .map(|p| (p.id.as_str(), p.name.as_str()))
            .collect();
        assert_eq!(ids, vec![("", "Other"), ("p1", "one")]);
        assert_eq!(sum.projects[1].spend.main, tok(31, 3));
        assert_eq!(sum.projects[1].spend.subagents, tok(7, 3));
        assert_eq!(sum.projects[1].features_done, 1);
        assert_eq!(sum.projects[0].spend.all, tok(100, 0));

        let opus = sum.models.iter().find(|m| m.model == "opus").unwrap();
        assert_eq!(opus.main, tok(30, 3));
        let flash = sum.models.iter().find(|m| m.model == "hive/flash").unwrap();
        assert_eq!(flash.subagents, tok(7, 3));
        assert_eq!(flash.main, Tokens::default());

        assert_eq!(sum.features.len(), 1);
        assert_eq!(sum.features[0].spend.all, tok(1000, 0));

        let keys: Vec<&str> = sum.runs.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["pi:child", "codex:x", "claude:main"]);
        let child = &sum.runs[0];
        assert!(child.subagent);
        assert_eq!(child.parent_key.as_deref(), Some("claude:main"));
        assert_eq!(child.project_name, "one");
        assert_eq!(child.title, "pi:child task");
        let main = &sum.runs[2];
        assert_eq!(main.model, "opus");
        assert_eq!(main.spend, tok(31, 3));
        assert_eq!(
            main.duration_ms,
            ms("2026-10-23T10:30:00Z") - ms("2026-10-22T10:00:00Z")
        );
        assert_eq!(sum.runs[1].project_id, "");
    }

    #[test]
    fn run_events_bucket_by_their_own_local_day() {
        let now = ms("2026-10-23T05:00:00Z");
        let runs = vec![(
            Some("p1".to_string()),
            run(
                "claude:main",
                "claude",
                false,
                &[
                    ("2026-10-22T16:59:59Z", "opus", 1, 0),
                    ("2026-10-22T17:00:00Z", "opus", 20, 0),
                    ("2026-10-23T04:00:00Z", "opus", 300, 0),
                    ("2026-10-23T05:00:01Z", "opus", 4000, 0),
                ],
            ),
        )];
        let sum = summarize_runs(&project(Vec::new()), &runs, now, 2, 25200);
        assert_eq!(sum.per_day[0].spend, tok(1, 0));
        assert_eq!(sum.per_day[1].spend, tok(320, 0));
        assert_eq!(sum.totals.all, tok(321, 0));
        assert_eq!(per_day_sum(&sum), sum.totals.all);
        assert_eq!(sum.runs[0].spend, tok(321, 0));
        assert!(summarize(&project(Vec::new()), now, 2, 0).runs.is_empty());
    }

    #[test]
    fn local_offset_holder_keeps_the_first_value() {
        set_local_offset_secs(3600);
        let after = local_offset_secs();
        set_local_offset_secs(-1);
        assert_eq!(local_offset_secs(), after);
    }
}
