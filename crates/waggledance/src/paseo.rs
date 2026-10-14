use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use waggledance_core::config::PaseoConfig;
use waggledance_core::domain::Project;

pub const ROLE_LABEL: &str = "waggledance.role=leader";
pub const PROJECT_LABEL_KEY: &str = "waggledance.project";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, thiserror::Error)]
pub enum PaseoError {
    #[error("paseo binary `{binary}` could not be started: {message}")]
    Spawn { binary: String, message: String },
    #[error("`{binary} {command}` timed out after {secs}s")]
    Timeout {
        binary: String,
        command: String,
        secs: u64,
    },
    #[error("`{binary} {command}` exited with {status}")]
    Failed {
        binary: String,
        command: String,
        status: String,
    },
    #[error("`{binary} {command}` printed output that could not be parsed: {message}")]
    Unparsable {
        binary: String,
        command: String,
        message: String,
    },
    #[error(
        "project {project} has more than one local Paseo workspace at its root ({ids}); \
         name one with `[[paseo.leaders]]` project = \"{project}\" and workspace = \"<id>\" \
         in ~/.waggledance/config.toml"
    )]
    AmbiguousWorkspace { project: String, ids: String },
    #[error("paseo leader lock failed: {0}")]
    Lock(String),
    #[error("paseo prompt file failed: {0}")]
    PromptFile(String),
}

pub type Result<T> = std::result::Result<T, PaseoError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentState {
    pub status: String,
    pub archived: bool,
    pub pending_permissions: usize,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Workspace {
    #[serde(rename = "workspaceId")]
    pub id: String,
    #[serde(default)]
    pub isolation: String,
    #[serde(default)]
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSpec {
    pub workspace: String,
    pub cwd: PathBuf,
    pub title: String,
    pub labels: Vec<String>,
    pub provider: String,
    pub mode: String,
    pub thinking: String,
    pub task: String,
}

#[async_trait]
pub trait Paseo: Send + Sync {
    async fn leaders(&self, project_id: &str) -> Result<Vec<String>>;
    async fn inspect(&self, id: &str) -> Result<AgentState>;
    async fn workspaces(&self) -> Result<Vec<Workspace>>;
    async fn create_workspace(&self, path: &Path) -> Result<String>;
    async fn run(&self, spec: &RunSpec) -> Result<String>;
    async fn send(&self, id: &str, text: &str) -> Result<()>;
    async fn logs_since(&self, id: &str, since: &str) -> Result<String>;
}

pub fn project_label(project_id: &str) -> String {
    format!("{PROJECT_LABEL_KEY}={project_id}")
}

pub fn leaders_args(project_id: &str) -> Vec<String> {
    vec![
        "ls".into(),
        "-g".into(),
        "--label".into(),
        ROLE_LABEL.into(),
        "--label".into(),
        project_label(project_id),
        "--json".into(),
    ]
}

pub fn run_args(spec: &RunSpec) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "-q".into(),
        "--workspace".into(),
        spec.workspace.clone(),
        "--cwd".into(),
        spec.cwd.to_string_lossy().into_owned(),
        "--title".into(),
        spec.title.clone(),
    ];
    for label in &spec.labels {
        args.push("--label".into());
        args.push(label.clone());
    }
    for (flag, value) in [
        ("--provider", &spec.provider),
        ("--mode", &spec.mode),
        ("--thinking", &spec.thinking),
    ] {
        if !value.is_empty() {
            args.push(flag.into());
            args.push(value.clone());
        }
    }
    args.push("--".into());
    args.push(spec.task.clone());
    args
}

pub struct PaseoCli {
    binary: String,
}

impl PaseoCli {
    pub fn new(config: &PaseoConfig) -> Self {
        Self {
            binary: config.binary().to_string(),
        }
    }

