use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use waggledance_core::config::PaseoConfig;
use waggledance_core::domain::Project;
use waggledance_core::project_settings::ProjectPaseo;

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
    #[error("`{binary} {command}` failed with {code}: {message}")]
    Reported {
        binary: String,
        command: String,
        code: String,
        message: String,
    },
    #[error("no Paseo provider is available and enabled: set [paseo] provider or enable one listed by `paseo provider ls`")]
    NoProvider,
    #[error("`{binary} {command}` printed output that could not be parsed: {message}")]
    Unparsable {
        binary: String,
        command: String,
        message: String,
    },
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
    pub permissions: Vec<PendingPermission>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PendingPermission {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub tool: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEnd {
    Completed,
    Permission,
    Other { status: String, message: String },
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoredWorkspace {
    pub workspace_id: String,
    pub created_at: Option<String>,
    pub archived_at: Option<String>,
    pub is_paseo_owned_worktree: bool,
}

pub fn store_path(paseo_home: Option<OsString>) -> Option<PathBuf> {
    paseo_home
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".paseo")))
        .map(|home| home.join("projects").join("workspaces.json"))
}

pub fn read_store(path: &Path) -> Vec<StoredWorkspace> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
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
    async fn providers(&self) -> Result<Vec<String>>;
    fn stored_workspaces(&self) -> Vec<StoredWorkspace>;
    async fn create_workspace(&self, path: &Path) -> Result<String>;
    async fn run_and_wait(&self, spec: &RunSpec, timeout: Duration) -> Result<(String, TurnEnd)>;
    async fn send_and_wait(&self, id: &str, text: &str, timeout: Duration) -> Result<TurnEnd>;
    async fn send(&self, id: &str, text: &str) -> Result<()>;
    async fn wait(&self, id: &str, timeout: Duration) -> Result<TurnEnd>;
    async fn logs_since(&self, id: &str, since: &str) -> Result<String>;
}

pub fn delivery_heartbeat_path(data_dir: &Path) -> PathBuf {
    data_dir.join("paseo-delivery.heartbeat")
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
        "--json".into(),
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

fn is_startup_log(line: &str) -> bool {
    let Some((clock, _)) = line.split_once(" › ") else {
        return false;
    };
    let b = clock.as_bytes();
    b.len() == 12
        && b.iter().enumerate().all(|(i, c)| match i {
            2 | 5 => *c == b':',
            8 => *c == b'.',
            _ => c.is_ascii_digit(),
        })
}

pub fn resolve_binary(configured: &str, paseo_cli: Option<OsString>) -> String {
    if !configured.is_empty() {
        return configured.to_string();
    }
    match paseo_cli {
        Some(cli) if Path::new(&cli).is_file() => cli.to_string_lossy().into_owned(),
        _ => "paseo".to_string(),
    }
}

const DETECT_TTL: Duration = Duration::from_secs(600);
const NPM_CLI_SUFFIX: &str = "@getpaseo/cli/bin/paseo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub binary: Option<String>,
    pub rejected: Vec<(String, String)>,
    pub notes: Vec<String>,
}

impl Detection {
    pub fn unavailable(&self) -> String {
        let tried: Vec<String> = self
            .rejected
            .iter()
            .map(|(candidate, why)| format!("`{candidate}` ({why})"))
            .collect();
        format!(
            "no working Paseo CLI found — tried {}; install it with `npm install -g @getpaseo/cli` \
             or set `binary` under `[paseo]` in ~/.waggledance/config.toml",
            tried.join("; ")
        )
    }
}

fn is_npm_cli(path: &Path) -> bool {
    std::fs::canonicalize(path).is_ok_and(|p| p.ends_with(NPM_CLI_SUFFIX))
}

fn is_appimage_wrapper(path: &Path) -> bool {
    use std::io::Read;
    let mut head = Vec::new();
    path.extension().is_some_and(|e| e == "AppImage")
        || std::fs::File::open(path)
            .and_then(|f| f.take(4096).read_to_end(&mut head))
            .is_ok()
            && head.starts_with(b"#!")
            && String::from_utf8_lossy(&head).contains(".AppImage")
}

fn mise_clis(root: &Path) -> Vec<PathBuf> {
    let version =
        |name: &str| -> Vec<u64> { name.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let mut found: Vec<(Vec<u64>, PathBuf)> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            (
                version(&e.file_name().to_string_lossy()),
                e.path().join("bin").join("paseo"),
            )
        })
        .filter(|(_, p)| is_npm_cli(p))
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, p)| p).collect()
}

pub fn candidates(
    configured: &str,
    path: Option<OsString>,
    mise_root: Option<PathBuf>,
    paseo_cli: Option<OsString>,
) -> (Vec<String>, Vec<String>) {
    let dirs: Vec<PathBuf> = path.iter().flat_map(std::env::split_paths).collect();
    let npm: Vec<String> = dirs
        .iter()
        .map(|d| d.join("paseo"))
        .filter(|p| is_npm_cli(p))
        .chain(mise_root.map(|m| mise_clis(&m)).unwrap_or_default())
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut notes = Vec::new();
    let demoted =
        !configured.is_empty() && !npm.is_empty() && is_appimage_wrapper(Path::new(configured));
    if demoted {
        notes.push(format!(
            "configured binary {configured} is an AppImage wrapper; the npm CLI is tried first"
        ));
    }
    let mut ordered = Vec::new();
    if !configured.is_empty() && !demoted {
        ordered.push(configured.to_string());
    }
    ordered.extend(npm);
    if demoted {
        ordered.push(configured.to_string());
    }
    if let Some(cli) = paseo_cli.filter(|c| Path::new(c).is_file()) {
        ordered.push(cli.to_string_lossy().into_owned());
    }
    ordered.push("paseo".to_string());
    let key = |c: &str| {
        let found = if c.contains('/') {
            Some(PathBuf::from(c))
        } else {
            dirs.iter().map(|d| d.join(c)).find(|p| p.is_file())
        };
        found
            .and_then(|p| std::fs::canonicalize(p).ok())
            .map_or_else(|| c.to_string(), |p| p.to_string_lossy().into_owned())
    };
    let mut seen = std::collections::HashSet::new();
    ordered.retain(|c| seen.insert(key(c)));
    (ordered, notes)
}

