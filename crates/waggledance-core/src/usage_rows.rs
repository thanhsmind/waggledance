use crate::agent_usage::AgentRun;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use time::{Date, OffsetDateTime, UtcOffset};

pub const SAIGON_OFFSET_SECS: i32 = 7 * 3600;
pub const TEAM: &str = "waggledance";
pub const COST_SOURCE: &str = "transcript";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunMeta {
    pub preset_label: Option<String>,
    pub task: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageRow {
    pub day: String,
    pub team: String,
    pub project: String,
    pub agent: String,
    pub provider: String,
    pub model: String,
    pub session: String,
    pub preset: Option<String>,
    pub run_title: Option<String>,
    pub parent: Option<String>,
    pub subagent: bool,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub tokens: u64,
    pub cost: Option<f64>,
    pub cost_input: Option<f64>,
    pub cost_output: Option<f64>,
    pub cost_cache_read: Option<f64>,
    pub cost_cache_write: Option<f64>,
    pub cost_source: Option<String>,
    pub messages: u64,
    pub tool_calls: u64,
    pub errors: u64,
}

fn saigon() -> UtcOffset {
    UtcOffset::from_whole_seconds(SAIGON_OFFSET_SECS).unwrap_or(UtcOffset::UTC)
}

pub fn day_of(ts_ms: i64) -> Option<Date> {
    let at = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_ms) * 1_000_000).ok()?;
    Some(at.to_offset(saigon()).date())
}

pub fn today() -> Date {
    OffsetDateTime::now_utc().to_offset(saigon()).date()
}

pub fn day_start_ms(day: Date) -> i64 {
    let at = day.midnight().assume_offset(saigon());
    (at.unix_timestamp_nanos() / 1_000_000) as i64
}

fn session_of(key: &str) -> String {
    key.split_once(':').map_or(key, |(_, s)| s).to_string()
}

fn bare_model(provider: &str, model: &str) -> String {
    if provider.is_empty() {
        return model.to_string();
    }
    model
        .strip_prefix(provider)
        .and_then(|m| m.strip_prefix('/'))
        .unwrap_or(model)
        .to_string()
}

fn first_line(task: &str) -> Option<String> {
    let line = task.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.chars().take(200).collect())
}

fn add_opt(slot: &mut Option<f64>, v: Option<f64>) {
    if let Some(v) = v {
        *slot = Some(slot.unwrap_or(0.0) + v);
    }
}

