---
artifact_contract: bee-plan/v2
mode: standard
---

# Plan: bee-bulk-upgrade

## Summary

You can upgrade bee in every registered project with one command, one board
button, or one MCP call. By default, waggledance only shows what would change
in each project: current version, target version, and the files involved.
With apply, it runs `bee onboard --apply` in each project and commits only
bee's files. A project is skipped, with the reason, when it has no bee, is
mid-merge or mid-rebase, is a linked worktree, or when bee would write a file
the user is already editing.

Mode: `standard` — 3 risk flags: external systems (it writes and commits in
other repos), public contracts (a new CLI command and a new MCP tool),
multi-domain (core, CLI, MCP and board).
Why this is the least workflow that protects the work: one core function
carries every safety rule, and each surface is a thin caller of it, so the
rules are proven once and cannot drift between surfaces.

## Requirements (from CONTEXT.md)

- D1 (557900d6): one shared core with three surfaces — the CLI `waggledance bee upgrade`, a board action, and the MCP tool `waggledance_bee_upgrade`.
- D2 (6ba61ced): only projects that already hold `.bee/onboarding.json` are upgraded; the others are listed as skipped and never installed.
- D3 (3439d31f): only the files onboard changed are staged and committed; the user's other work is never staged; a project mid-merge or mid-rebase is skipped.
- D4 (02f08071): when a planned path already carries uncommitted edits, the project is skipped and the report names those files.
- D5 (f9ce5ef6): the bee source is found among the registered projects (`packages/bee/AGENTS.block.md` plus `.bee/bin/bee`); with no match, the run refuses and names the file it looked for.
- D6 (b95c1c4e): a linked git worktree is skipped.
- D7 (0b11d69f): the overlap check and the commit use expanded repo paths. A skill item maps through its target root; a directory covers its files by prefix. The commit holds only newly dirty files under those paths.
- D8 (19377eb1): a project with a live bee session is skipped.
- D9 (bb1a29e2): the MCP tool previews by default. Apply needs a non-empty projects list.
- D10 (f3604e70): a missing root, the bee source itself, and a detached HEAD are skipped. Rows report branch and notices. A failed commit after apply is `applied_not_committed`.

## Load-bearing claims

Labels: `read` means the file was opened at that line. `ran` means the command
was executed. The evidence column holds bytes copied from the anchor.

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | `bee onboard` works only when run from a bee source checkout | ran | `cd /mnt/data/Projects/goglbe/waggledance && .bee/bin/bee onboard --repo-root /home/thanhsmind/Projects/goglbe/jarvis --json` | `"status": "blocked_no_engine",` |
| 2 | Run from the beehive checkout, a dry run returns the target version and a plan of paths | ran | `cd /home/thanhsmind/Projects/goglbe/beehive && .bee/bin/bee onboard --repo-root /home/thanhsmind/Projects/goglbe/jarvis --json` | `"status": "changes_needed",` |
| 3 | The beehive checkout holds the marker file D5 looks for | ran | `ls /home/thanhsmind/Projects/goglbe/beehive/packages/bee/AGENTS.block.md` | `/home/thanhsmind/Projects/goglbe/beehive/packages/bee/AGENTS.block.md` |
| 4 | CLI commands call the core engine directly, not the daemon | read | `crates/waggledance/src/cli.rs:397` | `    let engine = runtime::build_engine()?;` |
| 5 | The CLI has one match on the command enum to extend | read | `crates/waggledance/src/cli.rs:145` | `        Command::List { json } => cmd_list(json),` |
| 6 | MCP tools are dispatched by name in one match | read | `crates/waggledance/src/mcp.rs:344` | `        "waggledance_projects" => handle_projects(id, engine),` |
| 7 | A test pins the MCP tool count, so the new tool must update it | read | `crates/waggledance/src/mcp.rs:1943` | `    fn tools_list_has_eight_schemas() {` |
| 8 | Board POST actions are axum routes in one router | read | `crates/waggledance/src/server.rs:485` | `        .route("/api/projects/:id/refresh", post(refresh_project))` |
| 9 | The home page has a tab enum to add a Bee tab to | read | `crates/waggledance/src/views.rs:174` | `pub enum HomeTab {` |
| 10 | Every registered project carries its root path | read | `crates/waggledance-core/src/domain.rs:11` | `    pub root_path: PathBuf,` |
| 11 | A skill item's path is a bare skill name; its folder comes from the matching target | ran | `cd /home/thanhsmind/Projects/goglbe/beehive && .bee/bin/bee onboard --repo-root /home/thanhsmind/Projects/goglbe/jarvis --json` | `"kind": "repo-claude", "target_root": "/home/thanhsmind/Projects/goglbe/jarvis/.claude/skills"` |
| 12 | waggledance already reads bee session liveness | read | `crates/waggledance-core/src/bee.rs:723` | `    pub live: bool,` |

