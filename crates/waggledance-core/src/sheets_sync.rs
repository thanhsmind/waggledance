//! Google Sheets task sync pure domain model and merge engine (D1, D2, D5, D6, D7).
//!
//! Pure functions and data structures for two-way synchronization between
//! project backlog PBIs / work cells and a Google spreadsheet tab.
//! This module contains no network, I/O, or async runtime dependencies.

use crate::bee::{BeeCell, BeePbi};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Column headers for the Google Sheets task tab (Row 1).
pub const HEADER: [&str; 6] = ["ID", "Type", "Title", "Status", "Description", "Feature"];

/// The five valid PBI statuses recognized by bee backlog and Sheets sync.
pub const PBI_STATUSES: [&str; 5] = ["proposed", "in-flight", "parked", "done", "declined"];

/// Whether a row represents a top-level task (PBI) or a subtask (work cell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RowKind {
    #[default]
    Task,
    Subtask,
}

pub type SheetRowKind = RowKind;

/// One parsed row from the Google Sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SheetRow {
    /// 0-based index of this row within the sheet values table (header is row 0).
    pub sheet_index: usize,
    /// Column A: PBI ID (or cell ID for subtasks, empty for newly added tasks).
    pub id: String,
    /// Column B: Type ("task" or "subtask").
    pub kind: RowKind,
    /// Column C: Title (PBI title or subtask title prefixed with '↳ ').
    pub title: String,
    /// Column D: Status (PBI status or cell status).
    pub status: String,
    /// Column E: Description (PBI condition of satisfaction or cell outcome).
    pub description: String,
    /// Column F: Feature slug.
    pub feature: String,
}

impl SheetRow {
    pub fn new(
        sheet_index: usize,
        id: impl Into<String>,
        kind: RowKind,
        title: impl Into<String>,
        status: impl Into<String>,
        description: impl Into<String>,
        feature: impl Into<String>,
    ) -> Self {
        Self {
            sheet_index,
            id: id.into(),
            kind,
            title: title.into(),
            status: status.into(),
            description: description.into(),
            feature: feature.into(),
        }
    }

    pub fn row_index(&self) -> usize {
        self.sheet_index
    }
}

/// One PBI record in the saved sync snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SnapPbi {
    pub title: String,
    pub cos: String,
    pub status: String,
}

/// Snapshot of the synced state saved at the end of each successful sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncSnapshot {
    pub spreadsheet_id: String,
    pub sheet_id: Option<i64>,
    pub tab_title: Option<String>,
    pub pbis: BTreeMap<String, SnapPbi>,
}

/// Operations to apply back to bee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncOp {
    /// Add a new PBI from an ID-less row on the sheet.
    Add {
        row_index: usize,
        title: String,
        cos: String,
        status: String,
    },
    /// Amend an existing PBI's title and/or condition-of-satisfaction.
    Amend {
        id: String,
        title: Option<String>,
        cos: Option<String>,
    },
    /// Update an existing PBI's status.
    SetStatus {
        id: String,
        to: String,
    },
}

/// Result of planning a sync against current bee state and previous snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncPlan {
    pub ops: Vec<SyncOp>,
    pub conflicts: Vec<String>,
    pub skipped: Vec<String>,
}

/// Render the spreadsheet values for a project tab: header, then each PBI followed
/// by its feature's cells as subtask rows (D1, CONTEXT.md 'Sheet layout').
///
/// PBIs with an empty feature get no subtasks.
pub fn render_rows(
    pbis: &[BeePbi],
    cells_for: impl Fn(&str) -> Vec<BeeCell>,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    rows.push(HEADER.iter().map(|s| s.to_string()).collect());

    for pbi in pbis {
        rows.push(vec![
            pbi.id.clone(),
            "task".to_string(),
            pbi.title.clone(),
            pbi.status.clone(),
            pbi.cos.clone(),
            pbi.feature.clone(),
        ]);

        if !pbi.feature.trim().is_empty() {
            let cells = cells_for(&pbi.feature);
            for cell in cells {
                rows.push(vec![
                    cell.id.clone(),
                    "subtask".to_string(),
                    format!("↳ {}", cell.title),
                    cell.status.clone(),
                    cell.outcome.unwrap_or_default(),
                    cell.feature.clone(),
                ]);
            }
        }
    }

    rows
}

