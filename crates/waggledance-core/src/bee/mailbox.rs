//! Reader for `<root>/.bee/human-mailbox/` — the letters bee files for the
//! human (board-visibility slice 2, `docs/history/board-visibility/plan-slice-2.md`).
//!
//! This is a sibling of every other reader in [`crate::bee`]: strictly
//! read-only (bee-cockpit D4 — a letter file is NEVER opened for writing here
//! or anywhere else in waggledance; bee's own `bee mailbox mark` is the one
//! sanctioned mutation, human-mailbox D6), infallible, and error-tolerant.
//!
//! ## The contract is bee's, and it is closed
//!
//! human-mailbox D3 freezes the frontmatter key set — `subject`, `run`,
//! `project`, `filed_at`, `status`, `items[]`, `needs_you[]` — and makes the
//! first five required at read. A letter missing any of them is
//! [`BeeMailboxEntry::Unreadable`], never a letter with guessed fields.
//! `status` is the closed set `unread | read`; any other value is invalid.
//!
//! Two rules of bee's that shape this reader more than they look:
//!
//! - **`needs_human_decision` is not read at all.** bee emits it into each
//!   `needs_you` entry, but its own source (verbs/mailbox.rs, `NeedsYou::
//!   needs_human_decision`) says it is DERIVED on read and never authoritative,
//!   precisely so "a hand-edited `needs_human_decision:` line in a filed letter
//!   cannot claim a flag the kind does not give it". Reading it here would
//!   re-open the hole bee closed. `kind` is the only field it derives from, and
//!   `kind` is the one bee calls "the younger field": it is OPTIONAL, and a
//!   missing `kind` is never a parse failure.
//! - **An unreadable letter is surfaced, never dropped.** bee's own words
//!   (verbs/mailbox.rs, D12's preamble): "a silently missing letter is worse
//!   than a noisy one: the human would read an empty mailbox as a quiet night
//!   rather than as a broken store." So a broken letter becomes an
//!   [`BeeMailboxEntry::Unreadable`] carrying its file name and why, and it
//!   travels to the board beside the readable ones.
//!
//! ## The id is the file name
//!
//! human-mailbox D11: one letter per run, named
//! `<UTC-timestamp>-<short-run-slug>.md`, and "a directory listing is the
//! index" — there is no manifest and no index stream. So [`BeeLetter::id`] is
//! the file's own name (what `bee mailbox mark --id` takes), and the entries
//! come back sorted by it, which is chronological by construction.
//!
//! ## Why yaml-rust
//!
//! bee JSON-quotes every scalar it emits and says so at its emitter: the
//! emitted block "must be valid YAML for the consuming inbox's real parser".
//! Hand-rolling a parser here would work against that stated contract, so this
//! module uses `yaml-rust`, which this build already compiled transitively —
//! promoting it to a direct dependency adds no new supply-chain surface. The
//! trade-off (the crate is unmaintained) is named in the slice plan.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use yaml_rust::{yaml::Hash, Yaml, YamlLoader};

/// `.bee/`-relative name of the mailbox directory (bee's `mailbox_dir`).
const MAILBOX_DIR: &str = "human-mailbox";

/// Where a project's letters live. Public because the surfaces built on this
/// reader (rendering a letter's body, handing an id to `bee mailbox mark`)
/// need the same path, and one module should own it.
pub fn mailbox_dir(root: &Path) -> PathBuf {
    root.join(".bee").join(MAILBOX_DIR)
}

/// A letter's read state — bee's closed set, so an unrecognized value cannot
/// be represented here at all (it makes the letter
/// [`BeeMailboxEntry::Unreadable`] instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeeLetterStatus {
    Unread,
    Read,
}

impl BeeLetterStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BeeLetterStatus::Unread => "unread",
            BeeLetterStatus::Read => "read",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "unread" => Some(BeeLetterStatus::Unread),
            "read" => Some(BeeLetterStatus::Read),
            _ => None,
        }
    }
}

