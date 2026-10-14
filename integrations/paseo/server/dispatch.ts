import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import type { Failure } from "./waggledance.ts";

export const SEND_TIMEOUT_MS = 60_000;

export type Sent = { ok: true; run_id: string; agent_id: string; workspace: string | null; leader: "started" | "reused" };

type Message = {
  id?: number;
  error?: { message?: string };
  result?: { isError?: boolean; content?: { text?: string }[]; structuredContent?: Omit<Sent, "ok"> };
};

const failed = (error: string): Failure => ({ ok: false, code: "failed", error });

function outcome(msg: Message): Sent | Failure {
  if (msg.error) return failed(msg.error.message ?? "waggledance mcp refused the call.");
  const r = msg.result;
  const text = (r?.content ?? []).map((c) => c.text ?? "").join("\n");
  if (r?.isError) return failed(text || "waggledance_dispatch failed without a message.");
  const s = r?.structuredContent;
  if (!s?.run_id) return failed(text || "waggledance_dispatch answered without a run id.");
  return { ok: true, run_id: s.run_id, agent_id: s.agent_id, workspace: s.workspace ?? null, leader: s.leader };
}

export function sendToLeader(bin: string, project: string, task: string, timeoutMs = SEND_TIMEOUT_MS): Promise<Sent | Failure> {
  return new Promise((resolve) => {
    const child = spawn(bin, ["mcp"], { stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, NO_COLOR: "1" } });
    let stderr = "";
    let done = false;
    const finish = (r: Sent | Failure) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      child.kill();
      resolve(r);
    };
    const timer = setTimeout(
      () => finish(failed(`waggledance did not answer within ${timeoutMs / 1000}s. The request may have been sent; check Runs before retrying.`)),
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
        if (msg.error) return finish(outcome(msg));
        write({ method: "notifications/initialized" });
        write({ id: 2, method: "tools/call", params: { name: "waggledance_dispatch", arguments: { project, task, leader: true } } });
      } else if (msg.id === 2) finish(outcome(msg));
    });
    write({ id: 1, method: "initialize", params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "waggledance-paseo", version: "0.0.0" } } });
  });
}
