import type { PluginServerContext } from "@getpaseo/plugin/server";
import { runStatus as readRunStatus, sendToLeader } from "./server/dispatch.ts";
import { waggledancePath } from "./server/installed.ts";
import { loadBoard, pickBin } from "./server/waggledance.ts";
import { board, runStatus, send, settings } from "./shared/rpc.ts";

export default function contribute(server: PluginServerContext) {
  const stored = server.registerSettings(settings);

  const values = async () => {
    const state = await stored.read();
    return state.status === "ready" ? state.values : { fetch_url: "", link_base: "", path: "" };
  };

  server.handle(board, async ({ dir }) => {
    const v = await values();
    return loadBoard({ fetchUrl: v.fetch_url, linkBase: v.link_base, bin: pickBin(v.path, waggledancePath), dir });
  });

  const bin = async () => pickBin((await values()).path, waggledancePath);

  server.handle(send, async ({ project_id, task, request_id }) => sendToLeader(await bin(), project_id, task, request_id));

  server.handle(runStatus, async ({ run_id }) => readRunStatus(await bin(), run_id));

  return () => {};
}
