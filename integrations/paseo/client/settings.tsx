import type { PluginTheme } from "@getpaseo/plugin";
import { useSettings, type PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useState } from "react";
import { Pressable, ScrollView, Text, TextInput, View } from "react-native";
import { DEFAULT_FETCH_URL } from "../shared/board.ts";
import { settings } from "../shared/rpc.ts";

type Values = { fetch_url: string; link_base: string; path: string };

const FIELDS: { key: keyof Values; label: string; help: string; placeholder: string }[] = [
  {
    key: "fetch_url",
    label: "Fetch URL",
    help: "Where the Paseo daemon reads the board from. Keep it the local Waggle Dance on the Paseo host.",
    placeholder: DEFAULT_FETCH_URL,
  },
  {
    key: "link_base",
    label: "Link base",
    help: "Where a row opens Waggle Dance. Empty uses the fetch URL; a phone needs a host it can reach, such as the tunnel URL.",
    placeholder: "the fetch URL",
  },
  {
    key: "path",
    label: "waggledance binary",
    help: "A full path, used to match a workspace to its project. Empty uses the waggledance that installed this plugin, or else the one in the Paseo daemon's PATH.",
    placeholder: "waggledance",
  },
];

function Field({ theme, label, help, placeholder, value, onChange }: { theme: PluginTheme; label: string; help: string; placeholder: string; value: string; onChange: (v: string) => void }) {
  const c = theme.colors;
  return (
    <View style={{ gap: 4 }}>
      <Text style={{ color: c.foreground, fontSize: 14 }}>{label}</Text>
      <Text style={{ color: c.foregroundMuted, fontSize: 12 }}>{help}</Text>
      <TextInput
        value={value}
        onChangeText={onChange}
        accessibilityLabel={label}
        placeholder={placeholder}
        placeholderTextColor={c.foregroundMuted}
        autoCapitalize="none"
        autoCorrect={false}
        style={{ color: c.foreground, fontSize: 14, borderWidth: 1, borderColor: c.border, borderRadius: 6, paddingHorizontal: 8, paddingVertical: 6 }}
      />
    </View>
  );
}

export function WaggleDanceSettings({ theme }: PluginSurfaceProps) {
  const state = useSettings(settings);
  const [draft, setDraft] = useState<Values | null>(null);
  const c = theme.colors;
  if (state.status !== "ready") {
    const loading = state.status === "loading";
    return <Text style={{ color: loading ? c.foregroundMuted : c.statusDanger, fontSize: 12, padding: 12 }}>{loading ? "Loading…" : state.error}</Text>;
  }
  const value = draft ?? state.values;
  const trimmed: Values = { fetch_url: value.fetch_url.trim(), link_base: value.link_base.trim(), path: value.path.trim() };
  const changed = FIELDS.some((f) => trimmed[f.key] !== state.values[f.key]);
  return (
    <ScrollView contentContainerStyle={{ padding: 12, gap: 12 }}>
      {FIELDS.map((f) => (
        <Field key={f.key} theme={theme} label={f.label} help={f.help} placeholder={f.placeholder} value={value[f.key]} onChange={(v) => setDraft({ ...value, [f.key]: v })} />
      ))}
      {state.saveError ? <Text style={{ color: c.statusDanger, fontSize: 12 }}>{state.saveError}</Text> : null}
      <Pressable
        accessibilityRole="button"
        disabled={!changed || state.saving}
        onPress={async () => {
          if (await state.save(trimmed, state.revision)) setDraft(null);
        }}
        style={{ alignSelf: "flex-start", paddingHorizontal: 12, paddingVertical: 6, borderRadius: 6, borderWidth: 1, borderColor: c.border, backgroundColor: changed ? c.surface2 : "transparent" }}
      >
        <Text style={{ color: changed ? c.foreground : c.foregroundMuted, fontSize: 12 }}>{state.saving ? "Saving…" : "Save"}</Text>
      </Pressable>
    </ScrollView>
  );
}
