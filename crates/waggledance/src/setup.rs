use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum Target {
    Pi,
    Paseo,
    #[value(name = "openclaw")]
    OpenClaw,
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
        "client/composer.tsx",
        include_str!("../../../integrations/paseo/client/composer.tsx"),
    ),
    (
        "client/settings.tsx",
        include_str!("../../../integrations/paseo/client/settings.tsx"),
    ),
    (
        "server/dispatch.ts",
        include_str!("../../../integrations/paseo/server/dispatch.ts"),
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

const OPENCLAW_FILES: &[(&str, &str)] = &[
    (
        "SKILL.md",
        include_str!("../../../integrations/openclaw/waggledance-orchestrator/SKILL.md"),
    ),
    (
        "bin/wd-bridge.py",
        include_str!("../../../integrations/openclaw/waggledance-orchestrator/bin/wd-bridge.py"),
    ),
];

const SUPERVISOR_SKILL_FILES: &[(&str, &str)] = &[(
    "SKILL.md",
    include_str!("../../../docs/waggledance-supervisor-skill-template.md"),
)];

const HOOK_SESSION_KEY: &str = "hook:waggledance";

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

fn openclaw_files() -> Files {
    files(OPENCLAW_FILES, "", Path::new(""))
}

fn supervisor_skill_files() -> Files {
    files(SUPERVISOR_SKILL_FILES, "", Path::new(""))
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

struct OpenClawPaths {
    data: PathBuf,
    home: PathBuf,
    agent_skills: PathBuf,
}

impl OpenClawPaths {
    fn skill(&self) -> PathBuf {
        self.home
            .join("workspace-main")
            .join("skills")
            .join("waggledance-orchestrator")
    }

    fn supervisor_skill(&self) -> PathBuf {
        self.agent_skills.join("waggledance-supervisor")
    }

    fn backups(&self) -> PathBuf {
        self.data.join("backups")
    }

    fn json(&self) -> PathBuf {
        self.home.join("openclaw.json")
    }
}

fn expand_home(raw: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match raw.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if raw == "~" => home,
        None => PathBuf::from(raw),
    }
}

fn backup_name(name: &str, now: u64) -> String {
    format!("{name}.bak-{now}")
}

fn newest_backup(dir: &Path, name: &str) -> Option<PathBuf> {
    let prefix = format!("{name}.bak-");
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            let secs: u64 = file.strip_prefix(&prefix)?.parse().ok()?;
            Some((secs, e.path()))
        })
        .max_by_key(|(secs, _)| *secs)
        .map(|(_, path)| path)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn set_aside_foreign(dst: &Path, backups: &Path, now: u64) -> Result<Option<PathBuf>> {
    if !dst.exists() || dst.join("VERSION").is_file() {
        return Ok(None);
    }
    std::fs::create_dir_all(backups).with_context(|| format!("creating {}", backups.display()))?;
    let to = backups.join(backup_name(&file_name(dst), now));
    std::fs::rename(dst, &to)
        .with_context(|| format!("moving {} to {}", dst.display(), to.display()))?;
    Ok(Some(to))
}

fn install_skill(dst: &Path, want: &Files, backups: &Path, now: u64) -> Result<Vec<String>> {
    let mut done = Vec::new();
    if let Some(to) = set_aside_foreign(dst, backups, now)? {
        done.push(format!(
            "moved the existing {} to {}",
            dst.display(),
            to.display()
        ));
    }
    if write_tree(dst, want)? {
        done.push(format!("wrote {}", dst.display()));
    }
    Ok(done)
}

