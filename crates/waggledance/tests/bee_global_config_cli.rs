use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

struct Env {
    home: TempDir,
    beehive: Option<String>,
    xdg: Option<String>,
}

impl Env {
    fn new() -> Self {
        let mut env = Self::bare();
        env.beehive = Some(env.home.path().join("bh").display().to_string());
        env
    }

    fn bare() -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
            beehive: None,
            xdg: None,
        }
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_waggledance"));
        cmd.env("HOME", self.home.path())
            .env_remove("BEEHIVE_CONFIG_DIR")
            .env_remove("XDG_CONFIG_HOME")
            .stdin(Stdio::null());
        if let Some(b) = &self.beehive {
            cmd.env("BEEHIVE_CONFIG_DIR", b);
        }
        if let Some(x) = &self.xdg {
            cmd.env("XDG_CONFIG_HOME", x);
        }
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut full = vec!["bee", "config"];
        full.extend_from_slice(args);
        self.cmd().args(&full).output().unwrap()
    }

    fn file(&self) -> PathBuf {
        PathBuf::from(self.beehive.as_ref().unwrap()).join("config.json")
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn ok(o: Output) -> String {
    assert!(
        o.status.success(),
        "stdout={} stderr={}",
        stdout(&o),
        stderr(&o)
    );
    stdout(&o)
}

fn path_line(env: &Env) -> String {
    ok(env.run(&["path"]))
}

fn starts_with_path(line: &str, expected: &Path) {
    assert!(
        line.starts_with(&expected.display().to_string()),
        "{line:?} does not start with {}",
        expected.display()
    );
}

#[test]
fn path_honours_beehive_dir_then_xdg_then_home_with_empty_as_unset() {
    let mut env = Env::bare();
    let home = env.home.path().to_path_buf();
    let bh = home.join("bh");
    let xdg = home.join("xdg");

    env.beehive = Some(bh.display().to_string());
    env.xdg = Some(xdg.display().to_string());
    let line = path_line(&env);
    starts_with_path(&line, &bh.join("config.json"));
    assert!(line.contains("does not exist"), "{line}");

    env.beehive = Some(String::new());
    starts_with_path(&path_line(&env), &xdg.join("beehive/config.json"));
    env.beehive = None;
    starts_with_path(&path_line(&env), &xdg.join("beehive/config.json"));

    env.xdg = Some(String::new());
    starts_with_path(&path_line(&env), &home.join(".config/beehive/config.json"));
    env.xdg = None;
    starts_with_path(&path_line(&env), &home.join(".config/beehive/config.json"));

    env.beehive = Some(bh.display().to_string());
    ok(env.run(&["set", "a", "1"]));
    let line = path_line(&env);
    starts_with_path(&line, &bh.join("config.json"));
    assert!(line.contains("(exists)"), "{line}");
}

#[test]
fn set_get_show_round_trip_writes_pretty_json_with_one_trailing_newline() {
    let env = Env::new();
    ok(env.run(&["set", "team", r#"{"name":"core","size":3}"#]));
    ok(env.run(&["set", "herding.agents", r#"["claude","codex"]"#]));
    ok(env.run(&["set", "label", "hello world"]));

    let got: Value = serde_json::from_str(&ok(env.run(&["get", "herding.agents"]))).unwrap();
    assert_eq!(got, json!(["claude", "codex"]));
    let got: Value = serde_json::from_str(&ok(env.run(&["get", "team.size"]))).unwrap();
    assert_eq!(got, json!(3));
    let got: Value = serde_json::from_str(&ok(env.run(&["get", "label"]))).unwrap();
    assert_eq!(got, json!("hello world"));

    let expected = json!({
        "team": {"name": "core", "size": 3},
        "herding": {"agents": ["claude", "codex"]},
        "label": "hello world"
    });
    let shown: Value = serde_json::from_str(&ok(env.run(&["show", "--json"]))).unwrap();
    assert_eq!(shown, expected);

    let bytes = std::fs::read_to_string(env.file()).unwrap();
    let mut pretty = serde_json::to_string_pretty(&expected).unwrap();
    pretty.push('\n');
    assert_eq!(bytes, pretty);
    assert!(bytes.ends_with("}\n") && !bytes.ends_with("\n\n"));
    assert!(bytes.contains("\n  \"team\": {\n    \"name\": \"core\""));
}

#[test]
fn get_of_a_missing_key_exits_non_zero() {
    let env = Env::new();
    let o = env.run(&["get", "nope"]);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("nope"), "{}", stderr(&o));
}

#[test]
fn per_project_keys_are_refused_and_leave_the_file_untouched() {
    let env = Env::new();
    for (key, value) in [
        ("guards.x", "true"),
        ("gate_bypass", "full"),
        ("hooks", "{}"),
    ] {
        let o = env.run(&["set", key, value]);
        assert!(!o.status.success(), "{key} was accepted");
        let first = key.split('.').next().unwrap();
        assert!(stderr(&o).contains(first), "{key}: {}", stderr(&o));
        assert!(!env.file().exists(), "{key} created the file");
    }

    ok(env.run(&["set", "keep", "1"]));
    let before = std::fs::read(env.file()).unwrap();
    for (key, value) in [
        ("guards.x", "true"),
        ("gate_bypass", "full"),
        ("hooks", "{}"),
    ] {
        let o = env.run(&["set", key, value]);
        assert!(!o.status.success(), "{key} was accepted");
        assert_eq!(
            std::fs::read(env.file()).unwrap(),
            before,
            "{key} changed the file"
        );
    }
}

#[test]
fn unset_cleans_a_hand_written_guards_key() {
    let env = Env::new();
    std::fs::create_dir_all(env.file().parent().unwrap()).unwrap();
    let hand = r#"{"guards":{"x":true},"keep":1}"#;
    std::fs::write(env.file(), hand).unwrap();

    let o = env.run(&["set", "other", "2"]);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("guards"), "{}", stderr(&o));
    assert_eq!(std::fs::read_to_string(env.file()).unwrap(), hand);

    ok(env.run(&["unset", "guards"]));
    assert_eq!(
        std::fs::read_to_string(env.file()).unwrap(),
        "{\n  \"keep\": 1\n}\n"
    );
    ok(env.run(&["set", "other", "2"]));
}

#[test]
fn unset_of_a_missing_key_reports_it() {
    let env = Env::new();
    ok(env.run(&["set", "keep", "1"]));
    let before = std::fs::read(env.file()).unwrap();
    let out = ok(env.run(&["unset", "absent.deep"]));
    assert!(
        out.contains("absent.deep") && out.contains("not set"),
        "{out}"
    );
    assert_eq!(std::fs::read(env.file()).unwrap(), before);
}

#[test]
fn a_malformed_file_makes_set_fail_without_changing_its_bytes() {
    let env = Env::new();
    std::fs::create_dir_all(env.file().parent().unwrap()).unwrap();
    for body in ["{not json", "[1,2]"] {
        std::fs::write(env.file(), body).unwrap();
        let o = env.run(&["set", "a", "1"]);
        assert!(!o.status.success(), "{body} accepted");
        assert!(stderr(&o).contains("config.json"), "{}", stderr(&o));
        assert_eq!(std::fs::read_to_string(env.file()).unwrap(), body);
    }
}

#[test]
fn a_set_keeps_performance_jsonl_bytes() {
    let env = Env::new();
    let dir = env.file().parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    let perf = dir.join("performance.jsonl");
    let body = b"{\"a\":1}\n{\"b\":2}\n";
    std::fs::write(&perf, body).unwrap();
    ok(env.run(&["set", "k", "v"]));
    ok(env.run(&["unset", "k"]));
    assert_eq!(std::fs::read(&perf).unwrap(), body);
}

#[test]
fn mcp_lists_a_read_only_bee_config_tool_that_returns_the_value() {
    let env = Env::new();
    ok(env.run(&["set", "herding.agents", r#"["claude"]"#]));

    let mut child = env
        .cmd()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut call = |id: u64, method: &str, params: Value| -> Value {
        let req = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(stdin, "{req}").unwrap();
        stdin.flush().unwrap();
        loop {
            let line = lines.next().expect("mcp closed stdout").unwrap();
            let msg: Value = serde_json::from_str(&line).unwrap();
            if msg.get("id") == Some(&json!(id)) {
                return msg;
            }
        }
    };

    let init = call(
        1,
        "initialize",
        json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}),
    );
    assert!(init.get("result").is_some(), "{init}");

    let list = call(2, "tools/list", json!({}));
    let tool = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "waggledance_bee_config")
        .cloned()
        .expect("waggledance_bee_config not listed");
    let actions: Vec<&str> = tool["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for write in ["set", "unset", "write", "delete", "put"] {
        assert!(!actions.contains(&write), "schema exposes {write}");
    }
    assert!(actions.contains(&"get"), "{actions:?}");

    let res = call(
        3,
        "tools/call",
        json!({"name": "waggledance_bee_config", "arguments": {"action": "get", "key": "herding.agents"}}),
    );
    let sc = &res["result"]["structuredContent"];
    assert_eq!(sc["found"], json!(true), "{res}");
    assert_eq!(sc["value"], json!(["claude"]), "{res}");

    let res = call(
        4,
        "tools/call",
        json!({"name": "waggledance_bee_config", "arguments": {"action": "set", "key": "x"}}),
    );
    assert!(
        res["result"]["isError"] == json!(true) || res.get("error").is_some(),
        "{res}"
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