## Discovery

On 2026-10-30, `waggledance list` showed 30 projects. Reading each
`.bee/onboarding.json` showed 21 with bee, 7 of them behind 2.67.0, and most
with uncommitted work. A dry-run onboard against jarvis from the beehive
checkout returned a plan of `{action, path}` entries. The same call from
waggledance's checkout refused. Code survey: board, CLI and MCP all reach the
engine directly. The only external-process runner with a deadline is
`git_diff::run_git`, which is private. Its poll-and-kill pattern is copied,
not shared.

## Approach

Recommended path:

1. Shared runner: lift the generic poll-deadline-kill body of
   `git_diff::run_git` into one core fn in `process.rs`. It takes a
   `Command`, a timeout and an output cap. `run_git` calls it, and so does
   the upgrade. There is one copy of the pattern.
2. Core: `crates/waggledance-core/src/bee_upgrade.rs` takes the registered
   projects and options `{apply, only: project ids, timeout}`. It finds the
   source (D5) and records the source's HEAD sha, branch and dirty flag in the
   report header. Projects run in a plain serial loop: a dry run measured
   23–80 ms per project. Each project is classified in this order:
   - missing root, no bee (D2), bee source, linked worktree (D6), detached
     HEAD, live bee session (D8): skipped (D10);
   - otherwise, a dry-run onboard. Its plan items are expanded to repo paths
     (D7): a skill item maps through `skills.targets[].target_root`, a
     directory covers the files under it, and an item outside the repo is
     only noted.
   - The dry-run result is `up_to_date`, `pending`, or `failed`. It carries
     `branch`, `notices`, and the expanded files.

   Apply, for a `pending` project:
   - mid-merge or mid-rebase: skipped (D3);
   - an expanded planned path overlaps an uncommitted file, compared by
     prefix: skipped, naming the files (D4, D7);
   - otherwise, run `onboard --apply`. The commit set is the files newly
     dirty after apply that fall under an expanded planned path (D7). They are
     committed with `git add` and `git commit -m "Upgrade bee to <v>"` using
     `--literal-pathspecs` and `--pathspec-from-file=- --pathspec-file-nul`.
     The user's other staged work stays out (D3).
   - A failed commit after a successful apply is `applied_not_committed`
     (D10).
3. CLI: `waggledance bee upgrade [--apply] [--project <id>]... [--json]`
   prints a table or JSON.
4. MCP: `waggledance_bee_upgrade {apply?: bool, projects?: [id]}` returns the
   same report. `apply=true` needs a non-empty `projects` list (D9).
5. Board: a Bee tab on the home page shows the dry-run table. It follows the
   precedent of the Usage tab; D1 locks only "a board action". Each pending
   row has its own POST form (one id), and there is one "Upgrade all pending"
   form. Both post to `/api/bee/upgrade` inside the existing router, so the
   cross-site POST guard covers them. The response re-renders the tab with
   the applied result inline.
6. Docs: one knowledge concept.

Rejected alternatives:

- Each project's own vendored bee: it refuses outside a source checkout
  (claim 1).
- A configured source path: the user rejected hand-edited per-machine lists.
- A plain `git commit`: it would sweep in the user's staged work.
- An exact match on `plan[].path`: it misses every skill folder (claim 11).
- A thread pool: a serial run over 22 projects takes about 1–2 s.
- A checkbox table with repeated-key form parsing: per-row forms need no
  new parser.

Risk map:

