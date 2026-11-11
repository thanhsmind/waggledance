use crate::cli::{Cli, Command, OutputArgs, RunsAction, TaskVerb};
use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, ValueEnum};
use serde_json::{json, Map, Value};
use std::io::{IsTerminal, Read};

const VERBS: [&str; 8] = [
    "projects", "state", "dispatch", "await", "runs", "task", "schema", "search",
];
const DELTA_TAIL_LINES: usize = 20;
const RECENT_DECISIONS_KEPT: usize = 3;
const SEARCH_MATCHES: usize = 3;
const STOPWORDS: [&str; 10] = [
    "the", "and", "for", "with", "from", "into", "that", "this", "are", "you",
];

enum Shape {
    Plain,
    Projects,
    StateRollup,
    StateOne,
    Runs,
    Await,
}

struct Call {
    tool: &'static str,
    args: Value,
    shape: Shape,
    out: OutputArgs,
}

pub fn run(command: Command) -> Result<()> {
    let value = match &command {
        Command::Schema { verb } => schema(verb.as_deref())?,
        Command::Search { text } => search(&text.join(" ")),
        other => {
            let call = build(other)?;
            let response = crate::mcp::call_tool(call.tool, call.args.clone())?;
            shape(&call, response)?
        }
    };
    print_json(&value)
}

fn print_json(value: &Value) -> Result<()> {
    let text = if std::io::stdout().is_terminal() {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    println!("{text}");
    Ok(())
}

fn read_task(task: &Option<String>, task_file: &Option<std::path::PathBuf>) -> Result<String> {
    if let Some(task) = task {
        return Ok(task.clone());
    }
    if let Some(path) = task_file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("could not read --task-file {}", path.display()));
    }
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("could not read the task from stdin")?;
    Ok(text)
}

fn put(args: &mut Value, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        args[key] = value;
    }
}

fn task_verb_name(verb: TaskVerb) -> String {
    verb.to_possible_value()
        .map(|v| v.get_name().to_string())
        .unwrap_or_default()
}

fn build(command: &Command) -> Result<Call> {
    let call = match command {
        Command::Projects(out) => Call {
            tool: "waggledance_projects",
            args: json!({}),
            shape: Shape::Projects,
            out: out.clone(),
        },
        Command::State { project, out } => {
            let mut args = json!({});
            put(&mut args, "project", project.as_ref().map(|p| json!(p)));
            Call {
                tool: "waggledance_ask_state",
                args,
                shape: if project.is_some() {
                    Shape::StateOne
                } else {
                    Shape::StateRollup
                },
                out: out.clone(),
            }
        }
        Command::Dispatch {
            project,
            leader,
            preset,
            pane_id,
            task,
            task_file,
            request_id,
            follow_up_of,
            handoff,
            no_handoff,
            steer,
        } => {
            let mut args = json!({ "project": project, "task": read_task(task, task_file)? });
            put(&mut args, "leader", leader.then(|| json!(true)));
            put(&mut args, "preset", preset.as_ref().map(|v| json!(v)));
            put(&mut args, "pane_id", pane_id.as_ref().map(|v| json!(v)));
            put(
                &mut args,
                "request_id",
                request_id.as_ref().map(|v| json!(v)),
            );
            put(
                &mut args,
                "follow_up_of",
                follow_up_of.as_ref().map(|v| json!(v)),
            );
            put(
                &mut args,
                "handoff",
                if *handoff {
                    Some(json!(true))
                } else if *no_handoff {
                    Some(json!(false))
                } else {
                    None
                },
            );
            put(&mut args, "steer", steer.then(|| json!(true)));
            Call {
                tool: "waggledance_dispatch",
                args,
                shape: Shape::Plain,
                out: OutputArgs::default(),
            }
        }
        Command::Await {
            run_id,
            timeout,
            out,
        } => {
            let mut args = json!({ "run_id": run_id });
            put(&mut args, "timeout_seconds", timeout.map(|t| json!(t)));
            Call {
                tool: "waggledance_await",
                args,
                shape: Shape::Await,
                out: out.clone(),
            }
        }
        Command::Runs {
            project,
            out,
            action,
        } => {
            let mut args = json!({});
            put(&mut args, "project", project.as_ref().map(|p| json!(p)));
            let shape = match action {
                Some(RunsAction::Show { run_id }) => {
                    args["run_id"] = json!(run_id);
                    Shape::Plain
                }
                None => Shape::Runs,
            };
            Call {
                tool: "waggledance_runs",
                args,
                shape,
                out: out.clone(),
            }
        }
        Command::Task {
            corr,
            action,
            text,
            req_id,
            out,
        } => {
            let mut args = json!({ "corr": corr, "action": task_verb_name(*action) });
            put(&mut args, "text", text.as_ref().map(|v| json!(v)));
            put(&mut args, "req_id", req_id.as_ref().map(|v| json!(v)));
            Call {
                tool: "waggledance_task",
                args,
                shape: Shape::Plain,
                out: out.clone(),
            }
        }
        _ => return Err(anyhow!("not an agent verb")),
    };
    Ok(call)
}

