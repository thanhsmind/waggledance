# Project registry

A user adds a repository with `waggledance register <path>`, sees it with `list` and
`projects`, and removes it with `unregister`. Unregistering never deletes the project's
files.

## Sub-features

- `registry-empty` — `list` on an empty registry exits 0.
- `registry-register` — `register <path> --name <n> --json` prints the id, the name and
  `root_path`, and writes a row.
- `registry-list` — `list` shows the root path, `projects` prints `{"projects":[…]}`, and
  `status` counts the project.
- `registry-unregister` — `unregister <id>` removes the row and leaves the repo on disk.

## How to get to it (user POV)

- Run `waggledance register ~/code/app [--name app] [--json]`.
- Run `waggledance list` (human) or `waggledance projects` (JSON).
- Run `waggledance unregister <id>`.
- Agents reach the same list through the MCP tool `waggledance_projects` (see `mcp-server`).

## Driving it with control-waggledance

Preconditions:

- The scratch registry is empty, and the scratch repo exists at `.bee/tmp/verify-app/<run>/repo`.

- **Empty list.** Run `list`. It exits 0.
- **Register.** Run `register <repo> --name verify-scratch --json`. Exit 0, and stdout holds
  `"root_path":"<repo>"`. A second view, `select count(*) from projects where
  root_path='<repo>'` in `registry.db`, returns `1`.
- **List.** `list` shows `<repo>`, `projects` shows `"name":"verify-scratch"`, and `status`
  shows `projects: 1`.
- **Unregister.** Run `unregister repo`. Exit 0, `registry.db` holds 0 rows, and
  `<repo>/.git` still exists.
- **Restore.** The script registers the repo again so later features see one project.
- **Drive it.** `bash .bee/verify/verify-app/control-waggledance drive --fast registry` (the
  default smoke path).

## Gotchas

- The id comes from the directory name (`repo`), not from `--name`. Unregister by id.
- A scratch daemon that can reach Paseo fills the registry with the user's real projects.
  Doctor's project-count check catches it.
- Paths inside `/tmp` or equal to `$HOME` are not eligible for discovery. Explicit
  `register` still accepts them.
