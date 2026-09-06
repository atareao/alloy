import { Button, Group, Paper, Progress, ScrollArea, Stack, Text } from "@mantine/core";
import type { UpdateProgress } from "../types";

export interface BatchResults {
  total: number;
  updated: number;
  uptodate: number;
  failed: number;
  done: number;
  errors: string[];
}

export interface BatchProgressProps {
  phase: "idle" | "checking" | "updating";
  batchProgress: { current: number; total: number };
  batchCurrentItem: string;
  checkResults: BatchResults;
  progress: Map<string, UpdateProgress>;
  onCancel: () => void;
}

export default function BatchProgress({
  phase,
  batchProgress,
  batchCurrentItem,
  checkResults,
  progress,
  onCancel,
}: BatchProgressProps) {
  if (phase === "idle") return null;

  const isUpdatePhase = phase === "updating";
  const total = isUpdatePhase ? checkResults.updated : batchProgress.total;
  const pct = total > 0 ? (batchProgress.current / total) * 100 : 0;

  // Build a sorted list of progress entries for the live log
  const logEntries = Array.from(progress.entries())
    .filter(([_, p]) => {
      // In checking phase, show all; in updating phase, show only update-related
      if (isUpdatePhase) {
        return (
          p.status.startsWith("🔄") ||
          p.status.startsWith("✅") ||
          p.status.startsWith("❌") ||
          p.status.startsWith("⚠️") ||
          p.status.startsWith("📥") ||
          p.status.startsWith("⬇️") ||
          p.status.startsWith("⏭️")
        );
      }
      return true;
    })
    .sort(([a], [b]) => a.localeCompare(b));

  const logColor = (p: UpdateProgress) => {
    if (!p.done) return "yellow";
    if (p.error) return "red";
    return "green";
  };

  const logEmoji = (p: UpdateProgress) => {
    if (!p.done) return "🔄";
    if (p.error) return "❌";
    return "✅";
  };

  // Compute live counts from progress map
  const liveDone = Array.from(progress.values()).filter(p => p.done && !p.error).length;
  const liveFailed = Array.from(progress.values()).filter(p => p.done && p.error).length;
  const livePending = Array.from(progress.values()).filter(p => !p.done).length;

  return (
    <Paper shadow="sm" p="md" mb="md" withBorder>
      <Stack gap="xs">
        <Group justify="space-between">
          <Text size="sm" fw={500}>
            {isUpdatePhase
              ? "⬆️ Actualizando containers..."
              : "🔍 Comprobando actualizaciones..."}
          </Text>
          <Group gap="xs">
            {!isUpdatePhase && (
              <Text size="xs" c="dimmed" mr="sm">
                ✅ {liveDone} ok{liveFailed > 0 ? ` · ❌ ${liveFailed}` : ""}{livePending > 0 ? ` · 🔄 ${livePending}` : ""}
              </Text>
            )}
            <Button
              size="xs"
              color="red"
              variant="outline"
              onClick={onCancel}
            >
              Cancelar
            </Button>
          </Group>
        </Group>

        <Progress
          value={pct}
          animated
          color={isUpdatePhase ? "yellow" : "cyan"}
        />

        <Group justify="space-between">
          <Text size="xs" c="dimmed">
            {batchProgress.current} / {total} —{" "}
            {batchCurrentItem || "iniciando..."}
          </Text>
          {isUpdatePhase && (
            <Text size="xs" c="dimmed">
              ✅ {liveDone} hechos{liveFailed > 0 ? ` · ❌ ${liveFailed} errores` : ""}{livePending > 0 ? ` · 🔄 ${livePending}` : ""}
            </Text>
          )}
        </Group>

        {/* Live log of container statuses */}
        <Text size="xs" c="dimmed">
          📊 Progreso: {progress.size} entradas · {logEntries.length} visibles
        </Text>
        {logEntries.length > 0 && (
          <ScrollArea h={180} type="always" offsetScrollbars>
            <Stack gap={2}>
              {logEntries.map(([name, p]) => (
                <Paper
                  key={name}
                  p="xs"
                  withBorder={false}
                  style={{
                    background: p.done
                      ? p.error
                        ? "var(--mantine-color-red-0)"
                        : "var(--mantine-color-green-0)"
                      : "var(--mantine-color-yellow-0)",
                    borderLeft: `3px solid var(--mantine-color-${logColor(p)}-6)`,
                  }}
                >
                  <Group gap="sm" wrap="nowrap">
                    <Text size="xs" fw={500} style={{ minWidth: 120 }} truncate>
                      {logEmoji(p)} {name}
                    </Text>
                    <Text size="xs" c="dimmed" truncate>
                      {p.status}
                    </Text>
                  </Group>
                </Paper>
              ))}
            </Stack>
          </ScrollArea>
        )}
      </Stack>
    </Paper>
  );
}
