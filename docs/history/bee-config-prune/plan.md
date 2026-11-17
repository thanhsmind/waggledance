# bee-config-prune — plan

Lane: small. Decisions: `docs/history/bee-config-prune/CONTEXT.md` (D1–D8).
Architect skipped: one data shape (a prune report row), no competing designs.

## Data shape

```text
PruneLayer = Tracked(.bee/config.json) | Local(.bee/config.local.json) | Team(.bee/team.json)
PruneAction = Pruned | Differs | Absent
PruneRow { id, layer: Option<PruneLayer>, action, file: Option<path>, applied: bool, error: Option<String>, skipped: Option<String> }
```

A pure function decides the action from (layer value, global value, force);
a thin shell reads and writes the files.

## Throughput checkpoint

- Blocking first steps: n/a — gate is approved before the one cell.
- Independent workstreams: core module + CLI wiring share one call chain; docs follow.
- Shared mutable state: the project files are written only under `--apply`.
- Smallest safe decomposition: one code cell (core + CLI + tests), one docs cell.

## Cells

- bcp-1 (code): `crates/waggledance-core/src/bee_global_config.rs` gains
  `prune(projects, key, only, force, apply) -> Result<Vec<PruneRow>>` per D1–D8;
  `crates/waggledance/src/cli.rs` gains `BeeConfigAction::Prune`; unit tests in
  the module, a CLI test in `crates/waggledance/tests/bee_global_config_cli.rs`.
- bcp-2 (docs): `docs/knowledge/areas/supervisor-daemon.md` names the prune verb.

## Load-bearing claims

| # | Claim | Label | Anchor | Verbatim evidence |
|---|---|---|---|---|
| 1 | bee's own unset refuses the governed keys, so waggledance must edit project files itself | ran | `.bee/bin/bee config unset --help` | "Refuses, with zero change, a key that is, sits inside, or holds a governed key the config write guard protects (team, herding.agents, herding.agent_command, hooks): the user edits those directly." |
| 2 | Global sets team but not herding today | ran | `waggledance bee config show` | `['team']` |
| 3 | 21 projects share one identical tracked herding value | ran | `waggledance bee config explain herding --json` grouped by hash | `0b21e777 21 [... 'waggledance(tracked)']` |
| 4 | JSON key order survives a parse/write round trip | read | `Cargo.toml:14` | `serde_json = { version = "1", features = ["preserve_order"] }` |
| 5 | GlobalConfig::save cannot write project files, which hold guards/hooks | read | `crates/waggledance-core/src/bee_global_config.rs:180` | `let bad = forbidden_keys_in(&self.doc);` |
