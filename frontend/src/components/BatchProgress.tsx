import { Button, Card, Flex, Progress, Space, Typography } from "antd";
import type { UpdateProgress } from "../types";

const { Text } = Typography;

export interface BatchResults {
  total: number;
  updated: number;
  uptodate: number;
  failed: number;
  done: number;
  errors: string[];
}

export interface BatchProgressProps {
  phase: "idle" | "active";
  batchProgress: { current: number; total: number };
  progress: Map<string, UpdateProgress>;
  onCancel: () => void;
}

export default function BatchProgress({
  phase,
  batchProgress,
  progress,
  onCancel,
}: BatchProgressProps) {
  if (phase === "idle") return null;

  const total = batchProgress.total;
  const current = batchProgress.current;
  const pct = total > 0 ? (current / total) * 100 : 0;

  // Determine if batch is complete based on counters, not per-container done flag
  const isBatchComplete = total > 0 && current >= total;

  // Find the first non-done entry for current status text
  const entries = Array.from(progress.values());
  const currentEntry = entries.find(p => !p.done);

  // Use latest entry for aggregate counters (checked, updated, errors)
  const latestProgress = entries.length > 0 ? entries[entries.length - 1] : null;
  const checked = latestProgress !== null && latestProgress !== undefined
    ? latestProgress.checked
    : current;
  const updatedCount = latestProgress?.updated ?? 0;
  const errorsCount = latestProgress?.errors ?? 0;
  const pending = total > 0 ? total - checked : 0;

  const currentText = isBatchComplete
    ? "✅ Completado"
    : currentEntry
      ? currentEntry.status
      : "🔍 Verificando...";

  return (
    <Card bordered style={{ marginBottom: 16, padding: 16 }}>
      <Space direction="vertical" size="small" style={{ width: "100%" }}>
        <Flex justify="space-between" align="center">
          <Text style={{ fontSize: 14, fontWeight: 500 }}>
            🔄 Revisando y actualizando containers...
          </Text>
          <Flex gap={4} align="center">
            <Button
              size="small"
              danger
              onClick={onCancel}
            >
              Cancelar
            </Button>
          </Flex>
        </Flex>

        <Progress
          percent={Math.round(pct)}
          strokeColor="#13c2c2"
          showInfo={false}
        />

        <Flex justify="space-between" align="center">
          <Text type="secondary" style={{ fontSize: 12 }}>
            {batchProgress.current} / {total} — {currentText}
          </Text>
        </Flex>

        {/* Live summary from backend counters */}
        <Flex justify="space-between" align="center" style={{ marginTop: 4 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            Total: {total} containers
          </Text>
          <Space size="small">
            <Text type="secondary" style={{ fontSize: 12 }}>
              ✅ {checked} revisados
            </Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              ⬆️ {updatedCount} actualizados
            </Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              ❌ {errorsCount} errores
            </Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              ⏸️ {pending} pendientes
            </Text>
          </Space>
        </Flex>
      </Space>
    </Card>
  );
}