import type { PluginTheme } from "@getpaseo/plugin";
import { useRpc } from "@getpaseo/plugin/client";
import { useState } from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import { send } from "../shared/rpc.ts";

export function Composer({ theme, project }: { theme: PluginTheme; project: { id: string; name: string } }) {
  const call = useRpc(send);
  const [task, setTask] = useState("");
  const [sending, setSending] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const c = theme.colors;
  const ready = task.trim() !== "" && !sending;
  const submit = async () => {
    setSending(true);
    setNote(null);
    try {
      const r = await call({ project_id: project.id, task });
      if (r.ok) {
        setNote({ ok: true, text: `Sent to ${project.name} leader (${r.leader}${r.workspace ? `, workspace ${r.workspace}` : ""})` });
        setTask("");
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
        onChangeText={setTask}
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
    </View>
  );
}
