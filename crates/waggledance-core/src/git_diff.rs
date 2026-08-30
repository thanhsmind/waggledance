//! The working-tree diff behind the Changes screen: what `git` reports for
//! the project's own files, compared against `HEAD` (D2 — staged, unstaged,
//! and untracked all in one view; no base picker).
//!
//! Shells out to the system `git` binary rather than linking a git library:
//! the viewer already depends on a checkout being on disk, and a C-built
//! crate would cost more than it buys here.
//!
//! Four read-only calls per page, all with `-C <project_root>`, a timeout,
//! and a kill:
//!
//! 1. `rev-parse --show-toplevel` — is this a repository at all. `git diff`
//!    cannot answer that: outside a repository it silently falls into
//!    `--no-index` mode and fails with a message about `HEAD` instead of
//!    about the repository, so D3's "not a git repository" state needs its
//!    own probe.
//! 2. `diff HEAD -M --name-status -z --relative` — the authoritative list of
//!    changed paths. NUL-separated, so a path with spaces or non-ASCII bytes
//!    survives whole, and `--relative` both scopes the output to the project
//!    root and prints paths relative to it (a project nested below the repo
//!    toplevel therefore shows only its own files).
//! 3. `diff HEAD -M -U100000 --relative` — the same comparison as a patch,
//!    with a context radius large enough that each changed file comes back
//!    whole. One linear walk then reconstructs the FULL old text (context +
//!    removed) and the FULL new text (context + added) per file, so no blob
//!    fetch and no hunk-pairing pass are needed, and a deleted file's old
//!    side comes from the patch itself.
//! 4. `ls-files -o --exclude-standard -z` — untracked files (git's own
//!    exclude rules), each read through [`code_source::read_source`] and
//!    shown as a full add.
//!
//! D5 holds over every path either list produces: each one goes through
//! [`code_source::resolve_source_path`] with the project's exclude patterns
//! before it can reach a [`FileChange`], and a refused path is dropped into
//! one aggregate count — never named, matching the Code section's own
//! no-disclosure rule.

use crate::code_source::{self, SourceContent};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Wall-clock budget for one git call. Past it the child is killed and the
/// whole page falls back to the unavailable state — a hung subprocess must
/// never hold the request.
pub const GIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Cap on each reconstructed side of one file. Past it the file renders a
/// truncation banner instead of the rest of its content. Same number as
/// `code_source`'s own read cap, for the same reason.
pub const MAX_SIDE_BYTES: usize = 2 * 1024 * 1024;

/// Cap on how many file sections carry content on one page. Every changed
/// file is still listed; the ones past this render a stub pointing at the
/// Code view.
pub const MAX_SECTIONS: usize = 100;

/// Cap on how much stdout one git call may produce. Past it the read stops,
/// the child is closed out, and the page says the diff was truncated.
pub const MAX_STDOUT_BYTES: usize = 48 * 1024 * 1024;

const MAX_STDERR_BYTES: usize = 8 * 1024;

/// Why the Changes screen has no diff to show. Every variant renders D3's
/// explained empty state — never a 500, and never a bare stack of git's own
/// words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GitDiffError {
    /// No usable `git` binary (missing, not executable, spawn refused).
    #[error("git is unavailable: {0}")]
    GitUnavailable(String),
    /// The project root is not inside a git repository.
    #[error("not a git repository")]
    NotARepo,
    /// A git call outlived [`GIT_TIMEOUT`] and was killed.
    #[error("git timed out after {0}s")]
    Timeout(u64),
}

/// The status letter a changed file carries. Git's rarer letters map onto
/// these four conservatively (`T`→M, `C`→A, `U`→M plus a "conflicted" note,
/// anything unknown→M) so the screen never invents a badge nobody can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
}

impl ChangeStatus {
    pub fn letter(self) -> &'static str {
        match self {
            ChangeStatus::Modified => "M",
            ChangeStatus::Added => "A",
            ChangeStatus::Deleted => "D",
            ChangeStatus::Renamed => "R",
        }
    }
}

/// Which side(s) of the diff a reconstructed line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Removed,
    Added,
}

/// One line of a file's diff, carrying the line numbers each side gives it —
/// the pairing a side-by-side table renders from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<usize>,
    pub new_no: Option<usize>,
    pub text: String,
}

