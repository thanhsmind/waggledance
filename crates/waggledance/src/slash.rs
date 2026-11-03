//! Slash suggestions — the list of slash commands and skills available to the
//! agent behind a pane (feature `composer-slash-suggest`, D2 `ae531e75`).
//!
//! The composer's `/` popup asks the daemon what it may offer, and this module
//! is the answer: a filesystem scan of the two places the agent CLIs
//! themselves look, the project root and the user's home. Nothing here
//! interprets or executes a slash command — the entries are text the browser
//! inserts into a reply box, and the agent CLI on the other side of the pane
//! stays the only thing that runs one.
//!
//! Scan order is also shadow order (D2, plus the CONTEXT's discretion note on
//! dedup): project `.claude/commands/*.md`, project `.claude/skills/*/SKILL.md`,
//! project `.agents/skills/*/SKILL.md`, then user `~/.claude/commands/*.md` and
//! `~/.claude/skills/*/SKILL.md`. The first entry seen for a given
//! `(kind, name)` wins, so a project skill shadows the user skill of the same
//! name exactly the way the CLIs resolve it.
//!
//! Every read here is best-effort: a missing `.claude/commands` directory is
//! the normal case, not an error, and an unreadable file drops out of the list
//! rather than failing the request.
//!
//! Feature `slash-builtin-commands` adds a third source after those two: the
//! commands the agent CLI itself answers (`/model`, `/usage`, …), taken from
//! the static per-vendor table in [`crate::slash_builtins`] (D1 `86c7ef5f`).
//! They are appended LAST, so the same first-seen-wins loop makes the shadow
//! order project → user → builtin (D3 `2104c427`), and they appear only when
//! the caller resolved a vendor for the pane's agent (D2 `fe5b9256`) — a
//! shell pane suggesting `/model` would be a lie.

use std::collections::HashSet;
use std::path::Path;

use crate::slash_builtins::CLAUDE_BUILTINS;

/// How long a description may be before it is cut — the popup shows one line.
const DESCRIPTION_MAX: usize = 120;

/// Which of the three things a suggestion is. Serialized as the bare lowercase
/// word (`"command"` / `"skill"` / `"builtin"`) the endpoint's JSON contract
/// promises — the popup badges an entry by this string, so the third value
/// needs no client change (D3 `2104c427`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SlashKind {
    Command,
    Skill,
    /// A command the agent CLI answers itself, from the vendor table — never
    /// a file under `.claude/`.
    Builtin,
}

/// One suggestion: what the user types after `/`, what it is, and the one line
/// shown beside it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SlashEntry {
    /// The bare name — a command file's stem, a skill directory's name. The
    /// popup inserts `/<name> `.
    pub name: String,
    pub kind: SlashKind,
    /// One line from the file's frontmatter `description:`, or its first real
    /// body line; empty when neither exists.
    pub description: String,
}

/// The vendor whose built-in table applies to an agent of this `kind`, or
/// `None` when nothing is known for it (D2 `fe5b9256`).
///
/// Deliberately the same substring classification `views.rs::bee_hub_agent_logo`
/// uses, so `claude-code`, `Claude`, and `claude` all land on one vendor. Only
/// `claude` has a table today (D1 `86c7ef5f`): every other kind answers `None`
/// rather than a guessed list, because no other agent CLI was installed to
/// extract one from and a list written from memory is provably wrong.
pub fn vendor_for_kind(kind: &str) -> Option<&'static str> {
    let k = kind.trim().to_ascii_lowercase();
    if k.contains("claude") {
        Some("claude")
    } else {
        None
    }
}

/// The vendor's own commands as suggestions. An unknown vendor yields nothing.
fn builtin_entries(vendor: &str) -> Vec<SlashEntry> {
    let table: &[(&str, &str, &str)] = match vendor {
        "claude" => CLAUDE_BUILTINS,
        _ => return Vec::new(),
    };
    table
        .iter()
        .map(|(name, _hint, description)| SlashEntry {
            name: (*name).to_string(),
            kind: SlashKind::Builtin,
            description: truncate(description),
        })
        .collect()
}

