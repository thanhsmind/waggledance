---
artifact_contract: bee-plan/v2
mode: small
---

# Plan: openclaw-bee-skills

## Summary

`waggledance setup openclaw` installs wd's bridge skill into one agent
workspace and the supervisor skill into `~/.agents/skills`. After this
change it also installs the whole bee skill set (bee-hive and every other
`bee-*` skill) into OpenClaw's global skills dir, so every OpenClaw agent
can run the bee workflow.

Mode: `small`. One code file changes. Risk: setup writes outside the repo,
into the user's OpenClaw home; it reuses the existing guarded install path
(foreign dirs set aside, VERSION marker, restore on remove).

## Requirements

- 3ee49e87 (user, 2026-11-02): the source is `.claude/skills/bee-*` of
  the bee-onboarded repo root that contains the current directory (found by
  walking up to a dir holding `.bee/onboarding.json`). The target is
  `<openclaw home>/skills/<name>`. Each installed dir gets a `VERSION`
  marker. A foreign same-named dir is moved to backups. A marked `bee-*`
  dir that the source no longer holds is removed. `--remove` removes the
  marked `bee-*` dirs. Outside an onboarded repo, setup skips the bee skills
  with a note and still installs the rest.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|-------|-------|--------|-------------------|
| 1 | Skill installs already set foreign dirs aside and mark ownership | read | `crates/waggledance/src/setup.rs:310` | `fn install_skill(dst: &Path, want: &Files, backups: &Path, now: u64) -> Result<Vec<String>> {` |
| 2 | OpenClaw's global skills dir is `<home>/skills` | ran | `ls ~/.openclaw/skills` | `deep-research` |
| 3 | Every bee skill file is text, no symlinks or executables | ran | `find bee-* -type f \| xargs file --mime-encoding` | no non-utf-8/ascii file |
| 4 | The bee projections are not tracked in git | ran | `git ls-files .claude/skills \| grep -c bee-` | `0` |

## Approach

In `setup.rs`:

- `bee_skills_source(cwd)`: walk up from cwd to the first dir holding
  `.bee/onboarding.json`; return `<root>/.claude/skills` when it holds
  `bee-hive/SKILL.md`.
- `bee_skill_sets(src)`: read each `bee-*` dir into a `Files` map and add
  `VERSION` (the `bee_version` from `.bee/onboarding.json`, else the
  waggledance version).
- `OpenClawPaths` gains `global_skills` (`home/skills`) and `bee_source`.
- `openclaw_install` installs each set with `install_skill`, then removes
  marked `bee-*` dirs in `global_skills` that are not in the set. With no
  source it pushes a note.
- `openclaw_remove` calls `remove_skill` on every marked `bee-*` dir.
- README line under the OpenClaw section names the new install.

## Cells

- **obs-1** (role `code`): the change above plus unit tests in the
  existing `setup.rs` test module, using temp dirs for the source repo and
  the OpenClaw home: fresh install, rerun writes nothing, stale marked dir
  removed, foreign dir set aside, remove, no-source note.

## Verify

`cargo test --profile fast -p waggledance setup::` then the full
`cargo test --profile fast -p waggledance -p waggledance-core`, and a live
`waggledance setup openclaw` run from the waggledance repo with
`ls ~/.openclaw/skills | grep -c bee-`.
