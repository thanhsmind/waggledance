import type { Agent, Board, Feature } from "./rpc.ts";

export const DEFAULT_FETCH_URL = "http://127.0.0.1:7700";
export const BUSY_MS = 5000;
export const QUIET_MS = 30000;

export type Sections = { waiting: Feature[]; progress: Feature[]; agents: Agent[] };

export function sections(board: Board, agents: Agent[]): Sections {
  return {
    waiting: board.in_progress.filter((f) => f.waiting),
    progress: [...board.in_progress.filter((f) => !f.waiting), ...board.ready_to_merge],
    agents,
  };
}

export function pollEvery(board: Board | undefined, agents: Agent[] | undefined): number {
  const busy = (board?.in_progress ?? []).some((f) => f.waiting) || (agents ?? []).some((a) => a.status === "working");
  return busy ? BUSY_MS : QUIET_MS;
}

export function trimBase(url: string): string {
  return url.trim().replace(/\/+$/, "");
}

export function linkTo(base: string, url: string): string {
  return /^https?:\/\//.test(url) ? url : trimBase(base) + (url.startsWith("/") ? url : `/${url}`);
}

export function featureMeta(f: Feature): string {
  const parts = [f.project_name];
  if (f.total) parts.push(`${f.done ?? 0}/${f.total}`);
  if (f.waiting && f.reason) parts.push(f.reason);
  else if (f.done === undefined) parts.push("ready to merge");
  return parts.join(" · ");
}

export function agentMeta(a: Agent): string {
  return [a.status, a.project_name, a.title].filter((s) => s).join(" · ");
}