/// A named departure from the plan, attached to one item. `null` in the
/// letter, or all three of `what`/`why`/`kind` — bee emits no other shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeeLetterDeparture {
    pub what: String,
    pub why: String,
    pub kind: String,
}

/// One thing the run did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeeLetterItem {
    pub what: String,
    /// Empty when the item touched no files — bee emits `files: []` for that.
    pub files: Vec<String>,
    pub commit: Option<String>,
    pub proof: Option<String>,
    pub departure: Option<BeeLetterDeparture>,
}

/// One ask only the human can answer (human-mailbox D13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeeLetterNeedsYou {
    pub id: String,
    pub what: String,
    pub blocks: String,
    /// bee's "younger field": every ask filed before `kind` existed carries
    /// none, and a missing `kind` is never a parse failure. The
    /// `needs_human_decision` flag bee derives from this field is deliberately
    /// NOT read — see the module doc.
    pub kind: Option<String>,
}

/// One readable letter, its frontmatter typed. The body is not read here: the
/// markdown pipeline already renders a file from disk, and holding every
/// letter's prose in every snapshot would put the whole mailbox behind each
/// page load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeeLetter {
    /// The letter's file name, which D11 makes its only id — exactly what
    /// `bee mailbox mark --id` takes.
    pub id: String,
    pub subject: String,
    pub run: String,
    pub project: String,
    pub filed_at: String,
    pub status: BeeLetterStatus,
    /// `None` means the `items` key was ABSENT — a shape bee's emitter never
    /// writes, since it always emits at least `items: []`. `Some(vec![])` is
    /// the honest "this run listed nothing". Use [`BeeLetter::items_or_empty`]
    /// when the difference does not matter.
    pub items: Option<Vec<BeeLetterItem>>,
    /// Same absent-vs-empty distinction as [`BeeLetter::items`].
    pub needs_you: Option<Vec<BeeLetterNeedsYou>>,
}

impl BeeLetter {
    pub fn items_or_empty(&self) -> &[BeeLetterItem] {
        self.items.as_deref().unwrap_or(&[])
    }

    pub fn needs_you_or_empty(&self) -> &[BeeLetterNeedsYou] {
        self.needs_you.as_deref().unwrap_or(&[])
    }
}

/// One entry in a project's mailbox. A letter that cannot be parsed is an
/// `Unreadable` entry rather than a gap in the list — bee's own reason: a
/// silently missing letter reads to the human as a quiet night rather than as
/// a broken store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BeeMailboxEntry {
    Letter(BeeLetter),
    Unreadable { file: String, reason: String },
}

impl BeeMailboxEntry {
    pub fn letter(&self) -> Option<&BeeLetter> {
        match self {
            BeeMailboxEntry::Letter(l) => Some(l),
            BeeMailboxEntry::Unreadable { .. } => None,
        }
    }

    /// The file this entry came from, readable or not — the letter's id.
    pub fn file(&self) -> &str {
        match self {
            BeeMailboxEntry::Letter(l) => &l.id,
            BeeMailboxEntry::Unreadable { file, .. } => file,
        }
    }

    /// True only for a letter that parsed AND is still unread. An unreadable
    /// entry is never counted as unread — it is its own, louder signal.
    pub fn is_unread(&self) -> bool {
        matches!(
            self,
            BeeMailboxEntry::Letter(BeeLetter {
                status: BeeLetterStatus::Unread,
                ..
            })
        )
    }
}

/// Read every `*.md` letter under `<bee_dir>/human-mailbox/`, sorted by file
/// name (which D11 makes chronological).
///
/// A missing mailbox directory yields an empty list, not an error — that is
/// the normal shape for a checkout whose unattended loop has never run, and
/// bee only composes a letter from an armed run. A directory that exists but
/// cannot be listed is a different thing entirely and comes back as a single
/// `Unreadable` entry, because that one IS a broken store.
pub(crate) fn read_mailbox(bee_dir: &Path) -> Vec<BeeMailboxEntry> {
    let dir = bee_dir.join(MAILBOX_DIR);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect(),
        Err(e) => {
            return vec![BeeMailboxEntry::Unreadable {
                file: format!(".bee/{MAILBOX_DIR}"),
                reason: format!("could not list the mailbox directory ({e})"),
            }]
        }
    };
    files.sort();
    files.iter().map(|p| read_letter(p)).collect()
}

