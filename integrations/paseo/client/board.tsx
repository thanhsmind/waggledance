import type { PluginTheme } from "@getpaseo/plugin";
import { openExternalUrl, useRpc, useWorkspace, type PluginSurfaceProps, type PluginWorkspacePanelProps } from "@getpaseo/plugin/client";
import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { Pressable, ScrollView, Text, View } from "react-native";
import { agentMeta, featureMeta, linkTo, pollEvery, sections } from "../shared/board.ts";
import { board } from "../shared/rpc.ts";
import { Composer } from "./composer.tsx";

function useBoard(dir: string | undefined, enabled: boolean) {
  const call = useRpc(board);
  return useQuery({
    queryKey: ["waggledance.board", dir ?? ""],
    enabled,
    retry: false,
    queryFn: async () => {
      const r = await call({ dir });
      if (!r.ok) throw new Error(r.error);
      return r;
    },
    refetchInterval: (q) => pollEvery(q.state.data?.board, q.state.data?.agents),
  });
}

type Query = ReturnType<typeof useBoard>;

function Line({ theme, children, tone = "foregroundMuted" }: { theme: PluginTheme; children: ReactNode; tone?: keyof PluginTheme["colors"] }) {
  return <Text style={{ color: theme.colors[tone], fontSize: 12 }}>{children}</Text>;
}

function Row({ theme, title, meta, tone, href }: { theme: PluginTheme; title: string; meta: string; tone?: keyof PluginTheme["colors"]; href: string }) {
  return (
    <Pressable
      accessibilityRole="link"
      accessibilityLabel={`Open ${title} in Waggle Dance`}
      onPress={() => void openExternalUrl(href)}
      style={{ paddingVertical: 8, borderBottomWidth: 1, borderColor: theme.colors.border, gap: 2 }}
    >
      <Text style={{ color: theme.colors.foreground, fontSize: 14 }} numberOfLines={1}>
        {title}
      </Text>
      <Line theme={theme} tone={tone}>
        {meta}
      </Line>
    </Pressable>
  );
}

function Section({ theme, title, children }: { theme: PluginTheme; title: string; children: ReactNode }) {
  return (
    <View style={{ gap: 4 }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13, fontWeight: "600" }}>{title}</Text>
      {children}
    </View>
  );
}

function BoardView({ theme, compact, q }: { theme: PluginTheme; compact: boolean; q: Query }) {
  const pad = compact ? 12 : 20;
  const frame = (body: ReactNode) => (
    <ScrollView style={{ flex: 1, backgroundColor: theme.colors.surface0 }} contentContainerStyle={{ padding: pad, gap: 16 }}>
      {body}
    </ScrollView>
  );
  const d = q.data;
  if (!d) return frame(<Line theme={theme} tone={q.error ? "statusDanger" : "foregroundMuted"}>{q.error ? q.error.message : "Loading…"}</Line>);
  const stale = q.isError ? (
    <Line theme={theme} tone="statusWarning">
      {`Stale: showing the last board read. ${q.error?.message ?? ""}`}
    </Line>
  ) : null;
  if (d.project === null) {
    return frame(
      <>
        {stale}
        <Line theme={theme}>This workspace is not a Waggle Dance project</Line>
      </>,
    );
  }
  const s = sections(d.board, d.agents);
  const href = (url: string) => linkTo(d.link_base, url);
  return frame(
    <>
      {stale}
      {d.project ? <Composer theme={theme} project={d.project} /> : null}
      {s.waiting.length + s.progress.length === 0 ? <Line theme={theme}>Nothing in progress</Line> : null}
      {s.waiting.length ? (
        <Section theme={theme} title="Waiting on you">
          {s.waiting.map((f) => (
            <Row key={`${f.project_id}/${f.feature}`} theme={theme} title={f.title} meta={featureMeta(f)} tone="statusWarning" href={href(f.url)} />
          ))}
        </Section>
      ) : null}
      {s.progress.length ? (
        <Section theme={theme} title="In progress">
          {s.progress.map((f) => (
            <Row key={`${f.project_id}/${f.feature}`} theme={theme} title={f.title} meta={featureMeta(f)} href={href(f.url)} />
          ))}
        </Section>
      ) : null}
      <Section theme={theme} title="Live agents">
        {s.agents.length ? (
          s.agents.map((a) => <Row key={a.pane_id} theme={theme} title={a.name} meta={agentMeta(a)} tone={a.status === "working" ? "statusSuccess" : undefined} href={href(a.url)} />)
        ) : (
          <Line theme={theme}>{d.agents_note ?? "No live agents"}</Line>
        )}
      </Section>
    </>,
  );
}

export function BoardSurface({ theme, layout }: PluginSurfaceProps) {
  return <BoardView theme={theme} compact={layout.compact} q={useBoard(undefined, true)} />;
}

export function BoardPanel({ theme, layout, workspaceId }: PluginWorkspacePanelProps) {
  const dir = useWorkspace(workspaceId, (w) => w.directory);
  return <BoardView theme={theme} compact={layout.compact} q={useBoard(dir ?? undefined, dir !== null)} />;
}
