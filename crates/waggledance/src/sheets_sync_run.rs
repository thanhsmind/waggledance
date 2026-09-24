//! Google Sheets task sync runner (D4, D6, D7).
//!
//! Coordinates two-way synchronization between a project's backlog tasks/cells
//! and a dedicated Google Sheets tab for the project.

use crate::gsheets::{GSheetsError, SheetsClient};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use waggledance_core::bee::BeeCell;
use waggledance_core::config::SheetsConfig;
use waggledance_core::domain::Project;
use waggledance_core::sheets_sync::{
    self, RowKind, SheetRow, SyncOp, SyncSnapshot,
};

/// Detailed summary of a Sheets sync run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncReport {
    pub project: String,
    pub tab: String,
    pub added: usize,
    pub amended: usize,
    pub status_changed: usize,
    pub conflicts: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

impl SyncReport {
    /// Format a concise, human-readable one-line summary.
    pub fn summary(&self) -> String {
        format!(
            "{}: tab '{}': {} added, {} amended, {} status changed, {} conflicts, {} skipped, {} errors",
            self.project,
            self.tab,
            self.added,
            self.amended,
            self.status_changed,
            self.conflicts.len(),
            self.skipped.len(),
            self.errors.len(),
        )
    }
}

/// Errors produced during Sheets sync.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Google Sheets sync is not configured: spreadsheet_id and credentials_path must be set under [sheets]")]
    NotConfigured(String),
    #[error("Project '{0}' is not set up with bee (.bee/bin/bee missing)")]
    NoBee(String),
    #[error("Sync is already running for project '{0}'")]
    AlreadyRunning(String),
    #[error("Google Sheets error: {0}")]
    GSheets(#[from] GSheetsError),
    #[error("IO error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Exclusive per-project file lock guard. Removes the lock file when dropped in all paths.
pub struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn acquire_lock(lock_path: &Path, project_id: &str) -> Result<LockGuard, SyncError> {
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SyncError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(mut f) => {
            let now_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(f, "{now_secs}");
            Ok(LockGuard {
                path: lock_path.to_path_buf(),
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let is_stale = std::fs::metadata(lock_path)
                .and_then(|m| m.modified())
                .map(|mtime| {
                    SystemTime::now()
                        .duration_since(mtime)
                        .unwrap_or_default()
                        > Duration::from_secs(10 * 60)
                })
                .unwrap_or(false);

            if is_stale {
                let _ = std::fs::remove_file(lock_path);
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(lock_path)
                {
                    Ok(mut f) => {
                        let now_secs = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let _ = writeln!(f, "{now_secs}");
                        Ok(LockGuard {
                            path: lock_path.to_path_buf(),
                        })
                    }
                    Err(_) => Err(SyncError::AlreadyRunning(project_id.to_string())),
                }
            } else {
                Err(SyncError::AlreadyRunning(project_id.to_string()))
            }
        }
        Err(e) => Err(SyncError::Io {
            path: lock_path.to_path_buf(),
            source: e,
        }),
    }
}

/// Check if any other project's snapshot in `sync_dir` claims the given `target_sheet_id` (D7g).
fn check_sheet_id_claimed(
    sync_dir: &Path,
    current_project_id: &str,
    target_sheet_id: i64,
) -> bool {
    let Ok(entries) = std::fs::read_dir(sync_dir) else {
        return false;
    };
    let current_file_name = format!("{current_project_id}.json");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name != current_file_name {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(snap) = serde_json::from_str::<SyncSnapshot>(&content) {
                            if snap.sheet_id == Some(target_sheet_id) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Synchronize a project's backlog tasks and cells with its Google Sheets tab.
pub async fn sync_project(
    project: &Project,
    cfg: &SheetsConfig,
    client: &SheetsClient,
    state_dir: &Path,
) -> Result<SyncReport, SyncError> {
    // 1. Refuse with a setup hint when cfg is not configured or project has no bee binary
    if !cfg.is_configured() {
        return Err(SyncError::NotConfigured(
            "Google Sheets sync is not configured: spreadsheet_id and credentials_path must be set under [sheets]".to_string(),
        ));
    }
    let Some(bee_bin) = crate::server::project_bee_binary(&project.root_path) else {
        return Err(SyncError::NoBee(format!(
            "project '{}' has no vendored bee binary in .bee/bin/",
            project.name
        )));
    };

    // 2. Take exclusive per-project lock file
    let sync_dir = state_dir.join("sheets-sync");
    std::fs::create_dir_all(&sync_dir).map_err(|e| SyncError::Io {
        path: sync_dir.clone(),
        source: e,
    })?;
    let lock_path = sync_dir.join(format!("{}.lock", project.id));
    let _lock = acquire_lock(&lock_path, &project.id)?;

    // 3. Load snapshot from state_dir/sheets-sync/<project_id>.json
    let snap_path = sync_dir.join(format!("{}.json", project.id));
    let snapshot: Option<SyncSnapshot> = if snap_path.is_file() {
        std::fs::read_to_string(&snap_path)
            .ok()
            .and_then(|s| serde_json::from_str::<SyncSnapshot>(&s).ok())
            .filter(|s| s.spreadsheet_id == cfg.spreadsheet_id) // D7l: ignore snapshot for different spreadsheet
    } else {
        None
    };

    // 4. Find tab by stored sheet_id, else by title only if no other <id>.json claims that sheet_id,
    //    else add_tab(unique_tab_title) (D6, D7g)
    let tabs = client.list_tabs(&cfg.spreadsheet_id).await?;
    let mut tab_found: Option<(i64, String)> = None;

    if let Some(snap) = &snapshot {
        if let Some(stored_id) = snap.sheet_id {
            if let Some((_, title)) = tabs.iter().find(|(id, _)| *id == stored_id) {
                tab_found = Some((stored_id, title.clone()));
            }
        }
    }

    if tab_found.is_none() {
        if let Some((sheet_id, title)) = tabs.iter().find(|(_, t)| t == &project.name) {
            let sheet_id = *sheet_id;
            let is_claimed = check_sheet_id_claimed(&sync_dir, &project.id, sheet_id);
            if !is_claimed {
                tab_found = Some((sheet_id, title.clone()));
            }
        }
    }

    let (sheet_id, tab_title) = match tab_found {
        Some((id, title)) => (id, title),
        None => {
            let existing_titles: Vec<String> = tabs.iter().map(|(_, t)| t.clone()).collect();
            let unique_title =
                sheets_sync::unique_tab_title(&project.name, &existing_titles);
            let new_id = client
                .add_tab(&cfg.spreadsheet_id, &unique_title)
                .await?;
            (new_id, unique_title)
        }
    };

    // 5. Read values from sheet
    let values = client
        .read_values(&cfg.spreadsheet_id, &tab_title)
        .await?;
    let previous_row_count = values.len();
    let sheet_rows = sheets_sync::parse_rows(&values);

    // 6. Read bee snapshot for PBIs
    let bee_snap = waggledance_core::bee::read_snapshot(&project.root_path);
    let pbis = &bee_snap.backlog.pbis;

    // 7. Plan sync
    let plan = sheets_sync::plan_sync(
        pbis,
        &sheet_rows,
        snapshot.as_ref(),
        &cfg.spreadsheet_id,
    );

    // 8. Apply ops by running project's bee CLI
    let mut added = 0;
    let mut amended = 0;
    let mut status_changed = 0;
    let mut errors = Vec::new();
    let mut failed_adds: Vec<SheetRow> = Vec::new();
    let mut failed_fields: Vec<(String, String, String)> = Vec::new();

    for op in plan.ops {
        match op {
            SyncOp::Add {
                row_index,
                title,
                cos,
                status,
            } => {
                let mut cmd = tokio::process::Command::new(&bee_bin);
                cmd.arg("backlog").arg("pbi").arg("add");
                cmd.arg("--title").arg(&title);
                if !cos.is_empty() {
                    cmd.arg("--cos").arg(&cos);
                }
                cmd.arg("--status").arg(&status);
                cmd.arg("--json");
                cmd.current_dir(&project.root_path)
                    .stdin(std::process::Stdio::null());

                let out = match cmd.output().await {
                    Ok(out) => out,
                    Err(e) => {
                        errors.push(format!("failed to spawn bee backlog pbi add for '{title}': {e}"));
                        failed_adds.push(SheetRow {
                            sheet_index: row_index,
                            id: String::new(),
                            kind: RowKind::Task,
                            title,
                            status,
                            description: cos,
                            feature: String::new(),
                        });
                        continue;
                    }
                };

                if !out.status.success() {
                    let err = crate::server::stderr_tail(&out.stderr);
                    errors.push(format!("bee backlog pbi add failed for '{title}': {err}"));
                    failed_adds.push(SheetRow {
                        sheet_index: row_index,
                        id: String::new(),
                        kind: RowKind::Task,
                        title,
                        status,
                        description: cos,
                        feature: String::new(),
                    });
                    continue;
                }

                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Some(new_id) = crate::server::parse_pbi_id(&stdout) {
                    added += 1;
                    // Write ID back to cell A{row_index + 1} immediately (D7a)
                    let a1 = format!("A{}", row_index + 1);
                    if let Err(e) = client
                        .write_cell(&cfg.spreadsheet_id, &tab_title, &a1, &new_id)
                        .await
                    {
                        errors.push(format!(
                            "failed to write ID '{new_id}' back to cell {a1}: {e}"
                        ));
                    }
                } else {
                    errors.push(format!(
                        "bee backlog pbi add for '{title}' created PBI but its id could not be read"
                    ));
                }
            }
            SyncOp::Amend { id, title, cos } => {
                let mut cmd = tokio::process::Command::new(&bee_bin);
                cmd.arg("backlog").arg("pbi").arg("amend");
                cmd.arg("--id").arg(&id);
                if let Some(t) = &title {
                    cmd.arg("--title").arg(t);
                }
                if let Some(c) = &cos {
                    cmd.arg("--cos").arg(c);
                }
                cmd.current_dir(&project.root_path)
                    .stdin(std::process::Stdio::null());

                let out = match cmd.output().await {
                    Ok(out) => out,
                    Err(e) => {
                        errors.push(format!("failed to spawn bee backlog pbi amend for '{id}': {e}"));
                        if let Some(t) = title {
                            failed_fields.push((id.clone(), "title".to_string(), t));
                        }
                        if let Some(c) = cos {
                            failed_fields.push((id.clone(), "cos".to_string(), c));
                        }
                        continue;
                    }
                };

                if !out.status.success() {
                    let err = crate::server::stderr_tail(&out.stderr);
                    errors.push(format!("bee backlog pbi amend failed for '{id}': {err}"));
                    if let Some(t) = title {
                        failed_fields.push((id.clone(), "title".to_string(), t));
                    }
                    if let Some(c) = cos {
                        failed_fields.push((id.clone(), "cos".to_string(), c));
                    }
                    continue;
                }

                amended += 1;
            }
            SyncOp::SetStatus { id, to } => {
                let mut cmd = tokio::process::Command::new(&bee_bin);
                cmd.arg("backlog").arg("pbi").arg("status");
                cmd.arg("--id").arg(&id);
                cmd.arg("--to").arg(&to);
                cmd.current_dir(&project.root_path)
                    .stdin(std::process::Stdio::null());

                let out = match cmd.output().await {
                    Ok(out) => out,
                    Err(e) => {
                        errors.push(format!("failed to spawn bee backlog pbi status for '{id}': {e}"));
                        failed_fields.push((id, "status".to_string(), to));
                        continue;
                    }
                };

                if !out.status.success() {
                    let err = crate::server::stderr_tail(&out.stderr);
                    errors.push(format!("bee backlog pbi status failed for '{id}': {err}"));
                    failed_fields.push((id, "status".to_string(), to));
                    continue;
                }

                status_changed += 1;
            }
        }
    }

    // 10. Re-read bee and read_archived_cells per feature only now (D7l note)
    let fresh_bee = waggledance_core::bee::read_snapshot(&project.root_path);
    let cells_for = |feature: &str| -> Vec<BeeCell> {
        let mut cells = Vec::new();
        let b = &fresh_bee.buckets;
        for c in b.doing.iter().chain(&b.waiting).chain(&b.stuck).chain(&b.done) {
            if c.feature == feature {
                cells.push(c.clone());
            }
        }
        let archived = waggledance_core::bee::read_archived_cells(&project.root_path, feature);
        cells.extend(archived);
        cells
    };

    // 11. Render rows
    let (mut rows, _) =
        sheets_sync::render_rows_with_indices(&fresh_bee.backlog.pbis, cells_for);

    // 12. Overlay failed ops (D7d)
    sheets_sync::overlay_failed(&mut rows, &failed_adds, &failed_fields);
    let task_indices = sheets_sync::task_row_indices(&rows);

    // 13. Replace values with previous row count padding
    client
        .replace_values(&cfg.spreadsheet_id, &tab_title, &rows, previous_row_count)
        .await?;

    // 14. Set status dropdown on task rows
    client
        .set_status_dropdown(&cfg.spreadsheet_id, sheet_id, &task_indices)
        .await?;

    // 15. Save snapshot atomically (write temp + rename)
    let keep_sheet: Vec<(&str, &str)> = failed_fields
        .iter()
        .map(|(id, f, _)| (id.as_str(), f.as_str()))
        .collect();
    let new_snapshot = sheets_sync::snapshot_from(
        &cfg.spreadsheet_id,
        Some(sheet_id),
        Some(tab_title.clone()),
        &fresh_bee.backlog.pbis,
        snapshot.as_ref(),
        &keep_sheet,
    );

    let snap_json = serde_json::to_string_pretty(&new_snapshot)?;
    let snap_tmp = sync_dir.join(format!("{}.json.tmp", project.id));
    std::fs::write(&snap_tmp, snap_json).map_err(|e| SyncError::Io {
        path: snap_tmp.clone(),
        source: e,
    })?;
    std::fs::rename(&snap_tmp, &snap_path).map_err(|e| SyncError::Io {
        path: snap_path.clone(),
        source: e,
    })?;

    Ok(SyncReport {
        project: project.name.clone(),
        tab: tab_title,
        added,
        amended,
        status_changed,
        conflicts: plan.conflicts,
        skipped: plan.skipped,
        errors,
    })
}

#[cfg(test)]
pub(crate) use tests::TEST_PKCS8_PEM;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path as AxPath;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Json};
    use axum::routing::{get, post};
    use axum::Router;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub(crate) const TEST_PKCS8_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC5w1is4/YKniI0
+/EuH5b248KiYb5KigbWtNWENpd95hUqGiw4Bni/lC/amlFV3jnOON9uu0aBDQgH
ZODrzdhcYXAGMn/Cz9mtjggM3JxM2fdDDojDSP9fiCqMy69VXlpTW6n9uBbKYo8l
oLs5L70YYT0zwBulUwBtY0wMeccsrS8AB3yyXyiRm/tP/Giapb/TEJl/2WMWPkJL
B1JdaJZf/8SMRx0hvpeFuBCV52iiouLUaMHR+tgapHEEv+TEqmHbBWwfczVmNiHX
maJ7mdAax8/B5xdoLXPcqYKCUHJwg4EgCtS8hq0Rofvy5/f4v1wXLVu3A30DWQJ1
l7jBzr1nAgMBAAECggEAUcfCXjrtsrbzcEgKYbfeikqzATHuyHoDkLqD1XUDNJX6
c1KabP/MYOsoNiEB9b+jyzoWg3VHEMFf64Fp0+YHfoQXAWNyA53lwxZoTOFCnkLH
LGpVo0Obyb70nI9/XSHwugn8yIom3mqNgo9EXWS65uTIQOQuR5ZG9eFlNYjXOr8x
TJsdaFFCDjxaoqt8s1iWl1FoTXB2gc0E4Y9xMeDPzLfyHu4Tj0jS3O1cC2lD1rzu
fp+f9lue2aKtSuyE62TahcVVlSkNorRBBEkFdQDIzA3dejldUU8UZfn6Vy23j5jZ
/WEBGyIy1xQPSI80GlEvGh+jCZ9Jnz+Y2bE1XbWuoQKBgQD+U9Wy6NcalcNGfeQ0
3sUD3NA5FY/RABPDLN5g01d1tBZ7C5Nl7PWNyvzSFSOhsZ0XSZPzZ11SG//M27Mh
xBCIWkrTtHyuoZRu+augpu3pLYgtYR6Mc6eZAuAjouYAO/e9sRkYAvuig78huN1w
AdaQrLllvAcKFS76JUnzErl/1QKBgQC6/BUFs6447tXS4/+38zKYitGccQYjJ9hP
W+ByCs/qNsqrFCrWo7pulKWoHmgAuLnwWdWQbkZPZc2ktslmehKTGZeJGU8oyfBH
gBuQEAdiEyE9cIQwtk2Dx+C4ntIgvZxAu+YS7q4W28Oibbrq+OKtboKbyRCCIjgo
Ppxb0owiSwKBgQCfaf8eKJKECHGhYtHDTgth82vVDC3cTFcW1K4DNRWyIpKgx825
asE0ApNcP4HZHVKhQS5quFSnXgOF7gQnWxUd0rLqZkxszb0Tno20ok2oBjFAeIrE
cM4Ru9qwm6/dX7lW/oYDJgOCh2ArjAvI5G4QuyNuoGAtMsACbZMfuJHrjQKBgHaQ
aXQYHx0StJro37lwiGCZhZhBQrcACPUpOQSXPocw/wnD2/YGE60SrIgR37i1aZGy
JO1UP1kRIHiVPtewCB8mVyQj3EHs6mMsJE4RG91ga+9dbTTGJBivoRrv+iWwlWem
vZi7p7/u0foowiFEd9eo/0BxGfqqYBmEufivZRRJAoGBAOIq1N3FRncJ0RDbq6du
UrcBjX1JqiNhZ/CWZXfLTjKbGCuDb7F9RwXjIVHzX5LA38M1mVaFR226ewBsE2Zf
3i+mY8Z1GR2tTlC6wNO+2enqY5zsBLv3di27mQm2UL/5ry2jO+LERXZl9XM1pAj+
sgjI3687NnKVf2hyV4epQthL
-----END PRIVATE KEY-----
"#;

    fn test_key() -> crate::gsheets::ServiceAccountKey {
        crate::gsheets::ServiceAccountKey {
            client_email: "test-agent@example.iam.gserviceaccount.com".to_string(),
            private_key: TEST_PKCS8_PEM.to_string(),
            token_uri: "http://127.0.0.1:0/token".to_string(),
        }
    }

    #[derive(Clone, Default)]
    struct MockSheetsState {
        tabs: Arc<Mutex<Vec<(i64, String)>>>,
        values: Arc<Mutex<HashMap<String, Vec<Vec<String>>>>>,
        write_cell_calls: Arc<Mutex<Vec<(String, String, String)>>>, // (tab, a1, val)
        fail_replace_values: Arc<AtomicBool>,
    }

    async fn start_mock_server(state: MockSheetsState) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route(
                "/token",
                post(|| async {
                    Json(serde_json::json!({
                        "access_token": "mock-token",
                        "expires_in": 3600,
                        "token_type": "Bearer"
                    }))
                }),
            )
            .route(
                "/spreadsheets/:id",
                get({
                    let st = state.clone();
                    move |AxPath(_id): AxPath<String>| {
                        let st = st.clone();
                        async move {
                            let tabs = st.tabs.lock().await;
                            let sheets_json: Vec<serde_json::Value> = tabs
                                .iter()
                                .map(|(sheet_id, title)| {
                                    serde_json::json!({
                                        "properties": {
                                            "sheetId": sheet_id,
                                            "title": title
                                        }
                                    })
                                })
                                .collect();
                            Json(serde_json::json!({ "sheets": sheets_json }))
                        }
                    }
                })
                .post({
                    let st = state.clone();
                    move |AxPath(_id): AxPath<String>, body: String| {
                        let st = st.clone();
                        async move {
                            let json: serde_json::Value =
                                serde_json::from_str(&body).unwrap_or_default();
                            let mut replies = Vec::new();

                            if let Some(requests) = json.get("requests").and_then(|r| r.as_array()) {
                                for req in requests {
                                    if let Some(add_sheet) = req.get("addSheet") {
                                        let title = add_sheet
                                            .get("properties")
                                            .and_then(|p| p.get("title"))
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("Untitled")
                                            .to_string();

                                        let mut tabs = st.tabs.lock().await;
                                        let new_id = (tabs.len() as i64) + 100;
                                        tabs.push((new_id, title.clone()));
                                        replies.push(serde_json::json!({
                                            "addSheet": {
                                                "properties": {
                                                    "sheetId": new_id,
                                                    "title": title
                                                }
                                            }
                                        }));
                                    } else {
                                        replies.push(serde_json::json!({}));
                                    }
                                }
                            }

                            Json(serde_json::json!({ "replies": replies }))
                        }
                    }
                }),
            )
            .route(
                "/spreadsheets/:id/values/*range",
                get({
                    let st = state.clone();
                    move |AxPath((_id, range)): AxPath<(String, String)>| {
                        let st = st.clone();
                        async move {
                            let clean_range = range.trim_start_matches('/');
                            let tab_title = extract_tab_title(clean_range);
                            let values_map = st.values.lock().await;
                            let rows = values_map.get(&tab_title).cloned().unwrap_or_default();
                            Json(serde_json::json!({
                                "range": clean_range,
                                "majorDimension": "ROWS",
                                "values": rows
                            }))
                        }
                    }
                })
                .put({
                    let st = state.clone();
                    move |AxPath((_id, range)): AxPath<(String, String)>, body: String| {
                        let st = st.clone();
                        async move {
                            let clean_range = range.trim_start_matches('/');
                            let tab_title = extract_tab_title(clean_range);
                            let payload: serde_json::Value =
                                serde_json::from_str(&body).unwrap_or_default();

                            let is_write_cell = clean_range.contains("!A") && !clean_range.ends_with("!A1") && !clean_range.contains(':');
                            if !is_write_cell && st.fail_replace_values.load(Ordering::SeqCst) {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(serde_json::json!({
                                        "error": { "message": "Simulated rewrite failure" }
                                    })),
                                )
                                    .into_response();
                            }

                            let incoming_rows: Vec<Vec<String>> = payload
                                .get("values")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .map(|row| {
                                            row.as_array()
                                                .map(|cells| {
                                                    cells
                                                        .iter()
                                                        .map(|c| c.as_str().unwrap_or("").to_string())
                                                        .collect()
                                                })
                                                .unwrap_or_default()
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            let mut values_map = st.values.lock().await;
                            // Check if this is a single cell update (e.g. A2)
                            if clean_range.contains("!A") && !clean_range.ends_with("!A1") && !clean_range.contains(':') {
                                // Single cell write
                                let a1 = clean_range.split('!').nth(1).unwrap_or("");
                                let cell_val = incoming_rows
                                    .first()
                                    .and_then(|r| r.first())
                                    .cloned()
                                    .unwrap_or_default();
                                st.write_cell_calls
                                    .lock()
                                    .await
                                    .push((tab_title.clone(), a1.to_string(), cell_val.clone()));

                                // If the tab already has rows, update that specific cell
                                if let Some(rows) = values_map.get_mut(&tab_title) {
                                    if let Ok(row_num) = a1.trim_start_matches('A').parse::<usize>() {
                                        if row_num > 0 && row_num <= rows.len() {
                                            if let Some(r) = rows.get_mut(row_num - 1) {
                                                if !r.is_empty() {
                                                    r[0] = cell_val;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Full replace
                                values_map.insert(tab_title, incoming_rows.clone());
                            }

                            (StatusCode::OK, Json(serde_json::json!({ "updatedRows": incoming_rows.len() }))).into_response()
                        }
                    }
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{addr}"), handle)
    }

    fn extract_tab_title(range: &str) -> String {
        let unquoted = range.split('!').next().unwrap_or(range);
        let unquoted = unquoted.trim_matches('\'');
        unquoted.replace("''", "'")
    }

    fn setup_test_project(name: &str, pbis: &[(&str, &str, &str, &str, &str)]) -> (tempfile::TempDir, Project) {
        let temp = tempfile::tempdir().unwrap();
        let bee_bin_dir = temp.path().join(".bee").join("bin");
        std::fs::create_dir_all(&bee_bin_dir).unwrap();

        let bee_script = bee_bin_dir.join("bee");
        let script_content = r#"#!/bin/sh
dir="$(cd "$(dirname "$0")/../.." && pwd)"
log_file="$dir/bee_argv.log"
fail_file="$dir/bee_fail"
no_id_file="$dir/bee_no_id"
echo "$@" >> "$log_file"

if [ -f "$fail_file" ]; then
    echo "simulated bee failure" >&2
    exit 1
fi

cmd="$1"
sub="$2"
act="$3"

if [ "$cmd" = "backlog" ] && [ "$sub" = "pbi" ]; then
    if [ "$act" = "add" ]; then
        if [ -f "$no_id_file" ]; then
            echo '{"kind":"pbi","id":"p-added-no-id","title":"Task Without Output ID","status":"proposed","cos":"","feature":""}' >> "$dir/.bee/backlog.jsonl"
            echo "created task successfully but no json id"
            exit 0
        fi
        echo '{"id": "p-test-add-1", "title": "Added task"}'
        exit 0
    elif [ "$act" = "amend" ]; then
        exit 0
    elif [ "$act" = "status" ]; then
        exit 0
    fi
fi
exit 0
"#;
        std::fs::write(&bee_script, script_content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bee_script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Write .bee/backlog.jsonl
        let mut backlog_lines = Vec::new();
        for &(id, title, status, cos, feature) in pbis {
            let row = serde_json::json!({
                "kind": "pbi",
                "id": id,
                "title": title,
                "status": status,
                "cos": cos,
                "feature": feature,
            });
            backlog_lines.push(row.to_string());
        }
        std::fs::write(
            temp.path().join(".bee").join("backlog.jsonl"),
            backlog_lines.join("\n"),
        )
        .unwrap();

        let project = Project {
            id: format!("proj-{}", name.to_lowercase().replace(' ', "-")),
            name: name.to_string(),
            root_path: temp.path().to_path_buf(),
            created_at: "2026-09-24T00:00:00Z".to_string(),
            last_seen_at: "2026-09-24T00:00:00Z".to_string(),
            orchestration_enabled: false,
        };

        (temp, project)
    }

    #[tokio::test]
    async fn test_sheets_sync_first_sync() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project(
            "FirstSyncProj",
            &[("p-1", "Initial PBI", "proposed", "cos1", "feat1")],
        );
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };

        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        let report = sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .expect("first sync succeeds");

        assert_eq!(report.added, 0);
        assert_eq!(report.amended, 0);
        assert_eq!(report.status_changed, 0);
        assert!(report.errors.is_empty(), "no errors on first sync: {:?}", report.errors);
        assert_eq!(report.tab, "FirstSyncProj");

        // Verify values on mock server
        let values_map = state.values.lock().await;
        let rows = values_map.get("FirstSyncProj").expect("tab rows written");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["ID", "Type", "Title", "Status", "Description", "Feature"]);
        assert_eq!(rows[1], vec!["p-1", "task", "Initial PBI", "proposed", "cos1", "feat1"]);

        // Verify snapshot written with spreadsheet_id
        let snap_file = state_dir.path().join("sheets-sync").join(format!("{}.json", project.id));
        assert!(snap_file.is_file());
        let snap_content = std::fs::read_to_string(&snap_file).unwrap();
        let snap: SyncSnapshot = serde_json::from_str(&snap_content).unwrap();
        assert_eq!(snap.spreadsheet_id, "sheet-123");
        assert!(snap.pbis.contains_key("p-1"));
    }

    #[tokio::test]
    async fn test_sheets_sync_amend() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project(
            "AmendProj",
            &[("p-1", "Original Title", "proposed", "cos1", "")],
        );
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // 1. Initial sync to establish snapshot
        sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();

        // 2. Edit title on the sheet
        {
            let mut values_map = state.values.lock().await;
            let rows = values_map.get_mut("AmendProj").unwrap();
            rows[1][2] = "Edited Title on Sheet".to_string();
        }

        // 3. Second sync
        let report = sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .expect("amend sync succeeds");

        assert_eq!(report.amended, 1);
        let log = std::fs::read_to_string(project.root_path.join("bee_argv.log")).unwrap();
        assert!(log.contains("backlog pbi amend --id p-1 --title Edited Title on Sheet"));
    }

    #[tokio::test]
    async fn test_sheets_sync_conflict() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project(
            "ConflictProj",
            &[("p-1", "Initial Title", "proposed", "cos1", "")],
        );
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // Initial sync
        sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();

        // Edit on sheet: "Sheet Title"
        {
            let mut values_map = state.values.lock().await;
            let rows = values_map.get_mut("ConflictProj").unwrap();
            rows[1][2] = "Sheet Title".to_string();
        }

        // Edit in bee: "Bee Title"
        let backlog_path = project.root_path.join(".bee").join("backlog.jsonl");
        let new_line = serde_json::json!({
            "kind": "pbi",
            "id": "p-1",
            "title": "Bee Title",
            "status": "proposed",
            "cos": "cos1",
            "feature": "",
        })
        .to_string();
        std::fs::write(&backlog_path, format!("{}\n{}", std::fs::read_to_string(&backlog_path).unwrap(), new_line)).unwrap();

        // Sync again -> conflict
        let report = sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();
        assert_eq!(report.conflicts.len(), 1);
        let conflict = &report.conflicts[0];
        assert!(conflict.contains("Sheet Title") && conflict.contains("Bee Title"), "conflict must name both values: {conflict}");
        assert_eq!(report.amended, 0);
    }

    #[tokio::test]
    async fn test_sheets_sync_id_writeback() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project(
            "WritebackProj",
            &[("p-1", "Existing PBI", "proposed", "cos1", "")],
        );
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // Initial sync
        sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();

        // Add ID-less row on the sheet
        {
            let mut values_map = state.values.lock().await;
            let rows = values_map.get_mut("WritebackProj").unwrap();
            rows.push(vec![
                "".to_string(),
                "task".to_string(),
                "Brand New Task".to_string(),
                "proposed".to_string(),
                "new desc".to_string(),
                "".to_string(),
            ]);
        }

        let report = sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();
        assert_eq!(report.added, 1);

        // Check write_cell calls: should have written "p-test-add-1" into cell A3 (row index 2 + 1)
        let write_calls = state.write_cell_calls.lock().await;
        assert_eq!(write_calls.len(), 1);
        assert_eq!(write_calls[0].0, "WritebackProj");
        assert_eq!(write_calls[0].1, "A3");
        assert_eq!(write_calls[0].2, "p-test-add-1");
    }

    #[tokio::test]
    async fn test_sheets_sync_second_sync_zero_adds() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project(
            "PartialFailProj",
            &[],
        );
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // First sync with no PBIs creates header
        sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();

        // Add ID-less row on the sheet
        {
            let mut values_map = state.values.lock().await;
            let rows = values_map.get_mut("PartialFailProj").unwrap();
            rows.push(vec![
                "".to_string(),
                "task".to_string(),
                "Failing Rewrite Task".to_string(),
                "proposed".to_string(),
                "desc".to_string(),
                "".to_string(),
            ]);
        }

        // Arm failure on replace_values
        state.fail_replace_values.store(true, Ordering::SeqCst);

        // Sync fails at step 13 (replace_values)
        let res = sync_project(&project, &cfg, &client, state_dir.path()).await;
        assert!(res.is_err(), "sync must fail when replace_values fails");

        // But write_cell succeeded before the failure! So the sheet now has the ID in column A
        {
            let values_map = state.values.lock().await;
            let rows = values_map.get("PartialFailProj").unwrap();
            assert_eq!(rows[1][0], "p-test-add-1");
        }

        // Record the added PBI in bee backlog
        let backlog_path = project.root_path.join(".bee").join("backlog.jsonl");
        let new_line = serde_json::json!({
            "kind": "pbi",
            "id": "p-test-add-1",
            "title": "Failing Rewrite Task",
            "status": "proposed",
            "cos": "desc",
            "feature": "",
        })
        .to_string();
        std::fs::write(&backlog_path, new_line).unwrap();

        // Disarm failure
        state.fail_replace_values.store(false, Ordering::SeqCst);

        // Second sync
        let report = sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .expect("second sync succeeds");

        assert_eq!(report.added, 0, "second sync must add 0 duplicate PBIs");
    }

    #[tokio::test]
    async fn test_sheets_sync_add_without_id_no_duplicate() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project("NoIdAddProj", &[]);
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // Initial sync creates header
        sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .unwrap();

        // Add ID-less row on the sheet
        {
            let mut values_map = state.values.lock().await;
            let rows = values_map.get_mut("NoIdAddProj").unwrap();
            rows.push(vec![
                "".to_string(),
                "task".to_string(),
                "Task Without Output ID".to_string(),
                "proposed".to_string(),
                "no id desc".to_string(),
                "".to_string(),
            ]);
        }

        // Arm stub bee to exit 0 printing no id
        std::fs::write(project.root_path.join("bee_no_id"), "no-id").unwrap();

        let report = sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .unwrap();

        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("created PBI but its id could not be read")),
            "must report that PBI was created without readable ID: {:?}",
            report.errors
        );

        // After the sync, the rewritten tab has no ID-less copy of that row
        let values_map = state.values.lock().await;
        let rows = values_map.get("NoIdAddProj").unwrap();
        for row in rows.iter().skip(1) {
            assert!(
                !row[0].is_empty(),
                "rewritten tab must not have an ID-less row, but found: {row:?}"
            );
        }
        assert!(
            rows.iter().any(|r| r[2] == "Task Without Output ID"),
            "the task row is present on the tab with its ID from bee: {rows:?}"
        );
        drop(values_map);

        // A second sync must issue zero adds and create no duplicate PBI
        let second_report = sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .unwrap();
        assert_eq!(second_report.added, 0, "second sync must add 0 duplicate PBIs");
    }

    #[tokio::test]
    async fn test_sheets_sync_failing_bee_op_overlay() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project(
            "OverlayProj",
            &[],
        );
        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // Initial sync
        sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();

        // Add ID-less row on sheet
        {
            let mut values_map = state.values.lock().await;
            let rows = values_map.get_mut("OverlayProj").unwrap();
            rows.push(vec![
                "".to_string(),
                "task".to_string(),
                "Will Fail In Bee".to_string(),
                "proposed".to_string(),
                "my description".to_string(),
                "".to_string(),
            ]);
        }

        // Arm failure in bee stub
        std::fs::write(project.root_path.join("bee_fail"), "fail").unwrap();

        let report = sync_project(&project, &cfg, &client, state_dir.path()).await.unwrap();
        assert!(!report.errors.is_empty(), "must report bee failure");
        assert_eq!(report.added, 0);

        // The sheet text must have been preserved on the tab via overlay_failed
        let values_map = state.values.lock().await;
        let rows = values_map.get("OverlayProj").unwrap();
        let preserved_row = rows.iter().find(|r| r[2] == "Will Fail In Bee");
        assert!(preserved_row.is_some(), "sheet text was preserved: {rows:?}");
        assert_eq!(preserved_row.unwrap()[0], ""); // still ID-less
    }

    #[tokio::test]
    async fn test_sheets_sync_two_same_name_projects() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp1, project1) = setup_test_project("SharedName", &[]);
        let (_temp2, mut project2) = setup_test_project("SharedName", &[]);
        project2.id = "proj-shared-name-second".to_string();

        let state_dir = tempfile::tempdir().unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        // Sync first project
        let r1 = sync_project(&project1, &cfg, &client, state_dir.path()).await.unwrap();
        assert_eq!(r1.tab, "SharedName");

        // Sync second project with same name
        let r2 = sync_project(&project2, &cfg, &client, state_dir.path()).await.unwrap();
        assert_eq!(r2.tab, "SharedName (2)");

        let tabs = state.tabs.lock().await;
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].1, "SharedName");
        assert_eq!(tabs[1].1, "SharedName (2)");
    }

    #[tokio::test]
    async fn test_sheets_sync_held_lock() {
        let state = MockSheetsState::default();
        let (base_url, _handle) = start_mock_server(state.clone()).await;

        let (_temp, project) = setup_test_project("LockProj", &[]);
        let state_dir = tempfile::tempdir().unwrap();

        // Create a lock file held by another process
        let sync_dir = state_dir.path().join("sheets-sync");
        std::fs::create_dir_all(&sync_dir).unwrap();
        let lock_file = sync_dir.join(format!("{}.lock", project.id));
        std::fs::write(&lock_file, "locked").unwrap();

        let cfg = SheetsConfig {
            spreadsheet_id: "sheet-123".to_string(),
            credentials_path: Some(PathBuf::from("/fake/path.json")),
        };
        let client = SheetsClient::new(test_key(), reqwest::Client::new())
            .with_endpoints(format!("{base_url}/token"), base_url);

        let err = sync_project(&project, &cfg, &client, state_dir.path())
            .await
            .expect_err("must fail on held lock");

        match err {
            SyncError::AlreadyRunning(ref pid) => {
                assert_eq!(pid, &project.id);
                assert_eq!(
                    err.to_string(),
                    format!("Sync is already running for project '{}'", project.id)
                );
            }
            other => panic!("expected AlreadyRunning, got: {other:?}"),
        }
    }
}