/// What a file section actually shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeBody {
    /// Both sides reconstructed whole, plus the line-by-line pairing.
    Text {
        old_text: String,
        new_text: String,
        lines: Vec<DiffLine>,
        /// A cap was hit (side size, or a gap between hunks in a file too
        /// large for one context radius): the sides are a prefix, not the
        /// whole file.
        truncated: bool,
    },
    /// Git reported a binary difference, or the file sniffs binary.
    Binary,
    /// The path is a submodule (a directory in the parent's tree).
    Submodule,
    /// The file is listed as changed but both sides are byte-equal — a mode
    /// change, a pure rename, or a line-ending/filter artifact.
    NoContentChange,
    /// Content deliberately not shown; the string is the reason to display.
    Omitted(String),
}

/// One changed file: what the sidebar lists and the main pane sections on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// Project-relative path, as the project sees it today (the new name of
    /// a rename, the old name of a deletion).
    pub path: String,
    /// The name a renamed file had at HEAD.
    pub old_path: Option<String>,
    pub status: ChangeStatus,
    /// A short qualifier shown beside the badge ("conflicted", "untracked").
    pub note: Option<String>,
    pub added: usize,
    pub removed: usize,
    pub body: ChangeBody,
}

/// The whole working-tree comparison for one project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkingTreeDiff {
    pub files: Vec<FileChange>,
    /// How many changed paths the project's own exclude rules refused. An
    /// aggregate on purpose: naming them would disclose exactly what the
    /// denylist exists to hide (D5).
    pub hidden: usize,
    /// A git call produced more than [`MAX_STDOUT_BYTES`].
    pub stdout_truncated: bool,
    /// More than [`MAX_SECTIONS`] files changed; the rest render as stubs.
    pub sections_capped: bool,
}

/// The working-tree diff for `root`, filtered by the project's `exclude`
/// patterns. See the module docs for the calls this makes.
pub fn working_tree_diff(root: &Path, exclude: &[String]) -> Result<WorkingTreeDiff, GitDiffError> {
    collect(root, exclude, "git", GIT_TIMEOUT)
}

/// The body of [`working_tree_diff`], with the git program and the timeout
/// injectable so the failure paths (missing binary, a call that never
/// returns) are testable without a 10-second test or a mutated PATH.
fn collect(
    root: &Path,
    exclude: &[String],
    git: &str,
    timeout: Duration,
) -> Result<WorkingTreeDiff, GitDiffError> {
    let probe = run_git(git, root, &["rev-parse", "--show-toplevel"], timeout, 4096)?;
    if !probe.ok {
        return Err(GitDiffError::NotARepo);
    }

    let mut diff = WorkingTreeDiff::default();

    let listing = run_git(
        git,
        root,
        &[
            "diff",
            "HEAD",
            "-M",
            "--name-status",
            "-z",
            "--relative",
            "--",
            ".",
        ],
        timeout,
        MAX_STDOUT_BYTES,
    )?;
    // A repository with no commits has no HEAD to compare against; its files
    // are all untracked, so the tracked half is simply empty rather than an
    // error the user has to read.
    let unborn_head = !listing.ok && head_is_unborn(&listing.stderr);
    if !listing.ok && !unborn_head {
        return Err(GitDiffError::GitUnavailable(first_line(&listing.stderr)));
    }
    diff.stdout_truncated |= listing.truncated;

    let entries = if unborn_head {
        Vec::new()
    } else {
        parse_name_status(&listing.stdout)
    };

    let mut sections: HashMap<String, DiffSection> = HashMap::new();
    if !entries.is_empty() {
        let patch = run_git(
            git,
            root,
            &[
                "-c",
                "core.quotePath=false",
                "diff",
                "HEAD",
                "-M",
                "--no-color",
                "--no-ext-diff",
                "--relative",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "-U100000",
                "--",
                ".",
            ],
            timeout,
            MAX_STDOUT_BYTES,
        )?;
        if !patch.ok {
            return Err(GitDiffError::GitUnavailable(first_line(&patch.stderr)));
        }
        diff.stdout_truncated |= patch.truncated;
        sections = parse_patch(&String::from_utf8_lossy(&patch.stdout));
    }

    for entry in entries {
        // D5: both names of a rename are checked — a file renamed away from
        // a denied name must not surface under its new one either.
        let allowed = code_source::resolve_source_path(root, &entry.path, exclude).is_ok()
            && entry
                .old_path
                .as_deref()
                .map(|old| code_source::resolve_source_path(root, old, exclude).is_ok())
                .unwrap_or(true);
        if !allowed {
            diff.hidden += 1;
            continue;
        }

        let key = format!(
            "a/{old} b/{new}",
            old = entry.old_path.as_deref().unwrap_or(&entry.path),
            new = entry.path
        );
        let section = sections.remove(&key);
        let body = if root.join(&entry.path).is_dir() {
            ChangeBody::Submodule
        } else {
            match section {
                Some(DiffSection::Binary) => ChangeBody::Binary,
                Some(DiffSection::Text {
                    old_text,
                    new_text,
                    lines,
                    truncated,
                }) => {
                    if old_text == new_text && !truncated {
                        ChangeBody::NoContentChange
                    } else {
                        ChangeBody::Text {
                            old_text,
                            new_text,
                            lines,
                            truncated,
                        }
                    }
                }
                // A pure rename and a mode-only change carry no hunks at
                // all; so does a header this parser could not match back to
                // its path. All three are honestly "nothing to show here".
                None => ChangeBody::NoContentChange,
            }
        };
        let (added, removed) = count_lines(&body);
        diff.files.push(FileChange {
            path: entry.path,
            old_path: entry.old_path,
            status: entry.status,
            note: entry.note,
            added,
            removed,
            body,
        });
    }

    let untracked = run_git(
        git,
        root,
        &["ls-files", "-o", "--exclude-standard", "-z"],
        timeout,
        MAX_STDOUT_BYTES,
    )?;
    if untracked.ok {
        diff.stdout_truncated |= untracked.truncated;
        for path in split_nul(&untracked.stdout) {
            let Ok(abs) = code_source::resolve_source_path(root, &path, exclude) else {
                diff.hidden += 1;
                continue;
            };
            if abs.is_dir() {
                continue;
            }
            let body = match code_source::read_source(&abs) {
                Ok(SourceContent::Binary { .. }) => ChangeBody::Binary,
                Ok(SourceContent::Text { text, truncated }) => full_add(&text, truncated),
                // Gone between the two calls, or unreadable: it is not a
                // change anyone can look at, so it is not a row.
                Err(_) => continue,
            };
            let (added, removed) = count_lines(&body);
            diff.files.push(FileChange {
                path,
                old_path: None,
                status: ChangeStatus::Added,
                note: Some("untracked".to_string()),
                added,
                removed,
                body,
            });
        }
    }

    if diff.files.len() > MAX_SECTIONS {
        diff.sections_capped = true;
        for file in diff.files.iter_mut().skip(MAX_SECTIONS) {
            file.body = ChangeBody::Omitted(
                "section hidden — too many changed files. Open it in the Code view.".to_string(),
            );
        }
    }

    Ok(diff)
}

