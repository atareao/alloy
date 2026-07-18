import { useState } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  Badge,
  Button,
  Group,
  Modal,
  Paper,
  Stack,
  Table,
  Text,
  Divider,
} from "@mantine/core";
import { apiFetch } from "./api";

// ═══════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════

interface HistoryEntry {
  container: string;
  image: string;
  old_digest: string;
  new_digest: string;
  timestamp: string;
  status: string;
  duration_ms: number;
}

// ═══════════════════════════════════════════════════════════════
// Page: History (histórico de updates)
// ═══════════════════════════════════════════════════════════════

interface HistoryPageProps {
  history: HistoryEntry[];
  setHistory: (h: HistoryEntry[]) => void;
}

export default function HistoryPage({ history, setHistory }: HistoryPageProps) {
  const isMobile = useMediaQuery("(max-width: 768px)");
  const [clearing, setClearing] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const handleClear = async () => {
    setClearing(true);
    try {
      await apiFetch("/api/history", { method: "DELETE" });
      setHistory([]);
    } catch {
      /* ignore */
    }
    setClearing(false);
    setConfirmClear(false);
  };

  const statusColor = (status: string) => {
    switch (status) {
      case "success":
        return "green";
      case "failed":
        return "red";
      case "skipped":
        return "yellow";
      default:
        return "gray";
    }
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
  };

  const formatDate = (ts: string) => {
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
  };

  const shortDigest = (d: string | undefined) => {
    if (!d) return "-";
    return d.length > 20 ? d.substring(0, 20) + "..." : d;
  };

  // ── Mobile card ─────────────────────────────────────────────
  const renderMobileCard = (entry: HistoryEntry, i: number) => (
    <Paper key={i} shadow="sm" p="sm" withBorder>
      <Stack gap="xs">
        <Group justify="space-between" wrap="nowrap">
          <Text size="sm" fw={500} truncate style={{ flex: 1 }}>
            {entry.container}
          </Text>
          <Badge size="sm" color={statusColor(entry.status)}>
            {entry.status}
          </Badge>
        </Group>
        <Divider />
        <Stack gap={2}>
          <Group gap="xs">
            <Text size="xs" c="dimmed">
              Imagen:
            </Text>
            <Text size="xs">{entry.image}</Text>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">
              Anterior:
            </Text>
            <Text size="xs" style={{ fontFamily: "monospace" }}>
              {shortDigest(entry.old_digest)}
            </Text>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">
              Nueva:
            </Text>
            <Text size="xs" style={{ fontFamily: "monospace" }}>
              {shortDigest(entry.new_digest)}
            </Text>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">
              Duración:
            </Text>
            <Text size="xs">{formatDuration(entry.duration_ms)}</Text>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">
              Fecha:
            </Text>
            <Text size="xs">{formatDate(entry.timestamp)}</Text>
          </Group>
        </Stack>
      </Stack>
    </Paper>
  );

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            📜 Histórico de actualizaciones · {history.length} entradas
          </Text>
          {history.length > 0 && (
            <Button
              onClick={() => setConfirmClear(true)}
              variant="filled"
              color="red"
              size={isMobile ? "xs" : "sm"}
            >
              🗑️ Limpiar
            </Button>
          )}
        </Group>
      </Paper>

      <Modal
        opened={confirmClear}
        onClose={() => setConfirmClear(false)}
        title="🗑️ Limpiar historial"
        size="sm"
      >
        <Text size="sm" mb="md">
          ¿Estás seguro de que deseas eliminar todo el historial de
          actualizaciones? Esta acción no se puede deshacer.
        </Text>
        <Group justify="flex-end">
          <Button variant="default" onClick={() => setConfirmClear(false)}>
            Cancelar
          </Button>
          <Button color="red" onClick={handleClear} loading={clearing}>
            Eliminar todo
          </Button>
        </Group>
      </Modal>

      {history.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay historial de actualizaciones. Cuando se actualice un
            container, aparecerá aquí.
          </Text>
        </Paper>
      ) : isMobile ? (
        <Stack gap="sm">
          {history.map((entry, i) => renderMobileCard(entry, i))}
        </Stack>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table.ScrollContainer minWidth={700}>
            <Table striped highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Container</Table.Th>
                  <Table.Th>Imagen</Table.Th>
                  <Table.Th>Versión anterior</Table.Th>
                  <Table.Th>Nueva versión</Table.Th>
                  <Table.Th>Estado</Table.Th>
                  <Table.Th>Duración</Table.Th>
                  <Table.Th>Fecha</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {history.map((entry, i) => (
                  <Table.Tr key={i}>
                    <Table.Td>
                      <Text size="sm" fw={500}>
                        {entry.container}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="xs" c="dimmed">
                        {entry.image}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text
                        size="xs"
                        c="dimmed"
                        style={{ fontFamily: "monospace" }}
                      >
                        {shortDigest(entry.old_digest)}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text
                        size="xs"
                        c="dimmed"
                        style={{ fontFamily: "monospace" }}
                      >
                        {shortDigest(entry.new_digest)}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Badge color={statusColor(entry.status)}>
                        {entry.status}
                      </Badge>
                    </Table.Td>
                    <Table.Td>
                      <Text size="xs" c="dimmed">
                        {formatDuration(entry.duration_ms)}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="xs" c="dimmed">
                        {formatDate(entry.timestamp)}
                      </Text>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </Table.ScrollContainer>
        </Paper>
      )}
    </Stack>
  );
}
