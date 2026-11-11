use crate::domain::Project;
use crate::process::{run_with_deadline, RunError};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const OUTPUT_CAP: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub worktree: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Swept {
        removed: Vec<Verdict>,
        kept: Vec<Verdict>,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSweep {
    pub project_id: String,
    pub outcome: Outcome,
}

pub fn bee_binary(root: &Path) -> Option<PathBuf> {
    let bin = root.join(".bee").join("bin").join("bee");
    let executable = std::fs::metadata(&bin)
        .map(|m| m.is_file() && is_executable(&m))
        .unwrap_or(false);
    let main_checkout = root.join(".git").is_dir();
    (executable && main_checkout).then_some(bin)
}

#[cfg(unix)]
fn is_executable(m: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    m.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_: &std::fs::Metadata) -> bool {
    true
}

pub fn parse(stdout: &str) -> (Vec<Verdict>, Vec<Verdict>) {
    let mut removed = Vec::new();
    let mut kept = Vec::new();
    for line in stdout.lines() {
        let Some((id, rest)) = line.split_once(": ") else {
            continue;
        };
        if id.contains(char::is_whitespace) {
            continue;
        }
        let verdict = |prefix: &str| Verdict {
            worktree: id.to_string(),
            reason: rest[prefix.len()..]
                .trim_start_matches([',', ' ', '—'])
                .to_string(),
        };
        if rest.starts_with("removed") {
            removed.push(verdict("removed"));
        } else if rest.starts_with("kept") {
            kept.push(verdict("kept"));
        }
    }
    (removed, kept)
}

pub fn sweep_project(bin: &Path, root: &Path, older_than_days: u64, timeout: Duration) -> Outcome {
    let mut cmd = Command::new(bin);
    cmd.current_dir(root)
        .arg("worktree")
        .arg("prune")
        .arg("--older-than-days")
        .arg(older_than_days.to_string());
    match run_with_deadline(&mut cmd, None, timeout, OUTPUT_CAP) {
        Ok(out) if out.ok => {
            let (removed, kept) = parse(&String::from_utf8_lossy(&out.stdout));
            Outcome::Swept { removed, kept }
        }
        Ok(out) => Outcome::Failed(
            out.stderr
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty() && !l.starts_with("[bee]"))
                .unwrap_or("bee worktree prune exited non-zero")
                .to_string(),
        ),
        Err(RunError::Timeout(secs)) => {
            Outcome::Failed(format!("bee worktree prune timed out after {secs}s"))
        }
        Err(RunError::Spawn(e)) => Outcome::Failed(format!("could not start bee: {e}")),
    }
}

pub fn sweep(projects: &[Project], older_than_days: u64, timeout: Duration) -> Vec<ProjectSweep> {
    projects
        .iter()
        .filter_map(|p| {
            let bin = bee_binary(&p.root_path)?;
            Some(ProjectSweep {
                project_id: p.id.clone(),
                outcome: sweep_project(&bin, &p.root_path, older_than_days, timeout),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn project(root: &Path, id: &str) -> Project {
        Project {
            id: id.into(),
            name: id.into(),
            root_path: root.to_path_buf(),
            created_at: String::new(),
            last_seen_at: String::new(),
            orchestration_enabled: false,
        }
    }

    fn fake_bee(root: &Path, script: &str) {
        let bin = root.join(".bee/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let path = bin.join("bee");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn parse_splits_removed_and_kept_and_skips_noise() {
        let out = "paseo: archived workspace wks_1\n\
                   repo--wt--a: removed, reclaimed 16.7 MB — wt/a is merged\n\
                   repo--wt--b: kept — the last commit on wt/b is younger than the age threshold; keeping.\n\
                   Lock a worktree to opt it out of prune permanently: \"git worktree lock <path>\"\n";
        let (removed, kept) = parse(out);
        assert_eq!(
            removed,
            vec![Verdict {
                worktree: "repo--wt--a".into(),
                reason: "reclaimed 16.7 MB — wt/a is merged".into()
            }]
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].worktree, "repo--wt--b");
        assert!(kept[0].reason.starts_with("the last commit"));
    }

    #[test]
    fn sweep_runs_prune_with_the_age_in_the_project_root_and_skips_projects_without_bee() {
        let base = tempfile::tempdir().unwrap();
        let with = base.path().join("with");
        let without = base.path().join("without");
        let linked = base.path().join("linked");
        std::fs::create_dir_all(&without).unwrap();
        fake_bee(
            &with,
            "echo \"$(basename \"$PWD\")--wt--x: removed, reclaimed 1 MB — args $*\"",
        );
        fake_bee(&linked, "echo never");
        std::fs::remove_dir_all(linked.join(".git")).unwrap();
        std::fs::write(linked.join(".git"), "gitdir: elsewhere").unwrap();
        let report = sweep(
            &[
                project(&with, "with"),
                project(&without, "without"),
                project(&linked, "linked"),
            ],
            3,
            Duration::from_secs(10),
        );
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].project_id, "with");
        let Outcome::Swept { removed, kept } = &report[0].outcome else {
            panic!("{:?}", report[0].outcome)
        };
        assert!(kept.is_empty());
        assert_eq!(removed[0].worktree, "with--wt--x");
        assert_eq!(
            removed[0].reason,
            "reclaimed 1 MB — args worktree prune --older-than-days 3"
        );
    }

    #[test]
    fn a_failing_prune_is_reported_not_fatal() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("p");
        fake_bee(
            &root,
            "echo 'bee: base ref did not resolve' >&2\necho '[bee] worktree prune 3ms' >&2\nexit 1",
        );
        let report = sweep(&[project(&root, "p")], 2, Duration::from_secs(10));
        assert_eq!(
            report[0].outcome,
            Outcome::Failed("bee: base ref did not resolve".into())
        );
    }
}
