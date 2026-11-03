# Context: bee-bulk-upgrade

Upgrade bee in every registered project at once, instead of running
`bee onboard --apply` by hand in each repo.

On 2026-10-30, waggledance had 30 registered projects. 21 of them carried
bee: 14 on 2.67.0, 2 on 2.66.0, 2 on 2.59.0, and one each on 2.41.0 and
2.22.0. The other 9 had no bee. Most of the 30 had uncommitted work.

## Locked decisions

| ID | Decision | Store id |
|---|---|---|
| D1 | One shared core, three surfaces: a `waggledance bee upgrade` CLI command, a board action, and an MCP tool. | 557900d6 |
| D2 | Only projects that already hold `.bee/onboarding.json` are upgraded. A project without it is skipped, listed in the report, and never installed. | 6ba61ced |
| D3 | After `bee onboard --apply`, the upgrade stages exactly the files onboard changed and commits them itself. The user's other uncommitted work is never staged. A project mid-merge or mid-rebase is skipped. | 3439d31f |
| D4 | Before the apply step, the dry-run plan paths are compared with the project's uncommitted files. If any planned path already has the user's edits, the project is skipped, and the report names those files. | 02f08071 |
| D5 | The bee source is discovered: the first registered project whose root holds `packages/bee/AGENTS.block.md` and an executable `.bee/bin/bee`. That binary runs with that root as its working directory. If no project matches, the run refuses and names the file it looked for. | f9ce5ef6 |
| D6 | A registered project that is a linked git worktree is skipped. It gets the upgrade through its main checkout. | b95c1c4e |

| D7 | The overlap check and the commit both use expanded repo paths. A skill item maps to `<target_root>/<path>`. A directory item covers the files under it. An item outside the repo is only reported. The commit holds only files that are newly dirty after apply and that fall under an expanded planned path. | 0b11d69f |
| D8 | A project with a live bee session is skipped, both in the dry run and on apply. | 19377eb1 |
| D9 | The MCP tool previews by default. `apply=true` requires a non-empty `projects` list. | bb1a29e2 |
| D10 | These projects are also skipped: a missing root, the bee source itself, and a detached HEAD. Each row reports its branch and bee's notices. A project whose files were written but whose commit failed gets the outcome `applied_not_committed`. | f3604e70 |

D1–D4 came from the user's answers on 2026-10-30. D5–D10 are agent
defaults, recorded under `gate_bypass=full`. D7–D10 came from the plan-step
review.

## Facts the decisions rest on

- `bee onboard` refuses with `blocked_no_engine` when it is run outside a bee
  source checkout. Run from the beehive checkout against jarvis, the same
  call returns `changes_needed` with a `plan[]` of `{action, path}` and
  `bee_version: 2.67.0`.
- Without `--apply`, `bee onboard` writes nothing.
