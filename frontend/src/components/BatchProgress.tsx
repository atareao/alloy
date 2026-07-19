import { Button, Group, Paper, Progress, Stack, Text } from "@mantine/core";

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
  updateResults: BatchResults;
  onCancel: () => void;
}

export default function BatchProgress({
  phase,
  batchProgress,
  batchCurrentItem,
  checkResults,
  updateResults,
  onCancel,
}: BatchProgressProps) {
  if (phase === "idle") return null;

  const isUpdatePhase = phase === "updating";
  const total = isUpdatePhase ? checkResults.updated : batchProgress.total;
  const pct = total > 0 ? (batchProgress.current / total) * 100 : 0;

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
                ✅ {checkResults.updated} upd · ⏹️ {checkResults.uptodate} ok
                {checkResults.failed > 0
                  ? ` · ❌ ${checkResults.failed}`
                  : ""}
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
              ✅ {updateResults.done} hechos
              {updateResults.failed > 0
                ? ` · ❌ ${updateResults.failed} errores`
                : ""}
            </Text>
          )}
        </Group>
      </Stack>
    </Paper>
  );
}