/// Returns the 0-based row indices of task rows within `rows` (where row 0 is header).
/// Used to configure the status dropdown validation only on task rows (D7k).
pub fn task_row_indices(rows: &[Vec<String>]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .skip(1) // skip header row 0
        .filter(|(_, r)| {
            r.get(1)
                .map(|t| t.trim().eq_ignore_ascii_case("task"))
                .unwrap_or(false)
        })
        .map(|(idx, _)| idx)
        .collect()
}

/// Convenience helper returning rendered rows alongside task row 0-based indices.
pub fn render_rows_with_indices(
    pbis: &[BeePbi],
    cells_for: impl Fn(&str) -> Vec<BeeCell>,
) -> (Vec<Vec<String>>, Vec<usize>) {
    let rows = render_rows(pbis, cells_for);
    let indices = task_row_indices(&rows);
    (rows, indices)
}

/// Parse raw values read from the sheet into `SheetRow`s.
/// Skips header and fully blank rows, tolerates short rows, and treats a row with
/// blank Type as Task (D7c). Retains each row's 0-based sheet index.
pub fn parse_rows(values: &[Vec<String>]) -> Vec<SheetRow> {
    let mut out = Vec::new();
    let mut header_skipped = false;

    for (sheet_index, row) in values.iter().enumerate() {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }

        let col = |idx: usize| -> String {
            row.get(idx).map(|s| s.trim().to_string()).unwrap_or_default()
        };

        if !header_skipped {
            let col0 = col(0);
            let col1 = col(1);
            if col0.eq_ignore_ascii_case("id") || col1.eq_ignore_ascii_case("type") {
                header_skipped = true;
                continue;
            }
            if sheet_index == 0 {
                let col2 = col(2);
                if col2.eq_ignore_ascii_case("title") {
                    header_skipped = true;
                    continue;
                }
            }
        }

        let type_str = col(1);
        let kind = if type_str.eq_ignore_ascii_case("subtask") {
            RowKind::Subtask
        } else {
            // Treats blank Type as Task (D7c)
            RowKind::Task
        };

        out.push(SheetRow {
            sheet_index,
            id: col(0),
            kind,
            title: col(2),
            status: col(3),
            description: col(4),
            feature: col(5),
        });
    }

    out
}

