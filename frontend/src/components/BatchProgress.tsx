import { Button, Card, Flex, Progress, Space, Typography } from "antd";
import type { BatchProgress as BatchProgressType } from "../types";

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
  progress: BatchProgressType;
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

  // Use BatchProgress counters directly
  const isBatchComplete =
    progress.total > 0 && progress.checked >= progress.total;
  const checked = progress.checked;
  const updatedCount = progress.updated;
  const errorsCount = progress.errors;
  const pending = progress.total > 0 ? progress.total - progress.checked : 0;

  const currentText = isBatchComplete
    ? "✅ Completado"
    : progress.checking && progress.checking !== "__batch__"
      ? `🔍 ${progress.checking}`
      : "🔍 Verificando...";

  return (
    <Card bordered style={{ marginBottom: 16, padding: 16 }}>
      <Space direction="vertical" size="small" style={{ width: "100%" }}>
        <Flex justify="space-between" align="center">
          <Text style={{ fontSize: 14, fontWeight: 500 }}>
            🔄 Revisando y actualizando containers...
          </Text>
          <Flex gap={4} align="center">
            <Button size="small" danger onClick={onCancel}>
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
            {progress.checked} / {progress.total} — {currentText}
          </Text>
        </Flex>

        {/* Live summary from backend counters */}
        <Flex justify="space-between" align="center" style={{ marginTop: 4 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            Total: {progress.total} containers
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
