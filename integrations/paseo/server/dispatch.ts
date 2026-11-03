import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import type { Failure } from "./waggledance.ts";

export const SEND_TIMEOUT_MS = 60_000;

export type Sent = { ok: true; run_id: string; status: string; duplicate: boolean; warnings: string[] };
export type RunStatus = { ok: true; status: string; age_secs?: number };

type Structured = { run_id?: string; status?: string; duplicate?: boolean; warnings?: string[]; age_secs?: number };

type Message = {
  id?: number;
  error?: { message?: string };
  result?: { isError?: boolean; content?: { text?: string }[]; structuredContent?: Structured };
};

const failed = (error: string): Failure => ({ ok: false, code: "failed", error });

function outcome(tool: string, msg: Message): Structured | Failure {
  if (msg.error) return failed(msg.error.message ?? "waggledance mcp refused the call.");
  const r = msg.result;
  const text = (r?.content ?? []).map((c) => c.text ?? "").join("\n");
  if (r?.isError) return failed(text || `${tool} failed without a message.`);
  return r?.structuredContent ?? failed(text || `${tool} answered without a result.`);
}

function callTool(bin: string, tool: string, args: object, timeoutMs: number): Promise<Structured | Failure> {
  return new Promise((resolve) => {
    const child = spawn(bin, ["mcp"], { stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, NO_COLOR: "1" } });
    let stderr = "";
    let done = false;
    const finish = (r: Structured | Failure) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      child.kill();
      resolve(r);
    };
    const timer = setTimeout(
      () => finish(failed(`waggledance did not answer within ${timeoutMs / 1000}s. The request may already be queued; retrying is safe.`)),
      timeoutMs,
    );
    const write = (msg: object) => child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", ...msg })}\n`);
    child.on("error", (error: NodeJS.ErrnoException) =>
      finish(failed(error.code === "ENOENT" ? `waggledance was not found at "${bin}". Set its full path in the Waggle Dance settings.` : error.message)),
    );
    child.on("close", (code) => finish(failed(stderr.trim() || `waggledance mcp exited with code ${code} before answering.`)));
    child.stdin.on("error", () => {});
    child.stderr.on("data", (d) => (stderr += d));
    createInterface({ input: child.stdout }).on("line", (line) => {
      let msg: Message;
      try {
        msg = JSON.parse(line);
      } catch {
        return;
      }
      if (msg.id === 1) {
        if (msg.error) return finish(outcome(tool, msg));
        write({ method: "notifications/initialized" });
        write({ id: 2, method: "tools/call", params: { name: tool, arguments: args } });
      } else if (msg.id === 2) finish(outcome(tool, msg));
    });
    write({ id: 1, method: "initialize", params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "waggledance-paseo", version: "0.0.0" } } });
  });
}

export async function sendToLeader(bin: string, project: string, task: string, requestId: string, timeoutMs = SEND_TIMEOUT_MS): Promise<Sent | Failure> {
  const s = await callTool(bin, "waggledance_dispatch", { project, task, leader: true, request_id: requestId }, timeoutMs);
  if ("ok" in s) return s;
  if (!s.run_id) return failed("waggledance_dispatch answered without a run id.");
  return { ok: true, run_id: s.run_id, status: s.status ?? "queued", duplicate: s.duplicate === true, warnings: s.warnings ?? [] };
}

export async function runStatus(bin: string, runId: string, timeoutMs = SEND_TIMEOUT_MS): Promise<RunStatus | Failure> {
  const s = await callTool(bin, "waggledance_await", { run_id: runId, timeout_seconds: 0 }, timeoutMs);
  if ("ok" in s) return s;
  if (!s.status) return failed("waggledance_await answered without a status.");
  return typeof s.age_secs === "number" ? { ok: true, status: s.status, age_secs: s.age_secs } : { ok: true, status: s.status };
}