| Component | Risk | Lands in | Proof needed |
|---|---|---|---|
| Overwriting a user's skill edit | HIGH | bbu-1 | A temp-repo test where a fake bee emits a `sync_skill` item while the user has a dirty file under that skill: the project is skipped and nothing is written |
| Commit isolation | HIGH | bbu-1 | A temp-repo test with pre-staged user work and a stray new file outside the plan: the commit holds only planned files |
| Shared runner regresses git diff | MEDIUM | bbu-1 | The existing `git_diff` tests stay green |
| Hung onboard | MEDIUM | bbu-1 | A fake bee that sleeps: the project fails with a timeout and the run continues |
| Agent mass-apply | MEDIUM | bbu-3 | A test that `apply=true` without projects is refused |
| Board apply is a mutating POST | MEDIUM | bbu-4 | Route tests: a GET never applies, and a POST with no id and no all-flag returns 400 |

Waves: bbu-1 runs first. bbu-2, bbu-3 and bbu-4 then run in parallel: their
files are disjoint and all three need the core API. bbu-5 runs last.

## Role assignments

```json
{
  "schema_version": "1.0",
  "runtime": "claude",
  "roster_sha256": "094d64e24e68c0b6fabaed19ae6619f3ac5e32bac9e9da2ab51728125c724d9b",
  "stages": [
    {"stage": "plan", "classification": "required", "role": "plan", "reason": "this plan, drafted by the leader"},
    {"stage": "hat-wave-facts-gaps", "classification": "required", "role": "hat-facts-gaps", "reason": "six product files open the wave"},
    {"stage": "hat-wave-alternatives", "classification": "required", "role": "hat-alternatives", "reason": "six product files open the wave"},
    {"stage": "hat-wave-user-impact", "classification": "required", "role": "hat-user-impact", "reason": "writes and commits in the user's other repos"},
    {"stage": "hat-wave-risks", "classification": "not-applicable", "role": "hat-risks", "reason": "not high-risk"},
    {"stage": "hat-wave-value", "classification": "not-applicable", "role": "hat-value", "reason": "not high-risk"},
    {"stage": "advisor", "classification": "not-applicable", "role": "advisor", "reason": "standard lane"},
    {"stage": "test", "classification": "required", "role": "test", "reason": "Rust cells are red-first test-plus-code work, per the usage-dashboard precedent"},
    {"stage": "docs", "classification": "required", "role": "docs", "reason": "bbu-5 knowledge concept"},
    {"stage": "code", "classification": "not-applicable", "role": "code", "reason": "Rust cells run on the test seat"},
    {"stage": "read", "classification": "conditional", "role": "read", "condition": "a worker needs a multi-file lookup", "reason": "gathers only"},
    {"stage": "extraction", "classification": "conditional", "role": "extraction", "condition": "a single-fact lookup is delegated", "reason": "cheap reader"},
    {"stage": "generation", "classification": "not-applicable", "role": "generation", "reason": "every cell names its role"},
    {"stage": "design", "classification": "not-applicable", "role": "design", "reason": "the Bee tab reuses existing table and form styles"},
    {"stage": "review", "classification": "conditional", "role": "review", "condition": "the user invokes bee-reviewing", "reason": "agents-review-user-invoked"},
    {"stage": "deploy", "classification": "not-applicable", "role": "deploy", "reason": "no release"},
    {"stage": "supervisor", "classification": "not-applicable", "role": "supervisor", "reason": "herding cockpit role, not used"},
    {"stage": "lane-1", "classification": "not-applicable", "role": "lane-1", "reason": "one design"},
    {"stage": "lane-2", "classification": "not-applicable", "role": "lane-2", "reason": "no blind lanes"},
    {"stage": "lane-3", "classification": "not-applicable", "role": "lane-3", "reason": "no blind lanes"}
  ]
}
```

## Shape

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 | The core upgrade function, with every safety rule | All three surfaces call it | Temp-repo tests: commit isolation, overlap skip, timeout | Phase 2 |
| 2 | CLI, MCP tool, and board Bee tab | The user picked all three (D1) | `waggledance bee upgrade` dry run over the real registry lists every project | User acceptance |
| 3 | Knowledge concept | Agents need to find the feature | The concept names each surface and its rules | Close |

