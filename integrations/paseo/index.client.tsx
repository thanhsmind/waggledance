import type { PluginClientContext } from "@getpaseo/plugin/client";
import { BoardPanel, BoardSurface } from "./client/board.tsx";
import { WaggleDanceSettings } from "./client/settings.tsx";

export default function contribute(client: PluginClientContext) {
  const cleanups = [
    client.addSurface("waggledance", BoardSurface),
    client.addSidebarItem({ id: "waggledance", title: "Waggle Dance", icon: "LayoutList", surface: "waggledance" }),
    client.addWorkspacePanel({ id: "waggledance", title: "Waggle Dance", icon: "LayoutList", context: "workspace", locations: ["explorer", "workspace"], Component: BoardPanel }),
    client.addSettingsScreen({ id: "waggledance", title: "Waggle Dance", icon: "LayoutList", Component: WaggleDanceSettings }),
  ];
  return () => cleanups.forEach((cleanup) => cleanup());
}
