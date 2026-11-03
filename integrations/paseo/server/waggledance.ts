import { execFile } from "node:child_process";
import { realpath } from "node:fs/promises";
import { DEFAULT_FETCH_URL, trimBase } from "../shared/board.ts";
import type { Agent, Board } from "../shared/rpc.ts";

export const TIMEOUT_MS = 5000;

export type Failure = { ok: false; code: "down" | "auth" | "failed"; error: string };
export type Project = { id: string; name: string; root_path: string };
type Reply = { ok: true; status: number; json: unknown } | Failure;

export type Loaded =
  | { ok: true; board: Board; agents: Agent[]; agents_note?: string; link_base: string; project?: { id: string; name: string } | null }
  | Failure;

export function pickBin(setting: string, installed: string): string {
  return setting.trim() || installed.trim() || "waggledance";
}

function down(base: string): Failure {
  return { ok: false, code: "down", error: `Waggle Dance is not running at ${base}. Start it on the Paseo host with \`waggledance serve\` (any \`waggledance open\` starts it too).` };
}

function auth(url: string): Failure {
  return {
    ok: false,
    code: "auth",
    error: `${url} answered with a redirect or a sign-in page, not Waggle Dance's JSON. Set the fetch URL to the local daemon (${DEFAULT_FETCH_URL}) and use the tunnel URL only as the link base.`,
  };
}

export async function getJson(base: string, path: string, timeoutMs = TIMEOUT_MS): Promise<Reply> {
  const url = base + path;
  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), timeoutMs);
  let res: Response;
  let body: string;
  try {
    res = await fetch(url, { redirect: "manual", signal: abort.signal, headers: { accept: "application/json" } });
    body = await res.text();
  } catch (error) {
    const e = error as { message?: string; cause?: { code?: string } };
    if (e.cause?.code === "ECONNREFUSED") return down(base);
    if (abort.signal.aborted) return { ok: false, code: "failed", error: `${url} did not answer within ${timeoutMs / 1000}s.` };
    return { ok: false, code: "failed", error: `${url}: ${e.cause?.code ?? e.message ?? String(error)}` };
  } finally {
    clearTimeout(timer);
  }
  if (res.status >= 300 && res.status < 400) return auth(url);
  try {
    return { ok: true, status: res.status, json: JSON.parse(body) };
  } catch {
    return res.ok ? auth(url) : { ok: true, status: res.status, json: undefined };
  }
}

export function agentsFrom(reply: Reply): { agents: Agent[]; note?: string } {
  if (!reply.ok) return { agents: [], note: reply.error };
  if (reply.status === 404) return { agents: [], note: "agent terminal is disabled" };
  if (reply.status === 502) return { agents: [], note: "herdr is not running" };
  if (reply.status === 200 && Array.isArray(reply.json)) return { agents: reply.json as Agent[] };
  return { agents: [], note: `/api/agents answered HTTP ${reply.status}` };
}

function boardFrom(reply: Reply, base: string): Board | Failure {
  if (!reply.ok) return reply;
  const b = reply.json as Partial<Board> | undefined;
  if (reply.status === 200 && Array.isArray(b?.in_progress) && Array.isArray(b?.ready_to_merge)) return b as Board;
  return { ok: false, code: "failed", error: `${base}/api/board answered HTTP ${reply.status} without a board. This plugin needs a waggledance that serves /api/board; update it.` };
}

export function contains(root: string, dir: string): boolean {
  const r = root.replace(/\/+$/, "");
  return dir === r || dir.startsWith(`${r}/`);
}

export function projectFor(projects: Project[], dir: string): Project | null {
  let best: Project | null = null;
  for (const p of projects) if (contains(p.root_path, dir) && (!best || p.root_path.length > best.root_path.length)) best = p;
  return best;
}

export function listProjects(bin: string, timeoutMs = TIMEOUT_MS): Promise<{ ok: true; projects: Project[] } | Failure> {
  return new Promise((resolve) => {
    execFile(bin, ["list", "--json"], { timeout: timeoutMs, maxBuffer: 16 << 20, env: { ...process.env, NO_COLOR: "1" } }, (error, stdout, stderr) => {
      if (error) {
        const err = error as NodeJS.ErrnoException & { killed?: boolean };
        if (err.code === "ENOENT") return resolve({ ok: false, code: "failed", error: `waggledance was not found at "${bin}". Set its full path in the Waggle Dance settings.` });
        if (err.killed) return resolve({ ok: false, code: "failed", error: `waggledance list did not answer within ${timeoutMs / 1000}s.` });
        return resolve({ ok: false, code: "failed", error: String(stderr).trim() || err.message });
      }
      try {
        const projects = (JSON.parse(stdout) as { projects?: Project[] }).projects;
        if (Array.isArray(projects)) return resolve({ ok: true, projects });
      } catch {}
      resolve({ ok: false, code: "failed", error: "waggledance list --json did not print a project list." });
    });
  });
}

export async function realDir(dir: string): Promise<string> {
  try {
    return await realpath(dir);
  } catch {
    return dir;
  }
}

export async function loadBoard(opts: { fetchUrl: string; linkBase: string; bin: string; dir?: string; timeoutMs?: number }): Promise<Loaded> {
  const base = trimBase(opts.fetchUrl) || DEFAULT_FETCH_URL;
  const [boardReply, agentsReply] = await Promise.all([getJson(base, "/api/board", opts.timeoutMs), getJson(base, "/api/agents", opts.timeoutMs)]);
  const board = boardFrom(boardReply, base);
  if ("ok" in board) return board;
  const { agents, note } = agentsFrom(agentsReply);
  const out: Loaded = { ok: true, board, agents, link_base: trimBase(opts.linkBase) || base };
  if (note) out.agents_note = note;
  if (opts.dir === undefined) return out;
  const listed = await listProjects(opts.bin, opts.timeoutMs);
  if (!listed.ok) return listed;
  const p = projectFor(listed.projects, await realDir(opts.dir));
  const keep = (id: string | null) => p !== null && id === p.id;
  return {
    ...out,
    board: { in_progress: board.in_progress.filter((f) => keep(f.project_id)), ready_to_merge: board.ready_to_merge.filter((f) => keep(f.project_id)) },
    agents: agents.filter((a) => keep(a.project_id)),
    project: p && { id: p.id, name: p.name },
  };
}
