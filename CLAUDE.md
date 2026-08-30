# Project Rules

## bee

This repo uses bee. The bare import below loads the BEE operating block from
AGENTS.md at context-load time. Never wrap it in backticks; that disables it.

@AGENTS.md

## Building (Waggledance)

For the dev loop — rebuilding the daemon to test a change — use the `fast`
profile, never `--release`:

```sh
cargo build --profile fast -p waggledance   # binary at target/fast/waggledance
```

`release` carries fat LTO + `codegen-units = 1`, which pin one core for ~43 s on
a one-line change; `fast` does the same rebuild in ~1 s. Reserve `--release` for
release builds and size checks — CI and `release.yml` already use it.

<!-- waggledance:START -->
## Documentation Viewing (Waggledance)

After creating or updating any markdown file, make it viewable in ONE call —
no project registration step needed:

### Using MCP (preferred)

Call `waggledance_view_file` with:

- `project_root`: absolute path to the project root
- `relative_path`: the file path relative to that root

It returns a browser `url`. Tell the user: "You can view this at: `<url>`".
The server auto-registers the project on first use and indexes the file
immediately.

### Using CLI fallback

```sh
waggledance open <absolute-path-to-file.md>
```

### When to render

Spin up a preview for long docs, tables, Mermaid diagrams, multi-file document
sets, or when the user asks to "preview"/"render". Skip it for short, trivial
snippets.
<!-- waggledance:END -->