fn remove_skill(dst: &Path, backups: &Path) -> Result<Vec<String>> {
    let mut done = Vec::new();
    if remove_tree(dst)? {
        done.push(format!("removed {}", dst.display()));
    }
    if let Some(from) = newest_backup(backups, &file_name(dst)) {
        std::fs::rename(&from, dst)
            .with_context(|| format!("restoring {} from {}", dst.display(), from.display()))?;
        done.push(format!(
            "restored {} from {}",
            dst.display(),
            from.display()
        ));
    }
    Ok(done)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("marking {} executable", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_: &Path) -> Result<()> {
    Ok(())
}

fn edit_openclaw_json(path: &Path, token: &str, now: u64) -> Result<Option<PathBuf>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut doc: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let root = doc
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("{} is not a JSON object", path.display()))?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("`hooks` in {} is not an object", path.display()))?;
    let want = [
        ("enabled", serde_json::json!(true)),
        ("token", serde_json::json!(token)),
        ("defaultSessionKey", serde_json::json!(HOOK_SESSION_KEY)),
        ("allowRequestSessionKey", serde_json::json!(false)),
    ];
    if want.iter().all(|(k, v)| hooks.get(*k) == Some(v)) {
        return Ok(None);
    }
    for (k, v) in want {
        hooks.insert(k.into(), v);
    }
    let bak = path.with_file_name(backup_name(&file_name(path), now));
    waggledance_core::config::write_owner_only(&bak, text.as_bytes())
        .with_context(|| format!("backing up {} to {}", path.display(), bak.display()))?;
    let mut out = serde_json::to_string_pretty(&doc)?;
    if text.ends_with('\n') {
        out.push('\n');
    }
    std::fs::write(path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(Some(bak))
}

fn openclaw_install(p: &OpenClawPaths, now: u64) -> Result<Vec<String>> {
    let mut done = Vec::new();
    let token_path = waggledance_core::config::hook_token_path(&p.data);
    if waggledance_core::config::ensure_hook_token(&p.data)
        .with_context(|| format!("writing {}", token_path.display()))?
    {
        done.push(format!("wrote {}", token_path.display()));
    }
    let token = waggledance_core::config::read_hook_token(&p.data)
        .ok_or_else(|| anyhow::anyhow!("{} is empty; delete it and rerun", token_path.display()))?;
    let skill = p.skill();
    let wrote = install_skill(&skill, &openclaw_files(), &p.backups(), now)?;
    if !wrote.is_empty() {
        make_executable(&skill.join("bin").join("wd-bridge.py"))?;
    }
    done.extend(wrote);
    done.extend(install_skill(
        &p.supervisor_skill(),
        &supervisor_skill_files(),
        &p.backups(),
        now,
    )?);
    if let Some(bak) = edit_openclaw_json(&p.json(), &token, now)? {
        done.push(format!(
            "enabled hooks in {} (backup {})",
            p.json().display(),
            bak.display()
        ));
    }
    Ok(done)
}

fn openclaw_remove(p: &OpenClawPaths) -> Result<Vec<String>> {
    refuse_foreign(&p.skill())?;
    refuse_foreign(&p.supervisor_skill())?;
    let mut done = remove_skill(&p.skill(), &p.backups())?;
    done.extend(remove_skill(&p.supervisor_skill(), &p.backups())?);
    let json = p.json();
    if let Some(bak) = newest_backup(&p.home, &file_name(&json)) {
        std::fs::rename(&bak, &json)
            .with_context(|| format!("restoring {} from {}", json.display(), bak.display()))?;
        done.push(format!(
            "restored {} from {}",
            json.display(),
            bak.display()
        ));
    }
    let token = waggledance_core::config::hook_token_path(&p.data);
    if token.exists() {
        std::fs::remove_file(&token).with_context(|| format!("removing {}", token.display()))?;
        done.push(format!("removed {}", token.display()));
    }
    Ok(done)
}