/// One letter file, opened for READING only.
fn read_letter(path: &Path) -> BeeMailboxEntry {
    let file = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            return BeeMailboxEntry::Unreadable {
                file,
                reason: format!("could not read the letter ({e})"),
            }
        }
    };
    match parse_letter(&file, &text) {
        Ok(letter) => BeeMailboxEntry::Letter(letter),
        Err(reason) => BeeMailboxEntry::Unreadable { file, reason },
    }
}

/// The frontmatter block between the opening `---` fence and its closing one.
/// Mirrors bee's own splitter, leading BOM included, so a letter bee writes
/// splits here exactly as it splits there.
fn split_frontmatter(text: &str) -> Option<&str> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == "---" {
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }
    None
}

/// Parse one letter's frontmatter, or say in one line why it cannot be read.
/// Every `Err` string here is written to be shown to a human beside the file
/// name: it names the field, not the parser.
fn parse_letter(id: &str, text: &str) -> Result<BeeLetter, String> {
    let front = split_frontmatter(text).ok_or_else(|| {
        "no YAML frontmatter: a letter opens with a `---` line and closes the block with another"
            .to_string()
    })?;
    let docs = YamlLoader::load_from_str(front)
        .map_err(|e| format!("frontmatter is not valid YAML ({e})"))?;
    let doc = docs
        .first()
        .ok_or_else(|| "frontmatter block is empty".to_string())?;
    let map = doc
        .as_hash()
        .ok_or_else(|| "frontmatter is not a block of key/value pairs".to_string())?;

    // The five human-mailbox D3 makes required at read. Order matters only
    // for which one a doubly-broken letter names first.
    let subject = required_str(map, "", "subject")?;
    let run = required_str(map, "", "run")?;
    let project = required_str(map, "", "project")?;
    let filed_at = required_str(map, "", "filed_at")?;
    let status_raw = required_str(map, "", "status")?;
    let status = BeeLetterStatus::parse(&status_raw).ok_or_else(|| {
        format!("`status` is {status_raw:?}, and a letter's status is exactly \"unread\" or \"read\"")
    })?;

    let items = match get(map, "items") {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => None,
        Some(v) => Some(parse_items(v)?),
    };
    let needs_you = match get(map, "needs_you") {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => None,
        Some(v) => Some(parse_needs_you(v)?),
    };

    // `needs_human_decision` is deliberately never read — see the module doc.

    Ok(BeeLetter {
        id: id.to_string(),
        subject,
        run,
        project,
        filed_at,
        status,
        items,
        needs_you,
    })
}

fn parse_items(v: &Yaml) -> Result<Vec<BeeLetterItem>, String> {
    let rows = v
        .as_vec()
        .ok_or_else(|| "`items` is not a list".to_string())?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let ctx = format!("items[{i}].");
        let m = row
            .as_hash()
            .ok_or_else(|| format!("items[{i}] is not a block of key/value pairs"))?;
        out.push(BeeLetterItem {
            what: required_str(m, &ctx, "what")?,
            files: string_list(m, &ctx, "files")?,
            commit: optional_str(m, &ctx, "commit")?,
            proof: optional_str(m, &ctx, "proof")?,
            departure: parse_departure(m, &ctx)?,
        });
    }
    Ok(out)
}

fn parse_departure(m: &Hash, ctx: &str) -> Result<Option<BeeLetterDeparture>, String> {
    let v = match get(m, "departure") {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => return Ok(None),
        Some(v) => v,
    };
    let d = v
        .as_hash()
        .ok_or_else(|| format!("`{ctx}departure` is neither null nor a what/why/kind block"))?;
    let ctx = format!("{ctx}departure.");
    Ok(Some(BeeLetterDeparture {
        what: required_str(d, &ctx, "what")?,
        why: required_str(d, &ctx, "why")?,
        kind: required_str(d, &ctx, "kind")?,
    }))
}

