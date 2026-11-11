# headless-supervisor — delivery

Code: a1518d2 (binary), c26c1ea (core), hs-3 (docs and integrations). Decisions D1-D5 in `CONTEXT.md`.

## Concepts changed

- `docs/knowledge/areas/supervisor-daemon.md` — new: records the headless daemon, the seven-tool MCP surface and the removed surfaces, so a reader looking for the board or search finds why they are gone.
- `docs/knowledge/patterns/a-tcp-health-probe-reads-a-listenerless-daemon-as-dead.md` — added by hs-1: status, stop and restart must key off the lock pid once the daemon binds no socket.
- `docs/knowledge/patterns/an-fts5-delete-by-unindexed-column-scans-the-whole-table.md` — retired and trimmed: the file store it warned about is dropped (D4); the lesson stays, anchors cut to files that still exist, critical flag cleared.
- `docs/knowledge/patterns/paseo-leader-dispatch.md` — removed the plugin request box, the `/api/board` leader-wait pill, `setup paseo` and the plugin pointers (D3, D5); leader dispatch itself is unchanged.
- `docs/knowledge/patterns/wd-direct-leader.md` — dropped the plugin request box from the "no other caller reaches wd's leader" line (D5).
- `docs/knowledge/patterns/paseo-native-supervisor.md` — already retired; removed the plugin request box from its dual-leader note (D5).
- `docs/knowledge/patterns/resolve-a-panes-project-from-the-form-not-the-page.md` — retired: the homepage Terminals tab is gone (D3).
- `docs/knowledge/patterns/read-the-live-themes-token-not-the-base-contracts.md` — retired: the console theme CSS is gone (D3).
- `docs/knowledge/patterns/a-js-only-behavior-can-still-be-proven.md` — retired: `app.js` is gone (D3).
- `docs/knowledge/patterns/measure-the-send-and-the-paint-separately.md` — retired: the terminal key grid it measured is gone (D3).

## Left as written

- `docs/knowledge/work/**` — dated delivery records of earlier features (board, terminals, kanban and similar); they say what shipped then and claim nothing about today.
- `docs/knowledge/patterns/shape-a-removal-by-its-seams-not-its-layers.md`, `the-binary-you-ran-is-not-the-one-you-built.md`, `prove-the-whole-path.md` — general lessons that still apply; the daemon proof used them.
