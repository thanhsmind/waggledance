---
artifact_contract: bee-plan/v1
mode: high-risk
---

# Plan: Paseo auto wiring

## Summary

Waggledance stops needing a hand-written Paseo block. It finds a `paseo` CLI
that really works (the npm one first), turns Paseo support on by itself, and
registers every Paseo workspace's folder as a waggledance project. Each repo
can carry a small `.waggledance.toml` — `orchestration = true` to accept
dispatched work, and optionally the leader's workspace and model. When a
project has several Paseo workspaces, the leader goes to the one it already
uses, else the oldest. `waggledance setup paseo` works again.

Mode: `high-risk` — 4 risk flags: external systems (Paseo CLI and its store),
public contracts (`.waggledance.toml`), authorization (a new source of the
dispatch opt-in), data model (projects registered without a human). Why this
is the least workflow that protects the work: auto-registration writes the
project registry and auto-picking sends work to a workspace, so both need the
full matrix; the opt-in still defaults off.

## Requirements (from CONTEXT.md)

- D1: every Paseo workspace root becomes a waggledance project automatically.
- D2: `.waggledance.toml` at the repo root holds per-project settings; dispatch needs `orchestration = true` there or the settings-page opt-in.
- D3: several local workspaces at one root → the leader's workspace, else the oldest local one; the file may name one; no refusal.
- D4: Paseo turns on when a working CLI is detected (npm `@getpaseo/cli`, `$PASEO_CLI`, AppImage wrapper; accepted after a JSON `ls`); the global block only overrides.
- D5: the file opt-in has the settings-page opt-in's scope, triggers included; refusals name both and report a broken file; accepted risk documented.
- D6: discovery only registers local, non-worktree git roots outside `$HOME`, `/`, `/tmp`, not archived, not nested either way; indexing off-task; start + every 5 min.
- D7: pick = file, deprecated entry, oldest `createdAt`, lowest id, reason reported; AppImage wrapper demoted below the npm CLI; only successes cached; `setup paseo` reports the choice.

## Load-bearing claims

