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
