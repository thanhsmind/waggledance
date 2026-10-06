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