Labels are `read` (file opened at the anchor) or `ran` (command executed); evidence is a verbatim substring of the anchor.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | The npm CLI's blocking `run` works from the daemon's systemd environment and prints a line before its JSON | ran | `systemd-run --user --wait --pipe ... node/24.19.0/bin/paseo run --json ... --title "wd probe4 (delete me)"` | `Using workspace wks_e75630b751320215` |
| 2 | The npm CLI lists workspaces as clean JSON from systemd | ran | `systemd-run --user --wait --pipe ... node/24.19.0/bin/paseo workspace ls --json` (kept in `.bee/cache/probe2/npm-workspace-ls.out`) | `"workspaceId": "wks_02145c30b3744429",` |
| 3 | The AppImage wrapper's `run` produced no JSON from the daemon (the incident D4 answers) | ran | `waggledance_await run-a07b58ecc3051860` | `printed output that could not be parsed: no JSON document on stdout` |
| 4 | The wrapper's `plugin ls` starts with log lines, which `setup paseo` parses as JSON today | ran | `systemd-run ... ~/.local/bin/paseo plugin ls --json` (kept in `.bee/cache/probe2/wrapper-plugin-ls.out`) | `› [desktop] app startup` |
| 5 | `setup paseo` parses `plugin ls` output directly with serde | read | `crates/waggledance/src/setup.rs:228` | `serde_json::from_str(&out).context(` |
| 6 | `setup paseo` runs a bare `paseo` from PATH | read | `crates/waggledance/src/setup.rs:284` | `let out = std::process::Command::new("paseo")` |
| 7 | Binary resolution today is config, then `$PASEO_CLI`, then `paseo` | read | `crates/waggledance/src/paseo.rs:173` | `pub fn resolve_binary(configured: &str, paseo_cli: Option<OsString>) -> String {` |
| 8 | The daemon gates delivery on the config flag | read | `crates/waggledance/src/main.rs:235` | `self.reconcile_delivery(engine.clone().filter(` |
| 9 | Leader dispatch gates on the config flag | read | `crates/waggledance/src/mcp.rs:1084` | `if !engine.config.paseo.enabled {` |
| 10 | The opt-in is the project's DB flag only today | read | `crates/waggledance-core/src/engine.rs:382` | `pub fn orchestration_allowed(&self, project: &Project) -> bool {` |
| 11 | The two-workspace refusal D3 removes lives in paseo.rs | read | `crates/waggledance/src/paseo.rs:38` | `"project {project} has more than one local Paseo workspace at its root ({ids}); \` |
| 12 | Projects are registered through one engine call | read | `crates/waggledance-core/src/engine.rs:124` | `pub fn register(&self, root: &Path, name: Option<&str>) -> Result<Project> {` |
| 13 | `workspace ls` gives no creation time; Paseo's store does | ran | `python3 -c ... ~/.paseo/projects/workspaces.json` | `'createdAt', 'cwd', 'displayName', 'isPaseoOwnedWorktree', 'kind'` |
| 14 | The config's `enabled` is a plain bool today, so "auto" needs a tri-state | read | `crates/waggledance-core/src/config.rs:25` | `pub enabled: bool,` |
| 15 | Triggers share the opt-in, so the file opt-in reaches them too (D5) | read | `crates/waggledance/src/trigger.rs:388` | `if !self.engine.orchestration_allowed(project)` |
| 16 | The opt-in refusal names only the settings page today | read | `crates/waggledance/src/mcp.rs:1042` | `has not opted into orchestrator dispatch` |
| 17 | Registering a project indexes it on the calling task | read | `crates/waggledance-core/src/engine.rs:80` | `IndexService::index_project(` |
| 18 | A git worktree's `.git` is a file, so discovery can tell it from a repo | ran | `cat .git` in `waggledance--wt--paseo-auto-wiring` | `gitdir: /mnt/data/Projects/goglbe/waggledance/.git/worktrees/waggledance--wt--paseo-auto-wiring` |

## Discovery

Probes on 2026-10-14 (throwaway agent `wd probe4`, deleted, `deletedCount: 1`)
and the live incident run `run-a07b58ecc3051860` (claims 1–4, 13). Every cell's
verify filter was run on the base before the gate.

## Approach

Playbook: `skills/bee-planning/playbooks/feature.md` (cited, not copied).

**Recommended path.**

- **CLI detection (paw-1).** `PaseoConfig.enabled` becomes optional (absent =
  auto; `true`/`false` force). `detect_binary(config)` tries, in order: the
  configured binary; every `paseo` on PATH and in
  `~/.local/share/mise/installs/node/*/bin/` whose real path ends in
  `@getpaseo/cli/bin/paseo`; an existing `$PASEO_CLI`; then `paseo`. A
  candidate is accepted when `ls -g --json` parses; the result is cached for
  10 minutes and logged once. `paseo_available()` replaces every
  `config.paseo.enabled` check (mcp, main). `setup paseo` uses the same
  binary and the same document parser, fixing the log-line failure.
- **In-repo settings (paw-2).** `waggledance-core` reads `.waggledance.toml`
  at a project root: `orchestration = bool` and a `[paseo]` table with
  optional `workspace`, `provider`, `mode`, `thinking`. Unknown keys warn,
  never fail. `orchestration_allowed` = DB flag OR file flag. Leader
  provider/mode/thinking come from the file first, then the global block.
- **Discovery + auto-pick (paw-3).** Inside the delivery task, at start and
  every 5 minutes: `workspace ls`, and register only local, non-worktree git
  roots outside `$HOME`/`/`/`/tmp`, not archived, not nested either way, on a
  blocking thread (D6). Leader start
  workspace: the file's `workspace`, else a deprecated `[[paseo.leaders]]`
  entry, else among local workspaces whose realpath equals the root — oldest `createdAt` from `~/.paseo/projects/workspaces.json`
  (read-only, ignored when unreadable), then lowest id. The refusal is
  removed. An existing leader is reused as today (its workspace is wherever
  it lives).
- **Accepted risk (D5).** A repo file grants dispatch consent, so anyone
  who can change the checkout can opt it in; documented in the concept.
- **Docs (paw-4).** Concept and README: no global block needed, the
  `.waggledance.toml` reference with an example, discovery and the pick rule;
  this repo gets its own `.waggledance.toml` (`orchestration = true`).

**Rejected.** Keeping `[[paseo.leaders]]` as the primary mechanism (owner
rule); auto-enabling orchestration for discovered projects (D2); reading
Paseo's store for anything beyond workspace ordering (the CLI stays the
contract); refusing ambiguous roots (D3).

**SMALLER PATH check.** Dropping discovery leaves new Paseo projects
invisible (D1); dropping the file leaves the global list (D2); dropping
detection leaves the broken wrapper as default (D4). PASS.

**Risk map.**

| Component | Risk | Lands in | Proof |
|---|---|---|---|
| Wrong CLI picked | HIGH | paw-1 | order tests with fake candidates; npm fixture |
| Dispatch to an unconsented project | HIGH | paw-2 | discovered project refuses until the file or DB opts in |
| Registering junk roots | MEDIUM | paw-3 | archived/missing roots skipped; nested roots not duplicated |
| Wrong workspace pick | MEDIUM | paw-3 | oldest/lowest-id tests with a store fixture |
| Config compat | MEDIUM | paw-1 | old `enabled = true` files still parse |

Waves: (paw-1 ∥ paw-2) → paw-3 → paw-4.

## Shape

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| CLI detection | D4 | no manual binary or switch | 1 | detection + setup tests |
| In-repo settings | D2 | per-project config in the repo | 1 | core tests |
| Discovery + pick | D1, D3 | no per-workspace entries | 1 | delivery/paseo tests |
| Docs | D1–D4 | the owner edits one file | 1 | doctor test |

Slice queue: one slice, four cells.

## Cells — current slice (preview)

```json
[
  {
    "id": "paw-1",
    "title": "Find a working Paseo CLI and turn Paseo support on by itself",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-auto-wiring",
    "deps": [],
    "decisions": [
      "D4",
      "D7"
    ],
    "files": [
      "crates/waggledance/src/paseo.rs",
      "crates/waggledance-core/src/config.rs",
      "crates/waggledance/src/mcp.rs",
      "crates/waggledance/src/main.rs",
      "crates/waggledance/src/setup.rs",
      "crates/waggledance/testdata/fixtures/paseo-0.11.2/*",
      "crates/waggledance/src/delivery.rs"
    ],
    "read_first": [
      "docs/history/paseo-auto-wiring/CONTEXT.md",
      "docs/history/paseo-auto-wiring/plan.md",
      "crates/waggledance/src/paseo.rs",
      "crates/waggledance/src/setup.rs",
      "crates/waggledance-core/src/config.rs"
    ],
    "action": "PaseoConfig.enabled becomes Option<bool> (absent = auto, true/false force) keeping old files parseable. Add detect_binary: candidates in order — config.paseo.binary when set (but when it is a script that execs an .AppImage and an npm CLI is found, the npm CLI wins and the demotion is logged); each `paseo` on PATH whose canonical path ends with `@getpaseo/cli/bin/paseo`; as a recorded host fallback for systemd's thin PATH, ~/.local/share/mise/installs/node/*/bin/paseo with the same suffix (dedupe by canonical path, prefer the highest version); $PASEO_CLI when the file exists; then plain `paseo`. A candidate is accepted when `ls -g --json` (through the existing document parser, child env scrubbed) parses as a list; cache only a successful winner for 10 minutes (a failure is retried on the next call); log the chosen path and the rejected candidates with their reasons. Add paseo_available(config) = enabled forced, else a detected binary exists. Replace every `config.paseo.enabled` check (mcp.rs leader gate, main.rs delivery gate) with it; when Paseo is unavailable the refusal lists the candidates tried and why each was rejected. PaseoCli uses the detected binary. setup.rs `waggledance setup paseo` runs the detected binary, parses `plugin ls`/other JSON through paseo.rs's document parser, prints the chosen CLI and rejected candidates, and warns about global [paseo] enabled/binary/[[paseo.leaders]] lines that are now unnecessary, naming the .waggledance.toml equivalent. Fix every test that sets config.paseo.enabled (delivery.rs, main.rs, mcp.rs, config.rs) for the Option type. Copy .bee/cache/probe2/*.out from this worktree into crates/waggledance/testdata/fixtures/paseo-0.11.2/ (npm-workspace-ls.out, npm-plugin-ls.out, wrapper-plugin-ls.out) and add a fixture text npm-run-blocking.out with exactly: a line `Using workspace wks_e75630b751320215` followed by the JSON document {\"agentId\": \"e7baf63c-e456-4359-b56e-6f4a19faab8c\", \"status\": \"permission\", \"provider\": \"claude\", \"cwd\": \"/home/thanhsmind/Projects/goglbe/waggledance\", \"title\": \"wd probe4 (delete me)\"} pretty-printed with two-space indent; extend the fixtures README (command, environment, version: npm CLI 0.10.3 against daemon 0.11.2, date 2026-10-14). Tests: candidate order and acceptance with fake scripts, cache, paseo_available forced/auto, setup parsing the wrapper fixture, run parsing the npm fixture. No code comments.",
    "verify": "cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance setup && cargo test --profile fast -p waggledance-core config",
    "must_haves": {
      "truths": [
        "the npm CLI is chosen over the AppImage wrapper when both work",
        "a candidate whose ls is not JSON is skipped",
        "Paseo support is on with no [paseo] block when a CLI is detected",
        "old config files with enabled = true still parse",
        "setup paseo parses plugin ls output that starts with log lines",
        "a configured AppImage wrapper loses to a detected npm CLI",
        "with no working CLI the refusal lists the candidates tried"
      ],
      "artifacts": [
        {
          "path": "crates/waggledance/src/paseo.rs",
          "substantive": "detect_binary, paseo_available, tests"
        }
      ],
      "key_links": [
        "mcp.rs and main.rs gate on paseo_available",
        "setup.rs uses the detected binary and document parser"
      ],
      "prohibitions": [
        "no shell",
        "no code comments"
      ]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "paw-2",
    "title": "Read per-project settings from .waggledance.toml in the repo",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-auto-wiring",
    "deps": [],
    "decisions": [
      "D2",
      "D5"
    ],
    "files": [
      "crates/waggledance-core/src/project_settings.rs",
      "crates/waggledance-core/src/lib.rs",
      "crates/waggledance-core/src/engine.rs",
      "crates/waggledance/src/mcp.rs"
    ],
    "read_first": [
      "docs/history/paseo-auto-wiring/plan.md",
      "crates/waggledance-core/src/engine.rs",
      "crates/waggledance-core/src/config.rs"
    ],
    "action": "Add waggledance_core::project_settings: ProjectSettings { orchestration: Option<bool>, paseo: ProjectPaseo { workspace, provider, mode, thinking: Option<String> } } loaded from <project root>/.waggledance.toml with serde defaults; a missing file is all-None; an unreadable or invalid file is all-None plus a warning string the caller can surface; unknown keys are ignored with a warning, never an error. Engine: project_settings(&Project) -> (ProjectSettings, Vec<String> warnings), cached by file mtime (triggers call orchestration_allowed every tick). orchestration_allowed(project) = project.orchestration_enabled || settings.orchestration == Some(true); this is the same opt-in triggers use (D5). In mcp.rs, not_opted_in names both remedies: the project's settings page, or `orchestration = true` in <absolute root>/.waggledance.toml, and appends any parse warning for that file. Tests: missing file, orchestration true/false, paseo table, invalid TOML -> off with warning, unknown key -> warning, DB flag still enables. No code comments.",
    "verify": "cargo test --profile fast -p waggledance-core project_settings && cargo test --profile fast -p waggledance-core orchestration",
    "must_haves": {
      "truths": [
        "a project with orchestration = true in .waggledance.toml is allowed without the DB flag",
        "a missing or invalid file never enables dispatch",
        "the DB flag alone still enables dispatch",
        "the opt-in refusal names the settings page and the absolute .waggledance.toml path",
        "a file that fails to parse is reported in the refusal"
      ],
      "artifacts": [
        {
          "path": "crates/waggledance-core/src/project_settings.rs",
          "substantive": "ProjectSettings loader with tests"
        }
      ],
      "key_links": [
        "Engine::orchestration_allowed reads the file"
      ],
      "prohibitions": [
        "no default-on orchestration",
        "no code comments"
      ]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "paw-3",
    "title": "Register Paseo workspaces as projects and pick a leader workspace automatically",
    "role": "code",
    "lane": "high-risk",
    "feature": "paseo-auto-wiring",
    "deps": [
      "paw-1",
      "paw-2"
    ],
    "decisions": [
      "D1",
      "D3",
      "D6",
      "D7"
    ],
    "files": [
      "crates/waggledance/src/paseo.rs",
      "crates/waggledance/src/delivery.rs",
      "crates/waggledance/src/discovery.rs",
      "crates/waggledance/src/main.rs",
      "crates/waggledance/testdata/fixtures/paseo-0.11.2/*"
    ],
    "read_first": [
      "docs/history/paseo-auto-wiring/plan.md",
      "crates/waggledance/src/paseo.rs",
      "crates/waggledance/src/delivery.rs",
      "crates/waggledance/src/main.rs",
      "crates/waggledance-core/src/engine.rs"
    ],
    "action": "Add crates/waggledance/src/discovery.rs with discover(paseo, engine) and call it from the delivery task at start and every 5 minutes (declare the module in main.rs; no new task). It runs workspace ls and registers a root only when: isolation is local; the realpath exists, is a git repository whose .git is a directory (a .git file means a worktree: skip); it is not $HOME, / or under /tmp; Paseo's store (~/.paseo/projects/workspaces.json, PASEO_HOME respected, read-only) does not mark it archived or isPaseoOwnedWorktree; and no registered project's realpath contains it or is contained by it. Registration indexes the tree, so run engine.register on a blocking thread (spawn_blocking), one root at a time, logging each. Never unregister. In paseo.rs resolve_leader's Start path: workspace = .waggledance.toml [paseo] workspace (engine.project_settings), else a [[paseo.leaders]] entry (deprecated), else the local workspaces whose realpath equals the root ordered by oldest createdAt from the store (unreadable -> skip), then lowest workspaceId; none -> create one at the root. Remove the two-workspace refusal. Return the chosen workspace and a reason (file|deprecated-entry|oldest|lowest-id|created) and include both in the delivery log and the run's transcript header. Leader provider/mode/thinking: project file first, then global config. Add testdata fixture workspaces-store.json (synthetic ids: three local workspaces at one root with distinct createdAt, one archived, one isPaseoOwnedWorktree) and tests: registers a new local repo root once; skips worktree (.git file), archived, Paseo-owned worktree, $HOME, /tmp, nested either way; pick order oldest > lowest id; file override; no refusal. No code comments.",
    "verify": "cargo test --profile fast -p waggledance discovery && cargo test --profile fast -p waggledance paseo && cargo test --profile fast -p waggledance delivery",
    "must_haves": {
      "truths": [
        "a local git root unknown to waggledance becomes a project at the next discovery",
        "git worktrees, archived and Paseo-owned worktree workspaces, $HOME and /tmp are never registered",
        "a root nested in, or containing, a registered project is not registered",
        "two local workspaces at one root no longer refuse; the oldest is chosen and the reason is reported",
        "a .waggledance.toml workspace overrides the pick"
      ],
      "artifacts": [
        {
          "path": "crates/waggledance/src/discovery.rs",
          "substantive": "discovery task with tests"
        }
      ],
      "key_links": [
        "the delivery task calls discovery at start and every 5 minutes",
        "resolve_leader reads project settings"
      ],
      "prohibitions": [
        "no automatic orchestration opt-in",
        "no unregistering",
        "no code comments"
      ]
    },
    "affects_skills": [],
    "affects_specs": []
  },
  {
    "id": "paw-4",
    "title": "Document zero-config Paseo wiring and opt this repo in from its own file",
    "role": "docs",
    "lane": "high-risk",
    "feature": "paseo-auto-wiring",
    "deps": [
      "paw-3"
    ],
    "decisions": [
      "D1",
      "D2",
      "D3",
      "D4"
    ],
    "files": [
      "docs/knowledge/patterns/paseo-leader-dispatch.md",
      "README.md",
      ".waggledance.toml",
      "docs/waggledance-supervisor-skill-template.md"
    ],
    "read_first": [
      "docs/knowledge/patterns/paseo-leader-dispatch.md",
      "docs/history/paseo-auto-wiring/CONTEXT.md",
      "crates/waggledance-core/src/project_settings.rs",
      "crates/waggledance/src/discovery.rs"
    ],
    "action": "Update the concept: no global [paseo] block needed (auto-detected CLI, order and why; the block only overrides), discovery registers every Paseo workspace root, .waggledance.toml reference (orchestration, [paseo] workspace/provider/mode/thinking) with a copy-paste example, the workspace pick order, and that [[paseo.leaders]] is a deprecated fallback; cite paseo-auto-wiring D1/D2/D3/D4. README: one short section on .waggledance.toml. Add this repo's own .waggledance.toml with `orchestration = true`. In the supervisor template, replace any instruction to edit ~/.waggledance/config.toml with the .waggledance.toml opt-in. Keep doctor assertions green.",
    "verify": "cargo test --profile fast -p waggledance doctor",
    "must_haves": {
      "truths": [
        "the concept shows a .waggledance.toml example that parses with project_settings",
        "no doc tells the owner to add per-workspace entries to ~/.waggledance/config.toml"
      ],
      "artifacts": [
        {
          "path": ".waggledance.toml",
          "substantive": "this repo's opt-in"
        }
      ],
      "key_links": [
        "the concept cites paseo-auto-wiring D1/D2/D3/D4"
      ],
      "prohibitions": [
        "no change to the spec-drop line format"
      ]
    },
    "affects_skills": [],
    "affects_specs": [
      "docs/knowledge/patterns/paseo-leader-dispatch.md"
    ]
  }
]
```

## Test matrix

| Dimension | Probe | Pass when |
|---|---|---|
| Detection | npm CLI and wrapper both answer | npm chosen |
| Detection | a candidate prints non-JSON | skipped |
| Detection | no candidate works | Paseo unavailable; dispatch refusal names the cause |
| Compat | old `enabled = true` / `binary = …` | parsed; forced values honored |
| Setup | `plugin ls` starting with log lines | parsed |
| Consent | discovered project, no file, no DB flag | dispatch refused |
| Consent | `orchestration = true` in the file | dispatch accepted |
| Consent | invalid file | refused, warning surfaced |
| Discovery | new root | registered once |
| Discovery | root inside a project / missing root | skipped |
| Pick | older vs lower id | oldest, then lowest id; reason reported |
| Discovery | worktree / archived / Paseo-owned / $HOME / /tmp | skipped |
| Consent | refusal text | names settings page and the file path; parse error shown |
| Detection | configured AppImage wrapper + npm CLI | npm CLI chosen, demotion logged |
| Pick | file names a workspace | that workspace |
| Pick | two at root, no file | no refusal |
| Regression | dispatch, await, delivery, reaper suites | green |

## Open Questions

(none)

## Out of scope

Registering bee worktrees as Paseo workspaces (bee's work, handed to beehive as `sup-20261014-6234`); unregistering projects whose Paseo workspace was archived; changing how requests are queued or delivered.
