import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, realpathSync, symlinkSync, writeFileSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { waggledancePath } from "../server/installed.ts";
import { loadBoard, pickBin, projectFor } from "../server/waggledance.ts";
import { linkTo, pollEvery, sections, BUSY_MS, QUIET_MS } from "../shared/board.ts";

type Route = { status: number; body: string; type?: string; location?: string };

const BOARD = {
  in_progress: [
    { project_id: "a", project_name: "A", feature: "f1", title: "One", done: 1, total: 3, waiting: true, reason: "gate", last_activity: null, url: "/p/a/_bee/feature/f1" },
    { project_id: "b", project_name: "B", feature: "f2", title: "Two", done: 0, total: 2, waiting: false, reason: null, last_activity: null, url: "/p/b/_bee/feature/f2" },
  ],
  ready_to_merge: [{ project_id: "a", project_name: "A", feature: "f3", title: "Three", url: "/p/a/_bee/feature/f3" }],
};
const AGENTS = [
  { project_id: "a", project_name: "A", pane_id: "1", name: "claude", status: "working", title: "", workspace: "", tab: "", url: "/p/a/_terminal/pane/1", bee_state: null, feature: null, mark: "" },
  { project_id: null, project_name: "Unassigned", pane_id: "2", name: "pi", status: "idle", title: "", workspace: "", tab: "", url: "/_terminal/unassigned", bee_state: null, feature: null, mark: "" },
];
const json = (v: unknown): Route => ({ status: 200, body: JSON.stringify(v), type: "application/json" });

async function serve(routes: Record<string, Route>): Promise<{ base: string; server: Server }> {
  const server = createServer((req, res) => {
    const r = routes[req.url ?? ""] ?? { status: 404, body: "" };
    res.writeHead(r.status, { "content-type": r.type ?? "text/plain", ...(r.location ? { location: r.location } : {}) });
    res.end(r.body);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { base: `http://127.0.0.1:${(server.address() as AddressInfo).port}`, server };
}

async function withServer(routes: Record<string, Route>, run: (base: string) => Promise<void>) {
  const { base, server } = await serve(routes);
  try {
    await run(base);
  } finally {
    server.close();
  }
}

function fakeBin(projects: unknown): string {
  const dir = mkdtempSync(join(tmpdir(), "wd-paseo-bin-"));
  const path = join(dir, "waggledance");
  writeFileSync(path, `#!/bin/sh\n[ "$1 $2" = "list --json" ] || exit 9\ncat <<'EOF'\n${JSON.stringify({ projects })}\nEOF\n`);
  chmodSync(path, 0o755);
  return path;
}

test("a daemon that refuses the connection is down and names waggledance serve", async () => {
  const { base, server } = await serve({});
  await new Promise<void>((resolve) => server.close(() => resolve()));
  const r = await loadBoard({ fetchUrl: base, linkBase: "", bin: "waggledance" });
  assert.equal(r.ok || r.code, "down");
  assert.match(r.ok ? "" : r.error, /`waggledance serve`/);
});

test("an HTML reply or a redirect is auth, pointing at the local fetch URL", async () => {
  await withServer({ "/api/board": { status: 200, body: "<html>sign in</html>", type: "text/html" }, "/api/agents": json(AGENTS) }, async (base) => {
    const r = await loadBoard({ fetchUrl: base, linkBase: "", bin: "waggledance" });
    assert.equal(r.ok || r.code, "auth");
    assert.match(r.ok ? "" : r.error, /link base/);
  });
  await withServer({ "/api/board": { status: 302, body: "", location: "https://login.example" }, "/api/agents": json(AGENTS) }, async (base) => {
    const r = await loadBoard({ fetchUrl: base, linkBase: "", bin: "waggledance" });
    assert.equal(r.ok || r.code, "auth");
  });
});

test("a daemon that never answers fails after the timeout", async () => {
  const server = createServer(() => {});
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  try {
    const r = await loadBoard({ fetchUrl: `http://127.0.0.1:${(server.address() as AddressInfo).port}`, linkBase: "", bin: "waggledance", timeoutMs: 200 });
    assert.equal(r.ok || r.code, "failed");
    assert.match(r.ok ? "" : r.error, /did not answer/);
  } finally {
    server.closeAllConnections();
    server.close();
  }
});

test("a daemon without /api/board fails with an update hint", async () => {
  await withServer({ "/api/agents": json(AGENTS) }, async (base) => {
    const r = await loadBoard({ fetchUrl: base, linkBase: "", bin: "waggledance" });
    assert.equal(r.ok || r.code, "failed");
    assert.match(r.ok ? "" : r.error, /update it/);
  });
});

test("agents 404 and 502 keep the board and set distinct notes", async () => {
  const notes: (string | undefined)[] = [];
  for (const status of [404, 502]) {
    await withServer({ "/api/board": json(BOARD), "/api/agents": { status, body: JSON.stringify({ error: "x" }), type: "application/json" } }, async (base) => {
      const r = await loadBoard({ fetchUrl: `${base}/`, linkBase: "", bin: "waggledance" });
      assert.ok(r.ok);
      assert.equal(r.board.in_progress.length, 2);
      assert.deepEqual(r.agents, []);
      assert.equal(r.link_base, base);
      assert.equal(r.project, undefined);
      notes.push(r.agents_note);
    });
  }
  assert.deepEqual(notes, ["agent terminal is disabled", "herdr is not running"]);
});

test("a workspace inside a registered root maps to that project, the longest root wins", async () => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "wd-paseo-root-")));
  const inner = join(root, "inner");
  mkdirSync(join(inner, "src"), { recursive: true });
  const link = join(mkdtempSync(join(tmpdir(), "wd-paseo-link-")), "ws");
  symlinkSync(join(inner, "src"), link);
  const bin = fakeBin([
    { id: "a", name: "A", root_path: root },
    { id: "b", name: "B", root_path: inner },
    { id: "c", name: "C", root_path: `${inner}-other` },
  ]);
  await withServer({ "/api/board": json(BOARD), "/api/agents": json(AGENTS) }, async (base) => {
    const b = await loadBoard({ fetchUrl: base, linkBase: "https://tunnel.example/", bin, dir: link });
    assert.ok(b.ok);
    assert.deepEqual(b.project, { id: "b", name: "B" });
    assert.deepEqual(b.board.in_progress.map((f) => f.feature), ["f2"]);
    assert.deepEqual(b.agents, []);
    assert.equal(b.link_base, "https://tunnel.example");

    const a = await loadBoard({ fetchUrl: base, linkBase: "", bin, dir: root });
    assert.ok(a.ok);
    assert.equal(a.project?.id, "a");
    assert.deepEqual(a.board.ready_to_merge.map((f) => f.feature), ["f3"]);
    assert.deepEqual(a.agents.map((x) => x.pane_id), ["1"]);

    const none = await loadBoard({ fetchUrl: base, linkBase: "", bin, dir: tmpdir() });
    assert.ok(none.ok);
    assert.equal(none.project, null);
    assert.deepEqual([none.board.in_progress, none.board.ready_to_merge, none.agents], [[], [], []]);
  });
  assert.equal(projectFor([{ id: "x", name: "X", root_path: "/a/b/" }], "/a/bc"), null);
});

