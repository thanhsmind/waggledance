use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum Target {
    Pi,
    Paseo,
}

const PASEO_ID: &str = "waggledance";

const PI_FILES: &[(&str, &str)] = &[
    (
        "index.ts",
        include_str!("../../../integrations/pi/index.ts"),
    ),
    (
        "installed.ts",
        include_str!("../../../integrations/pi/installed.ts"),
    ),
    (
        "package.json",
        include_str!("../../../integrations/pi/package.json"),
    ),
];

const PASEO_FILES: &[(&str, &str)] = &[
    (
        "paseo-plugin.json",
        include_str!("../../../integrations/paseo/paseo-plugin.json"),
    ),
    (
        "package.json",
        include_str!("../../../integrations/paseo/package.json"),
    ),
    (
        "tsconfig.json",
        include_str!("../../../integrations/paseo/tsconfig.json"),
    ),
    (
        "index.server.ts",
        include_str!("../../../integrations/paseo/index.server.ts"),
    ),
    (
        "index.client.tsx",
        include_str!("../../../integrations/paseo/index.client.tsx"),
    ),
    (
        "client/board.tsx",
        include_str!("../../../integrations/paseo/client/board.tsx"),
    ),
    (
        "client/settings.tsx",
        include_str!("../../../integrations/paseo/client/settings.tsx"),
    ),
    (
        "server/installed.ts",
        include_str!("../../../integrations/paseo/server/installed.ts"),
    ),
    (
        "server/waggledance.ts",
        include_str!("../../../integrations/paseo/server/waggledance.ts"),
    ),
    (
        "shared/board.ts",
        include_str!("../../../integrations/paseo/shared/board.ts"),
    ),
    (
        "shared/rpc.ts",
        include_str!("../../../integrations/paseo/shared/rpc.ts"),
    ),
];

type Files = BTreeMap<String, String>;
type Runner<'a> = &'a mut dyn FnMut(&[&str]) -> Result<String>;

fn files(table: &[(&str, &str)], installed: &str, exe: &Path) -> Files {
    let quoted = serde_json::to_string(&exe.to_string_lossy()).unwrap_or_default();
    let mut out: Files = table
        .iter()
        .map(|(name, body)| {
            let body = if *name == installed {
                body.replacen("\"\"", &quoted, 1)
            } else {
                body.to_string()
            };
            (name.to_string(), body)
        })
        .collect();
    out.insert("VERSION".into(), format!("{}\n", env!("CARGO_PKG_VERSION")));
    out
}

fn pi_files(exe: &Path) -> Files {
    files(PI_FILES, "installed.ts", exe)
}

fn paseo_files(exe: &Path) -> Files {
    files(PASEO_FILES, "server/installed.ts", exe)
}

fn read_tree(dir: &Path, base: &Path, out: &mut Files) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            read_tree(&path, base, out)?;
        } else {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let rel = rel.to_string_lossy().replace('\\', "/");
            out.insert(rel, std::fs::read_to_string(&path).unwrap_or_default());
        }
    }
    Ok(())
}

fn is_current(dst: &Path, want: &Files) -> bool {
    let mut have = Files::new();
    read_tree(dst, dst, &mut have).is_ok() && &have == want
}

fn refuse_foreign(dst: &Path) -> Result<()> {
    if dst.exists() && !dst.join("VERSION").is_file() {
        bail!(
            "{} was not written by waggledance setup (no VERSION file); move it away and rerun",
            dst.display()
        );
    }
    Ok(())
}

fn write_tree(dst: &Path, want: &Files) -> Result<bool> {
    if is_current(dst, want) {
        return Ok(false);
    }
    refuse_foreign(dst)?;
    let mut tmp = dst.as_os_str().to_owned();
    tmp.push(".waggledance-tmp");
    let tmp = PathBuf::from(tmp);
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).with_context(|| format!("clearing {}", tmp.display()))?;
    }
    for (name, body) in want {
        let path = tmp.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    }
    if dst.exists() {
        std::fs::remove_dir_all(dst).with_context(|| format!("replacing {}", dst.display()))?;
    }
    std::fs::rename(&tmp, dst).with_context(|| format!("moving into {}", dst.display()))?;
    Ok(true)
}

