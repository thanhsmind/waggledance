use crate::bee;
use crate::domain::Project;
use crate::process::{run_with_deadline, RunError, RunOutput};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const SOURCE_MARKER: &str = "packages/bee/AGENTS.block.md";
const OUTPUT_CAP: usize = 16 * 1024 * 1024;
const MANAGED_PREFIXES: [&str; 4] = [
    ".claude/skills/bee-",
    ".agents/skills/bee-",
    ".opencode/skills/bee-",
    ".pi/extensions/bee-guard",
];

#[derive(Serialize, Debug, Clone)]
pub struct UpgradeOptions {
    pub apply: bool,
    pub only: Vec<String>,
    pub timeout: Duration,
}

impl Default for UpgradeOptions {
    fn default() -> Self {
        Self {
            apply: false,
            only: Vec::new(),
            timeout: Duration::from_secs(120),
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct BeeSource {
    pub root: PathBuf,
    pub bin: PathBuf,
}

#[derive(Serialize, Debug, Clone)]
pub struct UpgradeReport {
    pub source_root: PathBuf,
    pub source_head: Option<String>,
    pub source_branch: Option<String>,
    pub source_dirty: bool,
    pub target_version: Option<String>,
    pub applied: bool,
    pub projects: Vec<ProjectUpgrade>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProjectUpgrade {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub branch: Option<String>,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub notices: Vec<String>,
    pub outside_repo: Vec<String>,
    pub outcome: Outcome,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    UpToDate,
    Pending { files: Vec<String> },
    Upgraded { commit: String, files: Vec<String> },
    AppliedNotCommitted { files: Vec<String>, error: String },
    Skipped { reason: String, files: Vec<String> },
    Failed { error: String },
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum BeeUpgradeError {
    NoSource { looked_for: String },
    UnknownProjects(Vec<String>),
}

impl fmt::Display for BeeUpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSource { looked_for } => write!(
                f,
                "no registered project holds {looked_for} with an executable .bee/bin/bee"
            ),
            Self::UnknownProjects(ids) => write!(f, "unknown project id(s): {}", ids.join(", ")),
        }
    }
}

impl std::error::Error for BeeUpgradeError {}

pub fn find_source(projects: &[Project]) -> Option<BeeSource> {
    projects.iter().find_map(|p| {
        let root = &p.root_path;
        let bin = root.join(".bee/bin/bee");
        (root.join(SOURCE_MARKER).is_file() && is_executable(&bin)).then(|| BeeSource {
            root: root.clone(),
            bin,
        })
    })
}

pub fn run(projects: &[Project], opts: &UpgradeOptions) -> Result<UpgradeReport, BeeUpgradeError> {
    let source = find_source(projects).ok_or_else(|| BeeUpgradeError::NoSource {
        looked_for: SOURCE_MARKER.to_string(),
    })?;
    let unknown: Vec<String> = opts
        .only
        .iter()
        .filter(|id| !projects.iter().any(|p| &p.id == *id))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(BeeUpgradeError::UnknownProjects(unknown));
    }
    let rows: Vec<ProjectUpgrade> = projects
        .iter()
        .filter(|p| opts.only.is_empty() || opts.only.contains(&p.id))
        .map(|p| upgrade_project(p, &source, opts))
        .collect();
    let t = opts.timeout;
    Ok(UpgradeReport {
        source_head: git_text(&source.root, &["rev-parse", "--short", "HEAD"], t),
        source_branch: git_text(&source.root, &["symbolic-ref", "--short", "-q", "HEAD"], t),
        source_dirty: git(&source.root, &["status", "--porcelain"], None, t)
            .is_ok_and(|o| o.ok && !o.stdout.is_empty()),
        source_root: source.root,
        target_version: rows.iter().find_map(|r| r.to_version.clone()),
        applied: opts.apply,
        projects: rows,
    })
}

fn upgrade_project(p: &Project, source: &BeeSource, opts: &UpgradeOptions) -> ProjectUpgrade {
    let mut row = ProjectUpgrade {
        id: p.id.clone(),
        name: p.name.clone(),
        root: p.root_path.clone(),
        branch: None,
        from_version: None,
        to_version: None,
        notices: Vec::new(),
        outside_repo: Vec::new(),
        outcome: Outcome::UpToDate,
    };
    row.outcome = classify(&mut row, source, opts);
    row
}

fn skipped(reason: &str) -> Outcome {
    Outcome::Skipped {
        reason: reason.to_string(),
        files: Vec::new(),
    }
}

fn classify(row: &mut ProjectUpgrade, source: &BeeSource, opts: &UpgradeOptions) -> Outcome {
    let root = row.root.clone();
    let t = opts.timeout;
    if !root.is_dir() {
        return skipped("root missing");
    }
    let onboarding = root.join(".bee/onboarding.json");
    if !onboarding.is_file() {
        return skipped("no bee");
    }
    row.from_version = std::fs::read_to_string(&onboarding)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| str_field(&v, "bee_version"));
    if same_path(&root, &source.root) {
        return skipped("bee source");
    }
    match is_linked_worktree(&root, t) {
        None => return skipped("not a git repository"),
        Some(true) => return skipped("linked worktree; upgrades through its main checkout"),
        Some(false) => {}
    }
    row.branch = git_text(&root, &["symbolic-ref", "--short", "-q", "HEAD"], t);
    if row.branch.is_none() {
        return skipped("detached HEAD");
    }
    if bee::read_snapshot(&root).sessions.iter().any(|s| s.live) {
        return skipped("live bee session");
    }

