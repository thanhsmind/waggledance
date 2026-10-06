import type { PluginServerContext } from "@getpaseo/plugin/server";
import { waggledancePath } from "./server/installed.ts";
import { loadBoard, pickBin } from "./server/waggledance.ts";
import { board, settings } from "./shared/rpc.ts";

export default function contribute(server: PluginServerContext) {
  const stored = server.registerSettings(settings);

  server.handle(board, async ({ dir }) => {
    const state = await stored.read();
    const v = state.status === "ready" ? state.values : { fetch_url: "", link_base: "", path: "" };
    return loadBoard({ fetchUrl: v.fetch_url, linkBase: v.link_base, bin: pickBin(v.path, waggledancePath), dir });
  });

  return () => {};
}
