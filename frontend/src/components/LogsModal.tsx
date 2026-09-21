import { Button, Flex, Input, Modal, Switch, Typography } from "antd";
import { SearchOutlined } from "@ant-design/icons";

const { Text } = Typography;

interface LogsModalProps {
  opened: boolean;
  onClose: () => void;
  containerName: string | null;
  logs: string[];
  logSearch: string;
  setLogSearch: (s: string) => void;
  logWrap: boolean;
  setLogWrap: (w: boolean) => void;
  logError: string | null;
  logTimeout: boolean;
}

export default function LogsModal({
  opened,
  onClose,
  containerName,
  logs,
  logSearch,
  setLogSearch,
  logWrap,
  setLogWrap,
  logError,
  logTimeout,
}: LogsModalProps) {
  const filteredLogs = logSearch
    ? logs.filter((l) => l.toLowerCase().includes(logSearch.toLowerCase()))
    : logs;

  return (
    <Modal
      open={opened}
      onCancel={onClose}
      title={`📋 Logs: ${containerName || ""}`}
      width={960}
      footer={
        <Flex justify="flex-end">
          <Button onClick={onClose}>Aceptar</Button>
        </Flex>
      }
    >
      <Flex gap={8} wrap="nowrap" align="center" style={{ marginBottom: 8 }}>
        <Input
          placeholder="Buscar en logs..."
          value={logSearch}
          onChange={(e) => setLogSearch(e.target.value)}
          prefix={<SearchOutlined />}
          size="small"
          allowClear
          style={{ flex: 1 }}
        />
        <Switch
          size="small"
          checkedChildren="Wrap"
          unCheckedChildren="Wrap"
          checked={logWrap}
          onChange={(checked) => setLogWrap(checked)}
          style={{ flex: "none" }}
        />
      </Flex>
      {logSearch && (
        <Text
          type="secondary"
          style={{
            fontSize: 12,
            textAlign: "right",
            display: "block",
            marginBottom: 4,
          }}
        >
          {filteredLogs.length} de {logs.length} líneas
        </Text>
      )}
      <div
        style={{
          maxHeight: 500,
          overflow: "auto",
          border: "1px solid var(--ant-color-border)",
          borderRadius: "var(--ant-border-radius-sm, 6px)",
          backgroundColor: "#0d0d0d",
        }}
      >
        <pre
          style={{
            whiteSpace: logWrap ? "pre-wrap" : "pre",
            backgroundColor: "transparent",
            color: "#e0e0e0",
            padding: 12,
            margin: 0,
            fontFamily:
              "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
          }}
        >
          {filteredLogs.length > 0
            ? filteredLogs.join("")
            : logError
              ? `❌ ${logError}`
              : logTimeout
                ? "No se recibieron logs (el contenedor puede no existir o estar detenido)"
                : logs.length > 0
                  ? "Sin resultados"
                  : "Esperando logs..."}
        </pre>
      </div>
    </Modal>
  );
}