/// An untracked file: every line an addition, both sides reconstructed the
/// same way a tracked file's are.
fn full_add(text: &str, truncated: bool) -> ChangeBody {
    let mut lines = Vec::new();
    for (i, line) in split_keep_last(text).into_iter().enumerate() {
        lines.push(DiffLine {
            kind: LineKind::Added,
            old_no: None,
            new_no: Some(i + 1),
            text: line,
        });
    }
    ChangeBody::Text {
        old_text: String::new(),
        new_text: text.to_string(),
        lines,
        truncated,
    }
}

/// Split into lines without inventing a trailing empty one for a file that
/// ends in a newline, and without losing a last line that does not.
fn split_keep_last(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let body = text.strip_suffix('\n').unwrap_or(text);
    body.split('\n').map(|l| l.to_string()).collect()
}

fn count_lines(body: &ChangeBody) -> (usize, usize) {
    match body {
        ChangeBody::Text { lines, .. } => {
            let added = lines.iter().filter(|l| l.kind == LineKind::Added).count();
            let removed = lines.iter().filter(|l| l.kind == LineKind::Removed).count();
            (added, removed)
        }
        _ => (0, 0),
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

/// The stderr of `git diff HEAD` in a repository that has no commits yet.
fn head_is_unborn(stderr: &str) -> bool {
    let s = stderr.to_lowercase();
    s.contains("bad revision") || s.contains("unknown revision") || s.contains("ambiguous argument")
}

// ---------------------------------------------------------------------------
// `--name-status -z` parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    status: ChangeStatus,
    path: String,
    old_path: Option<String>,
    note: Option<String>,
}

fn split_nul(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|f| !f.is_empty())
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect()
}