## Cells — current slice (preview)

| id | title | files | deps | you see | proof |
|---|---|---|---|---|---|
| bbu-1 | Upgrade bee across registered projects in one core call | `crates/waggledance-core/src/lib.rs`, `crates/waggledance-core/src/process.rs`, `crates/waggledance-core/src/git_diff.rs`, `crates/waggledance-core/src/bee_upgrade.rs` (new) | — | nothing yet; a tested core call plans and applies upgrades | `cargo test --profile fast -p waggledance-core` green (bee_upgrade and git_diff tests included) |
| bbu-2 | Add the `waggledance bee upgrade` command | `crates/waggledance/src/cli.rs` | bbu-1 | `waggledance bee upgrade` prints each project's branch, version and what would change; `--apply` upgrades and commits | `cargo test --profile fast -p waggledance cli` green; a live dry run over the real registry lists every project |
| bbu-3 | Expose the bulk upgrade as an MCP tool | `crates/waggledance/src/mcp.rs` | bbu-1 | agents see `waggledance_bee_upgrade`; applying needs named projects | `cargo test --profile fast -p waggledance mcp` green |
| bbu-4 | Show bee versions and upgrade buttons on a Bee tab | `crates/waggledance/src/server.rs`, `crates/waggledance/src/views.rs`, `crates/waggledance/assets/app.css` | bbu-1 | a Bee tab lists each project's branch, version and status, with an Upgrade button per pending row and one for all pending | `cargo test --profile fast -p waggledance` green |
| bbu-5 | Describe the bulk bee upgrade and its safety rules | `docs/knowledge/work/bee-bulk-upgrade/index.md` (new) | bbu-2, bbu-3, bbu-4 | the docs name the three surfaces, every skip rule and the source discovery | every backticked code name in the concept is found by rg in crates |

