use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

use waggledance_core::indexer::now_rfc3339;
use waggledance_core::Engine;

const RECIPIENT: &str = "wd";
const LEADER_KINDS: [&str; 4] = ["result", "question", "progress", "error"];
const DONE_PARTS: [&str; 4] = ["Done", "Proof", "Left", "Where"];

pub fn run(action: crate::cli::MailAction) -> Result<()> {
    match action {
        crate::cli::MailAction::Send {
            to,
            corr,
            kind,
            body_file,
        } => {
            let body = read_body(body_file.as_deref())?;
            let engine = crate::runtime::build_engine()?;
            match send(&engine, &to, &corr, &kind, &body, &now_rfc3339())? {
                Ok(seq) => {
                    println!("{}", serde_json::json!({ "seq": seq }));
                    Ok(())
                }
                Err(problems) => {
                    for problem in problems {
                        eprintln!("error: {problem}");
                    }
                    std::process::exit(2);
                }
            }
        }
    }
}

fn read_body(file: Option<&Path>) -> Result<String> {
    match file {
        Some(path) => {
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
        }
        None => {
            let mut body = String::new();
            std::io::stdin()
                .read_to_string(&mut body)
                .context("reading the body from stdin")?;
            Ok(body)
        }
    }
}

fn send(
    engine: &Engine,
    to: &str,
    corr: &str,
    kind: &str,
    body: &str,
    now: &str,
) -> Result<Result<i64, Vec<String>>> {
    let mut problems = Vec::new();
    if to != RECIPIENT {
        problems.push(format!(
            "recipient `{to}` is not accepted; the only recipient is {RECIPIENT}"
        ));
    }
    if !LEADER_KINDS.contains(&kind) {
        problems.push(format!(
            "kind `{kind}` is not one a leader may send; accepted kinds: {}",
            LEADER_KINDS.join(", ")
        ));
    }
    let project = engine.project_for_corr(corr)?;
    if project.is_none() {
        problems.push(format!("no run found for corr `{corr}`"));
    }
    if body.trim().is_empty() {
        problems.push("the body is empty".to_string());
    } else if kind == "result" {
        let missing = missing_parts(body);
        if !missing.is_empty() {
            problems.push(format!(
                "the result body is missing the done-report parts: {}",
                missing.join(", ")
            ));
        }
    }
    match project {
        Some(project) if problems.is_empty() => Ok(Ok(
            engine.insert_inbox_message(RECIPIENT, corr, &project, kind, body, now)?
        )),
        _ => Ok(Err(problems)),
    }
}

fn part_header(line: &str) -> Option<(usize, &str)> {
    let heading = line.trim_start().starts_with('#');
    let stripped = line.trim_start_matches([' ', '\t', '#', '-', '*']);
    DONE_PARTS.iter().enumerate().find_map(|(i, word)| {
        let head = stripped.get(..word.len())?;
        if !head.eq_ignore_ascii_case(word) {
            return None;
        }
        let rest = &stripped[word.len()..];
        match rest.strip_prefix(':') {
            Some(inline) => Some((i, inline)),
            None if heading && rest.trim().is_empty() => Some((i, "")),
            None => None,
        }
    })
}