fn shape(call: &Call, response: Value) -> Result<Value> {
    if let Some(message) = response.pointer("/error/message").and_then(Value::as_str) {
        return Err(anyhow!("{message}"));
    }
    let result = response.get("result").cloned().unwrap_or(Value::Null);
    let text = result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or("");
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(anyhow!("{text}"));
    }
    let structured = result
        .get("structuredContent")
        .cloned()
        .unwrap_or_else(|| json!({ "text": text }));
    let (value, rows_key) = if call.out.full {
        let key = match call.shape {
            Shape::Projects | Shape::StateRollup => Some("projects"),
            Shape::Runs => Some("runs"),
            Shape::StateOne => Some("project"),
            _ => None,
        };
        (structured, key)
    } else {
        match call.shape {
            Shape::Plain => (structured, None),
            Shape::Projects => (summarize_projects(structured), Some("projects")),
            Shape::StateRollup => (summarize_rollup(structured), Some("projects")),
            Shape::StateOne => (summarize_digest(structured), None),
            Shape::Runs => (summarize_runs(structured), Some("runs")),
            Shape::Await => (summarize_await(structured), None),
        }
    };
    Ok(narrow(value, rows_key, &call.out))
}

fn pick(row: &Value, keys: &[&str]) -> Value {
    let mut out = Map::new();
    for key in keys {
        if let Some(v) = row.get(*key) {
            out.insert((*key).to_string(), v.clone());
        }
    }
    Value::Object(out)
}

fn map_rows(structured: Value, key: &str, f: impl Fn(&Value) -> Value) -> Value {
    let rows: Vec<Value> = structured
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| rows.iter().map(f).collect())
        .unwrap_or_default();
    json!({ key: rows })
}

fn summarize_projects(structured: Value) -> Value {
    map_rows(structured, "projects", |p| {
        pick(p, &["id", "name", "root_path"])
    })
}

fn summarize_rollup(structured: Value) -> Value {
    map_rows(structured, "projects", |p| {
        let mut row = pick(
            p,
            &[
                "project_id",
                "feature",
                "phase",
                "mode",
                "waiting_on_live",
                "cell_counts",
            ],
        );
        let leaders = p
            .get("leaders")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        row["leader_count"] = json!(leaders);
        row
    })
}

fn summarize_digest(structured: Value) -> Value {
    let mut digest = structured.get("project").cloned().unwrap_or(structured);
    if let Some(map) = digest.as_object_mut() {
        map.remove("sessions");
        if let Some(decisions) = map
            .get_mut("recent_decisions")
            .and_then(Value::as_array_mut)
        {
            decisions.truncate(RECENT_DECISIONS_KEPT);
        }
    }
    digest
}

fn summarize_runs(structured: Value) -> Value {
    map_rows(structured, "runs", |r| {
        pick(
            r,
            &["id", "project_id", "status", "task_summary", "updated_at"],
        )
    })
}

fn summarize_await(structured: Value) -> Value {
    let mut out = pick(&structured, &["run_id", "status"]);
    if let Some(delta) = structured.get("delta").and_then(Value::as_str) {
        let lines: Vec<&str> = delta.lines().collect();
        let start = lines.len().saturating_sub(DELTA_TAIL_LINES);
        out["delta"] = json!(lines[start..].join("\n"));
    }
    out
}