/// `M\0path\0`, `R100\0old\0new\0`, … — the status field is NUL-terminated
/// like the paths are, and a rename or copy is followed by two of them.
fn parse_name_status(bytes: &[u8]) -> Vec<Entry> {
    let fields = split_nul(bytes);
    let mut out = Vec::new();
    let mut i = 0;
    while i < fields.len() {
        let raw = fields[i].clone();
        let letter = raw.chars().next().unwrap_or('M');
        i += 1;
        let two_paths = matches!(letter, 'R' | 'C');
        let Some(first) = fields.get(i).cloned() else {
            break;
        };
        i += 1;
        let second = if two_paths {
            let s = fields.get(i).cloned();
            i += 1;
            s
        } else {
            None
        };
        let (status, note) = match letter {
            'A' => (ChangeStatus::Added, None),
            'D' => (ChangeStatus::Deleted, None),
            'R' => (ChangeStatus::Renamed, None),
            'C' => (ChangeStatus::Added, Some("copied".to_string())),
            'T' => (ChangeStatus::Modified, Some("type changed".to_string())),
            'U' => (ChangeStatus::Modified, Some("conflicted".to_string())),
            'M' => (ChangeStatus::Modified, None),
            _ => (ChangeStatus::Modified, None),
        };
        let (path, old_path) = match (two_paths, second) {
            // A copy's source is not a rename's source: the file at the old
            // name is untouched, so the row is a plain add.
            (true, Some(new)) if letter == 'C' => (new, None),
            (true, Some(new)) => (new, Some(first)),
            _ => (first, None),
        };
        out.push(Entry {
            status,
            path,
            old_path,
            note,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// patch parsing
// ---------------------------------------------------------------------------

enum DiffSection {
    Binary,
    Text {
        old_text: String,
        new_text: String,
        lines: Vec<DiffLine>,
        truncated: bool,
    },
}

/// Cut the patch into its `diff --git a/<old> b/<new>` sections, keyed by
/// that header line's remainder. The prefixes are forced on the command
/// line, so a user's `diff.mnemonicPrefix` or `diff.noprefix` cannot change
/// the shape this reads, and the key matches back onto the authoritative
/// `--name-status` list without re-parsing a path out of it.
fn parse_patch(patch: &str) -> HashMap<String, DiffSection> {
    let mut out = HashMap::new();
    let mut key: Option<String> = None;
    let mut body: Vec<&str> = Vec::new();
    // The patch's own trailing newline must not read as one more empty
    // context line — that would append a line to both sides of the last file.
    for line in patch.strip_suffix('\n').unwrap_or(patch).split('\n') {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(k) = key.take() {
                out.insert(k, parse_section(&body));
            }
            body.clear();
            key = Some(rest.to_string());
            continue;
        }
        if key.is_some() {
            body.push(line);
        }
    }
    if let Some(k) = key {
        out.insert(k, parse_section(&body));
    }
    out
}

/// One file's section: a header block, then hunks. The walk reconstructs
/// both sides whole — context lines feed each, a removal feeds only the old
/// side, an addition only the new — and records the line numbers each side
/// gives every line, which is the pairing a side-by-side table renders from.
fn parse_section(body: &[&str]) -> DiffSection {
    let mut old_text = String::new();
    let mut new_text = String::new();
    let mut lines: Vec<DiffLine> = Vec::new();
    let mut truncated = false;
    let mut old_no = 0usize;
    let mut new_no = 0usize;
    // What the next hunk's old side must start at for the walk to be
    // continuous, and how many lines of the current hunk each side still
    // owes — the patch's own accounting, so nothing outside a hunk body is
    // ever mistaken for content.
    let mut next_old = 0usize;
    let mut rem_old = 0usize;
    let mut rem_new = 0usize;
    let mut last_kind: Option<LineKind> = None;

    for line in body {
        let line = *line;
        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            return DiffSection::Binary;
        }
        if let Some(hunk) = parse_hunk_header(line) {
            // With a context radius this large a second hunk means the file
            // outran it: the reconstruction skips whatever fell in the gap,
            // so the sides are a prefix rather than the whole file.
            if next_old != 0 && hunk.old_start != next_old {
                truncated = true;
            }
            old_no = hunk.old_start.saturating_sub(1);
            new_no = hunk.new_start.saturating_sub(1);
            next_old = hunk.old_start + hunk.old_count;
            rem_old = hunk.old_count;
            rem_new = hunk.new_count;
            last_kind = None;
            continue;
        }
        if line.starts_with('\\') {
            // "\ No newline at end of file" — the side(s) the previous line
            // fed do not actually end in one. Checked before the accounting
            // guard below because it counts against no hunk line total, and
            // on a file's last line it arrives with both sides already full.
            match last_kind {
                Some(LineKind::Context) => {
                    pop_newline(&mut old_text);
                    pop_newline(&mut new_text);
                }
                Some(LineKind::Removed) => pop_newline(&mut old_text),
                Some(LineKind::Added) => pop_newline(&mut new_text),
                None => {}
            }
            continue;
        }
        if rem_old + rem_new == 0 || truncated {
            continue; // between hunks, still in the header block, or past a cap
        }
        if old_text.len() > MAX_SIDE_BYTES || new_text.len() > MAX_SIDE_BYTES {
            truncated = true;
            continue;
        }
        let (kind, text) = match line.chars().next() {
            Some(' ') => (LineKind::Context, &line[1..]),
            Some('-') => (LineKind::Removed, &line[1..]),
            Some('+') => (LineKind::Added, &line[1..]),
            // git writes an empty context line as a bare space, but a patch
            // whose config dropped it still means an empty context line here.
            None => (LineKind::Context, ""),
            _ => continue,
        };
        let (l_old, l_new) = match kind {
            LineKind::Context => {
                old_no += 1;
                new_no += 1;
                rem_old = rem_old.saturating_sub(1);
                rem_new = rem_new.saturating_sub(1);
                old_text.push_str(text);
                old_text.push('\n');
                new_text.push_str(text);
                new_text.push('\n');
                (Some(old_no), Some(new_no))
            }
            LineKind::Removed => {
                old_no += 1;
                rem_old = rem_old.saturating_sub(1);
                old_text.push_str(text);
                old_text.push('\n');
                (Some(old_no), None)
            }
            LineKind::Added => {
                new_no += 1;
                rem_new = rem_new.saturating_sub(1);
                new_text.push_str(text);
                new_text.push('\n');
                (None, Some(new_no))
            }
        };
        last_kind = Some(kind);
        lines.push(DiffLine {
            kind,
            old_no: l_old,
            new_no: l_new,
            text: text.to_string(),
        });
    }

    DiffSection::Text {
        old_text,
        new_text,
        lines,
        truncated,
    }
}

fn pop_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
    }
}

struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
}

/// `@@ -12,7 +12,9 @@ fn thing()` → starts 12/12, counts 7/9. A missing
/// count means one line; a zero start (an empty side) reads as line 1 of
/// nothing.
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let rest = line.strip_prefix("@@ ")?;
    let end = rest.find(" @@")?;
    let mut parts = rest[..end].split(' ');
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let (old_start, old_count) = parse_range(old)?;
    let (new_start, new_count) = parse_range(new)?;
    Some(Hunk {
        old_start: old_start.max(1),
        old_count,
        new_start: new_start.max(1),
        new_count,
    })
}

fn parse_range(field: &str) -> Option<(usize, usize)> {
    let mut halves = field.split(',');
    let start: usize = halves.next()?.parse().ok()?;
    let count: usize = match halves.next() {
        Some(c) => c.parse().ok()?,
        None => 1,
    };
    Some((start, count))
}

// ---------------------------------------------------------------------------
// subprocess plumbing
// ---------------------------------------------------------------------------

struct RunOutput {
    stdout: Vec<u8>,
    stderr: String,
    ok: bool,
    truncated: bool,
}

/// Run one git call with a hard deadline. stdout and stderr are drained by
/// their own threads, so a child that writes more than a pipe buffer can
/// hold never deadlocks against our own wait; the wait itself polls, so the
/// deadline is honoured even when the child never exits on its own.
fn run_git(
    git: &str,
    cwd: &Path,
    args: &[&str],
    timeout: Duration,
    cap: usize,
) -> Result<RunOutput, GitDiffError> {
    let mut cmd = Command::new(git);
    cmd.arg("-C")
        .arg(cwd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // A pager would never exit; a lock write would touch the user's
        // repository; a localized error message would defeat the stderr
        // checks above.
        .env("GIT_PAGER", "cat")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C");

    let mut child = cmd
        .spawn()
        .map_err(|e| GitDiffError::GitUnavailable(e.to_string()))?;
    let mut out_pipe = child.stdout.take().expect("stdout was piped");
    let mut err_pipe = child.stderr.take().expect("stderr was piped");
    let out_reader = thread::spawn(move || read_capped(&mut out_pipe, cap));
    let err_reader = thread::spawn(move || read_capped(&mut err_pipe, MAX_STDERR_BYTES));

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(e) => return Err(GitDiffError::GitUnavailable(e.to_string())),
        }
        if Instant::now() >= deadline {
            break None;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let Some(status) = status else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(GitDiffError::Timeout(timeout.as_secs()));
    };

    let (stdout, truncated) = out_reader.join().unwrap_or_else(|_| (Vec::new(), false));
    let (stderr, _) = err_reader.join().unwrap_or_else(|_| (Vec::new(), false));
    Ok(RunOutput {
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        ok: status.success(),
        truncated,
    })
}