/// Every slash command, skill, and built-in offerable behind a pane: the
/// project's own (when the page has a project) shadowing the user-level set,
/// both shadowing the pane agent's built-ins, sorted by name.
///
/// `project_root` is `None` for pages with no project — `/_slash` — which then
/// serves the user level alone. `vendor` is `None` when the caller could not
/// resolve an agent for the pane — no pane parameter, an unknown pane, or a
/// plain shell pane — and no built-in is then offered at all (D2 `fe5b9256`).
pub fn slash_entries(
    project_root: Option<&Path>,
    home: &Path,
    vendor: Option<&str>,
) -> Vec<SlashEntry> {
    let mut entries: Vec<SlashEntry> = Vec::new();
    let mut seen: HashSet<(SlashKind, String)> = HashSet::new();

    if let Some(root) = project_root {
        collect_commands(
            &root.join(".claude").join("commands"),
            &mut entries,
            &mut seen,
        );
        collect_skills(
            &root.join(".claude").join("skills"),
            &mut entries,
            &mut seen,
        );
        collect_skills(
            &root.join(".agents").join("skills"),
            &mut entries,
            &mut seen,
        );
    }
    collect_commands(
        &home.join(".claude").join("commands"),
        &mut entries,
        &mut seen,
    );
    collect_skills(
        &home.join(".claude").join("skills"),
        &mut entries,
        &mut seen,
    );

    // Built-ins go last so the file-based sources above already own their
    // names: a project or user command called `model` is the one the user
    // wrote, and it keeps the agent's own `/model` out of the list (D3
    // `2104c427`). Unlike the file sources, which shadow per name+kind, a
    // built-in loses to ANY earlier entry of the same name — the agent
    // resolves one `/model`, whatever kind of thing shadowed it.
    if let Some(vendor) = vendor {
        let taken: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        let fresh: Vec<SlashEntry> = builtin_entries(vendor)
            .into_iter()
            .filter(|e| !taken.contains(e.name.as_str()))
            .collect();
        entries.extend(fresh);
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// `<dir>/*.md` — one command per file, named by its stem.
fn collect_commands(
    dir: &Path,
    entries: &mut Vec<SlashEntry>,
    seen: &mut HashSet<(SlashKind, String)>,
) {
    for path in sorted_children(dir) {
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        push(entries, seen, name, SlashKind::Command, &path);
    }
}

/// `<dir>/*/SKILL.md` — one skill per directory, named by the directory.
fn collect_skills(
    dir: &Path,
    entries: &mut Vec<SlashEntry>,
    seen: &mut HashSet<(SlashKind, String)>,
) {
    for path in sorted_children(dir) {
        let skill = path.join("SKILL.md");
        if !skill.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        push(entries, seen, name, SlashKind::Skill, &skill);
    }
}

/// First writer wins: scan order is shadow order, so a project entry already
/// recorded keeps the user-level one of the same name+kind out.
fn push(
    entries: &mut Vec<SlashEntry>,
    seen: &mut HashSet<(SlashKind, String)>,
    name: &str,
    kind: SlashKind,
    file: &Path,
) {
    if !seen.insert((kind, name.to_string())) {
        return;
    }
    entries.push(SlashEntry {
        name: name.to_string(),
        kind,
        description: description_of(file),
    });
}

/// A directory's entries in name order, or nothing at all when it does not
/// exist — a missing `.claude/commands` is the ordinary case (D2 scan), never
/// an error worth surfacing.
fn sorted_children(dir: &Path) -> Vec<std::path::PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<std::path::PathBuf> =
        read.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    paths
}

/// The one line shown beside a name: the frontmatter `description:` when the
/// file has one (SKILL.md always does; a command `.md` may), else the first
/// non-empty, non-heading body line. Unreadable file → empty.
fn description_of(file: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(file) else {
        return String::new();
    };
    let (front, body) = split_frontmatter(&text);
    let desc = front
        .and_then(frontmatter_description)
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| first_body_line(body));
    truncate(&desc)
}

