import type { PluginTheme } from "@getpaseo/plugin";
import { useRpc } from "@getpaseo/plugin/client";
import { useEffect, useState } from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import { describeRun, describeSent, runStatus, send, STATUS_POLL_MS } from "../shared/rpc.ts";

const newRequestId = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;

export function Composer({ theme, project }: { theme: PluginTheme; project: { id: string; name: string } }) {
  const call = useRpc(send);
  const readStatus = useRpc(runStatus);
  const [task, setTask] = useState("");
  const [requestId, setRequestId] = useState(newRequestId);
  const [runId, setRunId] = useState<string | null>(null);
  const [runText, setRunText] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const c = theme.colors;
  const ready = task.trim() !== "" && !sending;
  useEffect(() => {
    if (!runId) return;
    let stopped = false;
    const tick = async () => {
      try {
        const r = await readStatus({ run_id: runId });
        if (stopped) return;
        if (!r.ok) return setRunText(r.error);
        const d = describeRun(r.status, r.age_secs);
        setRunText(`Run ${runId}: ${d.text}`);
        if (d.final) stop();
      } catch (error) {
        if (!stopped) setRunText(error instanceof Error ? error.message : String(error));
      }
    };
    const timer = setInterval(() => void tick(), STATUS_POLL_MS);
    const stop = () => {
      stopped = true;
      clearInterval(timer);
    };
    void tick();
    return stop;
  }, [runId, readStatus]);
  const edit = (text: string) => {
    setTask(text);
    setRequestId(newRequestId());
  };
  const submit = async () => {
    setSending(true);
    setNote(null);
    try {
      const r = await call({ project_id: project.id, task, request_id: requestId });
      if (r.ok) {
        setNote({ ok: true, text: describeSent(project.name, r) });
        setTask("");
        setRequestId(newRequestId());
        setRunText(null);
        setRunId(r.run_id);
      } else setNote({ ok: false, text: r.error });
    } catch (error) {
      setNote({ ok: false, text: error instanceof Error ? error.message : String(error) });
    } finally {
      setSending(false);
    }
  };
  return (
    <View style={{ gap: 6 }}>
      <TextInput
        value={task}
        onChangeText={edit}
        multiline
        accessibilityLabel={`Request for the ${project.name} leader`}
        placeholder={`Ask the ${project.name} leader…`}
        placeholderTextColor={c.foregroundMuted}
        style={{ color: c.foreground, fontSize: 14, minHeight: 64, textAlignVertical: "top", borderWidth: 1, borderColor: c.border, borderRadius: 6, paddingHorizontal: 8, paddingVertical: 6 }}
      />
      <Pressable
        accessibilityRole="button"
        disabled={!ready}
        onPress={() => void submit()}
        style={{ alignSelf: "flex-start", paddingHorizontal: 12, paddingVertical: 6, borderRadius: 6, borderWidth: 1, borderColor: c.border, backgroundColor: ready ? c.surface2 : "transparent" }}
      >
        <Text style={{ color: ready ? c.foreground : c.foregroundMuted, fontSize: 12 }}>{sending ? "Sending…" : "Send"}</Text>
      </Pressable>
      {note ? <Text style={{ color: note.ok ? c.statusSuccess : c.statusDanger, fontSize: 12 }}>{note.text}</Text> : null}
      {runText ? <Text style={{ color: c.foregroundMuted, fontSize: 12 }}>{runText}</Text> : null}
    </View>
  );
}
