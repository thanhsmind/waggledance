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
    fn add_session(&mut self, s: &SessionUsage) {
        for t in s.models.values() {
            self.main.add(t);
            self.all.add(t);
        }
        for t in s.subagent_models.values() {
            self.subagents.add(t);
            self.all.add(t);
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
pub struct UsageSummary {
    pub days: u32,
    pub since_ms: i64,
    pub now_ms: i64,
    pub totals: SpendSplit,
    pub projects: Vec<ProjectSpend>,
    pub models: Vec<ModelSpend>,
    pub per_day: Vec<DaySpend>,
    pub features: Vec<FeatureRow>,
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
    let since_ms = now_ms - i64::from(days) * DAY_MS;
    let in_window = |t: i64| t > since_ms && t <= now_ms;
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

    let mut totals = SpendSplit::default();
    let mut models: BTreeMap<String, (Tokens, Tokens)> = BTreeMap::new();
    let mut project_rows = Vec::new();
    let mut features = Vec::new();

    for p in projects {
        let mut spend = SpendSplit::default();
        let mut active_ms = 0;
        let mut features_done = 0;
        for f in &p.features {
            for s in &f.sessions {
                let Some(at) = s.ended_ms.or(f.closed_at_ms) else {
                    continue;
                };
                if !in_window(at) {
                    continue;
                }
                let mut one = SpendSplit::default();
                one.add_session(s);
                spend.add_session(s);
                totals.add_session(s);
                add_models(&mut models, s);
                active_ms += s.active_ms();
                if let Some(&i) = local_date(at, offset).and_then(|d| day_index.get(&d)) {
                    per_day[i].spend.add(&one.all);
                }
            }
            if f.closed_at_ms.is_some_and(in_window) {
                features_done += 1;
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
        project_rows.push(ProjectSpend {
            id: p.id.clone(),
            name: p.name.clone(),
            spend,
            features_done,
            active_ms,
        });
    }

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
        let since = now - 7 * DAY_MS;
        let input = project(vec![feature(
            "f",
            Some(now),
            vec![
                session(Some(since + 1), 10, 5),
                session(Some(since), 100, 50),
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

    #[test]
    fn local_offset_holder_keeps_the_first_value() {
        set_local_offset_secs(3600);
        let after = local_offset_secs();
        set_local_offset_secs(-1);
        assert_eq!(local_offset_secs(), after);
    }
}