test("a missing binary fails the workspace read and names the setting", async () => {
  await withServer({ "/api/board": json(BOARD), "/api/agents": json(AGENTS) }, async (base) => {
    const r = await loadBoard({ fetchUrl: base, linkBase: "", bin: join(tmpdir(), "no-such-waggledance"), dir: tmpdir() });
    assert.equal(r.ok || r.code, "failed");
    assert.match(r.ok ? "" : r.error, /settings/);
  });
});

test("the binary is the setting, else the installed path, else PATH", () => {
  assert.equal(waggledancePath, "");
  assert.equal(pickBin(" /x/wd ", "/y/wd"), "/x/wd");
  assert.equal(pickBin("", "/y/wd"), "/y/wd");
  assert.equal(pickBin("", ""), "waggledance");
});

test("sections split waiting from in progress, and polling speeds up while anything waits or works", () => {
  const s = sections(BOARD, AGENTS);
  assert.deepEqual(s.waiting.map((f) => f.feature), ["f1"]);
  assert.deepEqual(s.progress.map((f) => f.feature), ["f2", "f3"]);
  assert.equal(pollEvery(BOARD, []), BUSY_MS);
  assert.equal(pollEvery({ in_progress: [], ready_to_merge: [] }, AGENTS), BUSY_MS);
  assert.equal(pollEvery({ in_progress: [BOARD.in_progress[1]], ready_to_merge: [] }, [AGENTS[1]]), QUIET_MS);
  assert.equal(pollEvery(undefined, undefined), QUIET_MS);
  assert.equal(linkTo("https://t.example/", "/p/a"), "https://t.example/p/a");
});