fn read_capped<R: Read>(reader: &mut R, cap: usize) -> (Vec<u8>, bool) {
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return (buf, false),
            Ok(n) => {
                if buf.len() + n > cap {
                    let room = cap.saturating_sub(buf.len());
                    buf.extend_from_slice(&chunk[..room]);
                    return (buf, true);
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            Err(_) => return (buf, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Every test here drives the real `git` binary against a real fixture
    /// repository — the parsers exist to read what that binary actually
    /// prints, and a hand-written sample would only prove the sample. A
    /// machine without git skips them rather than failing on someone else's
    /// missing tool.
    fn git_present() -> bool {
        Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// A fixture git call: the developer's own global/system config is cut
    /// out, so a machine with (say) `core.autocrlf` set cannot change what
    /// these fixtures contain.
    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/nonexistent/waggledance-git-global")
            .env("GIT_CONFIG_SYSTEM", "/nonexistent/waggledance-git-system")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git runs");
        assert!(status.success(), "fixture git {args:?} failed");
    }

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    /// A project root is always canonical in production (`Engine::register`
    /// canonicalises it), and `code_source`'s containment check compares
    /// against it — so a fixture whose temp dir sits behind a symlinked
    /// `/tmp` would fail the guard for the wrong reason.
    fn root_of(tmp: &TempDir) -> std::path::PathBuf {
        fs::canonicalize(tmp.path()).unwrap()
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-q", "."]);
        git(dir, &["config", "core.autocrlf", "false"]);
        git(dir, &["config", "commit.gpgsign", "false"]);
    }

    fn commit_all(dir: &Path, msg: &str) {
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", msg]);
    }

    fn find<'a>(diff: &'a WorkingTreeDiff, path: &str) -> &'a FileChange {
        diff.files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("no row for {path} in {:?}", paths(diff)))
    }

    fn paths(diff: &WorkingTreeDiff) -> Vec<&str> {
        diff.files.iter().map(|f| f.path.as_str()).collect()
    }

    fn sides(body: &ChangeBody) -> (&str, &str) {
        match body {
            ChangeBody::Text {
                old_text, new_text, ..
            } => (old_text, new_text),
            other => panic!("expected reconstructed text, got {other:?}"),
        }
    }

    #[test]
    fn every_status_lands_with_byte_exact_sides() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "mod.txt", "a\nb\nc\n");
        write(root, "del.txt", "old\n");
        write(root, "ren ame.txt", "r1\nr2\n");
        commit_all(root, "init");

        write(root, "mod.txt", "a\nB\nc\n");
        fs::remove_file(root.join("del.txt")).unwrap();
        git(root, &["mv", "ren ame.txt", "renamed uy.txt"]);
        write(root, "renamed uy.txt", "r1\nr2\nr3\n");
        write(root, "added.txt", "new\n");
        git(root, &["add", "added.txt"]);
        write(root, "un.txt", "untracked\n");

        let diff = working_tree_diff(root, &[]).unwrap();
        assert_eq!(diff.hidden, 0);
        assert!(!diff.sections_capped);

        let m = find(&diff, "mod.txt");
        assert_eq!(m.status, ChangeStatus::Modified);
        assert_eq!(sides(&m.body), ("a\nb\nc\n", "a\nB\nc\n"));
        assert_eq!((m.added, m.removed), (1, 1));
        // The pairing a side-by-side table renders from: three old lines,
        // three new ones, the changed pair sharing row 2 on each side.
        let ChangeBody::Text { lines, .. } = &m.body else {
            panic!("text body");
        };
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[1].kind, LineKind::Removed);
        assert_eq!(lines[1].old_no, Some(2));
        assert_eq!(lines[1].new_no, None);
        assert_eq!(lines[2].kind, LineKind::Added);
        assert_eq!(lines[2].new_no, Some(2));

        let a = find(&diff, "added.txt");
        assert_eq!(a.status, ChangeStatus::Added);
        assert_eq!(sides(&a.body), ("", "new\n"));

        let d = find(&diff, "del.txt");
        assert_eq!(d.status, ChangeStatus::Deleted);
        assert_eq!(sides(&d.body), ("old\n", ""));

        let r = find(&diff, "renamed uy.txt");
        assert_eq!(r.status, ChangeStatus::Renamed);
        assert_eq!(r.old_path.as_deref(), Some("ren ame.txt"));
        assert_eq!(sides(&r.body), ("r1\nr2\n", "r1\nr2\nr3\n"));

        let u = find(&diff, "un.txt");
        assert_eq!(u.status, ChangeStatus::Added);
        assert_eq!(u.note.as_deref(), Some("untracked"));
        assert_eq!(sides(&u.body), ("", "untracked\n"));
    }

    #[test]
    fn file_without_trailing_newline_reconstructs_byte_exact() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "nl.txt", "one\ntwo");
        commit_all(root, "init");
        write(root, "nl.txt", "one\ntwo\nthree");

        let diff = working_tree_diff(root, &[]).unwrap();
        assert_eq!(
            sides(&find(&diff, "nl.txt").body),
            ("one\ntwo", "one\ntwo\nthree")
        );
    }

    #[test]
    fn a_changed_submodule_is_a_labelled_row_without_hunks() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "readme.md", "# outer\n");
        let inner = root.join("inner");
        fs::create_dir_all(&inner).unwrap();
        init_repo(&inner);
        write(&inner, "a.txt", "one\n");
        commit_all(&inner, "inner init");
        // A gitlink: the outer tree records the inner repository's commit,
        // which is what makes the outer path a directory rather than a file.
        commit_all(root, "outer init");
        write(&inner, "a.txt", "two\n");
        commit_all(&inner, "inner moves on");

        let diff = working_tree_diff(root, &[]).unwrap();
        let sub = find(&diff, "inner");
        assert_eq!(sub.status, ChangeStatus::Modified);
        assert_eq!(sub.body, ChangeBody::Submodule);
    }

    #[test]
    fn pure_rename_renders_as_a_row_without_hunks() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "same.txt", "unchanged\n");
        commit_all(root, "init");
        git(root, &["mv", "same.txt", "moved.txt"]);

        let diff = working_tree_diff(root, &[]).unwrap();
        let r = find(&diff, "moved.txt");
        assert_eq!(r.status, ChangeStatus::Renamed);
        assert_eq!(r.old_path.as_deref(), Some("same.txt"));
        assert_eq!(r.body, ChangeBody::NoContentChange);
    }

    #[test]
    fn denied_path_is_skipped_and_only_counted() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "src/lib.rs", "pub fn a() {}\n");
        write(root, "id_rsa", "-----BEGIN OPENSSH PRIVATE KEY-----\n");
        write(root, "node_modules/pkg/index.js", "module.exports = {}\n");
        commit_all(root, "init");
        write(root, "src/lib.rs", "pub fn b() {}\n");
        write(
            root,
            "id_rsa",
            "-----BEGIN OPENSSH PRIVATE KEY-----\nrotated\n",
        );
        write(root, "node_modules/pkg/index.js", "module.exports = 1\n");
        // An untracked denied file must be counted the same way a tracked
        // one is — the two lists reach the filter by different routes.
        write(root, "secrets.txt", "token=1\n");

        let diff = working_tree_diff(root, &["node_modules".to_string()]).unwrap();
        assert_eq!(paths(&diff), vec!["src/lib.rs"]);
        assert_eq!(diff.hidden, 3);
        let rendered = format!("{diff:?}");
        for denied in ["id_rsa", "node_modules", "secrets.txt"] {
            assert!(
                !rendered.contains(denied),
                "a denied path must never reach the diff, not even as a name: {denied}"
            );
        }
    }

    #[test]
    fn project_root_below_the_toplevel_shows_only_its_own_files() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let top = &root_of(&tmp);
        init_repo(top);
        write(top, "outside.txt", "o\n");
        write(top, "sub/inside.txt", "i\n");
        write(top, "sub/nested/tệp.txt", "x\n");
        commit_all(top, "init");
        write(top, "outside.txt", "O\n");
        write(top, "sub/inside.txt", "I\n");
        write(top, "sub/nested/tệp.txt", "y\n");
        write(top, "sub/untracked có dấu.txt", "u\n");

        let diff = working_tree_diff(&top.join("sub"), &[]).unwrap();
        assert_eq!(
            paths(&diff),
            vec!["inside.txt", "nested/tệp.txt", "untracked có dấu.txt"],
            "paths are project-relative and the toplevel's own files stay out"
        );
        assert_eq!(sides(&find(&diff, "nested/tệp.txt").body), ("x\n", "y\n"));
    }

    #[test]
    fn binary_change_is_a_row_without_hunks() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        fs::write(root.join("blob.bin"), [0x00u8, 0x01, 0x02]).unwrap();
        commit_all(root, "init");
        fs::write(root.join("blob.bin"), [0x00u8, 0x01, 0x03]).unwrap();
        fs::write(root.join("fresh.bin"), [0x00u8, 0xff]).unwrap();

        let diff = working_tree_diff(root, &[]).unwrap();
        assert_eq!(find(&diff, "blob.bin").body, ChangeBody::Binary);
        assert_eq!(find(&diff, "fresh.bin").body, ChangeBody::Binary);
    }

    #[test]
    fn untracked_file_past_the_read_cap_is_truncated_not_dropped() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "seed.txt", "seed\n");
        commit_all(root, "init");
        let mut big = String::with_capacity(MAX_SIDE_BYTES + 4096);
        while big.len() < MAX_SIDE_BYTES + 2048 {
            big.push_str(&"x".repeat(120));
            big.push('\n');
        }
        write(root, "big.txt", &big);

        let diff = working_tree_diff(root, &[]).unwrap();
        let ChangeBody::Text {
            new_text,
            truncated,
            ..
        } = &find(&diff, "big.txt").body
        else {
            panic!("text body");
        };
        assert!(truncated, "past the cap the row says so");
        assert!(new_text.len() <= MAX_SIDE_BYTES);
    }

    #[test]
    fn more_files_than_the_section_cap_still_all_get_rows() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "seed.txt", "seed\n");
        commit_all(root, "init");
        for i in 0..MAX_SECTIONS + 5 {
            write(root, &format!("f{i:03}.txt", i = i), "body\n");
        }

        let diff = working_tree_diff(root, &[]).unwrap();
        assert_eq!(diff.files.len(), MAX_SECTIONS + 5);
        assert!(diff.sections_capped);
        assert!(matches!(
            diff.files[MAX_SECTIONS].body,
            ChangeBody::Omitted(_)
        ));
        assert!(matches!(
            diff.files[MAX_SECTIONS - 1].body,
            ChangeBody::Text { .. }
        ));
    }

    #[test]
    fn repository_without_commits_lists_its_files_as_untracked() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        init_repo(root);
        write(root, "first.txt", "hello\n");

        let diff = working_tree_diff(root, &[]).unwrap();
        assert_eq!(paths(&diff), vec!["first.txt"]);
        assert_eq!(find(&diff, "first.txt").status, ChangeStatus::Added);
    }

    #[test]
    fn a_project_that_is_not_a_repository_is_typed_not_an_error_page() {
        if !git_present() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = &root_of(&tmp);
        write(root, "readme.md", "# hi\n");
        assert_eq!(
            working_tree_diff(root, &[]).unwrap_err(),
            GitDiffError::NotARepo
        );
    }

    #[test]
    fn a_missing_git_binary_is_typed() {
        let tmp = TempDir::new().unwrap();
        let err = collect(
            tmp.path(),
            &[],
            "/nonexistent/waggledance-git-binary",
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(err, GitDiffError::GitUnavailable(_)), "{err:?}");
    }

    #[cfg(unix)]
    #[test]
    fn a_git_call_that_never_returns_is_killed_and_typed() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let stub = tmp.path().join("slow-git");
        fs::write(&stub, "#!/bin/sh\nsleep 30\n").unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();

        let started = Instant::now();
        let err = collect(
            tmp.path(),
            &[],
            stub.to_str().unwrap(),
            Duration::from_millis(200),
        )
        .unwrap_err();
        assert_eq!(err, GitDiffError::Timeout(0));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the deadline must end the call, not the child's own lifetime"
        );
    }

    #[test]
    fn name_status_parses_renames_and_rare_letters() {
        let raw = b"M\0mod.txt\0R100\0old name.txt\0new name.txt\0U\0conflict.txt\0T\0link.txt\0X\0odd.txt\0";
        let entries = parse_name_status(raw);
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[1].status, ChangeStatus::Renamed);
        assert_eq!(entries[1].path, "new name.txt");
        assert_eq!(entries[1].old_path.as_deref(), Some("old name.txt"));
        assert_eq!(entries[2].note.as_deref(), Some("conflicted"));
        assert_eq!(entries[2].status, ChangeStatus::Modified);
        assert_eq!(entries[3].status, ChangeStatus::Modified);
        // An unknown letter is a row, never a dropped change.
        assert_eq!(entries[4].path, "odd.txt");
        assert_eq!(entries[4].status, ChangeStatus::Modified);
    }
}
