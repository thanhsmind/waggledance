import { defineRpc, defineSettings } from "@getpaseo/plugin";
import { z } from "zod";
import { DEFAULT_FETCH_URL } from "./board.ts";

const Feature = z.object({
  project_id: z.string(),
  project_name: z.string(),
  feature: z.string(),
  title: z.string(),
  url: z.string(),
  done: z.number().optional(),
  total: z.number().optional(),
  waiting: z.boolean().optional(),
  reason: z.string().nullable().optional(),
});

const Agent = z.object({
  project_id: z.string().nullable(),
  project_name: z.string(),
  pane_id: z.string(),
  name: z.string(),
  status: z.string(),
  title: z.string(),
  url: z.string(),
});

const BoardData = z.object({ in_progress: z.array(Feature), ready_to_merge: z.array(Feature) });

const Failure = z.object({ ok: z.literal(false), code: z.enum(["down", "auth", "failed"]), error: z.string() });

export const board = defineRpc({
  name: "waggledance.board",
  input: z.object({ dir: z.string().optional() }),
  output: z.union([
    z.object({
      ok: z.literal(true),
      board: BoardData,
      agents: z.array(Agent),
      agents_note: z.string().optional(),
      link_base: z.string(),
      project: z.object({ id: z.string(), name: z.string() }).nullable().optional(),
    }),
    Failure,
  ]),
});

export const send = defineRpc({
  name: "waggledance.send",
  input: z.object({ project_id: z.string().min(1), task: z.string().min(1), request_id: z.string().min(1) }),
  output: z.union([
    z.object({ ok: z.literal(true), run_id: z.string(), status: z.string(), duplicate: z.boolean(), warnings: z.array(z.string()) }),
    Failure,
  ]),
});

export const runStatus = defineRpc({
  name: "waggledance.run_status",
  input: z.object({ run_id: z.string().min(1) }),
  output: z.union([z.object({ ok: z.literal(true), status: z.string(), age_secs: z.number().optional() }), Failure]),
});

export const STATUS_POLL_MS = 10_000;

export function describeSent(leader: string, r: { run_id: string; duplicate: boolean; warnings: string[] }): string {
  const head = r.duplicate ? `Already sent (run ${r.run_id})` : `Queued for ${leader} leader (run ${r.run_id})`;
  return [head, ...r.warnings].join(" — ");
}

export function describeRun(status: string, ageSecs?: number): { text: string; final: boolean } {
  switch (status) {
    case "queued":
      return { text: ageSecs === undefined ? "queued" : `queued for ${ageSecs}s`, final: false };
    case "working":
    case "timeout":
      return { text: "working", final: false };
    case "blocked":
      return { text: "blocked — waiting for a permission in the leader's chat", final: false };
    case "done":
      return { text: "done", final: true };
    case "lost":
      return { text: "lost", final: true };
    default:
      return { text: status, final: false };
  }
}

export const settings = defineSettings({
  id: "waggledance",
  scope: "host",
  version: 1,
  schema: z.object({
    fetch_url: z.string().default(DEFAULT_FETCH_URL),
    link_base: z.string().default(""),
    path: z.string().default(""),
  }),
});

export type Feature = z.infer<typeof Feature>;
export type Agent = z.infer<typeof Agent>;
export type Board = z.infer<typeof BoardData>;