pub fn build(
    runs: &[(Option<String>, Arc<AgentRun>)],
    presets: &HashMap<String, RunMeta>,
    from: Date,
    to: Date,
) -> Vec<UsageRow> {
    let mut groups: BTreeMap<(Date, String, String, String, String, String), UsageRow> =
        BTreeMap::new();
    for (project, run) in runs {
        let project = project.clone().unwrap_or_else(|| "other".into());
        let meta = presets.get(&run.session_id);
        let preset = meta.and_then(|m| m.preset_label.clone());
        let agent = preset.clone().unwrap_or_else(|| run.provider.clone());
        let session = session_of(&run.key);
        let run_title = Some(run.title.trim())
            .filter(|t| !t.is_empty())
            .map(String::from)
            .or_else(|| meta.and_then(|m| first_line(&m.task)));
        let parent = run.parent_key.as_deref().map(session_of);
        for e in &run.events {
            let Some(day) = day_of(e.ts_ms).filter(|d| *d >= from && *d <= to) else {
                continue;
            };
            let provider = if e.provider.is_empty() {
                run.provider.clone()
            } else {
                e.provider.clone()
            };
            let model = bare_model(&e.provider, &e.model);
            let key = (
                day,
                project.clone(),
                agent.clone(),
                provider.clone(),
                model.clone(),
                session.clone(),
            );
            let row = groups.entry(key).or_insert_with(|| UsageRow {
                day: day.to_string(),
                team: TEAM.into(),
                project: project.clone(),
                agent: agent.clone(),
                provider,
                model,
                session: session.clone(),
                preset: preset.clone(),
                run_title: run_title.clone(),
                parent: parent.clone(),
                subagent: run.subagent,
                ..UsageRow::default()
            });
            row.input += e.input;
            row.output += e.output;
            row.cache_read += e.cache_read;
            row.cache_write += e.cache_write;
            row.tokens += e.tokens();
            row.messages += 1;
            row.tool_calls += e.tool_calls;
            row.errors += u64::from(e.error);
            if let Some(c) = e.cost {
                add_opt(&mut row.cost, Some(c.total));
                add_opt(&mut row.cost_input, c.input);
                add_opt(&mut row.cost_output, c.output);
                add_opt(&mut row.cost_cache_read, c.cache_read);
                add_opt(&mut row.cost_cache_write, c.cache_write);
                row.cost_source = Some(COST_SOURCE.into());
            }
        }
    }
    groups.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_usage::{Cost, UsageEvent};
    use time::format_description::well_known::Rfc3339;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), day).unwrap()
    }

    fn ms(s: &str) -> i64 {
        (OffsetDateTime::parse(s, &Rfc3339)
            .unwrap()
            .unix_timestamp_nanos()
            / 1_000_000) as i64
    }

    fn event(ts: &str, provider: &str, model: &str, cost: Option<Cost>) -> UsageEvent {
        UsageEvent {
            ts_ms: ms(ts),
            provider: provider.into(),
            model: model.into(),
            input: 10,
            output: 2,
            cache_read: 30,
            cache_write: 4,
            cost,
            tool_calls: 1,
            error: false,
        }
    }

    fn run(key: &str, provider: &str, session: &str, events: Vec<UsageEvent>) -> Arc<AgentRun> {
        Arc::new(AgentRun {
            key: key.into(),
            provider: provider.into(),
            session_id: session.into(),
            events,
            ..AgentRun::default()
        })
    }

    #[test]
    fn a_message_at_five_pm_utc_lands_on_the_next_saigon_day() {
        assert_eq!(day_of(ms("2026-11-03T16:59:59Z")), Some(d(2026, 11, 3)));
        assert_eq!(day_of(ms("2026-11-03T17:00:00Z")), Some(d(2026, 11, 4)));
        assert_eq!(day_start_ms(d(2026, 11, 4)), ms("2026-11-03T17:00:00Z"));
    }

    #[test]
    fn rows_group_by_day_and_session_and_sum_every_number() {
        let pi_cost = Cost {
            input: Some(0.1),
            output: Some(0.2),
            cache_read: Some(0.0),
            cache_write: None,
            total: 0.3,
        };
        let mut errored = event("2026-11-04T01:00:00Z", "hive", "hive/flash", Some(pi_cost));
        errored.error = true;
        let runs = vec![
            (
                Some("proj".to_string()),
                run(
                    "pi:p1",
                    "pi",
                    "p1",
                    vec![
                        event("2026-11-04T00:00:00Z", "hive", "hive/flash", Some(pi_cost)),
                        errored,
                        event("2026-11-04T18:00:00Z", "hive", "hive/flash", Some(pi_cost)),
                        event("2026-10-01T00:00:00Z", "hive", "hive/flash", Some(pi_cost)),
                    ],
                ),
            ),
            (
                None,
                run(
                    "claude:s1",
                    "claude",
                    "s1",
                    vec![event(
                        "2026-11-04T02:00:00Z",
                        "anthropic",
                        "claude-opus-5-5",
                        None,
                    )],
                ),
            ),
        ];
        let rows = build(&runs, &HashMap::new(), d(2026, 11, 1), d(2026, 11, 5));
        assert_eq!(rows.len(), 3);
        let claude = &rows[0];
        assert_eq!(claude.project, "other");
        assert_eq!(claude.agent, "claude");
        assert_eq!(claude.provider, "anthropic");
        assert_eq!(claude.model, "claude-opus-5-5");
        assert_eq!(claude.cost, None);
        assert_eq!(claude.cost_source, None);
        let pi = &rows[1];
        assert_eq!(
            (pi.day.as_str(), pi.project.as_str()),
            ("2026-11-04", "proj")
        );
        assert_eq!(
            (pi.agent.as_str(), pi.provider.as_str(), pi.model.as_str()),
            ("pi", "hive", "flash")
        );
        assert_eq!(pi.session, "p1");
        assert_eq!(
            (
                pi.input,
                pi.output,
                pi.cache_read,
                pi.cache_write,
                pi.tokens
            ),
            (20, 4, 60, 8, 92)
        );
        assert_eq!((pi.messages, pi.tool_calls, pi.errors), (2, 2, 1));
        assert!((pi.cost.unwrap() - 0.6).abs() < 1e-9);
        assert!((pi.cost_output.unwrap() - 0.4).abs() < 1e-9);
        assert_eq!(pi.cost_cache_write, None);
        assert_eq!(pi.cost_source.as_deref(), Some("transcript"));
        assert_eq!(rows[2].day, "2026-11-05");
    }

    #[test]
    fn a_matched_run_names_the_agent_preset_and_title() {
        let sub = Arc::new(AgentRun {
            key: "claude:s1/agent-a1".into(),
            provider: "claude".into(),
            session_id: "s1".into(),
            parent_key: Some("claude:s1".into()),
            subagent: true,
            title: "review".into(),
            events: vec![event(
                "2026-11-04T02:00:00Z",
                "anthropic",
                "claude-opus-5-5",
                None,
            )],
            ..AgentRun::default()
        });
        let main = run(
            "claude:s1",
            "claude",
            "s1",
            vec![event(
                "2026-11-04T02:00:00Z",
                "anthropic",
                "claude-opus-5-5",
                None,
            )],
        );
        let presets = HashMap::from([(
            "s1".to_string(),
            RunMeta {
                preset_label: Some("claude-sonnet".into()),
                task: "\n  Build the rows feed\nsecond line".into(),
            },
        )]);
        let runs = vec![(Some("p".to_string()), main), (Some("p".to_string()), sub)];
        let rows = build(&runs, &presets, d(2026, 11, 4), d(2026, 11, 4));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].session, "s1");
        assert_eq!(rows[0].agent, "claude-sonnet");
        assert_eq!(rows[0].preset.as_deref(), Some("claude-sonnet"));
        assert_eq!(rows[0].run_title.as_deref(), Some("Build the rows feed"));
        assert_eq!(rows[0].parent, None);
        assert!(!rows[0].subagent);
        assert_eq!(rows[1].session, "s1/agent-a1");
        assert_eq!(rows[1].parent.as_deref(), Some("s1"));
        assert_eq!(rows[1].run_title.as_deref(), Some("review"));
        assert!(rows[1].subagent);
    }

    #[test]
    fn a_row_serialises_with_the_agent_works_field_names() {
        let v = serde_json::to_value(UsageRow::default()).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut want = vec![
            "day",
            "team",
            "project",
            "agent",
            "provider",
            "model",
            "session",
            "preset",
            "runTitle",
            "parent",
            "subagent",
            "input",
            "output",
            "cacheRead",
            "cacheWrite",
            "tokens",
            "cost",
            "costInput",
            "costOutput",
            "costCacheRead",
            "costCacheWrite",
            "costSource",
            "messages",
            "toolCalls",
            "errors",
        ];
        want.sort_unstable();
        assert_eq!(keys, want);
        assert!(v["cost"].is_null() && v["preset"].is_null() && v["costSource"].is_null());
    }
}
