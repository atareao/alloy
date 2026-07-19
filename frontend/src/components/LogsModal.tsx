import { TextInput, Switch, ScrollArea, Code, Group, Stack, Text, Button, Modal } from "@mantine/core";

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
      opened={opened}
      onClose={onClose}
      title={`📋 Logs: ${containerName || ""}`}
      size="xl"
      scrollAreaComponent={ScrollArea}
    >
      <Stack>
        <Group gap="sm" wrap="nowrap" align="center">
          <TextInput
            placeholder="Buscar en logs..."
            value={logSearch}
            onChange={(e) => setLogSearch(e.currentTarget.value)}
            leftSection="🔍"
            size="sm"
            style={{ flex: 1 }}
          />
          <Switch
            size="xs"
            label="Wrap"
            checked={logWrap}
            onChange={(e) => setLogWrap(e.currentTarget.checked)}
            flex="none"
          />
        </Group>
        {logSearch && (
          <Text size="xs" c="dimmed" ta="right">
            {filteredLogs.length} de {logs.length} líneas
          </Text>
        )}
        <ScrollArea
          h={500}
          style={{
            border: "1px solid var(--mantine-color-dark-4)",
            borderRadius: "var(--mantine-radius-sm)",
            backgroundColor: "#0d0d0d",
          }}
        >
          <Code
            block
            style={{
              whiteSpace: logWrap ? "pre-wrap" : "pre",
              backgroundColor: "transparent",
              color: "#e0e0e0",
              padding: "var(--mantine-spacing-sm)",
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
          </Code>
        </ScrollArea>
        <Group justify="flex-end">
          <Button variant="light" onClick={onClose}>
            Aceptar
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}