/// Plans synchronization between bee PBIs and sheet rows using 3-way merge rules (D5, D7).
///
/// Rules:
/// - Only Task rows are considered (subtask rows never yield ops, D2).
/// - A snapshot whose spreadsheet_id differs is treated as absent (D7l).
/// - Per field, three-way compare trimmed sheet vs snapshot vs bee:
///   - sheet-only change -> op;
///   - both changed and differ -> conflict line carrying both values, no op;
///   - no snapshot entry -> no op for existing IDs;
/// - A title or cos emptied on the sheet -> skipped with reason 'bee cannot clear a field' (D7b).
/// - ID-less row with non-empty title -> Add (status defaults proposed; invalid status -> Add with proposed plus a skipped note).
/// - Invalid status on existing ID -> skipped with reason carrying both values.
/// - ID not in bee -> skipped with reason.
/// - Rows missing from the sheet -> nothing (deleting a row never deletes a PBI).
pub fn plan_sync(
    bee: &[BeePbi],
    sheet: &[SheetRow],
    snapshot: Option<&SyncSnapshot>,
    spreadsheet_id: &str,
) -> SyncPlan {
    let mut ops = Vec::new();
    let mut conflicts = Vec::new();
    let mut skipped = Vec::new();

    // D7l: A snapshot for another spreadsheet_id is ignored
    let effective_snapshot = match snapshot {
        Some(s) if s.spreadsheet_id == spreadsheet_id => Some(s),
        _ => None,
    };

    for row in sheet {
        // D2: Subtask rows never yield ops
        if row.kind != RowKind::Task {
            continue;
        }

        let id = row.id.trim();

        if id.is_empty() {
            // ID-less row
            let title = row.title.trim();
            if title.is_empty() {
                continue;
            }

            let raw_status = row.status.trim();
            let status = if raw_status.is_empty() {
                "proposed".to_string()
            } else if PBI_STATUSES.contains(&raw_status) {
                raw_status.to_string()
            } else {
                skipped.push(format!(
                    "row {}: invalid status '{raw_status}' on new task '{title}'; defaulted to proposed",
                    row.sheet_index
                ));
                "proposed".to_string()
            };

            ops.push(SyncOp::Add {
                row_index: row.sheet_index,
                title: title.to_string(),
                cos: row.description.trim().to_string(),
                status,
            });
        } else {
            // Existing ID
            let Some(bee_pbi) = bee.iter().find(|p| p.id == id) else {
                skipped.push(format!(
                    "task '{id}': not in bee (sheet has '{}')",
                    row.title.trim()
                ));
                continue;
            };

            let sheet_status = row.status.trim();
            let status_valid = PBI_STATUSES.contains(&sheet_status);
            if !status_valid {
                // Status 'bogus' on an existing ID is skipped with a reason carrying both values
                skipped.push(format!(
                    "task '{id}': invalid status '{sheet_status}' (bee has '{}')",
                    bee_pbi.status.trim()
                ));
            }

            // No snapshot entry -> no op for existing IDs (First sync without snapshot yields no ops)
            let Some(snap_pbi) = effective_snapshot.and_then(|s| s.pbis.get(id)) else {
                continue;
            };

            // 3-way check on title
            let sheet_title = row.title.trim();
            let snap_title = snap_pbi.title.trim();
            let bee_title = bee_pbi.title.trim();
            let mut new_title = None;

            if sheet_title != snap_title && bee_title != snap_title {
                if sheet_title != bee_title {
                    conflicts.push(format!(
                        "task '{id}' title conflict: sheet has '{sheet_title}', bee has '{bee_title}'"
                    ));
                }
            } else if sheet_title != snap_title && bee_title == snap_title {
                if sheet_title.is_empty() {
                    skipped.push(format!(
                        "task '{id}': title emptied on sheet; skipped: bee cannot clear a field (bee has '{bee_title}')"
                    ));
                } else {
                    new_title = Some(sheet_title.to_string());
                }
            }

            // 3-way check on cos / description
            let sheet_cos = row.description.trim();
            let snap_cos = snap_pbi.cos.trim();
            let bee_cos = bee_pbi.cos.trim();
            let mut new_cos = None;

            if sheet_cos != snap_cos && bee_cos != snap_cos {
                if sheet_cos != bee_cos {
                    conflicts.push(format!(
                        "task '{id}' description conflict: sheet has '{sheet_cos}', bee has '{bee_cos}'"
                    ));
                }
            } else if sheet_cos != snap_cos && bee_cos == snap_cos {
                if sheet_cos.is_empty() {
                    skipped.push(format!(
                        "task '{id}': description emptied on sheet; skipped: bee cannot clear a field (bee has '{bee_cos}')"
                    ));
                } else {
                    new_cos = Some(sheet_cos.to_string());
                }
            }

            if new_title.is_some() || new_cos.is_some() {
                ops.push(SyncOp::Amend {
                    id: id.to_string(),
                    title: new_title,
                    cos: new_cos,
                });
            }

            // 3-way check on status
            if status_valid {
                let snap_status = snap_pbi.status.trim();
                let bee_status = bee_pbi.status.trim();

                if sheet_status != snap_status && bee_status != snap_status {
                    if sheet_status != bee_status {
                        conflicts.push(format!(
                            "task '{id}' status conflict: sheet has '{sheet_status}', bee has '{bee_status}'"
                        ));
                    }
                } else if sheet_status != snap_status && bee_status == snap_status {
                    ops.push(SyncOp::SetStatus {
                        id: id.to_string(),
                        to: sheet_status.to_string(),
                    });
                }
            }
        }
    }

    SyncPlan {
        ops,
        conflicts,
        skipped,
    }
}

