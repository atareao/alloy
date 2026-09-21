import { Button, Card, Flex, Modal, Space, Typography } from "antd";
import type { BatchResults } from "./BatchProgress";

const { Text } = Typography;

export interface SummaryDialogProps {
  opened: boolean;
  onClose: () => void;
  checkResults: BatchResults;
  updateResults: BatchResults;
  phase: string;
}

export default function SummaryDialog({
  opened,
  onClose,
  checkResults,
  updateResults,
  phase,
}: SummaryDialogProps) {
  if (phase !== "idle") return null;

  const hasUpdates =
    updateResults.done > 0 || updateResults.failed > 0;
  const allErrors = [...checkResults.errors, ...updateResults.errors];

  return (
    <Modal
      open={opened}
      onCancel={onClose}
      title={
        hasUpdates
          ? "📋 Resumen de actualización"
          : "📋 Resumen de comprobación"
      }
      width={400}
      footer={
        <Flex justify="flex-end">
          <Button onClick={onClose}>Cerrar</Button>
        </Flex>
      }
    >
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* Check results */}
        <Card bordered size="small">
          <Text type="secondary" style={{ fontSize: 14, marginBottom: 4, display: "block" }}>
            🔍 Comprobación
          </Text>
          <Flex gap={24}>
            <Space direction="vertical" align="center" size={0}>
              <Text style={{ fontSize: 20, fontWeight: 700 }}>
                {checkResults.total}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                Comprobados
              </Text>
            </Space>
            <Space direction="vertical" align="center" size={0}>
              <Text style={{ fontSize: 20, fontWeight: 700, color: "var(--ant-color-warning)" }}>
                {checkResults.updated}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                Actualizables
              </Text>
            </Space>
            <Space direction="vertical" align="center" size={0}>
              <Text style={{ fontSize: 20, fontWeight: 700, color: "var(--ant-color-success)" }}>
                {checkResults.uptodate}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                Actuales
              </Text>
            </Space>
            <Space direction="vertical" align="center" size={0}>
              <Text style={{ fontSize: 20, fontWeight: 700, color: "var(--ant-color-error)" }}>
                {checkResults.failed}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                Errores
              </Text>
            </Space>
          </Flex>
        </Card>

        {/* Update results */}
        {hasUpdates && (
          <Card bordered size="small">
            <Text type="secondary" style={{ fontSize: 14, marginBottom: 4, display: "block" }}>
              ⬆️ Actualización
            </Text>
            <Flex gap={24}>
              <Space direction="vertical" align="center" size={0}>
                <Text style={{ fontSize: 20, fontWeight: 700, color: "var(--ant-color-success)" }}>
                  {updateResults.done}
                </Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  Actualizados
                </Text>
              </Space>
              <Space direction="vertical" align="center" size={0}>
                <Text style={{ fontSize: 20, fontWeight: 700, color: "var(--ant-color-error)" }}>
                  {updateResults.failed}
                </Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  Fallos
                </Text>
              </Space>
            </Flex>
          </Card>
        )}

        {/* Errors */}
        {allErrors.length > 0 && (
          <Card
            bordered
            size="small"
            style={{ background: "#fff2f0" }}
          >
            <Text style={{ fontSize: 12, fontWeight: 500, marginBottom: 4, display: "block", color: "var(--ant-color-error)" }}>
              Errores:
            </Text>
            {allErrors.map((err, i) => (
              <Text key={i} style={{ fontSize: 12, color: "var(--ant-color-error)" }}>
                {err}
              </Text>
            ))}
          </Card>
        )}
      </Space>
    </Modal>
  );
}