fn missing_parts(body: &str) -> Vec<&'static str> {
    let mut filled = [false; DONE_PARTS.len()];
    let mut current: Option<usize> = None;
    for line in body.lines() {
        match part_header(line) {
            Some((i, inline)) => {
                current = Some(i);
                filled[i] |= !inline.trim().is_empty();
            }
            None => {
                if let Some(i) = current {
                    filled[i] |= !line.trim().is_empty();
                }
            }
        }
    }
    DONE_PARTS
        .iter()
        .zip(filled)
        .filter(|(_, ok)| !ok)
        .map(|(word, _)| *word)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use waggledance_core::domain::{Project, Run};
    use waggledance_core::{Config, SqliteStore};

    const NOW: &str = "2026-10-22T10:00:00Z";
    const FULL: &str =
        "Done: shipped the fix\nProof: cargo test green\nLeft: nothing\nWhere: commit abc123\n";

    fn engine() -> Engine {
        let engine = Engine::new(SqliteStore::open_in_memory().unwrap(), Config::default());
        for id in ["proj-1", "proj-2"] {
            engine
                .store
                .upsert_project(&Project {
                    id: id.into(),
                    name: id.into(),
                    root_path: std::env::temp_dir().join(id),
                    created_at: NOW.into(),
                    last_seen_at: NOW.into(),
                    orchestration_enabled: true,
                })
                .unwrap();
        }
        engine
    }

    fn run_at(engine: &Engine, id: &str, project: &str, request_id: &str, at: &str) {
        let run = Run {
            id: id.into(),
            project_id: project.into(),
            pane_id: "paseo:leader".into(),
            preset_label: None,
            task: "t".into(),
            baseline: String::new(),
            marker: String::new(),
            status: "working".into(),
            created_at: at.into(),
            updated_at: at.into(),
        };
        engine
            .insert_run_with_request(&run, None, Some(request_id), Some("openclaw:wd"))
            .unwrap();
    }

    fn refused(out: Result<Result<i64, Vec<String>>>) -> String {
        out.unwrap().expect_err("mail should be refused").join("\n")
    }

    #[test]
    fn a_valid_result_inserts_one_row_and_returns_its_seq() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", NOW);
        let seq = send(&e, "wd", "c-7", "result", FULL, NOW).unwrap().unwrap();
        let msg = e.next_inbox_message("wd").unwrap().unwrap();
        assert_eq!(msg.seq, seq);
        assert_eq!(msg.project_id, "proj-1");
        assert_eq!(msg.kind, "result");
        assert_eq!(msg.corr, "c-7");
        assert_eq!(msg.body, FULL);
    }

    #[test]
    fn a_result_missing_proof_and_where_names_both_and_writes_nothing() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", NOW);
        let why = refused(send(
            &e,
            "wd",
            "c-7",
            "result",
            "Done: x\nLeft: none\n",
            NOW,
        ));
        assert!(why.contains("Proof") && why.contains("Where"), "{why}");
        assert!(!why.contains("Done,") && !why.contains("Left"), "{why}");
        assert!(e.next_inbox_message("wd").unwrap().is_none());
    }

    #[test]
    fn a_part_header_with_nothing_under_it_counts_as_missing() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", NOW);
        let body = "## Done\n- shipped\n## Proof\n\n## Left\nnone\n## Where\nmain\n";
        let why = refused(send(&e, "wd", "c-7", "result", body, NOW));
        assert!(why.contains("Proof") && !why.contains("Where"), "{why}");
    }

    #[test]
    fn markdown_headings_and_bullets_satisfy_the_grammar() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", NOW);
        let body = "# done\nshipped\n  - PROOF: tests green\n* left: none\n### Where\ncommit abc\n";
        assert!(send(&e, "wd", "c-7", "result", body, NOW).unwrap().is_ok());
    }

    #[test]
    fn an_unknown_kind_names_the_accepted_kinds() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", NOW);
        let why = refused(send(&e, "wd", "c-7", "board", "hi", NOW));
        assert!(why.contains("result, question, progress, error"), "{why}");
        assert!(e.next_inbox_message("wd").unwrap().is_none());
    }

    #[test]
    fn an_unknown_corr_is_named() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", NOW);
        let why = refused(send(&e, "wd", "c-8", "question", "which db?", NOW));
        assert!(why.contains("`c-8`"), "{why}");
        assert!(e.next_inbox_message("wd").unwrap().is_none());
    }

    #[test]
    fn a_recipient_other_than_wd_is_refused_with_every_other_problem() {
        let e = engine();
        let why = refused(send(&e, "bob", "c-9", "nope", "x", NOW));
        assert!(
            why.contains("`bob`") && why.contains("`nope`") && why.contains("`c-9`"),
            "{why}"
        );
    }

    #[test]
    fn the_corr_resolves_through_the_newest_run_sharing_its_prefix() {
        let e = engine();
        run_at(&e, "r1", "proj-1", "c-7", "2026-10-22T09:00:00Z");
        run_at(&e, "r2", "proj-2", "c-7:redo", "2026-10-22T09:30:00Z");
        run_at(&e, "r3", "proj-1", "c-77", "2026-10-22T09:45:00Z");
        send(&e, "wd", "c-7", "progress", "half way", NOW)
            .unwrap()
            .unwrap();
        assert_eq!(
            e.next_inbox_message("wd").unwrap().unwrap().project_id,
            "proj-2"
        );
    }
}