fn probe(binary: &str) -> std::result::Result<(), String> {
    use std::io::Read;
    let mut child = std::process::Command::new(binary)
        .args(["ls", "-g", "--json"])
        .env_remove("PASEO_AGENT_ID")
        .env_remove("PASEO_AGENT_CWD")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not be started: {e}"))?;
    let (tx, rx) = std::sync::mpsc::channel();
    if let Some(mut out) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = out.read_to_end(&mut buf);
            let _ = tx.send(buf);
        });
    }
    let Ok(buf) = rx.recv_timeout(TIMEOUT) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "`ls -g --json` timed out after {}s",
            TIMEOUT.as_secs()
        ));
    };
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("`ls -g --json` exited with {status}"));
    }
    parse_document::<Vec<serde_json::Value>>(&String::from_utf8_lossy(&buf))
        .map(|_| ())
        .map_err(|e| format!("`ls -g --json` printed no JSON list: {e}"))
}

pub fn detect(candidates: &[String], notes: Vec<String>) -> Detection {
    let mut rejected = Vec::new();
    for candidate in candidates {
        match probe(candidate) {
            Ok(()) => {
                return Detection {
                    binary: Some(candidate.clone()),
                    rejected,
                    notes,
                }
            }
            Err(why) => rejected.push((candidate.clone(), why)),
        }
    }
    Detection {
        binary: None,
        rejected,
        notes,
    }
}

type Detected = std::collections::HashMap<Vec<String>, (std::time::Instant, Detection)>;
static DETECTED: std::sync::LazyLock<std::sync::Mutex<Detected>> =
    std::sync::LazyLock::new(Default::default);

fn detect_cached(candidates: Vec<String>, notes: Vec<String>) -> Detection {
    let cache = || DETECTED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, hit)) = cache().get(&candidates) {
        if at.elapsed() < DETECT_TTL {
            return hit.clone();
        }
    }
    let found = detect(&candidates, notes);
    match &found.binary {
        Some(binary) => {
            tracing::info!(binary, rejected = ?found.rejected, notes = ?found.notes, "Paseo CLI detected");
            cache().insert(candidates, (std::time::Instant::now(), found.clone()));
        }
        None => tracing::warn!(rejected = ?found.rejected, "no working Paseo CLI found"),
    }
    found
}

pub fn detect_binary(config: &PaseoConfig) -> Detection {
    let (candidates, notes) = candidates(
        &config.binary,
        std::env::var_os("PATH"),
        dirs::home_dir().map(|h| h.join(".local/share/mise/installs/node")),
        std::env::var_os("PASEO_CLI"),
    );
    detect_cached(candidates, notes)
}

pub fn paseo_unavailable(config: &PaseoConfig) -> Option<String> {
    match config.enabled {
        Some(true) => None,
        Some(false) => Some(
            "Paseo leader dispatch is off — `enabled = false` under `[paseo]` in \
             ~/.waggledance/config.toml; remove that line to let waggledance detect the Paseo CLI"
                .to_string(),
        ),
        None => {
            let found = detect_binary(config);
            found.binary.is_none().then(|| found.unavailable())
        }
    }
}

pub fn paseo_available(config: &PaseoConfig) -> bool {
    paseo_unavailable(config).is_none()
}

pub fn command(binary: &str, args: &[String]) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(binary);
    cmd.args(args)
        .env_remove("PASEO_AGENT_ID")
        .env_remove("PASEO_AGENT_CWD")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    cmd
}

pub fn strip_noise(stdout: &str) -> String {
    let mut in_object = false;
    let mut kept = String::new();
    for line in stdout.split_inclusive('\n') {
        let bare = line.trim_end();
        if in_object {
            if bare == "}" {
                in_object = false;
                continue;
            }
            if bare.is_empty() || bare.starts_with(char::is_whitespace) {
                continue;
            }
            in_object = false;
        }
        if is_startup_log(bare) {
            in_object = bare.ends_with('{');
            continue;
        }
        kept.push_str(line);
    }
    kept
}

pub fn document(stdout: &str) -> Option<&str> {
    let mut offset = 0;
    for line in stdout.split_inclusive('\n') {
        if line.starts_with('{') || line.starts_with('[') {
            return Some(&stdout[offset..]);
        }
        offset += line.len();
    }
    None
}

pub fn parse_document<T: serde::de::DeserializeOwned>(
    stdout: &str,
) -> std::result::Result<T, String> {
    let doc = document(stdout).ok_or("no JSON document on stdout")?;
    serde_json::Deserializer::from_str(doc)
        .into_iter::<T>()
        .next()
        .ok_or("empty JSON document")?
        .map_err(|e| e.to_string())
}

pub fn last_id(stdout: &str) -> Option<String> {
    strip_noise(stdout)
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .map(str::to_string)
}

pub fn parse_leaders(stdout: &str) -> std::result::Result<Vec<String>, String> {
    let agents: Vec<ListedAgent> = parse_document(stdout)?;
    Ok(agents.into_iter().map(|a| a.id).collect())
}

pub fn parse_providers(stdout: &str) -> std::result::Result<Vec<String>, String> {
    let listed: Vec<ListedProvider> = parse_document(stdout)?;
    Ok(listed
        .into_iter()
        .filter(|p| p.status == "available" && p.enabled == "Enabled")
        .map(|p| p.provider)
        .collect())
}

pub fn reported_error(output: &str) -> Option<(String, String)> {
    let at = match output.starts_with('{') {
        true => 0,
        false => output.find("\n{")? + 1,
    };
    let doc: ErrorDocument = serde_json::Deserializer::from_str(&output[at..])
        .into_iter()
        .next()?
        .ok()?;
    Some((doc.error.code, doc.error.message))
}

pub fn parse_inspect(stdout: &str) -> std::result::Result<AgentState, String> {
    let i: Inspected = parse_document(stdout)?;
    Ok(AgentState {
        status: i.status,
        archived: i.archived,
        pending_permissions: i.pending_permissions.len(),
        permissions: i.pending_permissions,
        updated_at: i.updated_at,
    })
}

pub fn parse_turn_end(stdout: &str) -> std::result::Result<(String, TurnEnd), String> {
    let t: TurnReport = parse_document(stdout)?;
    let end = match t.status.as_str() {
        "completed" => TurnEnd::Completed,
        "permission" => TurnEnd::Permission,
        _ => TurnEnd::Other {
            status: t.status,
            message: t.message,
        },
    };
    Ok((t.agent_id, end))
}

pub struct PaseoCli {
    config: PaseoConfig,
    fixed: Option<String>,
}

impl PaseoCli {
    pub fn new(config: &PaseoConfig) -> Self {
        Self {
            config: config.clone(),
            fixed: None,
        }
    }

    #[cfg(test)]
    pub fn with_binary(binary: &str) -> Self {
        Self {
            config: PaseoConfig::default(),
            fixed: Some(binary.to_string()),
        }
    }