fn remove_tree(dst: &Path) -> Result<bool> {
    if !dst.exists() {
        return Ok(false);
    }
    refuse_foreign(dst)?;
    std::fs::remove_dir_all(dst).with_context(|| format!("removing {}", dst.display()))?;
    Ok(true)
}

fn pi_agent_dir() -> PathBuf {
    match std::env::var_os("PI_CODING_AGENT_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pi")
            .join("agent"),
    }
}

fn mcp_override(agent_dir: &Path) -> Option<String> {
    let path = agent_dir.join("mcp.json");
    let text = std::fs::read_to_string(&path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    let servers = parsed.get("mcpServers")?.as_object()?;
    let name = ["waggledance", "waggle_dance"]
        .into_iter()
        .find(|n| servers.contains_key(*n))?;
    Some(format!(
        "{} has an MCP server named {name}; it overrides this extension. Remove it: pi mcp remove {name}",
        path.display()
    ))
}

fn dev_build_warning(exe: &Path) -> Option<String> {
    let dev = exe
        .components()
        .any(|c| c.as_os_str() == "target" || c.as_os_str() == "cargo-target");
    dev.then(|| {
        format!(
            "{} is a build output path; the next cargo build overwrites it. Install the binary somewhere stable and rerun setup from there.",
            exe.display()
        )
    })
}

#[derive(serde::Deserialize)]
struct Listed {
    id: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    enabled: bool,
}

fn same_path(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

fn paseo_listed(run: Runner) -> Result<Option<Listed>> {
    let out = run(&["plugin", "ls", "--json"])?;
    let all: Vec<Listed> =
        serde_json::from_str(&out).context("reading `paseo plugin ls --json`")?;
    Ok(all.into_iter().find(|p| p.id == PASEO_ID))
}

fn paseo_install(dst: &Path, want: &Files, run: Runner) -> Result<Vec<String>> {
    let listed = paseo_listed(run)?;
    let wrote = write_tree(dst, want)?;
    let dst_str = dst.to_string_lossy();
    let cmds: Vec<Vec<&str>> = match &listed {
        Some(p) if !same_path(Path::new(&p.path), dst) => vec![
            vec!["plugin", "remove", PASEO_ID],
            vec!["plugin", "install", &dst_str],
        ],
        None => vec![vec!["plugin", "install", &dst_str]],
        Some(p) if !p.enabled => vec![vec!["plugin", "enable", PASEO_ID]],
        Some(_) if wrote => vec![vec!["plugin", "reload", PASEO_ID]],
        Some(_) => vec![],
    };
    let mut done = Vec::new();
    if wrote {
        done.push(format!("wrote {}", dst.display()));
    }
    if let Some(p) = listed
        .as_ref()
        .filter(|p| !same_path(Path::new(&p.path), dst))
    {
        done.push(format!("replaced the plugin installed from {}", p.path));
    }
    for cmd in cmds {
        run(&cmd)?;
        done.push(format!("ran paseo {}", cmd.join(" ")));
    }
    Ok(done)
}

fn paseo_remove(dst: &Path, run: Runner) -> Result<Vec<String>> {
    refuse_foreign(dst)?;
    let mut done = Vec::new();
    match paseo_listed(run)? {
        Some(p) if same_path(Path::new(&p.path), dst) => {
            run(&["plugin", "remove", PASEO_ID])?;
            done.push(format!("ran paseo plugin remove {PASEO_ID}"));
        }
        Some(p) => done.push(format!(
            "left the plugin installed from {} (not setup's): paseo plugin remove {PASEO_ID}",
            p.path
        )),
        None => {}
    }
    if remove_tree(dst)? {
        done.push(format!("removed {}", dst.display()));
    }
    Ok(done)
}

fn paseo_cli(args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("paseo")
        .args(args)
        .output()
        .context("running `paseo` (is Paseo installed and on PATH?)")?;
    if !out.status.success() {
        bail!(
            "paseo {} failed: {}{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim(),
            String::from_utf8_lossy(&out.stdout).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn run(target: Target, remove: bool, json: bool) -> Result<()> {
    let exe = std::env::current_exe().context("locating the waggledance binary")?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let mut warnings = Vec::new();
    let mut notes = Vec::new();
    let (dst, done) = match target {
        Target::Pi => {
            let agent = pi_agent_dir();
            let dst = agent.join("extensions").join("waggledance");
            let mut done = Vec::new();
            if remove {
                if remove_tree(&dst)? {
                    done.push(format!("removed {}", dst.display()));
                }
            } else {
                if write_tree(&dst, &pi_files(&exe))? {
                    done.push(format!("wrote {}", dst.display()));
                }
                notes.push(format!("the pi extension lives in {}", dst.display()));
                notes.push("running pi sessions pick it up after /reload".into());
                warnings.extend(mcp_override(&agent));
            }
            (dst, done)
        }
        Target::Paseo => {
            let dst = waggledance_core::config::data_dir().join("paseo");
            let mut paseo = |args: &[&str]| paseo_cli(args);
            let done = if remove {
                paseo_remove(&dst, &mut paseo)?
            } else {
                let done = paseo_install(&dst, &paseo_files(&exe), &mut paseo)?;
                notes.push("'Waggle Dance' appears in the Paseo sidebar".into());
                notes.push(
                    "turn on plugins in Paseo settings once if the sidebar does not show it".into(),
                );
                done
            };
            (dst, done)
        }
    };
    if !remove {
        warnings.extend(dev_build_warning(&exe));
    }
    let name = match target {
        Target::Pi => "pi",
        Target::Paseo => "paseo",
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "target": name, "path": dst, "removed": remove, "changed": !done.is_empty(),
                "actions": done, "notes": notes, "warnings": warnings
            })
        );
        return Ok(());
    }
    if done.is_empty() {
        println!("{name}: nothing changed ({})", dst.display());
    }
    for line in done.iter().chain(&notes) {
        println!("{name}: {line}");
    }
    for line in &warnings {
        println!("{name}: warning: {line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn exe() -> PathBuf {
        PathBuf::from("/opt/wd/bin/waggledance")
    }

    #[test]
    fn fresh_write_lays_down_every_file_and_version() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("paseo");
        let want = paseo_files(&exe());
        assert!(write_tree(&dst, &want).unwrap());
        let mut have = Files::new();
        read_tree(&dst, &dst, &mut have).unwrap();
        assert_eq!(have, want);
        assert_eq!(have["VERSION"].trim(), env!("CARGO_PKG_VERSION"));
        assert!(!dst.join("test").exists());
        assert!(!dst.join("node_modules").exists());
        assert!(!tmp.path().join("paseo.waggledance-tmp").exists());
    }

    #[test]
    fn rerun_with_nothing_changed_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("waggledance");
        let want = pi_files(&exe());
        assert!(write_tree(&dst, &want).unwrap());
        assert!(!write_tree(&dst, &want).unwrap());
        std::fs::write(dst.join("stray.ts"), "x").unwrap();
        assert!(write_tree(&dst, &want).unwrap());
        assert!(!dst.join("stray.ts").exists());
    }

    #[test]
    fn foreign_directory_is_refused_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("waggledance");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("mine.ts"), "keep").unwrap();
        let err = write_tree(&dst, &pi_files(&exe())).unwrap_err().to_string();
        assert!(err.contains(&dst.display().to_string()), "{err}");
        assert!(remove_tree(&dst).is_err());
        assert_eq!(
            std::fs::read_to_string(dst.join("mine.ts")).unwrap(),
            "keep"
        );
        assert!(!dst.join("VERSION").exists());
    }

    #[test]
    fn remove_deletes_only_a_versioned_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("waggledance");
        assert!(!remove_tree(&dst).unwrap());
        write_tree(&dst, &pi_files(&exe())).unwrap();
        assert!(remove_tree(&dst).unwrap());
        assert!(!dst.exists());
    }

    #[test]
    fn installed_ts_names_the_binary() {
        let pi = pi_files(&exe());
        assert_eq!(
            pi["installed.ts"].trim(),
            "export const waggledancePath = \"/opt/wd/bin/waggledance\";"
        );
        let paseo = paseo_files(&exe());
        assert_eq!(
            paseo["server/installed.ts"].trim(),
            "export const waggledancePath = \"/opt/wd/bin/waggledance\";"
        );
    }

    fn shipped(dir: &Path) -> BTreeSet<String> {
        let mut all = Files::new();
        read_tree(dir, dir, &mut all).unwrap();
        all.into_keys()
            .filter(|p| {
                !p.starts_with("node_modules/")
                    && !p.starts_with("test/")
                    && !p.ends_with(".test.ts")
                    && p != "package-lock.json"
                    && !p.starts_with('.')
            })
            .collect()
    }

    #[test]
    fn embedded_tables_match_the_integration_sources() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
        let names = |t: &[(&str, &str)]| t.iter().map(|(n, _)| n.to_string()).collect();
        let pi: BTreeSet<String> = names(PI_FILES);
        let paseo: BTreeSet<String> = names(PASEO_FILES);
        assert_eq!(pi, shipped(&root.join("pi")));
        assert_eq!(paseo, shipped(&root.join("paseo")));
    }

    fn install_calls(ls: &str, pre_write: bool) -> Vec<String> {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("paseo");
        let want = paseo_files(&exe());
        if pre_write {
            write_tree(&dst, &want).unwrap();
        }
        let ls = ls.replace("{dst}", &dst.to_string_lossy());
        let mut calls = Vec::new();
        let mut run = |args: &[&str]| -> Result<String> {
            calls.push(args.join(" "));
            Ok(if args[1] == "ls" {
                ls.clone()
            } else {
                String::new()
            })
        };
        paseo_install(&dst, &want, &mut run).unwrap();
        calls
            .into_iter()
            .filter(|c| c != "plugin ls --json")
            .map(|c| c.replace(&*dst.to_string_lossy(), "{dst}"))
            .collect()
    }

    #[test]
    fn paseo_install_decides_from_plugin_ls() {
        assert_eq!(install_calls("[]", false), ["plugin install {dst}"]);
        assert_eq!(
            install_calls(
                r#"[{"id":"waggledance","path":"/elsewhere","enabled":true}]"#,
                true
            ),
            ["plugin remove waggledance", "plugin install {dst}"]
        );
        assert_eq!(
            install_calls(
                r#"[{"id":"waggledance","path":"{dst}","enabled":false}]"#,
                true
            ),
            ["plugin enable waggledance"]
        );
        assert_eq!(
            install_calls(
                r#"[{"id":"waggledance","path":"{dst}","enabled":true}]"#,
                false
            ),
            ["plugin reload waggledance"]
        );
        assert!(install_calls(
            r#"[{"id":"other","path":"/x","enabled":true},{"id":"waggledance","path":"{dst}","enabled":true}]"#,
            true
        )
        .is_empty());
    }

    #[test]
    fn mcp_json_waggledance_server_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(mcp_override(tmp.path()).is_none());
        std::fs::write(
            tmp.path().join("mcp.json"),
            r#"{"mcpServers":{"waggle_dance":{"command":"waggledance","args":["mcp"]}}}"#,
        )
        .unwrap();
        let warning = mcp_override(tmp.path()).unwrap();
        assert!(warning.contains("pi mcp remove waggle_dance"), "{warning}");
    }

    #[test]
    fn dev_build_paths_are_flagged() {
        assert!(dev_build_warning(Path::new("/repo/target/fast/waggledance")).is_some());
        assert!(
            dev_build_warning(Path::new("/home/u/.cache/cargo-target/fast/waggledance")).is_some()
        );
        assert!(dev_build_warning(&exe()).is_none());
    }
}
