# Composer Slash Suggest — Context

**Feature slug:** composer-slash-suggest
**Date:** 2026-08-31
**Shaping session:** clear-ask fast path (gate bypass full)
**Scope:** Standard
**Domain types:** UI, HTTP

## Feature Boundary

Typing `/` as the first character of a reply composer pops a suggestion menu
of the slash commands and skills available to the agent behind that pane;
picking one inserts it into the box. It ends at insertion: nothing is sent,
no command is executed by waggledance, and the daemon never interprets the
text — the agent CLI on the other side of the pane remains the only thing
that runs a slash command.

Original ask (verbatim): «Thêm tính năng hỗ trợ command và skill bằng cách
đánh / thì sẽ snippet suggest teong box đánh command»

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never
reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 (`a4d73a4f`) | Slash suggestions bind to every `.term-reply__text` composer — project pane reply, unassigned pane reply, paseo composer. The new-task overlay textarea is excluded. | The reply boxes feed a live agent session, the only place a slash command means anything; the task box writes a PBI description. |
| D2 (`ae531e75`) | Suggestions come from a new GET endpoint: `/p/:id/_slash` scans the project root (`.claude/commands/*.md`, `.claude/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md`) plus the user level (`~/.claude/commands/*.md`, `~/.claude/skills/*/SKILL.md`); `/_slash` serves user-level only for pages with no project. Entries are `{name, kind: command\|skill, description}`. | `Project.root_path` (domain.rs:11) and the `/p/:id/_jump` precedent (server.rs:508) make a per-project scan the cheapest fresh source. |
| D3 (`4da60387`) | The popup opens only when `/` is the FIRST character of the composer text; it filters by prefix as typed; ArrowUp/Down move, Enter/Tab insert `/name ` (trailing space), Escape closes. Markup and styling imitate the jump palette idiom (app.js:315-426, app.css:1952-2027). | Slash commands only fire at message start in the agent CLIs; mid-text `/` (paths, dates) must never pop the menu. |

### Agent's Discretion

Popup DOM structure (JS-created overlay vs inline element), fetch caching per
page load, dedup rule when a project skill shadows a user skill of the same
name, and how the description line is extracted from SKILL.md frontmatter are
implementation choices — constrained only by D1–D3.

## Existing Code Context

- `crates/waggledance/assets/app.js:315-426` — jump palette: overlay creation,
  debounce, arrow/Enter/Escape handling. The popup idiom to imitate.
- `crates/waggledance/assets/app.css:1952-2027` — `.jump-*` styles, tokens
  (`--color-surface-raised`, `--elevation-lg`).
- `crates/waggledance/assets/app.js:3490-3530` (project panes), `:3713-3725`
  (unassigned), `:4222-4272` (paseo) — the three composer wiring sites; forms
  are wired once at init and survive the pane pollers (which repaint only
  `screenEl.innerHTML`).
- `crates/waggledance/src/server.rs:508` — `/p/:id/_jump` route registration;
  `jump_search` (server.rs:7471) shows the project-lookup + Json response shape.
- `crates/waggledance/src/views.rs:23898-23918` — the two-sided APP_JS/markup
  handshake test idiom to copy for the new wiring.