    async fn binary(&self) -> String {
        if let Some(binary) = &self.fixed {
            return binary.clone();
        }
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || {
            detect_binary(&config)
                .binary
                .unwrap_or_else(|| resolve_binary(&config.binary, std::env::var_os("PASEO_CLI")))
        })
        .await
        .unwrap_or_else(|_| "paseo".to_string())
    }

    async fn exec(&self, args: &[String], timeout: Duration) -> Result<(String, String)> {
        let binary = self.binary().await;
        let command = args.first().cloned().unwrap_or_default();
        let child = self::command(&binary, args)
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PaseoError::Spawn {
                binary: binary.clone(),
                message: e.to_string(),
            })?;
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| PaseoError::Timeout {
                binary: binary.clone(),
                command: command.clone(),
                secs: timeout.as_secs(),
            })?
            .map_err(|e| PaseoError::Spawn {
                binary: binary.clone(),
                message: e.to_string(),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let reported = reported_error(&stdout)
            .or_else(|| reported_error(&String::from_utf8_lossy(&output.stderr)));
        if let Some((code, message)) = reported {
            return Err(PaseoError::Reported {
                binary,
                command,
                code,
                message,
            });
        }
        if !output.status.success() {
            return Err(PaseoError::Failed {
                binary,
                command,
                status: output.status.to_string(),
            });
        }
        Ok((binary, stdout))
    }

    async fn turn(&self, args: &[String], timeout: Duration) -> Result<(String, TurnEnd)> {
        let (binary, out) = self.exec(args, timeout).await?;
        parse_turn_end(&out).map_err(|e| unparsable(&binary, &args[0], e))
    }
}

fn unparsable(binary: &str, command: &str, message: impl ToString) -> PaseoError {
    PaseoError::Unparsable {
        binary: binary.to_string(),
        command: command.to_string(),
        message: message.to_string(),
    }
}

#[derive(Deserialize)]
struct ListedAgent {
    id: String,
}

#[derive(Deserialize)]
struct ListedProvider {
    provider: String,
    status: String,
    enabled: String,
}

#[derive(Deserialize)]
struct ErrorDocument {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspected {
    status: String,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    pending_permissions: Vec<PendingPermission>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TurnReport {
    #[serde(default)]
    agent_id: String,
    status: String,
    #[serde(default)]
    message: String,
}

#[async_trait]
impl Paseo for PaseoCli {
    async fn leaders(&self, project_id: &str) -> Result<Vec<String>> {
        let (binary, out) = self.exec(&leaders_args(project_id), TIMEOUT).await?;
        parse_leaders(&out).map_err(|e| unparsable(&binary, "ls", e))
    }

    async fn inspect(&self, id: &str) -> Result<AgentState> {
        let (binary, out) = self
            .exec(&["inspect".into(), id.into(), "--json".into()], TIMEOUT)
            .await?;
        parse_inspect(&out).map_err(|e| unparsable(&binary, "inspect", e))
    }

    async fn workspaces(&self) -> Result<Vec<Workspace>> {
        let (binary, out) = self
            .exec(&["workspace".into(), "ls".into(), "--json".into()], TIMEOUT)
            .await?;
        parse_document(&out).map_err(|e| unparsable(&binary, "workspace", e))
    }

    async fn providers(&self) -> Result<Vec<String>> {
        let (binary, out) = self
            .exec(&["provider".into(), "ls".into(), "--json".into()], TIMEOUT)
            .await?;
        parse_providers(&out).map_err(|e| unparsable(&binary, "provider", e))
    }

    fn stored_workspaces(&self) -> Vec<StoredWorkspace> {
        store_path(std::env::var_os("PASEO_HOME"))
            .map(|p| read_store(&p))
            .unwrap_or_default()
    }

    async fn create_workspace(&self, path: &Path) -> Result<String> {
        let (binary, out) = self
            .exec(
                &[
                    "workspace".into(),
                    "create".into(),
                    "--path".into(),
                    path.to_string_lossy().into_owned(),
                    "--isolation".into(),
                    "local".into(),
                    "-q".into(),
                ],
                TIMEOUT,
            )
            .await?;
        last_id(&out).ok_or_else(|| unparsable(&binary, "workspace", "no id on stdout"))
    }

    async fn run_and_wait(&self, spec: &RunSpec, timeout: Duration) -> Result<(String, TurnEnd)> {
        self.turn(&run_args(spec), timeout).await
    }

    async fn send_and_wait(&self, id: &str, text: &str, timeout: Duration) -> Result<TurnEnd> {
        let file = PromptFile::write(text)?;
        let args = [
            "send".into(),
            id.into(),
            "--prompt-file".into(),
            file.0.to_string_lossy().into_owned(),
            "--json".into(),
        ];
        Ok(self.turn(&args, timeout).await?.1)
    }

    async fn send(&self, id: &str, text: &str) -> Result<()> {
        let file = PromptFile::write(text)?;
        self.exec(
            &[
                "send".into(),
                id.into(),
                "--prompt-file".into(),
                file.0.to_string_lossy().into_owned(),
                "--no-wait".into(),
            ],
            TIMEOUT,
        )
        .await
        .map(|_| ())
    }

    async fn wait(&self, id: &str, timeout: Duration) -> Result<TurnEnd> {
        let args = [
            "wait".into(),
            id.into(),
            "--timeout".into(),
            timeout.as_secs().to_string(),
            "--json".into(),
        ];
        Ok(self.turn(&args, timeout + TIMEOUT).await?.1)
    }

