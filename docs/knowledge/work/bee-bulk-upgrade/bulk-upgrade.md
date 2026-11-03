---
type: bee.area
title: bee-bulk-upgrade — upgrading bee across registered projects
description: "One core call upgrades bee in every registered project that already uses bee: a dry run by default, and on apply it runs bee onboard with apply in each project and commits only the files bee wrote. Three surfaces call it: the waggledance bee upgrade command, the waggledance_bee_upgrade MCP tool, and the home page's Bee tab."
timestamp: 2026-10-31
bee:
  id: bee-bulk-upgrade
  lifecycle: active
  areas: [bee-cockpit, web-interface, cli, mcp-surface]
  sources: [docs/history/bee-bulk-upgrade/CONTEXT.md, docs/history/bee-bulk-upgrade/plan.md]
---

# bee-bulk-upgrade — upgrading bee across registered projects

Locked decisions: bee-bulk-upgrade D1-D10 (`docs/history/bee-bulk-upgrade/CONTEXT.md`),
and bdc40074 (bee-managed paths, `docs/history/bee-upgrade-managed-paths/plan.md`).

## What it does

`bee_upgrade::run` takes the registered projects and an `UpgradeOptions`
(`apply`, `only`, `timeout`). It finds the bee source, then walks the projects
in a plain serial loop and returns an `UpgradeReport`: the source root, its
short HEAD sha (`source_head`), branch (`source_branch`) and dirty flag
(`source_dirty`), the `target_version`, whether the run applied, and one
`ProjectUpgrade` row per project. A row carries `branch`, `from_version` (read
from the project's `.bee/onboarding.json`), `to_version`, bee's `notices`,
`outside_repo`, and an `outcome`.

Without `apply`, the run calls `bee onboard` in dry-run form only and writes
nothing. `only` narrows the run to named project ids. An id that is not
registered refuses the whole run with `UnknownProjects`.

## Surfaces

All three surfaces call the same core function (D1).

### CLI command

`BeeAction::Upgrade`, run as waggledance bee upgrade, handled by
`cmd_bee_upgrade`. It builds the engine directly; it does not go through the
daemon.

- The apply flag runs the upgrade. Without it the command is a dry run.
- The project flag names one project id. Repeat it to name several.
- The json flag prints the `UpgradeReport` unchanged. Without it,
  `render_upgrade_text` prints a table.

### MCP tool

`waggledance_bee_upgrade`, schema `bee_upgrade_schema`, handler
`handle_bee_upgrade`. Arguments:

- `apply`: boolean, default `false`. Any other type is refused.
- `projects`: an array of project id strings. Anything else is refused.

`apply` set to `true` with no `projects`, or an empty list, is refused with
`apply true needs a non-empty projects list the human has approved` (D9). The
structured result is the core report.

### Bee tab

The home page's fourth tab, `HomeTab::Bee`, at `/?tab=bee`, also linked from
the desktop topbar. `bee_tab` renders the dry-run table. Each `pending` row has
its own form that posts `project` to `/api/bee/upgrade`; one "Upgrade all
pending" form posts `all`. The route sits in the existing router, so the
cross-site POST guard covers it. `bee_upgrade_apply` refuses a POST with
neither field with status 400, and upgrades nothing. On success the tab
re-renders with the applied report: each row shows its commit or its reason.
Skipped and failed rows have no Upgrade button.

bee's design gate checks HTML files only. It does not cover the Bee tab,
which Rust renders in `views.rs`.

## Source discovery

`find_source` returns the first registered project whose root holds
`packages/bee/AGENTS.block.md` and an executable `.bee/bin/bee` (D5). The
`bee onboard` call runs that binary with the source root as its working
directory. With no match, the run refuses with `NoSource`, which names the
marker file.

## Skip rules

A row is classified in this order. The first match ends it, and the reason is
the exact string in the row's `outcome`.

| Reason | Decision | When checked |
|---|---|---|
| `root missing` | D10 | every run |
| `no bee` | D2 | every run |
| `bee source` | D10 | every run |
| `not a git repository` | none: git cannot open the root | every run |
| `linked worktree; upgrades through its main checkout` | D6 | every run |
| `detached HEAD` | D10 | every run |
| `live bee session` | D8 | every run |
| `merge or rebase in progress` | D3 | apply only |
| `uncommitted edits in files bee would write` | D4, D7, bdc40074 | apply only |

The `no bee` skip never calls `bee onboard`, so it never installs bee.

The overlap skip splits the uncommitted files that fall `under` a planned path
into two sets (bdc40074). `is_managed` marks a file as bee-managed when it is:

- a path starting with `.claude/skills/bee-`, `.agents/skills/bee-`,
  `.opencode/skills/bee-` or `.pi/extensions/bee-guard` (`MANAGED_PREFIXES`)
- the exact path `.bee/onboarding.json`

Any other overlapping file skips the project, and the row's `files` names only
those other files. Bee-managed overlaps alone never skip: bee overwrites them,
and they are committed with the upgrade.

## Path expansion

`expand_paths` turns the dry-run `plan` into repo-relative paths (D7). A
`sync_skill` or `remove_skill` item joins its `path` onto the `target_root` of
the `skills/targets` entry whose `kind` matches the item's `target`. Any other
item uses its `path` as given. `repo_relative` drops an item that falls outside
the repo, and the item is listed in `outside_repo` only. `under` matches a
dirty path to a planned path by equality or by a directory prefix, so a skill
folder covers every file inside it. The bee-managed prefixes of bdc40074 are
matched against these expanded paths, never against the raw plan items.

## Commit isolation

On apply, `dirty_paths` reads `git status` before and after `bee onboard` with
apply. The commit set is the paths newly dirty after apply, plus the
bee-managed overlap paths still dirty after apply, kept only where they fall
`under` a planned path (D3, D7, bdc40074). `Upgraded` lists that set. Those paths go to git on stdin, NUL-separated, through
`--pathspec-from-file=-` and `--pathspec-file-nul`, first for add and then for
commit. `--literal-pathspecs` is passed as a top-level git option, so a path
is never read as a glob. The commit message is `Upgrade bee to` plus the
version. The commit passes `--only` with its pathspecs, so work the user
already staged stays staged and out of the commit.

Every git call sets `GIT_OPTIONAL_LOCKS=0`, so a read-only call such as
status never refreshes and rewrites a project's `.git/index`; a dry run leaves
the index untouched.

## Outcomes

`Outcome` serializes with a `status` tag.

| `status` | Meaning |
|---|---|
| `up_to_date` | bee reports nothing to do, or apply left no planned file changed |
| `pending` | dry run only: `files` lists the expanded planned paths |
| `upgraded` | applied and committed: `commit` and `files` |
| `applied_not_committed` | bee wrote the files but add or commit failed: `files` and `error` (D10) |
| `skipped` | a skip rule matched: `reason` and `files` |
| `failed` | `bee onboard` or git failed, or bee returned an unexpected status: `error` |

Apply succeeds only when bee's own status is `applied`. Any other status is
`failed`.

## Timeout

Every `bee onboard` and git call runs through `run_with_deadline`, with the
`timeout` from `UpgradeOptions`. The default is 120 seconds per call, and all
three surfaces use the default. A call past its deadline is killed, its row
fails with `timed out`, and the run goes on to the next project.

## Pointers

- `crates/waggledance-core/src/bee_upgrade.rs` — the core: `run`,
  `find_source`, skip rules, `expand_paths`, commit isolation, outcomes.
- `crates/waggledance-core/src/process.rs` — `run_with_deadline`, shared with
  `git_diff`.
- `crates/waggledance/src/cli.rs` — `BeeAction::Upgrade`, `cmd_bee_upgrade`,
  `render_upgrade_text`.
- `crates/waggledance/src/mcp.rs` — `bee_upgrade_schema`,
  `handle_bee_upgrade`.
- `crates/waggledance/src/server.rs` — `/api/bee/upgrade`,
  `bee_upgrade_apply`, `bee_upgrade_html`.
- `crates/waggledance/src/views.rs` — `HomeTab::Bee`, `bee_tab`.