    async fn exec(&self, args: &[String]) -> Result<String> {
        let command = args.first().cloned().unwrap_or_default();
        let child = tokio::process::Command::new(&self.binary)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| PaseoError::Spawn {
                binary: self.binary.clone(),
                message: e.to_string(),
            })?;
        let output = tokio::time::timeout(TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| PaseoError::Timeout {
                binary: self.binary.clone(),
                command: command.clone(),
                secs: TIMEOUT.as_secs(),
            })?
            .map_err(|e| PaseoError::Spawn {
                binary: self.binary.clone(),
                message: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(PaseoError::Failed {
                binary: self.binary.clone(),
                command,
                status: output.status.to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn unparsable(&self, command: &str, message: impl ToString) -> PaseoError {
        PaseoError::Unparsable {
            binary: self.binary.clone(),
            command: command.to_string(),
            message: message.to_string(),
        }
    }

    fn first_id(&self, command: &str, stdout: &str) -> Result<String> {
        stdout
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
            .ok_or_else(|| self.unparsable(command, "no id on stdout"))
    }
}

#[derive(Deserialize)]
struct ListedAgent {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspected {
    status: String,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    pending_permissions: Vec<serde_json::Value>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[async_trait]
impl Paseo for PaseoCli {
    async fn leaders(&self, project_id: &str) -> Result<Vec<String>> {
        let out = self.exec(&leaders_args(project_id)).await?;
        let agents: Vec<ListedAgent> =
            serde_json::from_str(&out).map_err(|e| self.unparsable("ls", e))?;
        Ok(agents.into_iter().map(|a| a.id).collect())
    }

    async fn inspect(&self, id: &str) -> Result<AgentState> {
        let out = self
            .exec(&["inspect".into(), id.into(), "--json".into()])
            .await?;
        let i: Inspected = serde_json::from_str(&out).map_err(|e| self.unparsable("inspect", e))?;
        Ok(AgentState {
            status: i.status,
            archived: i.archived,
            pending_permissions: i.pending_permissions.len(),
            updated_at: i.updated_at,
        })
    }

    async fn workspaces(&self) -> Result<Vec<Workspace>> {
        let out = self
            .exec(&["workspace".into(), "ls".into(), "--json".into()])
            .await?;
        serde_json::from_str(&out).map_err(|e| self.unparsable("workspace", e))
    }

    async fn create_workspace(&self, path: &Path) -> Result<String> {
        let out = self
            .exec(&[
                "workspace".into(),
                "create".into(),
                "--path".into(),
                path.to_string_lossy().into_owned(),
                "--isolation".into(),
                "local".into(),
                "-q".into(),
            ])
            .await?;
        self.first_id("workspace", &out)
    }

    async fn run(&self, spec: &RunSpec) -> Result<String> {
        let out = self.exec(&run_args(spec)).await?;
        self.first_id("run", &out)
    }

    async fn send(&self, id: &str, text: &str) -> Result<()> {
        let file = PromptFile::write(text)?;
        self.exec(&[
            "send".into(),
            id.into(),
            "--prompt-file".into(),
            file.0.to_string_lossy().into_owned(),
            "--no-wait".into(),
        ])
        .await
        .map(|_| ())
    }

    async fn logs_since(&self, id: &str, since: &str) -> Result<String> {
        self.exec(&[
            "logs".into(),
            id.into(),
            "--since".into(),
            since.into(),
            "--filter".into(),
            "text".into(),
        ])
        .await
    }
}

struct PromptFile(PathBuf);

impl PromptFile {
    fn write(text: &str) -> Result<Self> {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!(
            "waggledance-paseo-prompt-{}-{:016x}.txt",
            std::process::id(),
            rand::random::<u64>()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|e| PaseoError::PromptFile(format!("{}: {e}", path.display())))?;
        let guard = Self(path);
        file.write_all(text.as_bytes())
            .map_err(|e| PaseoError::PromptFile(format!("{}: {e}", guard.0.display())))?;
        Ok(guard)
    }
}

impl Drop for PromptFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[derive(Debug)]
pub struct LeaderLock {
    _file: std::fs::File,
}

impl LeaderLock {
    async fn acquire(data_dir: &Path, project_id: &str) -> Result<Self> {
        let dir = data_dir.join("paseo-locks");
        let name: String = project_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = dir.join(format!("{name}.lock"));
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&dir).map_err(|e| PaseoError::Lock(e.to_string()))?;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path)
                .map_err(|e| PaseoError::Lock(format!("{}: {e}", path.display())))?;
            file.lock()
                .map_err(|e| PaseoError::Lock(format!("{}: {e}", path.display())))?;
            Ok(Self { _file: file })
        })
        .await
        .map_err(|e| PaseoError::Lock(e.to_string()))?
    }
}

#[derive(Debug)]
pub enum Leader {
    Existing { id: String, busy: bool },
    Start(Start),
}

#[derive(Debug)]
pub struct Start {
    pub workspace: String,
    pub cwd: PathBuf,
    pub title: String,
    pub labels: Vec<String>,
    provider: String,
    mode: String,
    thinking: String,
    _lock: LeaderLock,
}

impl Start {
    pub async fn run(self, paseo: &dyn Paseo, task: &str) -> Result<String> {
        let spec = RunSpec {
            workspace: self.workspace.clone(),
            cwd: self.cwd.clone(),
            title: self.title.clone(),
            labels: self.labels.clone(),
            provider: self.provider.clone(),
            mode: self.mode.clone(),
            thinking: self.thinking.clone(),
            task: task.to_string(),
        };
        paseo.run(&spec).await
    }
}

fn realpath(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub async fn resolve_leader(
    paseo: &dyn Paseo,
    config: &PaseoConfig,
    project: &Project,
    data_dir: &Path,
) -> Result<Leader> {
    let lock = LeaderLock::acquire(data_dir, &project.id).await?;
    for id in paseo.leaders(&project.id).await? {
        let state = paseo.inspect(&id).await?;
        if !state.archived {
            let busy = matches!(state.status.as_str(), "running" | "initializing");
            return Ok(Leader::Existing { id, busy });
        }
    }
    let workspace = match config.workspace_for(&project.id, &project.name) {
        Some(w) => w.to_string(),
        None => {
            let root = realpath(&project.root_path);
            let matches: Vec<String> = paseo
                .workspaces()
                .await?
                .into_iter()
                .filter(|w| w.isolation == "local" && realpath(Path::new(&w.cwd)) == root)
                .map(|w| w.id)
                .collect();
            match matches.as_slice() {
                [] => paseo.create_workspace(&project.root_path).await?,
                [one] => one.clone(),
                _ => {
                    return Err(PaseoError::AmbiguousWorkspace {
                        project: project.name.clone(),
                        ids: matches.join(", "),
                    })
                }
            }
        }
    };
    Ok(Leader::Start(Start {
        workspace,
        cwd: project.root_path.clone(),
        title: format!("{} leader", project.name),
        labels: vec![ROLE_LABEL.to_string(), project_label(&project.id)],
        provider: config.provider.clone(),
        mode: config.mode.clone(),
        thinking: config.thinking.clone(),
        _lock: lock,
    }))
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Call {
        Leaders(String),
        Inspect(String),
        Workspaces,
        CreateWorkspace(PathBuf),
        Run(RunSpec),
        Send(String, String),
        LogsSince(String, String),
    }

    #[derive(Default)]
    pub struct FakeState {
        pub leaders: HashMap<String, Vec<String>>,
        pub agents: HashMap<String, AgentState>,
        pub workspaces: Vec<Workspace>,
        pub logs: HashMap<String, String>,
        pub calls: Vec<Call>,
    }

    #[derive(Default)]
    pub struct FakePaseo {
        pub state: Mutex<FakeState>,
    }

    impl FakePaseo {
        pub fn calls(&self) -> Vec<Call> {
            self.state.lock().unwrap().calls.clone()
        }

        pub fn add_leader(&self, project_id: &str, id: &str, status: &str, archived: bool) {
            let mut s = self.state.lock().unwrap();
            s.leaders
                .entry(project_id.to_string())
                .or_default()
                .push(id.to_string());
            s.agents.insert(
                id.to_string(),
                AgentState {
                    status: status.to_string(),
                    archived,
                    pending_permissions: 0,
                    updated_at: None,
                },
            );
        }

        pub fn add_workspace(&self, id: &str, isolation: &str, cwd: &Path) {
            self.state.lock().unwrap().workspaces.push(Workspace {
                id: id.to_string(),
                isolation: isolation.to_string(),
                cwd: cwd.to_string_lossy().into_owned(),
            });
        }

        fn record(&self, call: Call) {
            self.state.lock().unwrap().calls.push(call);
        }
    }

    #[async_trait]
    impl Paseo for FakePaseo {
        async fn leaders(&self, project_id: &str) -> Result<Vec<String>> {
            self.record(Call::Leaders(project_id.to_string()));
            tokio::task::yield_now().await;
            let s = self.state.lock().unwrap();
            Ok(s.leaders.get(project_id).cloned().unwrap_or_default())
        }

        async fn inspect(&self, id: &str) -> Result<AgentState> {
            self.record(Call::Inspect(id.to_string()));
            let s = self.state.lock().unwrap();
            s.agents.get(id).cloned().ok_or_else(|| PaseoError::Failed {
                binary: "fake".into(),
                command: "inspect".into(),
                status: "no such agent".into(),
            })
        }

        async fn workspaces(&self) -> Result<Vec<Workspace>> {
            self.record(Call::Workspaces);
            Ok(self.state.lock().unwrap().workspaces.clone())
        }

        async fn create_workspace(&self, path: &Path) -> Result<String> {
            self.record(Call::CreateWorkspace(path.to_path_buf()));
            let id = format!("wks_new{}", self.state.lock().unwrap().workspaces.len());
            self.add_workspace(&id, "local", path);
            Ok(id)
        }

        async fn run(&self, spec: &RunSpec) -> Result<String> {
            self.record(Call::Run(spec.clone()));
            tokio::task::yield_now().await;
            let id = format!("agent-{}", self.state.lock().unwrap().agents.len() + 1);
            if let Some(project) = spec
                .labels
                .iter()
                .find_map(|l| l.strip_prefix(&format!("{PROJECT_LABEL_KEY}=")))
            {
                self.add_leader(project, &id, "running", false);
            }
            Ok(id)
        }

        async fn send(&self, id: &str, text: &str) -> Result<()> {
            self.record(Call::Send(id.to_string(), text.to_string()));
            Ok(())
        }

        async fn logs_since(&self, id: &str, since: &str) -> Result<String> {
            self.record(Call::LogsSince(id.to_string(), since.to_string()));
            Ok(self
                .state
                .lock()
                .unwrap()
                .logs
                .get(id)
                .cloned()
                .unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{Call, FakePaseo};
    use super::*;
    use std::sync::Arc;
    use waggledance_core::config::PaseoLeader;

    fn project(root: &Path) -> Project {
        Project {
            id: "proj-1".into(),
            name: "proj".into(),
            root_path: root.to_path_buf(),
            created_at: String::new(),
            last_seen_at: String::new(),
            orchestration_enabled: true,
        }
    }

    fn start(leader: Leader) -> Start {
        match leader {
            Leader::Start(s) => s,
            other => panic!("expected Start, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_non_archived_labelled_leader_is_existing_and_busy_follows_status() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_leader("proj-1", "a-idle", "idle", false);
        let got = resolve_leader(
            &fake,
            &PaseoConfig::default(),
            &project(dir.path()),
            dir.path(),
        )
        .await
        .unwrap();
        assert!(matches!(got, Leader::Existing { ref id, busy: false } if id == "a-idle"));

        let fake = FakePaseo::default();
        fake.add_leader("proj-1", "a-init", "initializing", false);
        let got = resolve_leader(
            &fake,
            &PaseoConfig::default(),
            &project(dir.path()),
            dir.path(),
        )
        .await
        .unwrap();
        assert!(matches!(got, Leader::Existing { ref id, busy: true } if id == "a-init"));
    }

    #[tokio::test]
    async fn an_archived_leader_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_leader("proj-1", "old", "idle", true);
        fake.add_workspace("wks_root", "local", dir.path());
        let s = start(
            resolve_leader(
                &fake,
                &PaseoConfig::default(),
                &project(dir.path()),
                dir.path(),
            )
            .await
            .unwrap(),
        );
        assert_eq!(s.workspace, "wks_root");
        assert_eq!(s.title, "proj leader");
        assert_eq!(
            s.labels,
            vec![
                "waggledance.role=leader".to_string(),
                "waggledance.project=proj-1".to_string()
            ]
        );
        assert_eq!(s.cwd, dir.path());
    }

    #[tokio::test]
    async fn a_configured_workspace_wins_over_a_root_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_workspace("wks_root", "local", dir.path());
        let config = PaseoConfig {
            leaders: vec![PaseoLeader {
                project: "proj".into(),
                workspace: "wks_cfg".into(),
            }],
            ..PaseoConfig::default()
        };
        let s = start(
            resolve_leader(&fake, &config, &project(dir.path()), dir.path())
                .await
                .unwrap(),
        );
        assert_eq!(s.workspace, "wks_cfg");
        assert!(!fake.calls().contains(&Call::Workspaces));
    }

    #[tokio::test]
    async fn two_root_workspaces_with_no_config_entry_refuse_naming_both() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_workspace("wks_a", "local", dir.path());
        fake.add_workspace("wks_wt", "worktree", dir.path());
        fake.add_workspace("wks_b", "local", &dir.path().join("."));
        let err = resolve_leader(
            &fake,
            &PaseoConfig::default(),
            &project(dir.path()),
            dir.path(),
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(err.contains("wks_a") && err.contains("wks_b"), "{err}");
        assert!(!err.contains("wks_wt"), "{err}");
        assert!(
            err.contains("[[paseo.leaders]]") && err.contains("workspace"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn no_workspace_at_the_root_creates_one_there_and_runs_in_it() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_workspace("wks_other", "local", other.path());
        let s = start(
            resolve_leader(
                &fake,
                &PaseoConfig::default(),
                &project(dir.path()),
                dir.path(),
            )
            .await
            .unwrap(),
        );
        assert!(fake
            .calls()
            .contains(&Call::CreateWorkspace(dir.path().to_path_buf())));
        let ws = s.workspace.clone();
        assert_ne!(ws, "wks_other");
        let id = s.run(&fake, "do it").await.unwrap();
        let runs: Vec<RunSpec> = fake
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::Run(spec) => Some(spec),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].workspace, ws);
        assert_eq!(runs[0].task, "do it");
        assert!(!id.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_resolves_start_exactly_one_leader() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakePaseo::default());
        fake.add_workspace("wks_root", "local", dir.path());
        let mut handles = Vec::new();
        for _ in 0..4 {
            let fake = fake.clone();
            let root = dir.path().to_path_buf();
            handles.push(tokio::spawn(async move {
                let leader =
                    resolve_leader(&*fake, &PaseoConfig::default(), &project(&root), &root)
                        .await
                        .unwrap();
                if let Leader::Start(s) = leader {
                    s.run(&*fake, "task").await.unwrap();
                }
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let runs = fake
            .calls()
            .iter()
            .filter(|c| matches!(c, Call::Run(_)))
            .count();
        assert_eq!(runs, 1);
    }

    fn spec(provider: &str, mode: &str, thinking: &str, task: &str) -> RunSpec {
        RunSpec {
            workspace: "wks_1".into(),
            cwd: PathBuf::from("/root/proj"),
            title: "proj leader".into(),
            labels: vec![ROLE_LABEL.into(), project_label("proj-1")],
            provider: provider.into(),
            mode: mode.into(),
            thinking: thinking.into(),
            task: task.into(),
        }
    }

    #[test]
    fn run_argv_carries_workspace_cwd_labels_title_and_dashdash_before_the_task() {
        let args = run_args(&spec("", "", "", "--rm -rf; $(x)"));
        let pair = |flag: &str, value: &str| args.windows(2).any(|w| w[0] == flag && w[1] == value);
        assert_eq!(&args[..3], &["run", "-d", "-q"]);
        assert!(pair("--workspace", "wks_1"));
        assert!(pair("--cwd", "/root/proj"));
        assert!(pair("--title", "proj leader"));
        assert!(pair("--label", "waggledance.role=leader"));
        assert!(pair("--label", "waggledance.project=proj-1"));
        assert_eq!(&args[args.len() - 2..], &["--", "--rm -rf; $(x)"]);
    }

    #[test]
    fn empty_provider_mode_thinking_pass_no_flag() {
        let args = run_args(&spec("", "", "", "t"));
        for flag in ["--provider", "--mode", "--thinking"] {
            assert!(!args.iter().any(|a| a == flag), "{flag} in {args:?}");
        }
        let args = run_args(&spec("claude/opus", "plan", "high", "t"));
        let pair = |flag: &str, value: &str| args.windows(2).any(|w| w[0] == flag && w[1] == value);
        assert!(pair("--provider", "claude/opus"));
        assert!(pair("--mode", "plan"));
        assert!(pair("--thinking", "high"));
        assert_eq!(&args[args.len() - 2..], &["--", "t"]);
    }

    #[test]
    fn leaders_lookup_filters_by_both_labels_globally() {
        assert_eq!(
            leaders_args("proj-1"),
            vec![
                "ls",
                "-g",
                "--label",
                "waggledance.role=leader",
                "--label",
                "waggledance.project=proj-1",
                "--json"
            ]
        );
    }

    #[tokio::test]
    async fn a_missing_binary_errors_naming_the_binary() {
        let config = PaseoConfig {
            binary: "/nonexistent/waggledance-test/paseo-missing".into(),
            ..PaseoConfig::default()
        };
        let err = PaseoCli::new(&config)
            .workspaces()
            .await
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("/nonexistent/waggledance-test/paseo-missing"),
            "{err}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cli_parses_stdout_only_and_names_unparsable_output() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("paseo");
        std::fs::write(
            &bin,
            "#!/bin/sh\necho 'startup noise' >&2\ncase \"$1\" in\n  inspect) printf '{\"Status\":\"running\",\"Archived\":false,\"PendingPermissions\":[{}],\"UpdatedAt\":\"2026-10-14T14:04:56.399Z\"}' ;;\n  workspace) printf '[{\"workspaceId\":\"wks_1\",\"isolation\":\"local\",\"cwd\":\"/x\"}]' ;;\n  ls) printf 'not json' ;;\n  send) cat \"$4\" > \"$(dirname \"$0\")/sent\" ;;\nesac\n",
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cli = PaseoCli::new(&PaseoConfig {
            binary: bin.to_string_lossy().into_owned(),
            ..PaseoConfig::default()
        });
        let mut state = cli.inspect("a").await;
        for _ in 0..50 {
            match &state {
                Err(PaseoError::Spawn { message, .. }) if message.contains("Text file busy") => {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    state = cli.inspect("a").await;
                }
                _ => break,
            }
        }
        let state = state.unwrap();
        assert_eq!(
            state,
            AgentState {
                status: "running".into(),
                archived: false,
                pending_permissions: 1,
                updated_at: Some("2026-10-14T14:04:56.399Z".into()),
            }
        );
        assert_eq!(cli.workspaces().await.unwrap()[0].id, "wks_1");
        let err = cli.leaders("proj-1").await.unwrap_err();
        assert!(matches!(err, PaseoError::Unparsable { .. }), "{err}");
        assert!(err.to_string().contains("ls"), "{err}");
        cli.send("a", "hello $(not run)").await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sent")).unwrap(),
            "hello $(not run)"
        );
    }
}