/// Builds a new snapshot from bee PBIs.
/// Fields listed in `keep_sheet` (for failed ops, D7d) take the previous snapshot value
/// instead of bee's current value.
pub fn snapshot_from<I: AsRef<str>, F: AsRef<str>>(
    spreadsheet_id: impl Into<String>,
    sheet_id: Option<i64>,
    tab_title: Option<String>,
    bee: &[BeePbi],
    prev: Option<&SyncSnapshot>,
    keep_sheet: &[(I, F)],
) -> SyncSnapshot {
    let mut pbis = BTreeMap::new();

    for pbi in bee {
        let mut title = pbi.title.clone();
        let mut cos = pbi.cos.clone();
        let mut status = pbi.status.clone();

        if let Some(prev_snap) = prev {
            if let Some(prev_pbi) = prev_snap.pbis.get(&pbi.id) {
                for (id, field) in keep_sheet {
                    if id.as_ref() == pbi.id {
                        match field.as_ref() {
                            "title" => title = prev_pbi.title.clone(),
                            "cos" | "description" => cos = prev_pbi.cos.clone(),
                            "status" => status = prev_pbi.status.clone(),
                            _ => {}
                        }
                    }
                }
            }
        }

        pbis.insert(pbi.id.clone(), SnapPbi { title, cos, status });
    }

    SyncSnapshot {
        spreadsheet_id: spreadsheet_id.into(),
        sheet_id,
        tab_title,
        pbis,
    }
}

/// Re-appends failed Add rows without an ID and writes the user's sheet values into
/// failed fields on existing rows so user edits are not lost upon tab rewrite (D7d).
pub fn overlay_failed<I: AsRef<str>, F: AsRef<str>, V: AsRef<str>>(
    rows: &mut Vec<Vec<String>>,
    failed_adds: &[SheetRow],
    failed_fields: &[(I, F, V)],
) {
    for (id, field, val) in failed_fields {
        let id = id.as_ref();
        let field = field.as_ref();
        let val = val.as_ref();
        let col_idx = match field {
            "title" => 2,
            "status" => 3,
            "cos" | "description" => 4,
            "feature" => 5,
            _ => continue,
        };
        for row in rows.iter_mut() {
            if row.first().map(|s| s.as_str()) == Some(id) {
                while row.len() <= col_idx {
                    row.push(String::new());
                }
                row[col_idx] = val.to_string();
            }
        }
    }

    for add in failed_adds {
        rows.push(vec![
            String::new(), // without an ID
            "task".to_string(),
            add.title.clone(),
            add.status.clone(),
            add.description.clone(),
            add.feature.clone(),
        ]);
    }
}