/// Splits a leading `---` fenced YAML block off the top. Returns
/// `(frontmatter, rest)`; no fence means `(None, whole text)`.
fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let rest = match text.strip_prefix("---\n") {
        Some(rest) => rest,
        None => match text.strip_prefix("---\r\n") {
            Some(rest) => rest,
            None => return (None, text),
        },
    };
    // Find the closing fence line.
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed == "---" || trimmed == "..." {
            return (Some(&rest[..offset]), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (None, text)
}

/// The `description:` value from a frontmatter block, including YAML's folded
/// (`>-`) and literal (`|`) forms the repo's own SKILL.md files use — their
/// continuation lines are indented, and fold to one line here.
fn frontmatter_description(front: &str) -> Option<String> {
    let lines: Vec<&str> = front.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let Some(value) = line.strip_prefix("description:") else {
            continue;
        };
        let value = value.trim();
        if !value.is_empty() && !matches!(value, ">" | ">-" | ">+" | "|" | "|-" | "|+") {
            return Some(unquote(value));
        }
        // Block scalar: fold the indented continuation lines into one.
        let mut folded = String::new();
        for cont in &lines[idx + 1..] {
            if cont.trim().is_empty() {
                continue;
            }
            if !cont.starts_with([' ', '\t']) {
                break;
            }
            if !folded.is_empty() {
                folded.push(' ');
            }
            folded.push_str(cont.trim());
        }
        return Some(folded);
    }
    None
}

/// The first line that carries actual prose: no blanks, no markdown headings,
/// no stray fences.
fn first_body_line(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && *line != "---")
        .unwrap_or_default()
        .to_string()
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return value[1..value.len() - 1].to_string();
    }
    value.to_string()
}