fn parse_needs_you(v: &Yaml) -> Result<Vec<BeeLetterNeedsYou>, String> {
    let rows = v
        .as_vec()
        .ok_or_else(|| "`needs_you` is not a list".to_string())?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let ctx = format!("needs_you[{i}].");
        let m = row
            .as_hash()
            .ok_or_else(|| format!("needs_you[{i}] is not a block of key/value pairs"))?;
        out.push(BeeLetterNeedsYou {
            id: required_str(m, &ctx, "id")?,
            what: required_str(m, &ctx, "what")?,
            blocks: required_str(m, &ctx, "blocks")?,
            // Optional by bee's own rule; a missing `kind` never fails a parse.
            kind: optional_str(m, &ctx, "kind")?,
        });
    }
    Ok(out)
}

/// yaml-rust keys are `Yaml` values; every key bee emits is a plain string, so
/// this is the one lookup shape the whole module needs. A `BadValue` never
/// reaches here — an absent key is `None`, not a sentinel.
fn get<'a>(map: &'a Hash, key: &str) -> Option<&'a Yaml> {
    map.get(&Yaml::String(key.to_string()))
}

fn required_str(map: &Hash, ctx: &str, key: &str) -> Result<String, String> {
    match get(map, key) {
        None | Some(Yaml::BadValue) => Err(format!("missing required field `{ctx}{key}`")),
        Some(v) => v
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("`{ctx}{key}` is not a string")),
    }
}

fn optional_str(map: &Hash, ctx: &str, key: &str) -> Result<Option<String>, String> {
    match get(map, key) {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("`{ctx}{key}` is neither a string nor null")),
    }
}

