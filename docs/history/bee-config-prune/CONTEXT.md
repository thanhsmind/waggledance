# bee-config-prune — CONTEXT

## Ask

The owner set `team` once in bee's user-global config and wants every
registered project to inherit it, and the same for `herding`. Projects still
carry their own copies in `.bee/config.json` (tracked), `.bee/config.local.json`
(local) or `.bee/team.json` (team), which outrank the global file, so the
global value never reaches them. `waggledance bee config` edits only the global
file, and `bee config unset` refuses the governed keys `team`,
`herding.agents` and `herding.agent_command` ("the user edits those
directly"). The owner asked for the waggledance setup verb to remove them.

## Locked decisions

- D1. New verb `waggledance bee config prune <key> [--project <id>]... [--force] [--apply]`.
  A dry run unless `--apply`, like `waggledance bee upgrade`.
- D2. It removes the dotted key from every project layer that holds it:
  `.bee/config.json`, `.bee/config.local.json`, `.bee/team.json`. Parent
  objects the removal leaves empty are pruned too; other keys and their order
  are kept.
- D3. It refuses outright when the global file does not set the key, so a
  prune never leaves a project with no value.
- D4. Per project and layer it compares the layer's value with the global
  value. Equal: pruned. Different: reported as `differs` and kept, unless
  `--force`, which prunes it as well.
- D5. Keys `guards`, `gate_bypass`, `hooks` are refused (they are per-project
  only and never live in the global file).
- D6. It never commits in the target repos; it prints each file it changed.
- D7. Projects without `.bee/` or whose root is missing are skipped with a
  reason. A file that is not a JSON object is reported as an error row and left
  untouched.
- D8. Written files keep 2-space pretty JSON with a trailing newline,
  atomically (`write_atomic`).
