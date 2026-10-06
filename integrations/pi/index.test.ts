import { test } from "node:test";
import assert from "node:assert/strict";
import waggledance from "./index.ts";

function fakePi() {
	const calls: [string, Record<string, unknown>][] = [];
	const pi = { registerMcpServer: (name: string, config: Record<string, unknown>) => calls.push([name, config]) };
	return { calls, pi };
}

test("registers the waggledance MCP server with direct exposure", () => {
	delete process.env.WAGGLEDANCE_DISABLED;
	const { calls, pi } = fakePi();
	waggledance(pi as any);
	assert.equal(calls.length, 1);
	const [name, config] = calls[0];
	assert.equal(name, "waggledance");
	assert.equal(config.command, "waggledance");
	assert.deepEqual(config.args, ["mcp"]);
	assert.equal(config.exposure, "direct");
});

test("WAGGLEDANCE_DISABLED=1 registers nothing", () => {
	process.env.WAGGLEDANCE_DISABLED = "1";
	const { calls, pi } = fakePi();
	waggledance(pi as any);
	delete process.env.WAGGLEDANCE_DISABLED;
	assert.equal(calls.length, 0);
});