fn string_list(map: &Hash, ctx: &str, key: &str) -> Result<Vec<String>, String> {
    let v = match get(map, key) {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => return Ok(Vec::new()),
        Some(v) => v,
    };
    let rows = v
        .as_vec()
        .ok_or_else(|| format!("`{ctx}{key}` is not a list"))?;
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            r.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("`{ctx}{key}[{i}]` is not a string"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A letter EXACTLY as bee's `render_letter` emits one: every scalar
    /// JSON-quoted, the key order fixed, the `- ` marker riding the item's
    /// first key with siblings two further in, `departure: null` spelled out,
    /// and `needs_human_decision` emitted after each ask's `kind`. Read off
    /// beehive `packages/bee-rs/crates/bee/src/verbs/mailbox.rs`
    /// (`render_letter`), not imagined.
    const WELL_FORMED: &str = "\
---
subject: \"Run finished: 3 cells capped, 1 needs you\"
run: \"2026-08-30-nightly\"
project: \"waggledance\"
filed_at: \"2026-08-30T04:12:07Z\"
status: \"unread\"
items:
  - what: \"Read the filed letters out of each project's mailbox\"
    files:
      - \"crates/waggledance-core/src/bee/mailbox.rs\"
      - \"crates/waggledance-core/src/bee.rs\"
    commit: \"89786dd\"
    proof: \"cargo test -p waggledance-core — green — reader only\"
    departure: null
  - what: \"Skip the entries stream\"
    files: []
    commit: null
    proof: null
    departure:
      what: \"letters only, no entries/*.jsonl\"
      why: \"the entry line is free to grow keys; the letter is frozen\"
      kind: \"found a better route\"
needs_you:
  - id: \"arm-herding\"
    what: \"Decide whether to arm the unattended loop on this machine\"
    blocks: \"bi-5\"
    kind: \"decision\"
    needs_human_decision: true
  - id: \"name-the-route\"
    what: \"Confirm /inbox as the route\"
    blocks: \"bi-2\"
    kind: null
    needs_human_decision: false
---

The night's work, in one page.
";

    /// The same letter with `items` and `needs_you` emitted empty — bee's
    /// literal `items: []` / `needs_you: []`.
    const EMPTY_LISTS: &str = "\
---
subject: \"A quiet run\"
run: \"2026-08-30-quiet\"
project: \"waggledance\"
filed_at: \"2026-08-30T05:00:00Z\"
status: \"read\"
items: []
needs_you: []
---

Nothing to report.
";

    /// The bytes of a letter bee ACTUALLY composed, copied verbatim off
    /// `<worktree>/target/bi5-scratch/.bee/human-mailbox/20260830T114615Z-bi5-scratch-run.md`
    /// (`sha256 f48f7c17d453cb9ae2c18978c1d48a20dee909d0d96f22c7b3a1eb744ac15e00`).
    /// That file was produced by real verbs in a scratch store — `bee cells
    /// finish` for the cap, `bee cells block` for the blocker carrying a
    /// `needs_you`, then `bee work set --status done` firing the composer —
    /// and its `status` is `read` because bi-5's proof flipped it through
    /// `bee mailbox mark`. Kept here as a literal rather than read off disk
    /// because `target/` is scratch: the shape is what must not regress, and
    /// the shape has to survive a clean checkout.
    ///
    /// Recorded in `docs/history/board-visibility/proof-slice-2.md`, leg (a).
    const REAL_BEE_LETTER: &str = r#"---
subject: "Add the scratch reader file"
run: "bi5-scratch-run"
project: "bi5-scratch"
filed_at: "2026-08-30T11:46:15.336Z"
status: "read"
items:
  - what: "Add the scratch reader file"
    files:
      - "reader.txt"
    commit: null
    proof: "true — green — scratch fixture, one file"
    departure: null
  - what: "the store path is not settled — the scratch reader needs a decision on where it reads from"
    files: []
    commit: null
    proof: null
    departure: null
needs_you:
  - id: "sx-2"
    what: "the store path is not settled — the scratch reader needs a decision on where it reads from"
    blocks: "Point the scratch reader at the store"
    kind: "question"
    needs_human_decision: true
---

## Done

- Add the scratch reader file

## Broken or unfinished

- the store path is not settled — the scratch reader needs a decision on where it reads from

## Needs your call

- [sx-2] the store path is not settled — the scratch reader needs a decision on where it reads from — blocks: Point the scratch reader at the store
"#;

    fn parse(text: &str) -> Result<BeeLetter, String> {
        parse_letter("2026-08-30T04-12-07Z-nightly.md", text)
    }

    /// Drop the top-level line whose key is `key` (indent 0 only), the way a
    /// truncated or hand-edited letter loses a field.
    fn without_top_level(text: &str, key: &str) -> String {
        let prefix = format!("{key}:");
        let mut out: String = text
            .lines()
            .filter(|l| !l.starts_with(&prefix))
            .map(|l| format!("{l}\n"))
            .collect();
        if !text.ends_with('\n') {
            out.pop();
        }
        out
    }

    fn write_letter(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), body).unwrap();
    }

    // --- the happy path -------------------------------------------------

    #[test]
    fn well_formed_letter_parses_every_frontmatter_field() {
        let letter = parse(WELL_FORMED).expect("bee's own emitter shape must parse");
        assert_eq!(letter.id, "2026-08-30T04-12-07Z-nightly.md");
        assert_eq!(letter.subject, "Run finished: 3 cells capped, 1 needs you");
        assert_eq!(letter.run, "2026-08-30-nightly");
        assert_eq!(letter.project, "waggledance");
        assert_eq!(letter.filed_at, "2026-08-30T04:12:07Z");
        assert_eq!(letter.status, BeeLetterStatus::Unread);

        let items = letter.items_or_empty();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].what,
            "Read the filed letters out of each project's mailbox"
        );
        assert_eq!(
            items[0].files,
            vec![
                "crates/waggledance-core/src/bee/mailbox.rs".to_string(),
                "crates/waggledance-core/src/bee.rs".to_string(),
            ]
        );
        assert_eq!(items[0].commit.as_deref(), Some("89786dd"));
        assert_eq!(
            items[0].proof.as_deref(),
            Some("cargo test -p waggledance-core — green — reader only")
        );
        assert_eq!(items[0].departure, None);

        assert!(items[1].files.is_empty());
        assert_eq!(items[1].commit, None);
        assert_eq!(items[1].proof, None);
        let departure = items[1].departure.as_ref().expect("a 3-key departure map");
        assert_eq!(departure.what, "letters only, no entries/*.jsonl");
        assert_eq!(
            departure.why,
            "the entry line is free to grow keys; the letter is frozen"
        );
        assert_eq!(departure.kind, "found a better route");

        let asks = letter.needs_you_or_empty();
        assert_eq!(asks.len(), 2);
        assert_eq!(asks[0].id, "arm-herding");
        assert_eq!(
            asks[0].what,
            "Decide whether to arm the unattended loop on this machine"
        );
        assert_eq!(asks[0].blocks, "bi-5");
        assert_eq!(asks[0].kind.as_deref(), Some("decision"));
        assert_eq!(asks[1].kind, None);
    }

    /// The proof cell's own finding: fixtures are written by the people who
    /// wrote the parser, so they agree with it by construction. This one is
    /// not — it is the emitter's output, byte for byte. It pins the two
    /// shapes the hand-written fixtures were guessed into rather than
    /// observed: `items[].files` as a NESTED BLOCK SEQUENCE under an indented
    /// key, and explicit `null` scalars for `commit` / `proof` / `departure`
    /// rather than absent keys.
    #[test]
    fn the_letter_bee_actually_composed_parses() {
        let letter = parse_letter("20260830T114615Z-bi5-scratch-run.md", REAL_BEE_LETTER)
            .expect("a letter bee's own emitter wrote must be readable");

        assert_eq!(letter.id, "20260830T114615Z-bi5-scratch-run.md");
        assert_eq!(letter.subject, "Add the scratch reader file");
        assert_eq!(letter.run, "bi5-scratch-run");
        assert_eq!(letter.project, "bi5-scratch");
        assert_eq!(letter.filed_at, "2026-08-30T11:46:15.336Z");
        // `read`, because `bee mailbox mark --id … --status read` flipped it.
        assert_eq!(letter.status, BeeLetterStatus::Read);

        let items = letter.items_or_empty();
        assert_eq!(items.len(), 2);
        // The nested block sequence, which no hand-written fixture forced.
        assert_eq!(items[0].files, vec!["reader.txt".to_string()]);
        assert_eq!(items[0].commit, None); // an explicit `null`, not an absent key
        assert_eq!(
            items[0].proof.as_deref(),
            Some("true — green — scratch fixture, one file")
        );
        assert_eq!(items[0].departure, None); // also an explicit `null`
        assert!(items[1].files.is_empty());
        assert_eq!(items[1].proof, None);

        let asks = letter.needs_you_or_empty();
        assert_eq!(asks.len(), 1);
        assert_eq!(asks[0].id, "sx-2");
        assert_eq!(asks[0].blocks, "Point the scratch reader at the store");
        assert_eq!(asks[0].kind.as_deref(), Some("question"));
    }

    // --- the closed contract, one refusal at a time ---------------------

    #[test]
    fn a_letter_missing_any_required_field_is_unreadable_and_names_it() {
        for key in ["subject", "run", "project", "filed_at", "status"] {
            let text = without_top_level(WELL_FORMED, key);
            let err = parse(&text).expect_err(&format!("`{key}` is required at read"));
            assert!(
                err.contains(key),
                "the reason must name the missing field, got: {err}"
            );
            assert!(
                err.contains("missing required field"),
                "the reason must say what went wrong, got: {err}"
            );
        }
    }

    #[test]
    fn a_status_outside_the_closed_set_is_unreadable() {
        let text = WELL_FORMED.replace("status: \"unread\"", "status: \"archived\"");
        let err = parse(&text).expect_err("status is a closed set");
        assert!(err.contains("archived"), "{err}");
        assert!(err.contains("unread"), "{err}");
    }

    #[test]
    fn an_item_without_what_is_unreadable_and_names_the_item() {
        let text = WELL_FORMED.replace(
            "  - what: \"Skip the entries stream\"\n",
            "  - proof: null\n",
        );
        let err = parse(&text).expect_err("an item's `what` is required");
        assert!(err.contains("items[1].what"), "{err}");
    }

    #[test]
    fn a_half_written_departure_is_unreadable() {
        let text = WELL_FORMED.replace("      why: \"the entry line is free to grow keys; the letter is frozen\"\n", "");
        let err = parse(&text).expect_err("a departure is null or all three keys");
        assert!(err.contains("departure.why"), "{err}");
    }

    #[test]
    fn frontmatter_that_is_not_yaml_is_unreadable_not_a_panic() {
        let text = "---\nsubject: \"unclosed\n---\n\nbody\n";
        let err = parse(text).expect_err("a broken scalar must not parse");
        assert!(err.contains("YAML"), "{err}");
    }

    #[test]
    fn a_file_with_no_frontmatter_fence_is_unreadable() {
        let err = parse("Just some prose, no fence at all.\n")
            .expect_err("a letter without frontmatter is not a letter");
        assert!(err.contains("frontmatter"), "{err}");
    }

    // --- empty vs absent, and the younger field -------------------------

    #[test]
    fn empty_lists_parse_as_empty_and_stay_distinguishable_from_absent() {
        let empty = parse(EMPTY_LISTS).expect("`items: []` is bee's own empty shape");
        assert_eq!(empty.items, Some(Vec::new()));
        assert_eq!(empty.needs_you, Some(Vec::new()));
        assert_eq!(empty.status, BeeLetterStatus::Read);

        let absent_text = without_top_level(&without_top_level(EMPTY_LISTS, "items"), "needs_you");
        let absent = parse(&absent_text).expect("the five required fields are still there");
        assert_eq!(absent.items, None, "an absent key is not an empty list");
        assert_eq!(absent.needs_you, None);
        // Both shapes still read as "nothing to show" for a caller that does
        // not care which it was.
        assert!(absent.items_or_empty().is_empty());
        assert!(absent.needs_you_or_empty().is_empty());
    }

    #[test]
    fn a_needs_you_entry_without_kind_parses() {
        // Not `kind: null` — the key gone entirely, as every ask filed before
        // `kind` existed carries it.
        let text = WELL_FORMED.replace("    kind: \"decision\"\n", "");
        let letter = parse(&text).expect("a missing `kind` is never a parse failure");
        assert_eq!(letter.needs_you_or_empty()[0].kind, None);
        assert_eq!(letter.needs_you_or_empty()[0].id, "arm-herding");
    }

    #[test]
    fn needs_human_decision_is_never_read_into_the_record() {
        // bee derives that flag from `kind` and calls the stored line
        // non-authoritative. Hand-flip it on the kind-less ask: nothing in the
        // parsed record may move.
        let honest = parse(WELL_FORMED).unwrap();
        let tampered_text = WELL_FORMED.replace(
            "    kind: null\n    needs_human_decision: false\n",
            "    kind: null\n    needs_human_decision: true\n",
        );
        let tampered = parse(&tampered_text).unwrap();
        assert_eq!(honest, tampered);

        // And a letter that never carried the key at all parses the same way.
        let stripped: String = WELL_FORMED
            .lines()
            .filter(|l| !l.trim_start().starts_with("needs_human_decision:"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(parse(&stripped).unwrap(), honest);
    }

    // --- the directory ---------------------------------------------------

    #[test]
    fn a_missing_or_empty_mailbox_is_an_empty_list_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let bee_dir = tmp.path().join(".bee");
        fs::create_dir_all(&bee_dir).unwrap();
        assert!(read_mailbox(&bee_dir).is_empty(), "no mailbox directory");

        fs::create_dir_all(bee_dir.join(MAILBOX_DIR)).unwrap();
        assert!(read_mailbox(&bee_dir).is_empty(), "an empty mailbox");
    }

    #[test]
    fn letters_come_back_sorted_and_a_broken_one_is_surfaced_beside_them() {
        let tmp = tempfile::tempdir().unwrap();
        let bee_dir = tmp.path().join(".bee");
        let dir = bee_dir.join(MAILBOX_DIR);
        write_letter(&dir, "2026-08-30T05-00-00Z-quiet.md", EMPTY_LISTS);
        write_letter(&dir, "2026-08-30T04-12-07Z-nightly.md", WELL_FORMED);
        write_letter(
            &dir,
            "2026-08-30T06-00-00Z-broken.md",
            &without_top_level(WELL_FORMED, "subject"),
        );
        // Not letters: the directory listing is the index, and it indexes .md.
        fs::write(dir.join("notes.txt"), "not a letter").unwrap();
        fs::create_dir_all(dir.join("attachments.md")).unwrap();

        let entries = read_mailbox(&bee_dir);
        assert_eq!(entries.len(), 3, "{entries:#?}");
        assert_eq!(entries[0].file(), "2026-08-30T04-12-07Z-nightly.md");
        assert_eq!(entries[1].file(), "2026-08-30T05-00-00Z-quiet.md");
        assert!(entries[0].is_unread());
        assert!(!entries[1].is_unread(), "that one is marked read");

        match &entries[2] {
            BeeMailboxEntry::Unreadable { file, reason } => {
                assert_eq!(file, "2026-08-30T06-00-00Z-broken.md");
                assert!(reason.contains("subject"), "{reason}");
            }
            other => panic!("a broken letter must be surfaced, got {other:?}"),
        }
        assert!(
            !entries[2].is_unread(),
            "an unreadable entry is never counted as unread"
        );
        assert!(entries[2].letter().is_none());
    }

    #[test]
    fn reading_the_mailbox_opens_no_letter_for_writing() {
        let tmp = tempfile::tempdir().unwrap();
        let bee_dir = tmp.path().join(".bee");
        let dir = bee_dir.join(MAILBOX_DIR);
        write_letter(&dir, "2026-08-30T04-12-07Z-nightly.md", WELL_FORMED);
        let path = dir.join("2026-08-30T04-12-07Z-nightly.md");
        let before = fs::read(&path).unwrap();
        let before_mtime = fs::metadata(&path).unwrap().modified().unwrap();

        let entries = read_mailbox(&bee_dir);
        assert_eq!(entries.len(), 1);

        assert_eq!(fs::read(&path).unwrap(), before, "the bytes must not move");
        assert_eq!(
            fs::metadata(&path).unwrap().modified().unwrap(),
            before_mtime,
            "an mtime change means the file was opened for writing"
        );
    }

    // --- the whole path: the production caller, not just this module -----

    #[test]
    fn read_snapshot_surfaces_the_mailbox() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let bee_dir = root.join(".bee");
        fs::create_dir_all(bee_dir.join("cells")).unwrap();
        write_letter(
            &bee_dir.join(MAILBOX_DIR),
            "2026-08-30T04-12-07Z-nightly.md",
            WELL_FORMED,
        );

        let snapshot = crate::bee::read_snapshot(root);
        assert!(snapshot.present);
        assert_eq!(snapshot.mailbox.len(), 1);
        assert_eq!(
            snapshot.mailbox[0].letter().map(|l| l.subject.as_str()),
            Some("Run finished: 3 cells capped, 1 needs you")
        );
        assert!(
            snapshot.read_errors.is_empty(),
            "a readable mailbox is not a read error: {:?}",
            snapshot.read_errors
        );
        // A root with no `.bee/` at all reads as an empty mailbox, never a
        // fabricated one.
        let bare = tempfile::tempdir().unwrap();
        assert!(crate::bee::read_snapshot(bare.path()).mailbox.is_empty());
    }
}