    let plan = match onboard(source, &root, false, t) {
        Ok(plan) => plan,
        Err(error) => return Outcome::Failed { error },
    };
    row.to_version = str_field(&plan, "bee_version");
    row.notices = plan
        .get("notices")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    match plan.get("status").and_then(Value::as_str) {
        Some("up_to_date") => return Outcome::UpToDate,
        Some("changes_needed") => {}
        _ => {
            return Outcome::Failed {
                error: status_error(&plan),
            }
        }
    }
    let (files, outside) = expand_paths(&plan, &root);
    row.outside_repo = outside;
    if !opts.apply {
        return Outcome::Pending { files };
    }
    apply(&root, source, &files, row.to_version.as_deref(), t)
}

fn apply(
    root: &Path,
    source: &BeeSource,
    planned: &[String],
    version: Option<&str>,
    t: Duration,
) -> Outcome {
    match merge_or_rebase_in_progress(root, t) {
        Err(error) => return Outcome::Failed { error },
        Ok(true) => return skipped("merge or rebase in progress"),
        Ok(false) => {}
    }
    let before = match dirty_paths(root, t) {
        Ok(set) => set,
        Err(error) => return Outcome::Failed { error },
    };
    let (managed, other): (BTreeSet<String>, BTreeSet<String>) = before
        .iter()
        .filter(|p| under(p, planned))
        .cloned()
        .partition(|p| is_managed(p));
    if !other.is_empty() {
        return Outcome::Skipped {
            reason: "uncommitted edits in files bee would write".to_string(),
            files: other.into_iter().collect(),
        };
    }
    match onboard(source, root, true, t) {
        Ok(v) if v.get("status").and_then(Value::as_str) == Some("applied") => {}
        Ok(v) => {
            return Outcome::Failed {
                error: status_error(&v),
            }
        }
        Err(error) => return Outcome::Failed { error },
    }
    let after = match dirty_paths(root, t) {
        Ok(set) => set,
        Err(error) => return Outcome::Failed { error },
    };
    let files: Vec<String> = after
        .iter()
        .filter(|p| (!before.contains(*p) || managed.contains(*p)) && under(p, planned))
        .cloned()
        .collect();
    if files.is_empty() {
        return Outcome::UpToDate;
    }

    let pathspecs = files.join("\0").into_bytes();
    let message = match version {
        Some(v) => format!("Upgrade bee to {v}"),
        None => "Upgrade bee".to_string(),
    };
    let staged = git_checked(
        root,
        &[
            "--literal-pathspecs",
            "add",
            "--pathspec-from-file=-",
            "--pathspec-file-nul",
        ],
        pathspecs.clone(),
        t,
    );
    let committed = staged.and_then(|_| {
        git_checked(
            root,
            &[
                "--literal-pathspecs",
                "commit",
                "-q",
                "--only",
                "-m",
                &message,
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ],
            pathspecs,
            t,
        )
    });
    match committed {
        Ok(()) => Outcome::Upgraded {
            commit: git_text(root, &["rev-parse", "--short", "HEAD"], t).unwrap_or_default(),
            files,
        },
        Err(error) => Outcome::AppliedNotCommitted { files, error },
    }
}