fn keep_fields(row: &mut Value, fields: &[String]) {
    if fields.is_empty() {
        return;
    }
    if let Some(map) = row.as_object_mut() {
        map.retain(|k, _| fields.iter().any(|f| f == k));
    }
}

fn narrow(mut value: Value, rows_key: Option<&str>, out: &OutputArgs) -> Value {
    match rows_key.and_then(|k| value.get_mut(k)) {
        Some(Value::Array(rows)) => {
            if let Some(limit) = out.limit {
                rows.truncate(limit);
            }
            rows.iter_mut().for_each(|r| keep_fields(r, &out.fields));
        }
        Some(row) => keep_fields(row, &out.fields),
        None => keep_fields(&mut value, &out.fields),
    }
    value
}

fn verb_commands() -> Vec<clap::Command> {
    let mut root = Cli::command();
    root.build();
    VERBS
        .iter()
        .filter_map(|name| root.find_subcommand(name).cloned())
        .collect()
}

fn help_of(cmd: &clap::Command) -> String {
    cmd.get_about().map(|a| a.to_string()).unwrap_or_default()
}

fn flags_of(cmd: &clap::Command) -> Vec<Value> {
    cmd.get_arguments()
        .filter(|a| a.get_id() != "help" && a.get_id() != "version")
        .map(|a| {
            let name = match a.get_long() {
                Some(long) => format!("--{long}"),
                None => a.get_id().to_string(),
            };
            json!({
                "name": name,
                "positional": a.is_positional(),
                "required": a.is_required_set(),
                "takes_value": a.get_action().takes_values(),
                "help": a.get_help().map(|h| h.to_string()).unwrap_or_default()
            })
        })
        .collect()
}

fn describe(cmd: &clap::Command) -> Value {
    let subcommands: Vec<Value> = cmd
        .get_subcommands()
        .filter(|s| s.get_name() != "help")
        .map(describe)
        .collect();
    json!({
        "name": cmd.get_name(),
        "help": help_of(cmd),
        "flags": flags_of(cmd),
        "subcommands": subcommands
    })
}

fn schema(verb: Option<&str>) -> Result<Value> {
    let commands = verb_commands();
    match verb {
        None => Ok(json!({ "verbs": commands.iter().map(describe).collect::<Vec<_>>() })),
        Some(name) => commands
            .iter()
            .find(|c| c.get_name() == name)
            .map(describe)
            .ok_or_else(|| anyhow!("no such verb: {name}; known verbs: {}", VERBS.join(", "))),
    }
}

fn terms_of(text: &str) -> Vec<String> {
    let mut terms: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3 && !STOPWORDS.contains(t))
        .map(str::to_string)
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

fn score(cmd: &clap::Command, terms: &[String]) -> usize {
    let name = cmd.get_name().to_lowercase();
    let help = help_of(cmd).to_lowercase();
    let mut flag_texts: Vec<String> = flags_of(cmd)
        .iter()
        .map(|f| {
            format!(
                "{} {}",
                f["name"].as_str().unwrap_or(""),
                f["help"].as_str().unwrap_or("")
            )
            .to_lowercase()
        })
        .collect();
    for sub in cmd.get_subcommands().filter(|s| s.get_name() != "help") {
        flag_texts.push(format!("{} {}", sub.get_name(), help_of(sub)).to_lowercase());
        flag_texts.extend(flags_of(sub).iter().map(|f| {
            format!(
                "{} {}",
                f["name"].as_str().unwrap_or(""),
                f["help"].as_str().unwrap_or("")
            )
            .to_lowercase()
        }));
    }
    terms
        .iter()
        .map(|term| {
            let mut s = 0;
            if name.contains(term.as_str()) {
                s += 3;
            }
            if help.contains(term.as_str()) {
                s += 2;
            }
            s + flag_texts
                .iter()
                .filter(|t| t.contains(term.as_str()))
                .count()
        })
        .sum()
}

fn usage_of(cmd: &clap::Command) -> String {
    let mut cmd = cmd.clone();
    let usage = cmd.render_usage().to_string();
    usage.trim_start_matches("Usage: ").trim().to_string()
}