    async fn logs_since(&self, id: &str, since: &str) -> Result<String> {
        self.exec(
            &[
                "logs".into(),
                id.into(),
                "--since".into(),
                since.into(),
                "--filter".into(),
                "text".into(),
            ],
            TIMEOUT,
        )
        .await
        .map(|(_, out)| strip_noise(&out))
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
    pub async fn acquire(data_dir: &Path, project_id: &str) -> Result<Self> {
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
    pub reason: &'static str,
    provider: String,
    mode: String,
    thinking: String,
}

impl Start {
    pub fn spec(&self, task: &str) -> RunSpec {
        RunSpec {
            workspace: self.workspace.clone(),
            cwd: self.cwd.clone(),
            title: self.title.clone(),
            labels: self.labels.clone(),
            provider: self.provider.clone(),
            mode: self.mode.clone(),
            thinking: self.thinking.clone(),
            task: task.to_string(),
        }
    }
}

fn realpath(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub async fn resolve_leader(
    paseo: &dyn Paseo,
    config: &PaseoConfig,
    settings: &ProjectPaseo,
    project: &Project,
) -> Result<Leader> {
    for id in paseo.leaders(&project.id).await? {
        let state = paseo.inspect(&id).await?;
        if !state.archived {
            let busy = matches!(state.status.as_str(), "running" | "initializing");
            return Ok(Leader::Existing { id, busy });
        }
    }
    let either = |file: &Option<String>, global: &str| {
        file.clone()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| global.to_string())
    };
    let mut provider = either(&settings.provider, &config.provider);
    if provider.is_empty() {
        provider = paseo
            .providers()
            .await?
            .into_iter()
            .next()
            .ok_or(PaseoError::NoProvider)?;
    }
    let configured = settings
        .workspace
        .clone()
        .filter(|w| !w.is_empty())
        .map(|w| (w, "file"))
        .or_else(|| {
            config
                .workspace_for(&project.id, &project.name)
                .map(|w| (w.to_string(), "deprecated-entry"))
        });
    let (workspace, reason) = match configured {
        Some(found) => found,
        None => pick_workspace(paseo, &project.root_path).await?,
    };
    Ok(Leader::Start(Start {
        workspace,
        cwd: project.root_path.clone(),
        title: format!("{} leader", project.name),
        labels: vec![ROLE_LABEL.to_string(), project_label(&project.id)],
        reason,
        provider,
        mode: either(&settings.mode, &config.mode),
        thinking: either(&settings.thinking, &config.thinking),
    }))
}

async fn pick_workspace(paseo: &dyn Paseo, root: &Path) -> Result<(String, &'static str)> {
    let real = realpath(root);
    let created: std::collections::HashMap<String, String> = paseo
        .stored_workspaces()
        .into_iter()
        .filter_map(|w| Some((w.workspace_id, w.created_at?)))
        .collect();
    let picked = paseo
        .workspaces()
        .await?
        .into_iter()
        .filter(|w| w.isolation == "local" && realpath(Path::new(&w.cwd)) == real)
        .map(|w| (created.get(&w.id).cloned(), w.id))
        .min_by(|a, b| match (&a.0, &b.0) {
            (Some(x), Some(y)) => x.cmp(y).then_with(|| a.1.cmp(&b.1)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.1.cmp(&b.1),
        });
    Ok(match picked {
        Some((Some(_), id)) => (id, "oldest"),
        Some((None, id)) => (id, "lowest-id"),
        None => (paseo.create_workspace(root).await?, "created"),
    })
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Call {
        Leaders(String),
        Inspect(String),
        Workspaces,
        Providers,
        CreateWorkspace(PathBuf),
        Run(RunSpec),
        SendAndWait(String, String),
        Send(String, String),
        Wait(String),
        LogsSince(String, String),
    }

    #[derive(Default)]
    pub struct FakeState {
        pub leaders: HashMap<String, Vec<String>>,
        pub agents: HashMap<String, AgentState>,
        pub workspaces: Vec<Workspace>,
        pub workspaces_fail: bool,
        pub no_providers: bool,
        pub store: Vec<StoredWorkspace>,
        pub logs: HashMap<String, String>,
        pub turn_ends: VecDeque<TurnEnd>,
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
                    permissions: Vec::new(),
                    updated_at: None,
                },
            );
        }

        pub fn push_turn_end(&self, end: TurnEnd) {
            self.state.lock().unwrap().turn_ends.push_back(end);
        }

        pub fn set_status(&self, id: &str, status: &str) {
            if let Some(a) = self.state.lock().unwrap().agents.get_mut(id) {
                a.status = status.to_string();
            }
        }

        pub fn set_permissions(&self, id: &str, tools: &[&str]) {
            if let Some(a) = self.state.lock().unwrap().agents.get_mut(id) {
                a.permissions = tools
                    .iter()
                    .enumerate()
                    .map(|(i, tool)| PendingPermission {
                        id: format!("permission-{i}"),
                        tool: tool.to_string(),
                    })
                    .collect();
                a.pending_permissions = a.permissions.len();
            }
        }

        fn next_turn_end(&self) -> TurnEnd {
            self.state
                .lock()
                .unwrap()
                .turn_ends
                .pop_front()
                .unwrap_or(TurnEnd::Completed)
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
            let s = self.state.lock().unwrap();
            if s.workspaces_fail {
                return Err(PaseoError::Failed {
                    binary: "fake".into(),
                    command: "workspace".into(),
                    status: "exit status: 1".into(),
                });
            }
            Ok(s.workspaces.clone())
        }

        fn stored_workspaces(&self) -> Vec<StoredWorkspace> {
            self.state.lock().unwrap().store.clone()
        }

        async fn providers(&self) -> Result<Vec<String>> {
            self.record(Call::Providers);
            if self.state.lock().unwrap().no_providers {
                return Ok(Vec::new());
            }
            Ok(vec!["claude".into(), "codex".into()])
        }

        async fn create_workspace(&self, path: &Path) -> Result<String> {
            self.record(Call::CreateWorkspace(path.to_path_buf()));
            let id = format!("wks_new{}", self.state.lock().unwrap().workspaces.len());
            self.add_workspace(&id, "local", path);
            Ok(id)
        }

        async fn run_and_wait(
            &self,
            spec: &RunSpec,
            _timeout: Duration,
        ) -> Result<(String, TurnEnd)> {
            self.record(Call::Run(spec.clone()));
            tokio::task::yield_now().await;
            let id = format!("agent-{}", self.state.lock().unwrap().agents.len() + 1);
            if let Some(project) = spec
                .labels
                .iter()
                .find_map(|l| l.strip_prefix(&format!("{PROJECT_LABEL_KEY}=")))
            {
                self.add_leader(project, &id, "idle", false);
            }
            Ok((id, self.next_turn_end()))
        }

        async fn send_and_wait(&self, id: &str, text: &str, _timeout: Duration) -> Result<TurnEnd> {
            self.record(Call::SendAndWait(id.to_string(), text.to_string()));
            Ok(self.next_turn_end())
        }

        async fn send(&self, id: &str, text: &str) -> Result<()> {
            self.record(Call::Send(id.to_string(), text.to_string()));
            Ok(())
        }

        async fn wait(&self, id: &str, _timeout: Duration) -> Result<TurnEnd> {
            self.record(Call::Wait(id.to_string()));
            Ok(self.next_turn_end())
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
            &ProjectPaseo::default(),
            &project(dir.path()),
        )
        .await
        .unwrap();
        assert!(matches!(got, Leader::Existing { ref id, busy: false } if id == "a-idle"));

        let fake = FakePaseo::default();
        fake.add_leader("proj-1", "a-init", "initializing", false);
        let got = resolve_leader(
            &fake,
            &PaseoConfig::default(),
            &ProjectPaseo::default(),
            &project(dir.path()),
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
                &ProjectPaseo::default(),
                &project(dir.path()),
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
            resolve_leader(
                &fake,
                &config,
                &ProjectPaseo::default(),
                &project(dir.path()),
            )
            .await
            .unwrap(),
        );
        assert_eq!(
            (s.workspace.as_str(), s.reason),
            ("wks_cfg", "deprecated-entry")
        );
        assert!(!fake.calls().contains(&Call::Workspaces));
    }