fn openclaw_notes(hook_url: &str, token: &Path) -> Vec<String> {
    let curl = format!(
        "curl -s -o /dev/null -w '%{{http_code}}' -X POST {hook_url} -H 'Content-Type: application/json'"
    );
    vec![
        format!("check the hook refuses a call without the token (expect 401): {curl} -d '{{\"message\":\"x\"}}'"),
        format!(
            "check the hook reaches Telegram with the token (expect 200): {curl} -H \"Authorization: Bearer $(cat {})\" -d '{{\"message\":\"Chuyển nguyên văn cho người dùng: [waggledance] hook online\",\"deliver\":true,\"channel\":\"telegram\"}}'",
            token.display()
        ),
        "the OpenClaw gateway may need a restart to pick up `hooks`; setup never restarts it, restart it yourself".into(),
    ]
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
    let all: Vec<Listed> = crate::paseo::parse_document(&out)
        .map_err(|e| anyhow::anyhow!("reading `paseo plugin ls --json`: {e}"))?;
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

fn paseo_unneeded_lines(config: &waggledance_core::config::PaseoConfig) -> Vec<String> {
    let file = "~/.waggledance/config.toml";
    let mut lines = Vec::new();
    if config.enabled.is_some() {
        lines.push(format!(
            "`enabled` under [paseo] in {file} is no longer needed: Paseo support turns on when a \
             working CLI is found; a project accepts dispatched work with `orchestration = true` \
             in its own .waggledance.toml"
        ));
    }
    if !config.binary.is_empty() {
        lines.push(format!(
            "`binary` under [paseo] in {file} is no longer needed: waggledance finds the Paseo \
             CLI itself; keep it only to override that choice"
        ));
    }
    if !config.leaders.is_empty() {
        lines.push(format!(
            "[[paseo.leaders]] in {file} is deprecated: name the leader's workspace with \
             `workspace = \"<id>\"` under [paseo] in the project's .waggledance.toml"
        ));
    }
    lines
}

fn paseo_cli(binary: &str, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new(binary)
        .args(args)
        .env_remove("PASEO_AGENT_ID")
        .env_remove("PASEO_AGENT_CWD")
        .output()
        .with_context(|| format!("running `{binary}`"))?;
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
            let config = waggledance_core::Config::load();
            let found = crate::paseo::detect_binary(&config.paseo);
            let Some(binary) = found.binary.clone() else {
                bail!("{}", found.unavailable());
            };
            notes.push(format!("using the Paseo CLI {binary}"));
            notes.extend(
                found
                    .rejected
                    .iter()
                    .map(|(candidate, why)| format!("skipped Paseo CLI {candidate}: {why}")),
            );
            notes.extend(found.notes);
            warnings.extend(paseo_unneeded_lines(&config.paseo));
            let mut paseo = |args: &[&str]| paseo_cli(&binary, args);
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
        Target::OpenClaw => {
            let config = waggledance_core::Config::load();
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let paths = OpenClawPaths {
                data: waggledance_core::config::resolve_data_dir(None),
                home: expand_home(&config.openclaw.home),
                agent_skills: home.join(".agents").join("skills"),
            };
            let done = if remove {
                openclaw_remove(&paths)?
            } else {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or_default();
                let done = openclaw_install(&paths, now)?;
                notes.extend(openclaw_notes(
                    &config.openclaw.hook_url,
                    &waggledance_core::config::hook_token_path(&paths.data),
                ));
                done
            };
            (paths.skill(), done)
        }
    };
    if !remove {
        warnings.extend(dev_build_warning(&exe));
    }
    let name = match target {
        Target::Pi => "pi",
        Target::Paseo => "paseo",
        Target::OpenClaw => "openclaw",
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
                    && !p.contains("__pycache__/")
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
        let openclaw: BTreeSet<String> = names(OPENCLAW_FILES);
        assert_eq!(
            openclaw,
            shipped(&root.join("openclaw").join("waggledance-orchestrator"))
        );
    }

    #[test]
    fn embedded_openclaw_files_equal_the_repo_files() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let skill = repo.join("integrations/openclaw/waggledance-orchestrator");
        for (name, body) in OPENCLAW_FILES {
            assert_eq!(
                std::fs::read_to_string(skill.join(name)).unwrap(),
                *body,
                "{name}"
            );
        }
        assert_eq!(
            std::fs::read_to_string(repo.join("docs/waggledance-supervisor-skill-template.md"))
                .unwrap(),
            SUPERVISOR_SKILL_FILES[0].1
        );
    }

    const OPENCLAW_JSON: &str = r#"{
  "meta": {
    "lastTouchedVersion": "2026.3.13"
  },
  "hooks": {
    "internal": {
      "enabled": true,
      "entries": {
        "session-memory": {
          "enabled": true
        }
      }
    }
  },
  "channels": {
    "telegram": {
      "botToken": "keep-me"
    }
  },
  "zUnknown": [1, 2]
}
"#;

    fn openclaw_paths(root: &Path) -> OpenClawPaths {
        let p = OpenClawPaths {
            data: root.join("data"),
            home: root.join("openclaw"),
            agent_skills: root.join("agents").join("skills"),
        };
        std::fs::create_dir_all(&p.home).unwrap();
        std::fs::write(p.json(), OPENCLAW_JSON).unwrap();
        p
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn openclaw_json_edit_keeps_every_other_key_and_backs_up_first() {
        let tmp = tempfile::tempdir().unwrap();
        let p = openclaw_paths(tmp.path());
        let bak = edit_openclaw_json(&p.json(), "tok", 100).unwrap().unwrap();
        assert_eq!(bak, p.home.join("openclaw.json.bak-100"));
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), OPENCLAW_JSON);
        #[cfg(unix)]
        assert_eq!(mode(&bak), 0o600);
        let text = std::fs::read_to_string(p.json()).unwrap();
        assert!(text.ends_with("}\n"));
        assert!(text.contains("\n  \"meta\": {\n    \"lastTouchedVersion\""));
        let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
        let before: serde_json::Value = serde_json::from_str(OPENCLAW_JSON).unwrap();
        assert_eq!(doc["hooks"]["internal"], before["hooks"]["internal"]);
        assert_eq!(doc["channels"], before["channels"]);
        assert_eq!(doc["meta"], before["meta"]);
        assert_eq!(doc["zUnknown"], before["zUnknown"]);
        assert_eq!(doc["hooks"]["enabled"], true);
        assert_eq!(doc["hooks"]["token"], "tok");
        assert_eq!(doc["hooks"]["defaultSessionKey"], "hook:waggledance");
        assert_eq!(doc["hooks"]["allowRequestSessionKey"], false);
        let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["meta", "hooks", "channels", "zUnknown"]);
        assert!(edit_openclaw_json(&p.json(), "tok", 200).unwrap().is_none());
        assert!(!p.home.join("openclaw.json.bak-200").exists());
        assert_eq!(std::fs::read_to_string(p.json()).unwrap(), text);
    }

    #[test]
    fn hook_token_is_owner_only_and_never_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        let path = waggledance_core::config::hook_token_path(&data);
        assert!(waggledance_core::config::ensure_hook_token(&data).unwrap());
        let token = waggledance_core::config::read_hook_token(&data).unwrap();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        #[cfg(unix)]
        assert_eq!(mode(&path), 0o600);
        assert!(!waggledance_core::config::ensure_hook_token(&data).unwrap());
        assert_eq!(
            waggledance_core::config::read_hook_token(&data).unwrap(),
            token
        );
        std::fs::write(&path, "mine\n").unwrap();
        assert!(!waggledance_core::config::ensure_hook_token(&data).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine\n");
    }

    #[test]
    fn openclaw_install_then_remove_round_trips_the_host() {
        let tmp = tempfile::tempdir().unwrap();
        let p = openclaw_paths(tmp.path());
        std::fs::create_dir_all(p.skill().join("bin")).unwrap();
        std::fs::write(p.skill().join("SKILL.md"), "old skill").unwrap();
        std::fs::create_dir_all(p.supervisor_skill()).unwrap();
        std::fs::write(p.supervisor_skill().join("SKILL.md"), "old supervisor").unwrap();

        let done = openclaw_install(&p, 100).unwrap();
        assert!(done[0].contains("hook.token"), "{done:?}");
        let token = waggledance_core::config::read_hook_token(&p.data).unwrap();
        assert!(is_current(&p.skill(), &openclaw_files()));
        assert!(is_current(&p.supervisor_skill(), &supervisor_skill_files()));
        #[cfg(unix)]
        assert_eq!(mode(&p.skill().join("bin/wd-bridge.py")), 0o755);
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(p.json()).unwrap()).unwrap();
        assert_eq!(doc["hooks"]["token"], token.as_str());
        assert!(p
            .backups()
            .join("waggledance-orchestrator.bak-100")
            .is_dir());
        assert!(p.backups().join("waggledance-supervisor.bak-100").is_dir());
        assert!(openclaw_install(&p, 200).unwrap().is_empty());

        openclaw_remove(&p).unwrap();
        assert_eq!(std::fs::read_to_string(p.json()).unwrap(), OPENCLAW_JSON);
        assert!(!waggledance_core::config::hook_token_path(&p.data).exists());
        assert_eq!(
            std::fs::read_to_string(p.skill().join("SKILL.md")).unwrap(),
            "old skill"
        );
        assert_eq!(
            std::fs::read_to_string(p.supervisor_skill().join("SKILL.md")).unwrap(),
            "old supervisor"
        );
        assert!(!p.home.join("openclaw.json.bak-100").exists());
    }

    #[test]
    fn openclaw_remove_leaves_a_foreign_skill_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let p = openclaw_paths(tmp.path());
        std::fs::create_dir_all(p.skill()).unwrap();
        std::fs::write(p.skill().join("SKILL.md"), "mine").unwrap();
        assert!(openclaw_remove(&p).is_err());
        assert_eq!(
            std::fs::read_to_string(p.skill().join("SKILL.md")).unwrap(),
            "mine"
        );
    }

    #[test]
    fn openclaw_notes_carry_both_curl_checks_and_the_restart() {
        let notes = openclaw_notes(
            "http://127.0.0.1:18789/hooks/agent",
            Path::new("/d/hook.token"),
        );
        assert!(notes[0].contains("expect 401"));
        assert!(notes[0].contains(
            "curl -s -o /dev/null -w '%{http_code}' -X POST http://127.0.0.1:18789/hooks/agent -H 'Content-Type: application/json' -d '{\"message\":\"x\"}'"
        ));
        assert!(notes[1].contains("expect 200"));
        assert!(notes[1].contains("-H \"Authorization: Bearer $(cat /d/hook.token)\""));
        assert!(notes[1].contains(r#""deliver":true,"channel":"telegram""#));
        assert!(notes[2].contains("restart"));
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
    fn paseo_setup_reads_plugin_ls_past_the_wrappers_log_lines() {
        for out in [
            include_str!("../testdata/fixtures/paseo-0.11.2/wrapper-plugin-ls.out"),
            include_str!("../testdata/fixtures/paseo-0.11.2/npm-plugin-ls.out"),
        ] {
            let mut run = |_: &[&str]| -> Result<String> { Ok(out.to_string()) };
            let listed = paseo_listed(&mut run).unwrap().unwrap();
            assert_eq!(listed.path, "/home/thanhsmind/.waggledance/paseo");
            assert!(listed.enabled);
        }
        let mut noise =
            |_: &[&str]| -> Result<String> { Ok("23:51:21.452 › only noise\n".into()) };
        let err = paseo_listed(&mut noise).err().unwrap().to_string();
        assert!(err.contains("plugin ls"), "{err}");
    }

    #[test]
    fn paseo_setup_names_global_lines_that_are_no_longer_needed() {
        use waggledance_core::config::{PaseoConfig, PaseoLeader};
        assert!(paseo_unneeded_lines(&PaseoConfig::default()).is_empty());
        let old = PaseoConfig {
            enabled: Some(true),
            binary: "/home/u/.local/bin/paseo".into(),
            leaders: vec![PaseoLeader {
                project: "p".into(),
                workspace: "wks_a".into(),
            }],
            ..PaseoConfig::default()
        };
        let lines = paseo_unneeded_lines(&old);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(
            lines[0].contains("orchestration = true") && lines[0].contains(".waggledance.toml")
        );
        assert!(lines[1].contains("`binary`"));
        assert!(
            lines[2].contains("[[paseo.leaders]]") && lines[2].contains("workspace = \"<id>\"")
        );
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