fn onboard(source: &BeeSource, root: &Path, apply: bool, t: Duration) -> Result<Value, String> {
    let mut cmd = Command::new(&source.bin);
    cmd.current_dir(&source.root)
        .arg("onboard")
        .arg("--repo-root")
        .arg(root)
        .arg("--json");
    if apply {
        cmd.arg("--apply");
    }
    let out =
        run_with_deadline(&mut cmd, None, t, OUTPUT_CAP).map_err(|e| describe("bee onboard", e))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let parsed = text.find('{').and_then(|i| {
        serde_json::Deserializer::from_str(&text[i..])
            .into_iter::<Value>()
            .next()
            .and_then(Result::ok)
    });
    match (out.ok, parsed) {
        (true, Some(v)) => Ok(v),
        (false, Some(v)) => Err(status_error(&v)),
        (_, None) => Err(format!(
            "bee onboard printed no JSON: {}",
            first_lines(&out.stderr)
        )),
    }
}

fn status_error(v: &Value) -> String {
    let status = str_field(v, "status").unwrap_or_else(|| "unknown status".to_string());
    match str_field(v, "error").or_else(|| str_field(v, "reason")) {
        Some(detail) => format!("{status}: {detail}"),
        None => status,
    }
}

fn expand_paths(plan: &Value, root: &Path) -> (Vec<String>, Vec<String>) {
    let targets = plan
        .pointer("/skills/targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut files = BTreeSet::new();
    let mut outside = BTreeSet::new();
    for item in plan
        .get("plan")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(path) = item.get("path").and_then(Value::as_str) else {
            continue;
        };
        let action = item.get("action").and_then(Value::as_str).unwrap_or("");
        let expanded = if action == "sync_skill" || action == "remove_skill" {
            let target = item.get("target").and_then(Value::as_str);
            targets
                .iter()
                .find(|t| target.is_some() && t.get("kind").and_then(Value::as_str) == target)
                .and_then(|t| t.get("target_root").and_then(Value::as_str))
                .map(|target_root| Path::new(target_root).join(path))
        } else {
            Some(PathBuf::from(path))
        };
        match expanded.as_deref().and_then(|p| repo_relative(p, root)) {
            Some(rel) => files.insert(rel),
            None => outside.insert(
                expanded
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string()),
            ),
        };
    }
    (files.into_iter().collect(), outside.into_iter().collect())
}

fn repo_relative(path: &Path, root: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    if text.starts_with('~') || (text.contains(' ') && text.contains('(')) {
        return None;
    }
    let rel = if path.is_absolute() {
        let canonical = root.canonicalize().ok();
        path.strip_prefix(root)
            .ok()
            .or_else(|| canonical.as_deref().and_then(|c| path.strip_prefix(c).ok()))?
            .to_path_buf()
    } else {
        path.to_path_buf()
    };
    let rel = rel.to_string_lossy();
    let rel = rel.trim_start_matches("./").trim_end_matches('/');
    if rel.is_empty() || rel.split('/').any(|c| c == "..") {
        return None;
    }
    Some(rel.to_string())
}