fn search(text: &str) -> Value {
    let terms = terms_of(text);
    let mut scored: Vec<(usize, clap::Command)> = verb_commands()
        .into_iter()
        .map(|c| (score(&c, &terms), c))
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let matches: Vec<Value> = scored
        .iter()
        .take(SEARCH_MATCHES)
        .map(|(s, c)| {
            json!({
                "verb": c.get_name(),
                "score": s,
                "help": help_of(c),
                "usage": usage_of(c)
            })
        })
        .collect();
    json!({ "matches": matches })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use waggledance_core::domain::Run;
    use waggledance_core::{Config, Engine, SqliteStore};

    fn engine_with_runs(runs: usize) -> (Engine, Vec<tempfile::TempDir>) {
        let mut config = Config::default();
        config.paseo.enabled = Some(false);
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), config);
        let mut dirs = Vec::new();
        for name in ["alpha", "beta"] {
            let dir = tempfile::Builder::new().prefix(name).tempdir().unwrap();
            engine.register(dir.path(), Some(name)).unwrap();
            dirs.push(dir);
        }
        let project = engine.list_projects().unwrap().remove(0);
        let now = waggledance_core::engine::now_rfc3339();
        for i in 0..runs {
            let run = Run {
                id: format!("run-{i}"),
                project_id: project.id.clone(),
                pane_id: "w1:p1".to_string(),
                preset_label: None,
                task: format!("first line {i}\nsecond line"),
                baseline: String::new(),
                marker: "HERDR_DONE_x".to_string(),
                status: "working".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            engine.insert_run(&run, None).unwrap();
        }
        (engine, dirs)
    }

    fn parse(args: &[&str]) -> Command {
        let mut argv = vec!["waggledance"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv).unwrap().command
    }

    fn exec(engine: &Engine, args: &[&str]) -> Result<Value> {
        let call = build(&parse(args))?;
        let response = crate::mcp::call_tool_on(engine, call.tool, call.args.clone());
        shape(&call, response)
    }

    fn structured(engine: &Engine, tool: &str, args: Value) -> Value {
        crate::mcp::call_tool_on(engine, tool, args)["result"]["structuredContent"].clone()
    }

    #[test]
    fn state_summary_is_one_row_per_project_without_sessions() {
        let (engine, _dirs) = engine_with_runs(0);
        let out = exec(&engine, &["state"]).unwrap();
        let rows = out["projects"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert!(row.get("sessions").is_none() && row.get("recent_decisions").is_none());
            for key in [
                "project_id",
                "cell_counts",
                "leader_count",
                "waiting_on_live",
            ] {
                assert!(row.get(key).is_some(), "{key} missing in {row}");
            }
        }
    }

    #[test]
    fn state_full_equals_the_mcp_structured_content() {
        let (engine, _dirs) = engine_with_runs(0);
        let full = exec(&engine, &["state", "--full"]).unwrap();
        assert_eq!(
            full,
            structured(&engine, "waggledance_ask_state", json!({}))
        );
        let id = engine.list_projects().unwrap().remove(0).id;
        let one = exec(&engine, &["state", &id, "--full"]).unwrap();
        assert_eq!(
            one,
            structured(&engine, "waggledance_ask_state", json!({ "project": id }))
        );
    }

    #[test]
    fn one_project_state_drops_sessions_and_keeps_the_digest() {
        let (engine, _dirs) = engine_with_runs(0);
        let id = engine.list_projects().unwrap().remove(0).id;
        let out = exec(&engine, &["state", &id]).unwrap();
        assert!(out.get("sessions").is_none());
        assert_eq!(out["project_id"], json!(id));
    }

    #[test]
    fn runs_summary_has_task_summary_and_full_equals_mcp() {
        let (engine, _dirs) = engine_with_runs(2);
        let out = exec(&engine, &["runs"]).unwrap();
        let rows = out["runs"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(
                row["task_summary"]
                    .as_str()
                    .unwrap()
                    .starts_with("first line"),
                true
            );
            assert!(row.get("task").is_none() && row.get("pane_id").is_none());
        }
        let full = exec(&engine, &["runs", "--full"]).unwrap();
        assert_eq!(full, structured(&engine, "waggledance_runs", json!({})));
    }

    #[test]
    fn runs_show_prints_the_full_task() {
        let (engine, _dirs) = engine_with_runs(1);
        let out = exec(&engine, &["runs", "show", "run-0"]).unwrap();
        assert_eq!(out["runs"][0]["task"], "first line 0\nsecond line");
    }

    #[test]
    fn fields_and_limit_narrow_rows() {
        let (engine, _dirs) = engine_with_runs(3);
        let out = exec(&engine, &["runs", "--limit", "2", "--fields", "id,status"]).unwrap();
        let rows = out["runs"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            let mut keys: Vec<&String> = row.as_object().unwrap().keys().collect();
            keys.sort();
            assert_eq!(keys, ["id", "status"]);
        }
    }

    #[test]
    fn tool_error_becomes_an_err_with_its_text() {
        let (engine, _dirs) = engine_with_runs(0);
        let err = exec(&engine, &["runs", "show", "nope"]).unwrap_err();
        assert!(err.to_string().contains("nope"), "{err}");
    }

    #[test]
    fn dispatch_maps_flags_to_tool_arguments() {
        let call = build(&parse(&[
            "dispatch",
            "--project",
            "p",
            "--leader",
            "--task",
            "do it",
            "--request-id",
            "r1",
            "--follow-up-of",
            "run-9",
            "--no-handoff",
            "--steer",
        ]))
        .unwrap();
        assert_eq!(call.tool, "waggledance_dispatch");
        assert_eq!(
            call.args,
            json!({
                "project": "p", "task": "do it", "leader": true, "request_id": "r1",
                "follow_up_of": "run-9", "handoff": false, "steer": true
            })
        );
    }

    #[test]
    fn task_and_await_map_flags_to_tool_arguments() {
        let call = build(&parse(&["task", "c1", "reply", "--text", "go"])).unwrap();
        assert_eq!(
            call.args,
            json!({ "corr": "c1", "action": "reply", "text": "go" })
        );
        let call = build(&parse(&["await", "run-1", "--timeout", "5"])).unwrap();
        assert_eq!(
            call.args,
            json!({ "run_id": "run-1", "timeout_seconds": 5 })
        );
    }

    #[test]
    fn await_summary_keeps_status_and_the_delta_tail() {
        let structured = json!({
            "run_id": "r", "status": "done", "age_secs": 4,
            "delta": (0..40).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n")
        });
        let out = summarize_await(structured);
        assert_eq!(out["status"], "done");
        assert!(out.get("age_secs").is_none());
        let delta = out["delta"].as_str().unwrap();
        assert_eq!(delta.lines().count(), DELTA_TAIL_LINES);
        assert!(delta.ends_with("line 39"));
    }

    #[test]
    fn schema_covers_every_verb_with_flags() {
        let all = schema(None).unwrap();
        let names: Vec<&str> = all["verbs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, VERBS);
        let dispatch = schema(Some("dispatch")).unwrap();
        let flags: Vec<&str> = dispatch["flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        for flag in [
            "--project",
            "--leader",
            "--preset",
            "--pane-id",
            "--task",
            "--task-file",
            "--request-id",
            "--follow-up-of",
            "--handoff",
            "--no-handoff",
            "--steer",
        ] {
            assert!(flags.contains(&flag), "{flag} missing from {flags:?}");
        }
        let runs = schema(Some("runs")).unwrap();
        assert_eq!(runs["subcommands"][0]["name"], "show");
        assert!(schema(Some("bogus")).is_err());
    }

    #[test]
    fn search_ranks_dispatch_first_for_dispatch_shaped_queries() {
        for query in [
            "dispatch",
            "dispatch a task to the leader",
            "send work to the project leader",
        ] {
            let out = search(query);
            assert_eq!(out["matches"][0]["verb"], "dispatch", "{query}: {out}");
            assert!(out["matches"][0]["usage"]
                .as_str()
                .unwrap()
                .starts_with("waggledance dispatch"));
        }
    }

    #[test]
    fn top_level_help_names_search() {
        let help = Cli::command().render_help().to_string();
        assert!(help.contains("waggledance search"), "{help}");
    }
}