/// Generates a unique tab title when multiple projects share a name (D6),
/// appending ' (2)', ' (3)', etc.
pub fn unique_tab_title<S: AsRef<str>>(name: &str, taken: &[S]) -> String {
    if !taken.iter().any(|t| t.as_ref() == name) {
        return name.to_string();
    }
    let mut i = 2;
    loop {
        let candidate = format!("{name} ({i})");
        if !taken.iter().any(|t| t.as_ref() == candidate) {
            return candidate;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pbi(id: &str, title: &str, status: &str, cos: &str, feature: &str) -> BeePbi {
        BeePbi {
            id: id.to_string(),
            title: title.to_string(),
            status: status.to_string(),
            cos: cos.to_string(),
            feature: feature.to_string(),
        }
    }

    fn sample_cell(id: &str, title: &str, status: &str, outcome: Option<&str>, feature: &str) -> BeeCell {
        BeeCell {
            id: id.to_string(),
            feature: feature.to_string(),
            title: title.to_string(),
            lane: "code".to_string(),
            status: status.to_string(),
            tier: None,
            files: vec![],
            worker: None,
            claimed_at: None,
            capped_at: None,
            behavior_change: false,
            outcome: outcome.map(|s| s.to_string()),
            tests: None,
        }
    }

    #[test]
    fn render_rows_produces_header_and_subtasks() {
        let pbis = vec![
            sample_pbi("p-1", "Task 1", "proposed", "Cos 1", "feat-a"),
            sample_pbi("p-2", "Task 2", "done", "Cos 2", ""),
        ];
        let cells_for = |f: &str| {
            if f == "feat-a" {
                vec![
                    sample_cell("c-1", "Cell 1", "capped", Some("outcome 1"), "feat-a"),
                    sample_cell("c-2", "Cell 2", "open", None, "feat-a"),
                ]
            } else {
                vec![]
            }
        };

        let rows = render_rows(&pbis, cells_for);
        assert_eq!(rows.len(), 5); // Header + PBI 1 + 2 subtasks + PBI 2 (no subtasks)
        assert_eq!(rows[0], HEADER);
        assert_eq!(rows[1], vec!["p-1", "task", "Task 1", "proposed", "Cos 1", "feat-a"]);
        assert_eq!(rows[2], vec!["c-1", "subtask", "↳ Cell 1", "capped", "outcome 1", "feat-a"]);
        assert_eq!(rows[3], vec!["c-2", "subtask", "↳ Cell 2", "open", "", "feat-a"]);
        assert_eq!(rows[4], vec!["p-2", "task", "Task 2", "done", "Cos 2", ""]);

        let indices = task_row_indices(&rows);
        assert_eq!(indices, vec![1, 4]);
    }

    #[test]
    fn render_and_parse_roundtrip() {
        let pbis = vec![
            sample_pbi("p-1", "Task 1", "proposed", "Cos 1", "feat-a"),
        ];
        let cells_for = |f: &str| {
            if f == "feat-a" {
                vec![sample_cell("c-1", "Cell 1", "capped", Some("done"), "feat-a")]
            } else {
                vec![]
            }
        };

        let rendered = render_rows(&pbis, cells_for);
        let parsed = parse_rows(&rendered);
        assert_eq!(parsed.len(), 2);

        assert_eq!(parsed[0].sheet_index, 1);
        assert_eq!(parsed[0].id, "p-1");
        assert_eq!(parsed[0].kind, RowKind::Task);
        assert_eq!(parsed[0].title, "Task 1");
        assert_eq!(parsed[0].status, "proposed");
        assert_eq!(parsed[0].description, "Cos 1");
        assert_eq!(parsed[0].feature, "feat-a");

        assert_eq!(parsed[1].sheet_index, 2);
        assert_eq!(parsed[1].id, "c-1");
        assert_eq!(parsed[1].kind, RowKind::Subtask);
        assert_eq!(parsed[1].title, "↳ Cell 1");
        assert_eq!(parsed[1].status, "capped");
        assert_eq!(parsed[1].description, "done");
        assert_eq!(parsed[1].feature, "feat-a");
    }

    #[test]
    fn parse_rows_tolerates_short_and_blank_rows_and_defaults_blank_type_to_task() {
        let values = vec![
            vec!["ID".into(), "Type".into(), "Title".into()],
            vec!["".into(), "".into(), "".into()], // fully blank row
            vec!["p-new".into(), "".into(), "Titled Task".into()], // blank type -> Task (D7c)
            vec!["p-short".into()], // short row
        ];

        let parsed = parse_rows(&values);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].sheet_index, 2);
        assert_eq!(parsed[0].id, "p-new");
        assert_eq!(parsed[0].kind, RowKind::Task);
        assert_eq!(parsed[0].title, "Titled Task");

        assert_eq!(parsed[1].sheet_index, 3);
        assert_eq!(parsed[1].id, "p-short");
        assert_eq!(parsed[1].kind, RowKind::Task);
        assert_eq!(parsed[1].title, "");
    }

    #[test]
    fn truth_1_title_edited_only_on_sheet_yields_one_amend_op() {
        let bee = vec![sample_pbi("p-1", "Original Title", "proposed", "Cos", "feat")];
        let sheet = vec![SheetRow::new(1, "p-1", RowKind::Task, "New Title", "proposed", "Cos", "feat")];
        let mut pbis = BTreeMap::new();
        pbis.insert("p-1".to_string(), SnapPbi {
            title: "Original Title".to_string(),
            cos: "Cos".to_string(),
            status: "proposed".to_string(),
        });
        let snapshot = SyncSnapshot {
            spreadsheet_id: "sheet-1".to_string(),
            sheet_id: Some(1),
            tab_title: Some("tab".to_string()),
            pbis,
        };

        let plan = plan_sync(&bee, &sheet, Some(&snapshot), "sheet-1");
        assert_eq!(plan.conflicts.len(), 0);
        assert_eq!(plan.skipped.len(), 0);
        assert_eq!(plan.ops.len(), 1);
        assert_eq!(plan.ops[0], SyncOp::Amend {
            id: "p-1".to_string(),
            title: Some("New Title".to_string()),
            cos: None,
        });
    }

    #[test]
    fn truth_2_field_edited_on_both_sides_yields_conflict_line_and_no_op() {
        let bee = vec![sample_pbi("p-1", "Bee Title", "proposed", "Cos", "feat")];
        let sheet = vec![SheetRow::new(1, "p-1", RowKind::Task, "Sheet Title", "proposed", "Cos", "feat")];
        let mut pbis = BTreeMap::new();
        pbis.insert("p-1".to_string(), SnapPbi {
            title: "Snapshot Title".to_string(),
            cos: "Cos".to_string(),
            status: "proposed".to_string(),
        });
        let snapshot = SyncSnapshot {
            spreadsheet_id: "sheet-1".to_string(),
            sheet_id: Some(1),
            tab_title: Some("tab".to_string()),
            pbis,
        };

        let plan = plan_sync(&bee, &sheet, Some(&snapshot), "sheet-1");
        assert_eq!(plan.ops.len(), 0);
        assert_eq!(plan.conflicts.len(), 1);
        assert!(plan.conflicts[0].contains("Sheet Title"));
        assert!(plan.conflicts[0].contains("Bee Title"));
    }

    #[test]
    fn truth_3_id_less_titled_row_with_blank_type_yields_add_op() {
        let bee = vec![];
        let sheet = vec![SheetRow::new(2, "", RowKind::Task, "New Task", "proposed", "New Cos", "feat")];

        let plan = plan_sync(&bee, &sheet, None, "sheet-1");
        assert_eq!(plan.ops.len(), 1);
        assert_eq!(plan.ops[0], SyncOp::Add {
            row_index: 2,
            title: "New Task".to_string(),
            cos: "New Cos".to_string(),
            status: "proposed".to_string(),
        });
    }

    #[test]
    fn truth_4_first_sync_without_snapshot_yields_no_ops_for_existing_ids() {
        let bee = vec![sample_pbi("p-1", "Bee Title", "proposed", "Cos", "feat")];
        let sheet = vec![
            SheetRow::new(1, "p-1", RowKind::Task, "Sheet Title", "proposed", "Cos", "feat"),
            SheetRow::new(2, "", RowKind::Task, "Brand New", "proposed", "", "feat"),
        ];

        let plan = plan_sync(&bee, &sheet, None, "sheet-1");
        assert_eq!(plan.ops.len(), 1);
        match &plan.ops[0] {
            SyncOp::Add { title, .. } => assert_eq!(title, "Brand New"),
            _ => panic!("Expected Add op only"),
        }
    }

    #[test]
    fn truth_5_snapshot_for_another_spreadsheet_id_is_ignored() {
        let bee = vec![sample_pbi("p-1", "Bee Title", "proposed", "Cos", "feat")];
        let sheet = vec![SheetRow::new(1, "p-1", RowKind::Task, "Sheet Title", "proposed", "Cos", "feat")];
        let mut pbis = BTreeMap::new();
        pbis.insert("p-1".to_string(), SnapPbi {
            title: "Snap Title".to_string(),
            cos: "Cos".to_string(),
            status: "proposed".to_string(),
        });
        let snapshot = SyncSnapshot {
            spreadsheet_id: "other-sheet-id".to_string(),
            sheet_id: Some(1),
            tab_title: Some("tab".to_string()),
            pbis,
        };

        // Treated as absent -> existing IDs yield no ops
        let plan = plan_sync(&bee, &sheet, Some(&snapshot), "sheet-1");
        assert_eq!(plan.ops.len(), 0);
    }

    #[test]
    fn truth_6_emptied_description_on_existing_id_is_skipped_never_an_amend() {
        let bee = vec![sample_pbi("p-1", "Title", "proposed", "Old Cos", "feat")];
        let sheet = vec![SheetRow::new(1, "p-1", RowKind::Task, "Title", "proposed", "", "feat")];
        let mut pbis = BTreeMap::new();
        pbis.insert("p-1".to_string(), SnapPbi {
            title: "Title".to_string(),
            cos: "Old Cos".to_string(),
            status: "proposed".to_string(),
        });
        let snapshot = SyncSnapshot {
            spreadsheet_id: "sheet-1".to_string(),
            sheet_id: Some(1),
            tab_title: Some("tab".to_string()),
            pbis,
        };

        let plan = plan_sync(&bee, &sheet, Some(&snapshot), "sheet-1");
        assert_eq!(plan.ops.len(), 0);
        assert_eq!(plan.skipped.len(), 1);
        assert!(plan.skipped[0].contains("bee cannot clear a field"));
        assert!(plan.skipped[0].contains("Old Cos"));
    }

    #[test]
    fn truth_7_subtask_rows_never_yield_ops() {
        let bee = vec![sample_pbi("p-1", "Title", "proposed", "Cos", "feat")];
        let sheet = vec![
            SheetRow::new(1, "c-1", RowKind::Subtask, "Edited Subtask", "done", "Outcome", "feat"),
            SheetRow::new(2, "", RowKind::Subtask, "New Subtask", "proposed", "", "feat"),
        ];

        let plan = plan_sync(&bee, &sheet, None, "sheet-1");
        assert_eq!(plan.ops.len(), 0);
        assert_eq!(plan.conflicts.len(), 0);
        assert_eq!(plan.skipped.len(), 0);
    }

    #[test]
    fn truth_8_status_bogus_on_an_existing_id_is_skipped_with_a_reason() {
        let bee = vec![sample_pbi("p-1", "Title", "proposed", "Cos", "feat")];
        let sheet = vec![SheetRow::new(1, "p-1", RowKind::Task, "Title", "bogus", "Cos", "feat")];
        let mut pbis = BTreeMap::new();
        pbis.insert("p-1".to_string(), SnapPbi {
            title: "Title".to_string(),
            cos: "Cos".to_string(),
            status: "proposed".to_string(),
        });
        let snapshot = SyncSnapshot {
            spreadsheet_id: "sheet-1".to_string(),
            sheet_id: Some(1),
            tab_title: Some("tab".to_string()),
            pbis,
        };

        let plan = plan_sync(&bee, &sheet, Some(&snapshot), "sheet-1");
        assert_eq!(plan.ops.len(), 0);
        assert_eq!(plan.skipped.len(), 1);
        assert!(plan.skipped[0].contains("bogus"));
        assert!(plan.skipped[0].contains("proposed"));
    }

    #[test]
    fn invalid_status_on_new_task_defaults_to_proposed_with_skipped_note() {
        let bee = vec![];
        let sheet = vec![SheetRow::new(1, "", RowKind::Task, "New Task", "weird_status", "Cos", "feat")];

        let plan = plan_sync(&bee, &sheet, None, "sheet-1");
        assert_eq!(plan.ops.len(), 1);
        assert_eq!(plan.ops[0], SyncOp::Add {
            row_index: 1,
            title: "New Task".to_string(),
            cos: "Cos".to_string(),
            status: "proposed".to_string(),
        });
        assert_eq!(plan.skipped.len(), 1);
        assert!(plan.skipped[0].contains("weird_status"));
    }

    #[test]
    fn existing_id_not_in_bee_is_skipped() {
        let bee = vec![];
        let sheet = vec![SheetRow::new(1, "p-missing", RowKind::Task, "Gone", "proposed", "Cos", "feat")];

        let plan = plan_sync(&bee, &sheet, None, "sheet-1");
        assert_eq!(plan.ops.len(), 0);
        assert_eq!(plan.skipped.len(), 1);
        assert!(plan.skipped[0].contains("p-missing"));
    }

    #[test]
    fn set_status_op_on_sheet_status_change() {
        let bee = vec![sample_pbi("p-1", "Title", "proposed", "Cos", "feat")];
        let sheet = vec![SheetRow::new(1, "p-1", RowKind::Task, "Title", "done", "Cos", "feat")];
        let mut pbis = BTreeMap::new();
        pbis.insert("p-1".to_string(), SnapPbi {
            title: "Title".to_string(),
            cos: "Cos".to_string(),
            status: "proposed".to_string(),
        });
        let snapshot = SyncSnapshot {
            spreadsheet_id: "sheet-1".to_string(),
            sheet_id: Some(1),
            tab_title: Some("tab".to_string()),
            pbis,
        };

        let plan = plan_sync(&bee, &sheet, Some(&snapshot), "sheet-1");
        assert_eq!(plan.ops.len(), 1);
        assert_eq!(plan.ops[0], SyncOp::SetStatus {
            id: "p-1".to_string(),
            to: "done".to_string(),
        });
    }

    #[test]
    fn snapshot_from_keeps_failed_fields_from_prev_snapshot() {
        let bee = vec![
            sample_pbi("p-1", "New Bee Title", "done", "New Bee Cos", "feat"),
            sample_pbi("p-2", "P2 Title", "proposed", "P2 Cos", "feat"),
        ];

        let mut prev_pbis = BTreeMap::new();
        prev_pbis.insert("p-1".to_string(), SnapPbi {
            title: "Old Snap Title".to_string(),
            cos: "Old Snap Cos".to_string(),
            status: "in-flight".to_string(),
        });
        let prev = SyncSnapshot {
            spreadsheet_id: "sheet-1".to_string(),
            sheet_id: Some(42),
            tab_title: Some("Tab".to_string()),
            pbis: prev_pbis,
        };

        let keep_sheet = vec![("p-1", "title"), ("p-1", "status")];
        let snap = snapshot_from("sheet-1", Some(42), Some("Tab".to_string()), &bee, Some(&prev), &keep_sheet);

        let p1 = snap.pbis.get("p-1").unwrap();
        assert_eq!(p1.title, "Old Snap Title"); // kept from prev
        assert_eq!(p1.status, "in-flight");     // kept from prev
        assert_eq!(p1.cos, "New Bee Cos");      // took bee value

        let p2 = snap.pbis.get("p-2").unwrap();
        assert_eq!(p2.title, "P2 Title");
        assert_eq!(p2.status, "proposed");
    }

    #[test]
    fn overlay_failed_reappends_adds_and_overwrites_failed_fields() {
        let mut rows = vec![
            HEADER.iter().map(|s| s.to_string()).collect(),
            vec!["p-1".into(), "task".into(), "Bee Title".into(), "proposed".into(), "Cos".into(), "feat".into()],
        ];

        let failed_adds = vec![
            SheetRow::new(3, "", RowKind::Task, "Failed Add Task", "proposed", "Add Cos", "feat"),
        ];
        let failed_fields = vec![
            ("p-1", "title", "User Sheet Title"),
        ];

        overlay_failed(&mut rows, &failed_adds, &failed_fields);

        assert_eq!(rows.len(), 3);
        // p-1 has user's sheet title restored
        assert_eq!(rows[1][2], "User Sheet Title");
        // Failed add re-appended without an ID in col 0
        assert_eq!(rows[2][0], "");
        assert_eq!(rows[2][1], "task");
        assert_eq!(rows[2][2], "Failed Add Task");
    }

    #[test]
    fn unique_tab_title_appends_suffixes() {
        let taken = vec!["Project".to_string(), "Project (2)".to_string(), "Other".to_string()];
        assert_eq!(unique_tab_title("Unique", &taken), "Unique");
        assert_eq!(unique_tab_title("Project", &taken), "Project (3)");
        assert_eq!(unique_tab_title("Other", &taken), "Other (2)");
    }
}