fn is_managed(path: &str) -> bool {
    path == ".bee/onboarding.json" || MANAGED_PREFIXES.iter().any(|p| path.starts_with(p))
}

fn under(path: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|e| {
        path == e
            || path
                .strip_prefix(e.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

fn dirty_paths(root: &Path, t: Duration) -> Result<BTreeSet<String>, String> {
    let out = git(root, &["status", "--porcelain=v1", "-z", "-uall"], None, t)?;
    if !out.ok {
        return Err(format!("git status failed: {}", first_lines(&out.stderr)));
    }
    let mut paths = BTreeSet::new();
    let mut fields = out.stdout.split(|b| *b == 0);
    while let Some(entry) = fields.next() {
        if entry.len() < 4 {
            continue;
        }
        paths.insert(String::from_utf8_lossy(&entry[3..]).into_owned());
        if matches!(entry[0], b'R' | b'C') || matches!(entry[1], b'R' | b'C') {
            if let Some(origin) = fields.next() {
                paths.insert(String::from_utf8_lossy(origin).into_owned());
            }
        }
    }
    Ok(paths)
}

fn merge_or_rebase_in_progress(root: &Path, t: Duration) -> Result<bool, String> {
    let out = git(
        root,
        &[
            "rev-parse",
            "--git-path",
            "MERGE_HEAD",
            "--git-path",
            "rebase-merge",
            "--git-path",
            "rebase-apply",
        ],
        None,
        t,
    )?;
    if !out.ok {
        return Err(format!(
            "git rev-parse failed: {}",
            first_lines(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|line| root.join(line).exists()))
}

fn is_linked_worktree(root: &Path, t: Duration) -> Option<bool> {
    let out = git(
        root,
        &["rev-parse", "--git-dir", "--git-common-dir"],
        None,
        t,
    )
    .ok()
    .filter(|o| o.ok)?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut dirs = text.lines().map(|line| {
        let dir = root.join(line);
        dir.canonicalize().unwrap_or(dir)
    });
    let git_dir = dirs.next()?;
    let common_dir = dirs.next()?;
    Some(git_dir != common_dir)
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(String::from)
}

fn first_lines(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" / ")
}

fn describe(what: &str, e: RunError) -> String {
    match e {
        RunError::Spawn(msg) => format!("could not run {what}: {msg}"),
        RunError::Timeout(secs) => format!("{what} timed out after {secs}s"),
    }
}

fn git(
    root: &Path,
    args: &[&str],
    stdin: Option<Vec<u8>>,
    t: Duration,
) -> Result<RunOutput, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .env("GIT_OPTIONAL_LOCKS", "0");
    run_with_deadline(&mut cmd, stdin, t, OUTPUT_CAP).map_err(|e| describe("git", e))
}

fn git_checked(root: &Path, args: &[&str], stdin: Vec<u8>, t: Duration) -> Result<(), String> {
    let out = git(root, args, Some(stdin), t)?;
    if out.ok {
        Ok(())
    } else {
        let detail = first_lines(&out.stderr);
        Err(if detail.is_empty() {
            format!("git {} failed", args[1])
        } else {
            detail
        })
    }
}

fn git_text(root: &Path, args: &[&str], t: Duration) -> Option<String> {
    let out = git(root, args, None, t).ok().filter(|o| o.ok)?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    const FAKE_BEE: &str = r#"#!/bin/sh
echo "$*" >> calls.log
root="$3"
if [ -f "$root/.slow" ]; then sleep 5; fi
if [ "$5" = "--apply" ]; then
  mkdir -p "$root/.bee/expertise" "$root/.claude/skills/bee-hive"
  echo new > "$root/AGENTS.md"
  echo x > "$root/.bee/expertise/x.md"
  echo skill > "$root/.claude/skills/bee-hive/SKILL.md"
  if [ -f stray ]; then echo stray > "$root/stray.txt"; fi
  echo '{"status":"applied","bee_version":"9.9.9"}'
  exit 0
fi
echo "bee: preamble"
cat <<JSON
{"status":"changes_needed","bee_version":"9.9.9","plan":[{"action":"update_agents_block","path":"AGENTS.md"},{"action":"copy_expertise","path":".bee/expertise/x.md"},{"action":"sync_skill","path":"bee-hive","target":"repo-claude"}],"skills":{"targets":[{"kind":"repo-claude","target_root":"$root/.claude/skills"}]},"notices":["n1"]}
JSON
"#;

    const PLANNED: [&str; 3] = [
        ".bee/expertise/x.md",
        ".claude/skills/bee-hive",
        "AGENTS.md",
    ];
    const COMMITTED: [&str; 3] = [
        ".bee/expertise/x.md",
        ".claude/skills/bee-hive/SKILL.md",
        "AGENTS.md",
    ];

    fn project(id: &str, root: &Path) -> Project {
        Project {
            id: id.to_string(),
            name: id.to_string(),
            root_path: root.to_path_buf(),
            created_at: String::new(),
            last_seen_at: String::new(),
            orchestration_enabled: false,
        }
    }

    fn source(dir: &Path) -> Project {
        let root = dir.join("beehive");
        fs::create_dir_all(root.join("packages/bee")).unwrap();
        fs::create_dir_all(root.join(".bee/bin")).unwrap();
        fs::write(root.join(SOURCE_MARKER), "block").unwrap();
        fs::write(
            root.join(".bee/onboarding.json"),
            r#"{"bee_version":"9.9.9"}"#,
        )
        .unwrap();
        let bin = root.join(".bee/bin/bee");
        fs::write(&bin, FAKE_BEE).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        project("src", &root)
    }

    fn git_in(root: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn repo(dir: &Path, name: &str) -> Project {
        let root = dir.join(name);
        fs::create_dir_all(root.join(".bee")).unwrap();
        git_in(&root, &["init", "-q", "-b", "main"]);
        let hooks = root.join(".git/hooks");
        for (key, value) in [
            ("user.name", "Test"),
            ("user.email", "test@example.com"),
            ("commit.gpgsign", "false"),
            ("core.hooksPath", hooks.to_str().unwrap()),
        ] {
            git_in(&root, &["config", key, value]);
        }
        fs::write(
            root.join(".bee/onboarding.json"),
            r#"{"bee_version":"1.0.0"}"#,
        )
        .unwrap();
        fs::write(root.join("AGENTS.md"), "old\n").unwrap();
        fs::write(root.join("README.md"), "readme\n").unwrap();
        git_in(&root, &["add", "."]);
        git_in(&root, &["commit", "-q", "-m", "init"]);
        project(name, &root)
    }

    fn opts(apply: bool) -> UpgradeOptions {
        UpgradeOptions {
            apply,
            only: Vec::new(),
            timeout: Duration::from_secs(30),
        }
    }

    fn calls(src: &Project) -> String {
        fs::read_to_string(src.root_path.join("calls.log")).unwrap_or_default()
    }

    fn one(projects: &[Project], opts: &UpgradeOptions) -> ProjectUpgrade {
        let report = run(projects, opts).unwrap();
        assert_eq!(report.projects.len(), 1, "{report:?}");
        report.projects.into_iter().next().unwrap()
    }

    fn skipped_reason(outcome: &Outcome) -> &str {
        match outcome {
            Outcome::Skipped { reason, .. } => reason,
            other => panic!("expected skipped, got {other:?}"),
        }
    }

    fn head_files(root: &Path) -> Vec<String> {
        git_in(root, &["show", "--name-only", "--format=", "HEAD"])
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn find_source_picks_the_marked_root_and_none_without() {
        let tmp = TempDir::new().unwrap();
        let plain = project("plain", tmp.path());
        let src = source(tmp.path());
        let found = find_source(&[plain.clone(), src.clone()]).unwrap();
        assert_eq!(found.root, src.root_path);
        assert_eq!(found.bin, src.root_path.join(".bee/bin/bee"));
        assert!(find_source(&[plain]).is_none());
        fs::set_permissions(
            src.root_path.join(".bee/bin/bee"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(find_source(&[src]).is_none());
    }

    #[test]
    fn a_run_without_a_source_refuses_naming_the_marker() {
        let tmp = TempDir::new().unwrap();
        let err = run(&[repo(tmp.path(), "a")], &opts(false)).unwrap_err();
        assert_eq!(
            err,
            BeeUpgradeError::NoSource {
                looked_for: SOURCE_MARKER.to_string()
            }
        );
        assert!(err.to_string().contains(SOURCE_MARKER));
    }

    #[test]
    fn an_unknown_project_id_is_refused() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let options = UpgradeOptions {
            only: strings(&["nope"]),
            ..opts(false)
        };
        assert_eq!(
            run(&[src], &options).unwrap_err(),
            BeeUpgradeError::UnknownProjects(strings(&["nope"]))
        );
    }

    #[test]
    fn only_narrows_the_run_to_the_named_ids() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let b = repo(tmp.path(), "b");
        let options = UpgradeOptions {
            only: strings(&["b"]),
            ..opts(false)
        };
        let row = one(&[src, a, b], &options);
        assert_eq!(row.id, "b");
    }

    #[test]
    fn a_missing_root_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let gone = project("gone", &tmp.path().join("gone"));
        let row = one(
            &[src, gone],
            &UpgradeOptions {
                only: strings(&["gone"]),
                ..opts(false)
            },
        );
        assert_eq!(skipped_reason(&row.outcome), "root missing");
    }

    #[test]
    fn a_project_without_bee_is_skipped_and_onboard_never_runs() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let bare = tmp.path().join("bare");
        fs::create_dir_all(&bare).unwrap();
        let row = one(
            &[src.clone(), project("bare", &bare)],
            &UpgradeOptions {
                only: strings(&["bare"]),
                ..opts(true)
            },
        );
        assert_eq!(skipped_reason(&row.outcome), "no bee");
        assert_eq!(calls(&src), "");
        assert!(!bare.join(".bee").exists());
    }

    #[test]
    fn the_source_project_itself_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let row = one(std::slice::from_ref(&src), &opts(false));
        assert_eq!(skipped_reason(&row.outcome), "bee source");
        assert_eq!(calls(&src), "");
    }

    #[test]
    fn a_dry_run_lists_pending_paths_and_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let before = git_in(&a.root_path, &["status", "--porcelain=v1", "-uall"]);
        let report = run(
            &[src.clone(), a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(false)
            },
        )
        .unwrap();
        assert!(!report.applied);
        assert_eq!(report.target_version.as_deref(), Some("9.9.9"));
        assert_eq!(report.source_root, src.root_path);
        let row = &report.projects[0];
        assert_eq!(
            row.outcome,
            Outcome::Pending {
                files: strings(&PLANNED)
            }
        );
        assert_eq!(row.notices, strings(&["n1"]));
        assert_eq!(row.branch.as_deref(), Some("main"));
        assert_eq!(row.from_version.as_deref(), Some("1.0.0"));
        assert_eq!(row.to_version.as_deref(), Some("9.9.9"));
        assert!(row.outside_repo.is_empty());
        assert_eq!(
            git_in(&a.root_path, &["status", "--porcelain=v1", "-uall"]),
            before
        );
        assert_eq!(
            fs::read_to_string(a.root_path.join("AGENTS.md")).unwrap(),
            "old\n"
        );
        assert!(!calls(&src).contains("--apply"));
        let json = serde_json::to_value(&row.outcome).unwrap();
        assert_eq!(json["status"], "pending");
    }

    #[test]
    fn apply_on_a_clean_repo_commits_exactly_the_planned_files() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let row = one(
            &[src, a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        let Outcome::Upgraded { commit, files } = &row.outcome else {
            panic!("expected upgraded, got {:?}", row.outcome);
        };
        assert_eq!(files, &strings(&COMMITTED));
        assert_eq!(head_files(&a.root_path), strings(&COMMITTED));
        assert_eq!(
            commit,
            git_in(&a.root_path, &["rev-parse", "--short", "HEAD"]).trim()
        );
        assert_eq!(
            git_in(&a.root_path, &["log", "-1", "--format=%s"]).trim(),
            "Upgrade bee to 9.9.9"
        );
    }

    #[test]
    fn work_the_user_already_staged_is_never_committed() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        fs::write(a.root_path.join("README.md"), "mine\n").unwrap();
        git_in(&a.root_path, &["add", "README.md"]);
        let row = one(
            &[src, a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        assert!(
            matches!(row.outcome, Outcome::Upgraded { .. }),
            "{:?}",
            row.outcome
        );
        assert_eq!(head_files(&a.root_path), strings(&COMMITTED));
        assert_eq!(
            git_in(&a.root_path, &["diff", "--cached", "--name-only"]).trim(),
            "README.md"
        );
    }

    #[test]
    fn a_stray_file_outside_the_plan_is_not_committed() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        fs::write(src.root_path.join("stray"), "").unwrap();
        let a = repo(tmp.path(), "a");
        let row = one(
            &[src, a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        assert!(
            matches!(row.outcome, Outcome::Upgraded { .. }),
            "{:?}",
            row.outcome
        );
        assert_eq!(head_files(&a.root_path), strings(&COMMITTED));
        assert!(git_in(&a.root_path, &["status", "--porcelain=v1"]).contains("?? stray.txt"));
    }

    fn dirty_skill(root: &Path) {
        fs::create_dir_all(root.join(".claude/skills/bee-hive")).unwrap();
        fs::write(root.join(".claude/skills/bee-hive/SKILL.md"), "mine\n").unwrap();
    }

    #[test]
    fn an_uncommitted_bee_managed_file_is_overwritten_and_committed() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        dirty_skill(&a.root_path);
        let row = one(
            &[src, a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        let Outcome::Upgraded { files, .. } = &row.outcome else {
            panic!("expected upgraded, got {:?}", row.outcome);
        };
        assert_eq!(files, &strings(&COMMITTED));
        assert_eq!(head_files(&a.root_path), strings(&COMMITTED));
        assert_eq!(
            git_in(&a.root_path, &["status", "--porcelain=v1", "-uall"]),
            ""
        );
    }

    #[test]
    fn a_dry_run_leaves_the_git_index_untouched() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let old = std::time::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut roots = vec![a.root_path.clone()];
        let s = &src.root_path;
        git_in(s, &["init", "-q", "-b", "main"]);
        git_in(s, &["add", "packages"]);
        git_in(
            s,
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@e",
                "commit",
                "-q",
                "-m",
                "i",
            ],
        );
        roots.push(s.clone());
        let marks: Vec<_> = roots
            .iter()
            .map(|root| {
                fs::File::options()
                    .write(true)
                    .open(root.join(".git/index"))
                    .unwrap()
                    .set_modified(old)
                    .unwrap();
                root.join(".git/index")
            })
            .collect();
        fs::File::options()
            .write(true)
            .open(s.join(SOURCE_MARKER))
            .unwrap()
            .set_modified(std::time::SystemTime::now() + Duration::from_secs(60))
            .unwrap();
        run(
            &[src.clone(), a],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(false)
            },
        )
        .unwrap();
        for index in marks {
            assert_eq!(
                fs::metadata(&index).unwrap().modified().unwrap(),
                old,
                "{index:?}"
            );
        }
    }

    #[test]
    fn a_user_edit_beside_a_bee_managed_file_skips_naming_only_it() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        dirty_skill(&a.root_path);
        fs::write(a.root_path.join("AGENTS.md"), "mine\n").unwrap();
        let row = one(
            &[src.clone(), a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        assert_eq!(
            row.outcome,
            Outcome::Skipped {
                reason: "uncommitted edits in files bee would write".to_string(),
                files: strings(&["AGENTS.md"]),
            }
        );
        assert_eq!(
            fs::read_to_string(a.root_path.join(".claude/skills/bee-hive/SKILL.md")).unwrap(),
            "mine\n"
        );
        assert!(!calls(&src).contains("--apply"));
    }

    #[test]
    fn a_merge_in_progress_skips_the_apply() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let head = git_in(&a.root_path, &["rev-parse", "HEAD"]);
        fs::write(a.root_path.join(".git/MERGE_HEAD"), head).unwrap();
        let row = one(
            &[src.clone(), a],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        assert_eq!(skipped_reason(&row.outcome), "merge or rebase in progress");
        assert!(!calls(&src).contains("--apply"));
    }

    #[test]
    fn a_detached_head_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        git_in(&a.root_path, &["checkout", "-q", "--detach"]);
        let row = one(
            &[src, a],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(false)
            },
        );
        assert_eq!(skipped_reason(&row.outcome), "detached HEAD");
    }

    #[test]
    fn a_linked_worktree_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let linked = tmp.path().join("linked");
        git_in(
            &a.root_path,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "side",
                linked.to_str().unwrap(),
            ],
        );
        let row = one(
            &[src.clone(), project("linked", &linked)],
            &UpgradeOptions {
                only: strings(&["linked"]),
                ..opts(false)
            },
        );
        assert_eq!(
            skipped_reason(&row.outcome),
            "linked worktree; upgrades through its main checkout"
        );
        assert_eq!(calls(&src), "");
    }

    #[test]
    fn a_live_bee_session_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        fs::create_dir_all(a.root_path.join(".bee/sessions")).unwrap();
        fs::write(
            a.root_path.join(".bee/sessions/s1.json"),
            format!(r#"{{"id":"s1","last_heartbeat":"{now}"}}"#),
        )
        .unwrap();
        let row = one(
            &[src.clone(), a],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(false)
            },
        );
        assert_eq!(skipped_reason(&row.outcome), "live bee session");
        assert_eq!(calls(&src), "");
    }

    #[test]
    fn a_failing_commit_after_apply_is_applied_not_committed() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let a = repo(tmp.path(), "a");
        let hook = a.root_path.join(".git/hooks/pre-commit");
        fs::write(&hook, "#!/bin/sh\necho refused >&2\nexit 1\n").unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
        let row = one(
            &[src, a.clone()],
            &UpgradeOptions {
                only: strings(&["a"]),
                ..opts(true)
            },
        );
        let Outcome::AppliedNotCommitted { files, error } = &row.outcome else {
            panic!("expected applied_not_committed, got {:?}", row.outcome);
        };
        assert_eq!(files, &strings(&COMMITTED));
        assert!(error.contains("refused"), "{error}");
        assert_eq!(
            head_files(&a.root_path),
            strings(&[".bee/onboarding.json", "AGENTS.md", "README.md"])
        );
    }

    #[test]
    fn a_hung_onboard_fails_one_project_and_the_run_continues() {
        let tmp = TempDir::new().unwrap();
        let src = source(tmp.path());
        let slow = repo(tmp.path(), "slow");
        fs::write(slow.root_path.join(".slow"), "").unwrap();
        let next = repo(tmp.path(), "next");
        let options = UpgradeOptions {
            only: strings(&["slow", "next"]),
            timeout: Duration::from_secs(1),
            ..opts(false)
        };
        let report = run(&[src, slow, next], &options).unwrap();
        let Outcome::Failed { error } = &report.projects[0].outcome else {
            panic!("expected failed, got {:?}", report.projects[0].outcome);
        };
        assert!(error.contains("timed out"), "{error}");
        assert_eq!(
            report.projects[1].outcome,
            Outcome::Pending {
                files: strings(&PLANNED)
            }
        );
    }
}