/// Cut to [`DESCRIPTION_MAX`] characters — chars, never bytes, so a multibyte
/// description never splits mid-character.
fn truncate(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= DESCRIPTION_MAX {
        return text.to_string();
    }
    text.chars().take(DESCRIPTION_MAX).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn write(path: PathBuf, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn find<'a>(entries: &'a [SlashEntry], name: &str) -> &'a SlashEntry {
        entries
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no entry named {name} in {entries:?}"))
    }

    #[test]
    fn project_and_user_entries_merge_with_project_shadowing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let home = tmp.path().join("home");
        write(
            root.join(".claude/commands/review.md"),
            "---\ndescription: project review\n---\n\nbody\n",
        );
        write(
            root.join(".claude/skills/bee-hive/SKILL.md"),
            "---\nname: bee-hive\ndescription: project hive\n---\n",
        );
        write(
            home.join(".claude/commands/review.md"),
            "---\ndescription: user review\n---\n",
        );
        write(
            home.join(".claude/skills/bee-hive/SKILL.md"),
            "---\ndescription: user hive\n---\n",
        );
        write(
            home.join(".claude/commands/deploy.md"),
            "---\ndescription: user deploy\n---\n",
        );

        let entries = slash_entries(Some(&root), &home, None);

        assert_eq!(
            entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            vec!["bee-hive", "deploy", "review"],
            "merged, deduped, and sorted by name"
        );
        assert_eq!(find(&entries, "review").description, "project review");
        assert_eq!(find(&entries, "bee-hive").description, "project hive");
        assert_eq!(find(&entries, "deploy").description, "user deploy");
    }

    #[test]
    fn commands_and_skills_carry_their_kinds_including_agents_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let home = tmp.path().join("home");
        write(root.join(".claude/commands/x.md"), "run x\n");
        write(
            root.join(".claude/skills/y/SKILL.md"),
            "---\ndescription: y\n---\n",
        );
        write(
            root.join(".agents/skills/z/SKILL.md"),
            "---\ndescription: z\n---\n",
        );

        let entries = slash_entries(Some(&root), &home, None);

        assert_eq!(find(&entries, "x").kind, SlashKind::Command);
        assert_eq!(find(&entries, "y").kind, SlashKind::Skill);
        assert_eq!(find(&entries, "z").kind, SlashKind::Skill);
        assert_eq!(
            serde_json::to_value(find(&entries, "x")).unwrap(),
            serde_json::json!({"name": "x", "kind": "command", "description": "run x"}),
            "the JSON the endpoint promises"
        );
        assert_eq!(
            serde_json::to_value(find(&entries, "y")).unwrap()["kind"],
            serde_json::json!("skill")
        );
    }

    #[test]
    fn a_project_skill_shadows_a_user_skill_of_the_same_name_but_not_a_command() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let home = tmp.path().join("home");
        write(
            root.join(".claude/skills/plan/SKILL.md"),
            "---\ndescription: project plan\n---\n",
        );
        write(
            home.join(".claude/skills/plan/SKILL.md"),
            "---\ndescription: user plan\n---\n",
        );
        write(
            home.join(".claude/commands/plan.md"),
            "---\ndescription: user plan command\n---\n",
        );

        let entries = slash_entries(Some(&root), &home, None);

        let plans: Vec<_> = entries.iter().filter(|e| e.name == "plan").collect();
        assert_eq!(
            plans.len(),
            2,
            "shadowing is per name+kind, so the command survives"
        );
        assert_eq!(
            plans
                .iter()
                .find(|e| e.kind == SlashKind::Skill)
                .unwrap()
                .description,
            "project plan"
        );
        assert_eq!(
            plans
                .iter()
                .find(|e| e.kind == SlashKind::Command)
                .unwrap()
                .description,
            "user plan command"
        );
    }

    #[test]
    fn description_comes_from_frontmatter_then_from_the_body() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        write(
            home.join(".claude/commands/quoted.md"),
            "---\nname: quoted\ndescription: \"from frontmatter\"\n---\n\nbody line\n",
        );
        write(
            home.join(".claude/commands/bodied.md"),
            "# Heading\n\n\nThe first real line.\nA second line.\n",
        );
        write(
            home.join(".claude/commands/nothing.md"),
            "---\nname: nothing\n---\n",
        );
        write(
            home.join(".claude/skills/folded/SKILL.md"),
            "---\nname: folded\ndescription: >-\n  Folded over\n  two lines.\nmetadata:\n  version: '1'\n---\n\n# Folded\n",
        );
        write(
            home.join(".claude/commands/long.md"),
            &format!("---\ndescription: {}\n---\n", "x".repeat(200)),
        );

        let entries = slash_entries(None, &home, None);

        assert_eq!(find(&entries, "quoted").description, "from frontmatter");
        assert_eq!(find(&entries, "bodied").description, "The first real line.");
        assert_eq!(find(&entries, "nothing").description, "");
        assert_eq!(
            find(&entries, "folded").description,
            "Folded over two lines."
        );
        assert_eq!(
            find(&entries, "long").description.chars().count(),
            DESCRIPTION_MAX
        );
    }

    #[test]
    fn missing_directories_are_normal_not_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let home = tmp.path().join("home");

        assert!(
            slash_entries(Some(&root), &home, None).is_empty(),
            "nothing on disk at all yields an empty list, never a panic"
        );

        write(
            home.join(".claude/skills/only/SKILL.md"),
            "---\ndescription: only\n---\n",
        );
        fs::create_dir_all(root.join(".claude/skills")).unwrap();

        let entries = slash_entries(Some(&root), &home, None);
        assert_eq!(
            entries.len(),
            1,
            "the user skill survives a project with no commands dir"
        );
        assert_eq!(entries[0].name, "only");

        assert_eq!(
            slash_entries(None, &home, None),
            entries,
            "no project means the user-level list alone"
        );
    }

    /// D2 `fe5b9256`: the kind→vendor map is the substring classification
    /// `bee_hub_agent_logo` uses, and only `claude` has a table (D1) — every
    /// other kind, including a shell pane's absent one, resolves to nothing
    /// rather than to a guessed list.
    #[test]
    fn vendor_for_kind_matches_claude_by_substring_and_nothing_else_yet() {
        assert_eq!(vendor_for_kind("claude"), Some("claude"));
        assert_eq!(vendor_for_kind("Claude-Code"), Some("claude"));
        assert_eq!(vendor_for_kind("  claude  "), Some("claude"));
        assert_eq!(vendor_for_kind("codex"), None);
        assert_eq!(vendor_for_kind("gemini"), None);
        assert_eq!(vendor_for_kind(""), None);
    }

    /// A claude pane is offered claude's own commands beside the files, and a
    /// pane with no resolved vendor is offered none of them.
    #[test]
    fn a_claude_vendor_adds_builtins_and_no_vendor_adds_none() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let home = tmp.path().join("home");
        write(
            root.join(".claude/commands/x.md"),
            "---\ndescription: run x\n---\n",
        );

        let with = slash_entries(Some(&root), &home, Some("claude"));
        let model = find(&with, "model");
        assert_eq!(model.kind, SlashKind::Builtin);
        assert_eq!(
            serde_json::to_value(model).unwrap()["kind"],
            serde_json::json!("builtin"),
            "the third value of the endpoint's kind contract"
        );
        assert!(
            !model.description.is_empty(),
            "the table carries the vendor's own blurb: {model:?}"
        );
        assert_eq!(find(&with, "usage").kind, SlashKind::Builtin);
        assert_eq!(
            find(&with, "x").kind,
            SlashKind::Command,
            "the project's own command is still there beside the built-ins"
        );

        let without = slash_entries(Some(&root), &home, None);
        assert_eq!(
            without.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            vec!["x"],
            "no vendor means no built-in at all"
        );
        assert!(
            !without.iter().any(|e| e.kind == SlashKind::Builtin),
            "and no builtin-kinded entry either"
        );

        assert!(
            slash_entries(Some(&root), &home, Some("no-such-vendor"))
                .iter()
                .all(|e| e.kind != SlashKind::Builtin),
            "an unknown vendor has no table, and is not an error"
        );
    }

    /// D3 `2104c427`: built-ins come last, so a file-based command of the same
    /// name wins — and unlike the per-name+kind shadowing between files, the
    /// built-in loses to it outright rather than sitting beside it.
    #[test]
    fn a_project_command_shadows_the_builtin_of_the_same_name() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let home = tmp.path().join("home");
        write(
            root.join(".claude/commands/model.md"),
            "---\ndescription: our own model picker\n---\n",
        );
        write(
            home.join(".claude/skills/usage/SKILL.md"),
            "---\ndescription: user usage skill\n---\n",
        );

        let entries = slash_entries(Some(&root), &home, Some("claude"));

        let models: Vec<_> = entries.iter().filter(|e| e.name == "model").collect();
        assert_eq!(models.len(), 1, "one /model survives, not two: {models:?}");
        assert_eq!(models[0].kind, SlashKind::Command);
        assert_eq!(models[0].description, "our own model picker");

        let usages: Vec<_> = entries.iter().filter(|e| e.name == "usage").collect();
        assert_eq!(
            usages.len(),
            1,
            "a user SKILL shadows the built-in too, across kinds: {usages:?}"
        );
        assert_eq!(usages[0].kind, SlashKind::Skill);

        assert!(
            entries.iter().any(|e| e.name == "compact"),
            "the built-ins nobody shadowed are still offered"
        );
        assert!(
            entries.windows(2).all(|w| w[0].name <= w[1].name),
            "still sorted by name after the built-ins are appended"
        );
    }

    /// D1 `86c7ef5f`: the table is extracted data, not recall. These names
    /// were in a memory-written list and are NOT registered by claude 2.1.251;
    /// their presence would mean somebody hand-edited the table back.
    #[test]
    fn the_builtin_table_carries_no_hand_written_entries() {
        let entries = builtin_entries("claude");
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        for invented in ["cost", "review", "doctor", "todos", "rewind", "vim"] {
            assert!(
                !names.contains(&invented),
                "/{invented} is not registered by the extracted bundle"
            );
        }
        for real in ["model", "usage", "compact", "clear", "help"] {
            assert!(names.contains(&real), "/{real} must come from the table");
        }
        assert!(
            !names.contains(&"agents"),
            "a `(removed)` row must be dropped, not transcribed"
        );
        assert!(
            names.windows(2).all(|w| w[0] <= w[1]),
            "the table itself is stored sorted by name"
        );
    }
}