```json
[
 {"id":"bbu-1","feature":"bee-bulk-upgrade","title":"Upgrade bee across registered projects in one core call","lane":"standard","role":"test","status":"open","deps":[],
  "decisions":["D2","D3","D4","D5","D6","D7","D8","D10","6ba61ced","3439d31f","02f08071","f9ce5ef6","b95c1c4e","0b11d69f","19377eb1","f3604e70"],
  "files":["crates/waggledance-core/src/lib.rs","crates/waggledance-core/src/process.rs","crates/waggledance-core/src/git_diff.rs","crates/waggledance-core/src/bee_upgrade.rs"],
  "read_first":["docs/history/bee-bulk-upgrade/CONTEXT.md","docs/history/bee-bulk-upgrade/plan.md","crates/waggledance-core/src/git_diff.rs","crates/waggledance-core/src/process.rs","crates/waggledance-core/src/domain.rs"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach steps 1-2. (a) Shared runner: in process.rs add pub struct RunOutput {stdout: Vec<u8>, stderr: String, ok: bool, truncated: bool} and pub fn run_with_deadline(cmd: &mut std::process::Command, stdin: Option<Vec<u8>>, timeout: Duration, cap: usize) -> Result<RunOutput, RunError> where RunError is {Spawn(String), Timeout(u64)}; it pipes stdout/stderr through capped reader threads, writes stdin when given (else null stdin), polls try_wait against the deadline, kills and reaps on expiry. Move the body out of git_diff.rs `fn run_git` (search `fn run_git(` and `fn read_capped`) so run_git builds its Command with the same -C, GIT_PAGER, GIT_OPTIONAL_LOCKS, LC_ALL settings and maps RunError to GitDiffError::GitUnavailable / GitDiffError::Timeout; git_diff's behaviour and tests stay unchanged. (b) Create bee_upgrade.rs, register `pub mod bee_upgrade;` in lib.rs. Types (Serialize, Debug, Clone): UpgradeOptions {apply, only: Vec<String>, timeout: Duration} with Default (false, empty, 120 s); BeeSource {root, bin}; pub fn find_source(projects: &[Project]) -> Option<BeeSource> (D5: first root with packages/bee/AGENTS.block.md and executable .bee/bin/bee); UpgradeReport {source_root, source_head: Option<String> short sha, source_branch: Option<String>, source_dirty: bool, target_version: Option<String>, applied: bool, projects: Vec<ProjectUpgrade>}; ProjectUpgrade {id, name, root, branch: Option<String>, from_version: Option<String> (from .bee/onboarding.json bee_version), to_version: Option<String>, notices: Vec<String>, outside_repo: Vec<String>, outcome: Outcome}; Outcome serde tag `status` snake_case: UpToDate, Pending {files}, Upgraded {commit, files}, AppliedNotCommitted {files, error}, Skipped {reason, files}, Failed {error}. BeeUpgradeError {NoSource {looked_for}, UnknownProjects(Vec<String>)} with Display. pub fn run(projects, opts) -> Result<UpgradeReport, BeeUpgradeError>: unknown ids in only are an error, empty only = all; serial loop in input order. Per project, in order: root missing -> Skipped `root missing`; no .bee/onboarding.json -> Skipped `no bee` (never call onboard); root equals source root -> Skipped `bee source`; linked worktree (canonical git-dir differs from git-common-dir) -> Skipped `linked worktree; upgrades through its main checkout`; `git symbolic-ref --short -q HEAD` fails -> Skipped `detached HEAD`; crate::bee::read_snapshot(root).sessions has a session with live true whose status is not closed -> Skipped `live bee session` (check what BeeSession exposes; if it lacks status, treat live as the rule). Then dry run `<bin> onboard --repo-root <root> --json` with current_dir = source root (parse from the first `{` of stdout). status up_to_date -> UpToDate; changes_needed -> expand paths; any other status or non-zero exit -> Failed with the status and its error/reason text. Path expansion (D7): for each plan item, if action is sync_skill or remove_skill, find skills.targets[] whose kind equals the item's target and use target_root joined with item path, made relative to the repo root; otherwise the item path; a path starting with `~` or absolute outside the root, or containing a space and parenthesis like `.bee/ (worktree-local coordination stores)`, goes to outside_repo; dedup and sort; treat each as a prefix (a dirty path p overlaps entry e when p == e or p starts with e + '/'). Pending carries the expanded list; notices carry the dry run's notices strings. Apply (opts.apply and Pending): MERGE_HEAD, rebase-merge or rebase-apply under `git rev-parse --git-path` -> Skipped `merge or rebase in progress`; before = paths from `git status --porcelain=v1 -z -uall` (take both sides of a rename); overlap of before with the expanded prefixes non-empty -> Skipped `uncommitted edits in files bee would write` with those files; else run onboard with --apply (non-zero or non-ok status -> Failed), after = same status set, commit set = (after minus before) filtered to the expanded prefixes; empty -> UpToDate; else `git add --literal-pathspecs --pathspec-from-file=- --pathspec-file-nul` and `git commit --literal-pathspecs -m \"Upgrade bee to <to_version>\" --pathspec-from-file=- --pathspec-file-nul` feeding the NUL-joined set on stdin, then `git rev-parse --short HEAD` -> Upgraded; a failing add or commit -> AppliedNotCommitted with the set and stderr. Every external call uses process::run_with_deadline with opts.timeout. Tests in mod tests with temp dirs and real git repos (set user.name and user.email per repo; disable commit.gpgsign per repo): a fake source root with packages/bee/AGENTS.block.md and an executable shell script .bee/bin/bee printing JSON that has a plan with an update_agents_block AGENTS.md item, a copy_expertise .bee/expertise/x.md item and a sync_skill item {path: bee-hive, target: repo-claude}, plus skills.targets [{kind: repo-claude, target_root: <repo>/.claude/skills}], and notices [\"n1\"]; with --apply it writes AGENTS.md, .bee/expertise/x.md and .claude/skills/bee-hive/SKILL.md. Cases: find_source picks the marked root and None without; run -> NoSource without a source; root missing -> Skipped; no onboarding -> Skipped and the fake never runs (fake appends to a call log); dry run -> Pending listing AGENTS.md, .bee/expertise/x.md, .claude/skills/bee-hive with notices n1 and nothing written; apply on a clean repo -> Upgraded and `git show --name-only HEAD` lists exactly the three files; apply with an unrelated file already staged -> commit excludes it and it stays staged; apply when the fake also writes stray.txt outside the plan -> stray.txt not committed; dirty .claude/skills/bee-hive/SKILL.md before apply -> Skipped naming it and AGENTS.md unchanged; MERGE_HEAD present -> Skipped; detached HEAD -> Skipped; linked worktree -> Skipped; source project itself -> Skipped `bee source`; a pre-commit hook that exits 1 -> AppliedNotCommitted; fake that sleeps past a 1 s timeout -> Failed mentioning timed out and the next project still reports; unknown id -> UnknownProjects. No code comments.",
  "must_haves":{"truths":["A dry run writes nothing in any project","An upgrade commit contains only newly changed files under bee's expanded planned paths","Work the user already staged is never committed","A user edit inside a skill folder bee would sync skips the project","A project without bee, with a live session, on a detached HEAD, or that is the source is skipped with its reason","A hung onboard fails one project and the run continues","git_diff behaviour is unchanged"],"artifacts":[{"path":"crates/waggledance-core/src/bee_upgrade.rs","substantive":"find_source, run, path expansion, report types, tests"},{"path":"crates/waggledance-core/src/process.rs","substantive":"run_with_deadline shared runner"}],"key_links":["lib.rs exports bee_upgrade","git_diff::run_git and bee_upgrade both call process::run_with_deadline"],"prohibitions":["Never run git commit without a pathspec","Never pass --apply in dry-run mode","No configured source path","No thread pool","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance-core",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"bbu-2","feature":"bee-bulk-upgrade","title":"Add the waggledance bee upgrade command","lane":"standard","role":"test","status":"open","deps":["bbu-1"],
  "decisions":["D1","D10","557900d6","f3604e70"],
  "files":["crates/waggledance/src/cli.rs"],
  "read_first":["docs/history/bee-bulk-upgrade/plan.md","crates/waggledance-core/src/bee_upgrade.rs","crates/waggledance/src/cli.rs"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach step 3. Add `Bee { #[command(subcommand)] action: BeeAction }` to Command with BeeAction::Upgrade { #[arg(long)] apply: bool, #[arg(long = \"project\")] project: Vec<String>, #[arg(long)] json: bool }, dispatched next to the other arms in run(). cmd_bee_upgrade builds the engine with runtime::build_engine(), lists projects and calls waggledance_core::bee_upgrade::run with UpgradeOptions {apply, only: project, ..Default::default()}. JSON mode prints the UpgradeReport. Text mode comes from a pure fn render_upgrade_text(&UpgradeReport) -> String: header `bee source: <root> @ <branch> <sha>` plus ` (dirty)` when source_dirty, and `target: <version>`; one line per project with id, branch or `-`, from_version or `-`, the status word, then files count, reason, commit sha or error; a notices count when non-zero; a count line per status; in dry-run mode a last line `dry run — re-run with --apply to upgrade and commit`. NoSource or UnknownProjects returns Err with its Display text. Tests in cli.rs mod tests (create it if absent): the dry-run footer, an Upgraded line with its sha, a Skipped line with its reason, an AppliedNotCommitted line with its error, the dirty-source marker. After green, resolve the binary from `cargo build --profile fast -p waggledance --message-format=json` (compiler-artifact executable) and run `<bin> bee upgrade --json` once against the real registry; record in the cap the per-status counts it printed. Never run --apply live. No code comments.",
  "must_haves":{"truths":["waggledance bee upgrade without --apply changes no project","--project narrows the run to the named ids","--json prints the core report unchanged"],"artifacts":[{"path":"crates/waggledance/src/cli.rs","substantive":"Bee/Upgrade subcommand, cmd_bee_upgrade, render_upgrade_text, tests"}],"key_links":["cli.rs calls waggledance_core::bee_upgrade::run"],"prohibitions":["No live --apply run","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance cli",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"bbu-3","feature":"bee-bulk-upgrade","title":"Expose the bulk upgrade as an MCP tool","lane":"standard","role":"test","status":"open","deps":["bbu-1"],
  "decisions":["D1","D9","557900d6","bb1a29e2"],
  "files":["crates/waggledance/src/mcp.rs"],
  "read_first":["docs/history/bee-bulk-upgrade/plan.md","crates/waggledance-core/src/bee_upgrade.rs","crates/waggledance/src/mcp.rs"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach step 4. Add fn bee_upgrade_schema() named waggledance_bee_upgrade. Its description: upgrades bee in registered projects that already use bee; previews by default; with apply true it runs bee onboard --apply and commits only bee's files; apply requires a non-empty projects list the human has approved. inputSchema properties apply (boolean, default false) and projects (array of project id strings). List it in tools/list after projects_schema(), dispatch it in handle_tool_call's match (next to `\"waggledance_projects\" => handle_projects(id, engine),`) as handle_bee_upgrade(id, engine, &args). The handler rejects a non-boolean apply and apply=true with a missing or empty projects list (D9) using the error shape the other handlers use, then calls engine.list_projects() and waggledance_core::bee_upgrade::run; result text `<n> project(s): <counts per status>` plus structuredContent = the report; NoSource or UnknownProjects map to the tool error shape. Rename tools_list_has_eight_schemas to tools_list_has_nine_schemas and add the new schema to its array. Tests: apply true without projects is refused; non-boolean apply is refused; an empty registry returns the NoSource error. No code comments.",
  "must_haves":{"truths":["tools/list includes waggledance_bee_upgrade","apply defaults to false","apply true without named projects is refused","The structured result is the core report"],"artifacts":[{"path":"crates/waggledance/src/mcp.rs","substantive":"schema, dispatch arm, handler, tests"}],"key_links":["handle_tool_call routes waggledance_bee_upgrade to handle_bee_upgrade"],"prohibitions":["No change to the other tools' schemas","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance mcp",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"bbu-4","feature":"bee-bulk-upgrade","title":"Show bee versions and upgrade buttons on a Bee tab","lane":"standard","role":"test","status":"open","deps":["bbu-1"],
  "decisions":["D1","D10","557900d6","f3604e70"],
  "files":["crates/waggledance/src/server.rs","crates/waggledance/src/views.rs","crates/waggledance/assets/app.css"],
  "read_first":["docs/history/bee-bulk-upgrade/plan.md","crates/waggledance-core/src/bee_upgrade.rs","docs/knowledge/work/homepage-tabs/index.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"Red-first, follow plan.md Approach step 5. views.rs: add HomeTab::Bee and a fourth home_tabbar link `/?tab=bee` labelled Bee after Usage (search `fn home_tabbar` and `enum HomeTab`), with a TABBAR_ICON_BEE in the same svg idiom as the other tabbar icons; fix every HomeTab match. pub fn bee_tab(report: Option<&UpgradeReport>, error: Option<&str>) -> String renders: a header with source root, branch, short sha, a dirty warning when source_dirty, and the target version; a table with one row per project (name, branch, from_version, to_version, status word, files count, reason or error or commit sha, notices count); on each Pending row, when the report is not applied, a small POST form to /api/bee/upgrade with a hidden project=<id> and a button `Upgrade`; above the table, when any row is Pending and not applied, a POST form with hidden all=1 and a button `Upgrade all pending`; and one line saying the upgrade commits only bee's files and skips projects with overlapping edits or a live session. The error case renders the error text instead of the table. Escape every string with the existing html escape helper. server.rs: accept tab=bee in the home tab parsing; index_page runs bee_upgrade::run(projects, &UpgradeOptions::default()) in tokio::task::spawn_blocking only when tab is Bee and renders bee_tab inside the home page. Add `.route(\"/api/bee/upgrade\", post(bee_upgrade_apply))` inside the same router as the other POST routes (so the existing cross-site guard covers it), taking axum Form<BeeUpgradeForm {project: Option<String>, all: Option<String>}>: neither set -> 400 page that changes nothing; project set -> only = [id]; all set -> run a dry run first and set only = the ids whose outcome is Pending; then run with apply true in spawn_blocking and respond with the home page on the Bee tab rendering the applied report. app.css: .bee-upgrade-* classes on existing variables; the table scrolls horizontally on narrow screens. Tests in server.rs bee_route_tests: /?tab=bee renders the Bee tab with aria-current and the NoSource message on an empty registry; POST /api/bee/upgrade with an empty body returns 400; GET /api/bee/upgrade returns 405. views.rs tests: a Pending row has an Upgrade form carrying its id; a Skipped row has no form and shows its reason; an applied report renders no forms. No code comments.",
  "must_haves":{"truths":["The home tab bar shows a Bee tab","Only a POST naming a project or all applies","Skipped and failed rows have no Upgrade button","After apply the tab shows each project's commit or reason"],"artifacts":[{"path":"crates/waggledance/src/views.rs","substantive":"HomeTab::Bee, bee_tab"},{"path":"crates/waggledance/src/server.rs","substantive":"tab=bee wiring and /api/bee/upgrade route"}],"key_links":["server.rs calls waggledance_core::bee_upgrade::run in spawn_blocking"],"prohibitions":["Board, Agents and Usage tabs never run onboard","No new JS or crate dependency","No code comments"]},
  "verify":"cargo test --profile fast -p waggledance",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":true}},
 {"id":"bbu-5","feature":"bee-bulk-upgrade","title":"Describe the bulk bee upgrade and its safety rules","lane":"standard","role":"docs","status":"open","deps":["bbu-2","bbu-3","bbu-4"],
  "decisions":["D1","D2","D3","D4","D5","D6","D7","D8","D9","D10"],
  "files":["docs/knowledge/work/bee-bulk-upgrade/index.md"],
  "read_first":["docs/history/bee-bulk-upgrade/CONTEXT.md","docs/history/bee-bulk-upgrade/plan.md","crates/waggledance-core/src/bee_upgrade.rs","docs/knowledge/work/homepage-tabs/index.md"],
  "affects_skills":[],"affects_specs":[],
  "action":"Write one knowledge concept with the same frontmatter shape as docs/knowledge/work/homepage-tabs/index.md. Cover what the bulk upgrade does; the three surfaces (waggledance bee upgrade and its flags, waggledance_bee_upgrade and its arguments including the D9 rule, the Bee tab and POST /api/bee/upgrade); source discovery (D5); every skip rule with its decision id; path expansion (D7); commit isolation (pathspec add and commit); the outcomes; and the timeout. Put every code name in backticks. Pointers name bee_upgrade.rs, process.rs, cli.rs, mcp.rs, server.rs, views.rs. Before capping, extract every backticked identifier from the concept and rg each one in crates; every one must be found.",
  "must_haves":{"truths":["Every backticked code name in the concept exists in crates"],"artifacts":[{"path":"docs/knowledge/work/bee-bulk-upgrade/index.md","substantive":"surfaces, rules, outcomes, pointers"}],"key_links":["Pointers name bee_upgrade.rs"],"prohibitions":["Cite CONTEXT.md decisions, never restate them in new words"]},
  "verify":"rg -o '`[A-Za-z_:/.-]+`' docs/knowledge/work/bee-bulk-upgrade/index.md",
  "trace":{"worker":null,"outcome":null,"files_changed":[],"deviations":[],"friction":null,"capped_at":null,"behavior_change":false}}
]
```

## Test matrix

- Happy path: a dry run lists `pending` with files and writes nothing. Pass when the project's `git status` is byte-identical before and after. An apply on a clean repo commits. Pass when `git show --name-only HEAD` lists exactly the planned files.
- Edge cases:
  - A pre-staged user file is not committed. Pass when it stays in `git diff --cached` after the upgrade.
  - An overlapping dirty file skips the project. Pass when the outcome is `skipped` and names the file.
  - A project with no bee or a linked worktree is skipped. Pass when the outcome is `skipped` and the fake bee was never called.
  - `--project` narrows the run. Pass when the report holds only the named ids.
- Error paths:
  - No source. Pass when the run errors with the looked-for path.
  - Onboard hangs. Pass when the outcome is `failed` with `timed out` and later projects still report.
  - An unknown id. Pass when the error names it.
  - An empty board POST. Pass when it returns 400 and nothing changes.

## Open Questions

(none)

## Out of scope

- Installing bee into projects that do not have it (D2).
- Upgrading the beehive checkout itself from git, or building the bee binary.
  The source is used as it stands.
- Pushing commits to any remote.
