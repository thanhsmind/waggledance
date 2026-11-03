import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { waggledancePath } from "./installed.ts";

export default function waggledance(pi: ExtensionAPI): void {
	if (process.env.WAGGLEDANCE_DISABLED === "1") return;
	pi.registerMcpServer("waggledance", {
		command: waggledancePath || "waggledance",
		args: ["mcp"],
		exposure: "direct",
		description: "Waggle Dance: view docs in the browser, search docs and bee state across projects, dispatch work to agent panes",
	});
}