    #[tokio::test]
    async fn several_root_workspaces_pick_the_oldest_from_the_store_without_refusing() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        for id in ["wks_fixture_a", "wks_fixture_b", "wks_fixture_c"] {
            fake.add_workspace(id, "local", dir.path());
        }
        fake.add_workspace("wks_fixture_wt", "worktree", dir.path());
        fake.state.lock().unwrap().store = store_fixture();
        let s = start(
            resolve_leader(
                &fake,
                &PaseoConfig::default(),
                &ProjectPaseo::default(),
                &project(dir.path()),
            )
            .await
            .unwrap(),
        );
        assert_eq!(
            (s.workspace.as_str(), s.reason),
            ("wks_fixture_b", "oldest")
        );
    }

    #[tokio::test]
    async fn without_a_store_the_lowest_id_at_the_root_wins() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_workspace("wks_b", "local", dir.path());
        fake.add_workspace("wks_a", "local", &dir.path().join("."));
        let s = start(
            resolve_leader(
                &fake,
                &PaseoConfig::default(),
                &ProjectPaseo::default(),
                &project(dir.path()),
            )
            .await
            .unwrap(),
        );
        assert_eq!((s.workspace.as_str(), s.reason), ("wks_a", "lowest-id"));
    }

    #[tokio::test]
    async fn the_project_file_workspace_and_model_override_the_pick_and_the_global_block() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_workspace("wks_fixture_b", "local", dir.path());
        fake.state.lock().unwrap().store = store_fixture();
        let config = PaseoConfig {
            provider: "codex".into(),
            mode: "default".into(),
            leaders: vec![PaseoLeader {
                project: "proj".into(),
                workspace: "wks_cfg".into(),
            }],
            ..PaseoConfig::default()
        };
        let settings = ProjectPaseo {
            workspace: Some("wks_file".into()),
            provider: Some("claude".into()),
            ..ProjectPaseo::default()
        };
        let s = start(
            resolve_leader(&fake, &config, &settings, &project(dir.path()))
                .await
                .unwrap(),
        );
        assert_eq!((s.workspace.as_str(), s.reason), ("wks_file", "file"));
        let spec = s.spec("t");
        assert_eq!(
            (spec.provider.as_str(), spec.mode.as_str()),
            ("claude", "default")
        );
        assert!(!fake.calls().contains(&Call::Workspaces));
        assert!(!fake.calls().contains(&Call::Providers));
    }

    #[tokio::test]
    async fn without_a_configured_provider_the_first_available_one_starts_the_leader() {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakePaseo::default();
        fake.add_workspace("wks_a", "local", dir.path());
        let s = start(
            resolve_leader(
                &fake,
                &PaseoConfig::default(),
                &ProjectPaseo::default(),
                &project(dir.path()),
            )
            .await
            .unwrap(),
        );
        assert_eq!(s.spec("t").provider, "claude");
        fake.state.lock().unwrap().no_providers = true;
        let err = resolve_leader(
            &fake,
            &PaseoConfig::default(),
            &ProjectPaseo::default(),
            &project(dir.path()),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, PaseoError::NoProvider), "{err}");
        assert!(err.to_string().contains("paseo provider ls"), "{err}");
        assert_eq!(
            fake.calls()
                .iter()
                .filter(|c| matches!(c, Call::Workspaces))
                .count(),
            1
        );
    }

    fn store_fixture() -> Vec<StoredWorkspace> {
        read_store(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata/fixtures/paseo-0.11.2/workspaces-store.json"),
        )
    }

    #[test]
    fn the_store_lives_under_paseo_home_and_parses_the_fixture() {
        assert_eq!(
            store_path(Some("/srv/ph".into())),
            Some(PathBuf::from("/srv/ph/projects/workspaces.json"))
        );
        let store = store_fixture();
        assert_eq!(store.len(), 5);
        assert!(store
            .iter()
            .any(|w| w.workspace_id == "wks_fixture_archived" && w.archived_at.is_some()));
        assert!(store
            .iter()
            .any(|w| w.workspace_id == "wks_fixture_owned" && w.is_paseo_owned_worktree));
        assert!(read_store(Path::new("/nonexistent/workspaces.json")).is_empty());
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
                &ProjectPaseo::default(),
                &project(dir.path()),
            )
            .await
            .unwrap(),
        );
        assert!(fake
            .calls()
            .contains(&Call::CreateWorkspace(dir.path().to_path_buf())));
        let ws = s.workspace.clone();
        assert_ne!(ws, "wks_other");
        let (id, _) = fake
            .run_and_wait(&s.spec("do it"), Duration::from_secs(1))
            .await
            .unwrap();
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
                let _lock = LeaderLock::acquire(&root, "proj-1").await.unwrap();
                let leader = resolve_leader(
                    &*fake,
                    &PaseoConfig::default(),
                    &ProjectPaseo::default(),
                    &project(&root),
                )
                .await
                .unwrap();
                if let Leader::Start(s) = leader {
                    fake.run_and_wait(&s.spec("task"), Duration::from_secs(1))
                        .await
                        .unwrap();
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
        assert_eq!(&args[..2], &["run", "--json"]);
        assert!(!args.iter().any(|a| a == "-d" || a == "-q"), "{args:?}");
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
        let err = PaseoCli::with_binary("/nonexistent/waggledance-test/paseo-missing")
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
            concat!(
                "#!/bin/sh\n",
                "echo 'startup noise' >&2\n",
                "echo \"21:52:26.724 › [desktop] app startup { version: '0.11.2', platform: 'linux', arch: 'x64', isPackaged: true }\"\n",
                "echo \"21:52:26.731 › [linux-sandbox] { enabled: false, reason: 'requested by --no-sandbox' }\"\n",
                "case \"$1 $2\" in\n",
                "  inspect*) printf '{\"Status\":\"running\",\"Archived\":false,\"PendingPermissions\":[{}],\"UpdatedAt\":\"2026-10-14T14:04:56.399Z\"}' ;;\n",
                "  'workspace ls') printf '[{\"workspaceId\":\"wks_1\",\"isolation\":\"local\",\"cwd\":\"/x\"}]' ;;\n",
                "  'workspace create') echo wks_new ;;\n",
                "  'provider ls') printf '[{\"provider\":\"pi\",\"status\":\"available\",\"enabled\":\"Enabled\"}]' ;;\n",
                "  run*) printf '{\\n  \"agentId\": \"agent-9\",\\n  \"status\": \"completed\"\\n}\\n' ;;\n",
                "  wait*) printf '{\"agentId\":\"a\",\"status\":\"idle\",\"message\":\"Agent is idle.\"}' ;;\n",
                "  logs*) printf '[User] do it\\n12:00:00 done\\n' ;;\n",
                "  ls*) printf 'not json' ;;\n",
                "  send*) cat \"$4\" > \"$(dirname \"$0\")/sent\"; printf '{\"agentId\":\"a\",\"status\":\"permission\"}' ;;\n",
                "esac\n",
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cli = PaseoCli::with_binary(&bin.to_string_lossy());
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
                permissions: vec![PendingPermission {
                    id: String::new(),
                    tool: String::new(),
                }],
                updated_at: Some("2026-10-14T14:04:56.399Z".into()),
            }
        );
        assert_eq!(cli.workspaces().await.unwrap()[0].id, "wks_1");
        assert_eq!(cli.providers().await.unwrap(), vec!["pi"]);
        assert_eq!(
            cli.create_workspace(Path::new("/x")).await.unwrap(),
            "wks_new"
        );
        let turn = Duration::from_secs(5);
        assert_eq!(
            cli.run_and_wait(&spec("", "", "", "t"), turn)
                .await
                .unwrap(),
            ("agent-9".to_string(), TurnEnd::Completed)
        );
        assert_eq!(
            cli.send_and_wait("a", "x", turn).await.unwrap(),
            TurnEnd::Permission
        );
        assert_eq!(
            cli.wait("a", turn).await.unwrap(),
            TurnEnd::Other {
                status: "idle".into(),
                message: "Agent is idle.".into()
            }
        );
        assert_eq!(
            cli.logs_since("a", "2026-10-14T00:00:00Z").await.unwrap(),
            "[User] do it\n12:00:00 done\n"
        );
        let err = cli.leaders("proj-1").await.unwrap_err();
        assert!(matches!(err, PaseoError::Unparsable { .. }), "{err}");
        assert!(err.to_string().contains("ls"), "{err}");
        cli.send("a", "hello $(not run)").await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sent")).unwrap(),
            "hello $(not run)"
        );
    }

    #[test]
    fn binary_resolution_prefers_config_then_an_existing_paseo_cli_then_paseo() {
        let dir = tempfile::tempdir().unwrap();
        let cli = dir.path().join("paseo-bundled");
        std::fs::write(&cli, "").unwrap();
        let bundled = || Some(cli.clone().into_os_string());
        assert_eq!(resolve_binary("/opt/paseo", bundled()), "/opt/paseo");
        assert_eq!(resolve_binary("", bundled()), cli.to_string_lossy());
        assert_eq!(
            resolve_binary("", Some(dir.path().join("gone").into_os_string())),
            "paseo"
        );
        assert_eq!(resolve_binary("", Some(OsString::new())), "paseo");
        assert_eq!(resolve_binary("", None), "paseo");
    }

    #[cfg(unix)]
    const LISTS: &str =
        "echo \"23:51:21.452 › [desktop] app startup { version: '0.11.2' }\"\nprintf '[]\\n'";

    #[cfg(unix)]
    fn script(path: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        for _ in 0..50 {
            match std::process::Command::new(path).arg("settle").output() {
                Err(e) if e.to_string().contains("Text file busy") => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => break,
            }
        }
        path.to_path_buf()
    }

    #[cfg(unix)]
    fn npm_cli(prefix: &Path, body: &str) -> PathBuf {
        let real = script(
            &prefix.join("lib/node_modules/@getpaseo/cli/bin/paseo"),
            body,
        );
        let link = prefix.join("bin/paseo");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(real, &link).unwrap();
        link
    }

    #[cfg(unix)]
    fn lossy(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    #[cfg(unix)]
    #[test]
    fn candidates_run_configured_npm_on_path_mise_newest_first_paseo_cli_then_paseo() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let on_path = npm_cli(&d.join("node"), LISTS);
        let mise = d.join("mise");
        let old = npm_cli(&mise.join("9.11.0"), LISTS);
        let new = npm_cli(&mise.join("24.19.0"), LISTS);
        let plain = script(&d.join("plain/paseo"), LISTS);
        let configured = script(&d.join("cfg/paseo"), LISTS);
        let wrapper = script(
            &d.join("wrapper/paseo"),
            "exec /opt/Paseo.AppImage --no-sandbox \"$@\"",
        );
        let bundled = script(&d.join("bundled/paseo"), LISTS);
        let path = std::env::join_paths([
            plain.parent().unwrap(),
            on_path.parent().unwrap(),
            new.parent().unwrap(),
        ])
        .unwrap();
        let cli = || Some(bundled.clone().into_os_string());

        let (got, notes) = candidates(
            &lossy(&configured),
            Some(path.clone()),
            Some(mise.clone()),
            cli(),
        );
        assert_eq!(
            got,
            vec![
                lossy(&configured),
                lossy(&on_path),
                lossy(&new),
                lossy(&old),
                lossy(&bundled),
                "paseo".to_string()
            ]
        );
        assert!(notes.is_empty(), "{notes:?}");

        let (got, notes) = candidates(
            &lossy(&wrapper),
            Some(path.clone()),
            Some(mise.clone()),
            cli(),
        );
        assert_eq!(
            got[..4],
            [lossy(&on_path), lossy(&new), lossy(&old), lossy(&wrapper)]
        );
        assert!(notes[0].contains("AppImage wrapper"), "{notes:?}");

        let alone = std::env::join_paths([plain.parent().unwrap()]).unwrap();
        let gone = Some(d.join("missing").into_os_string());
        let (got, notes) = candidates(&lossy(&wrapper), Some(alone), None, gone);
        assert_eq!(got, vec![lossy(&wrapper), "paseo".to_string()]);
        assert!(notes.is_empty(), "{notes:?}");

        let npm_first = std::env::join_paths([on_path.parent().unwrap()]).unwrap();
        let (got, _) = candidates("", Some(npm_first), None, None);
        assert_eq!(got, vec![lossy(&on_path)]);
    }

    #[cfg(unix)]
    #[test]
    fn the_npm_cli_beats_a_working_configured_wrapper_and_non_json_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let npm = npm_cli(&d.join("node"), LISTS);
        let wrapper = script(
            &d.join("wrapper/paseo"),
            &format!(": /home/u/Applications/Paseo.AppImage\n{LISTS}"),
        );
        let path = std::env::join_paths([npm.parent().unwrap()]).unwrap();
        let (c, notes) = candidates(&lossy(&wrapper), Some(path), None, None);
        let found = detect(&c, notes);
        assert_eq!(found.binary, Some(lossy(&npm)));
        assert!(found.rejected.is_empty(), "{found:?}");
        assert!(found.notes[0].contains(&lossy(&wrapper)), "{found:?}");

        let noisy = script(&d.join("noisy/paseo"), "echo 'not json'");
        let found = detect(&[lossy(&noisy), lossy(&npm)], Vec::new());
        assert_eq!(found.binary, Some(lossy(&npm)));
        assert_eq!(found.rejected.len(), 1);
        assert_eq!(found.rejected[0].0, lossy(&noisy));
        assert!(found.rejected[0].1.contains("no JSON"), "{found:?}");
    }

    #[cfg(unix)]
    #[test]
    fn with_no_working_cli_the_refusal_lists_every_candidate_and_why() {
        let dir = tempfile::tempdir().unwrap();
        let noisy = script(&dir.path().join("noisy/paseo"), "echo 'not json'");
        let failing = script(&dir.path().join("failing/paseo"), "exit 3");
        let missing = dir.path().join("missing/paseo");
        let found = detect(
            &[lossy(&noisy), lossy(&failing), lossy(&missing)],
            Vec::new(),
        );
        assert_eq!(found.binary, None);
        let why = found.unavailable();
        for (path, reason) in [
            (&noisy, "no JSON"),
            (&failing, "exited"),
            (&missing, "could not be started"),
        ] {
            assert!(why.contains(&format!("`{}` (", lossy(path))), "{why}");
            assert!(why.contains(reason), "{why}");
        }
        assert!(why.contains("@getpaseo/cli"), "{why}");
    }

    #[cfg(unix)]
    #[test]
    fn only_a_successful_detection_is_cached() {
        let dir = tempfile::tempdir().unwrap();
        let counted = script(
            &dir.path().join("counted/paseo"),
            &format!("[ \"$1\" = ls ] && echo x >> \"$(dirname \"$0\")/count\"\n{LISTS}"),
        );
        let key = vec![lossy(&counted)];
        assert_eq!(
            detect_cached(key.clone(), Vec::new()).binary,
            Some(key[0].clone())
        );
        assert_eq!(
            detect_cached(key.clone(), Vec::new()).binary,
            Some(key[0].clone())
        );
        let count = std::fs::read_to_string(dir.path().join("counted/count")).unwrap();
        assert_eq!(count.lines().count(), 1);

        let late = script(
            &dir.path().join("late/paseo"),
            &format!("[ -f \"$(dirname \"$0\")/ok\" ] || exit 1\n{LISTS}"),
        );
        let key = vec![lossy(&late)];
        assert_eq!(detect_cached(key.clone(), Vec::new()).binary, None);
        std::fs::write(dir.path().join("late/ok"), "").unwrap();
        assert_eq!(
            detect_cached(key.clone(), Vec::new()).binary,
            Some(key[0].clone())
        );
    }

    #[cfg(unix)]
    #[test]
    fn paseo_is_available_when_forced_on_or_a_cli_is_detected_and_off_when_forced_off() {
        let dir = tempfile::tempdir().unwrap();
        let working = script(&dir.path().join("cfg/paseo"), LISTS);
        let config = |enabled, binary: &Path| PaseoConfig {
            enabled,
            binary: lossy(binary),
            ..PaseoConfig::default()
        };
        let missing = dir.path().join("missing/paseo");
        assert!(paseo_available(&config(Some(true), &missing)));
        assert!(!paseo_available(&config(Some(false), &working)));
        assert!(paseo_unavailable(&config(Some(false), &working))
            .unwrap()
            .contains("`enabled = false`"));
        assert!(paseo_available(&config(None, &working)));
        assert_eq!(
            detect_binary(&config(None, &working)).binary,
            Some(lossy(&working))
        );
    }

    #[test]
    #[ignore]
    fn real_host_detects_a_paseo_cli() {
        let found = detect_binary(&PaseoConfig::default());
        println!("{found:?}");
        assert!(found.binary.is_some(), "{}", found.unavailable());
    }

    #[test]
    fn child_processes_drop_the_caller_agent_and_keep_paseo_home() {
        let cmd = command("paseo", &["ls".into()]);
        let envs: Vec<_> = cmd.as_std().get_envs().collect();
        for key in ["PASEO_AGENT_ID", "PASEO_AGENT_CWD"] {
            assert!(
                envs.contains(&(std::ffi::OsStr::new(key), None)),
                "{envs:?}"
            );
        }
        assert!(!envs.iter().any(|(k, _)| *k == "PASEO_HOME"), "{envs:?}");
        assert_eq!(envs.len(), 2, "{envs:?}");
    }

    #[tokio::test]
    async fn the_fake_scripts_turn_ends_statuses_and_permissions_and_records_calls() {
        let fake = FakePaseo::default();
        fake.add_leader("proj-1", "a", "idle", false);
        fake.push_turn_end(TurnEnd::Permission);
        fake.set_status("a", "running");
        fake.set_permissions("a", &["Write"]);
        let turn = Duration::from_secs(1);
        assert_eq!(
            fake.send_and_wait("a", "go", turn).await.unwrap(),
            TurnEnd::Permission
        );
        let state = fake.inspect("a").await.unwrap();
        assert_eq!(state.status, "running");
        assert_eq!(state.pending_permissions, 1);
        assert_eq!(state.permissions[0].tool, "Write");
        assert_eq!(fake.wait("a", turn).await.unwrap(), TurnEnd::Completed);
        assert_eq!(
            fake.calls(),
            vec![
                Call::SendAndWait("a".into(), "go".into()),
                Call::Inspect("a".into()),
                Call::Wait("a".into()),
            ]
        );
    }

    macro_rules! fixture {
        ($name:literal) => {
            include_str!(concat!("../testdata/fixtures/paseo-0.11.2/", $name))
        };
    }

    #[test]
    fn blocking_run_and_send_fixtures_end_their_turn_completed() {
        assert_eq!(
            parse_turn_end(fixture!("run-blocking-cli.out")).unwrap(),
            (
                "9950faad-b964-46a9-ac31-121939d43222".to_string(),
                TurnEnd::Completed
            )
        );
        for out in [
            fixture!("send-blocking.out"),
            fixture!("send-while-busy.out"),
        ] {
            assert_eq!(parse_turn_end(out).unwrap().1, TurnEnd::Completed);
        }
        assert_eq!(
            parse_turn_end(fixture!("send-perm.out")).unwrap().1,
            TurnEnd::Permission
        );
        assert_eq!(
            parse_turn_end(fixture!("send-nowait.out")).unwrap().1,
            TurnEnd::Other {
                status: "sent".into(),
                message: "Message sent, not waiting for completion".into()
            }
        );
        assert!(parse_turn_end(fixture!("send-killed.out")).is_err());
    }

    #[test]
    fn wait_fixture_reports_idle_with_the_last_activity() {
        match parse_turn_end(fixture!("wait.out")).unwrap().1 {
            TurnEnd::Other { status, message } => {
                assert_eq!(status, "idle");
                assert!(message.starts_with("Agent is idle.\n"), "{message}");
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn the_after_permission_wait_ends_at_the_turn_not_the_allow() {
        let allowed = parse_inspect(fixture!("inspect-after-allow.out")).unwrap();
        assert_eq!(allowed.status, "running");
        assert_eq!(allowed.pending_permissions, 0);
        assert!(allowed.permissions.is_empty());
        match parse_turn_end(fixture!("wait-after-allow.out")).unwrap().1 {
            TurnEnd::Other { status, message } => {
                assert_eq!(status, "idle");
                assert!(message.contains("PERM-DONE"), "{message}");
            }
            other => panic!("expected Other, got {other:?}"),
        }
        assert_eq!(
            parse_turn_end(fixture!("wait-second-permission.out"))
                .unwrap()
                .1,
            TurnEnd::Permission
        );
    }

    #[test]
    fn inspect_fixtures_name_status_and_pending_permissions() {
        let pending = parse_inspect(fixture!("inspect-pending.out")).unwrap();
        assert_eq!(pending.status, "running");
        assert_eq!(pending.pending_permissions, 1);
        assert_eq!(
            pending.permissions,
            vec![PendingPermission {
                id: "permission-048112ac-89df-4631-ae6e-f0509a0980a1".into(),
                tool: "Write".into(),
            }]
        );
        let running = parse_inspect(fixture!("inspect-running.out")).unwrap();
        assert_eq!(running.status, "running");
        assert!(running.permissions.is_empty());
        let permits: Vec<serde_json::Value> = parse_document(fixture!("permit-ls.out")).unwrap();
        assert_eq!(permits.len(), 1);
        assert_eq!(permits[0]["name"], "Write");
    }

    #[test]
    fn the_inside_agent_wrapper_prints_only_noise_including_multi_line_objects() {
        let out = fixture!("run-blocking.out");
        assert_eq!(strip_noise(out), "");
        assert!(parse_turn_end(out).is_err());
        assert_eq!(last_id(out), None);
        let followed = format!("{out}{}", fixture!("send-blocking.out"));
        assert_eq!(parse_turn_end(&followed).unwrap().1, TurnEnd::Completed);
        assert_eq!(last_id(&format!("{out}wks_9\n")).as_deref(), Some("wks_9"));
    }

    #[test]
    fn the_systemd_wrapper_lists_agents_after_its_noise_lines() {
        let ids = parse_leaders(fixture!("wrapper-systemd-ls.out")).unwrap();
        assert_eq!(ids.len(), 5);
        assert_eq!(ids[0], "35eca12c-e112-4690-bf48-af0222f659e1");
        assert!(strip_noise(fixture!("wrapper-systemd-ls.out")).starts_with('['));
        assert!(parse_leaders(fixture!("wrapper-systemd-ls.err")).is_err());
    }

    #[test]
    fn the_npm_cli_fixtures_parse_past_their_leading_line() {
        assert_eq!(
            parse_turn_end(fixture!("npm-run-blocking.out")).unwrap(),
            (
                "e7baf63c-e456-4359-b56e-6f4a19faab8c".to_string(),
                TurnEnd::Permission
            )
        );
        let workspaces: Vec<Workspace> = parse_document(fixture!("npm-workspace-ls.out")).unwrap();
        assert_eq!(workspaces[0].id, "wks_02145c30b3744429");
        assert!(workspaces.iter().all(|w| w.isolation == "local"));
    }

    #[test]
    fn provider_ls_keeps_available_enabled_providers_in_listed_order() {
        assert_eq!(
            parse_providers(fixture!("npm-provider-ls.out")).unwrap(),
            vec!["claude", "codex", "pi", "antigravity"]
        );
    }

    #[test]
    fn a_json_error_object_names_its_code_and_message() {
        assert_eq!(
            reported_error(fixture!("npm-run-missing-provider.err")),
            Some(("MISSING_PROVIDER".into(), "Provider is required".into()))
        );
        for out in [
            fixture!("npm-run-blocking.out"),
            fixture!("npm-provider-ls.out"),
            fixture!("inspect-running.out"),
            fixture!("wrapper-systemd-ls.err"),
            fixture!("run-blocking.out"),
        ] {
            assert_eq!(reported_error(out), None);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_failing_command_surfaces_the_paseo_error_instead_of_its_exit_status() {
        let dir = tempfile::tempdir().unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/fixtures/paseo-0.11.2/npm-run-missing-provider.err");
        let bin = script(
            &dir.path().join("paseo"),
            &format!("cat '{}' >&2\nexit 1", fixture.display()),
        );
        let err = PaseoCli::with_binary(&lossy(&bin))
            .run_and_wait(&spec("", "", "", "t"), Duration::from_secs(5))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("MISSING_PROVIDER"), "{err}");
        assert!(err.contains("Provider is required"), "{err}");
        assert!(!err.contains("exit status"), "{err}");
    }

    #[test]
    fn logs_fixtures_keep_every_line_and_show_the_interrupted_turn() {
        let busy = fixture!("logs-after-busy.out");
        assert_eq!(strip_noise(busy), busy);
        assert!(busy.lines().any(|l| l == "SHORT-DONE"));
        assert!(!busy.lines().any(|l| l == "LONG-DONE"));
        let killed = fixture!("logs-after-kill.out");
        assert_eq!(strip_noise(killed), killed);
        assert!(killed
            .lines()
            .last()
            .unwrap()
            .starts_with("[Task notification]"));
    }

    #[tokio::test]
    #[ignore]
    async fn real_paseo_cli_lists_workspaces_and_leaders() {
        let cli = PaseoCli::new(&PaseoConfig::default());
        let workspaces = cli.workspaces().await.unwrap();
        let leaders = cli
            .leaders("waggledance-probe-no-such-project")
            .await
            .unwrap();
        println!("workspaces: {workspaces:?}");
        println!("leaders: {leaders:?}");
        assert!(leaders.is_empty());
    }
}
