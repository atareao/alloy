import { useEffect, useState } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  Accordion, Badge, Button, Code, Container, Group, Loader, Modal,
  Paper, ScrollArea, Stack, Table, Text, Title,
} from "@mantine/core";
import type { StackInfo, StackLogs } from "../types";
import { apiFetch } from "../api";

function statusColor(state: string): string {
  switch (state) {
    case "running": return "green";
    case "exited": return "red";
    case "paused": return "yellow";
    default: return "gray";
  }
}

export default function StacksPage() {
  const isMobile = useMediaQuery("(max-width: 768px)");
  const [stacks, setStacks] = useState<StackInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [logsModal, setLogsModal] = useState<{ project: string; logs: StackLogs | null } | null>(null);
  const [logsLoading, setLogsLoading] = useState(false);

  const fetchStacks = () => {
    apiFetch("/api/stacks")
      .then((res) => res.json())
      .then((data) => { setStacks(data); setLoading(false); })
      .catch(() => setLoading(false));
  };

  useEffect(() => { fetchStacks(); }, []);

  const doAction = async (project: string, action: "update" | "down") => {
    setActionLoading(`${project}/${action}`);
    try {
      await apiFetch(`/api/stacks/${encodeURIComponent(project)}/${action}`, { method: "POST" });
      fetchStacks();
    } catch {
      // ignore
    } finally {
      setActionLoading(null);
    }
  };

  const openLogs = async (project: string) => {
    setLogsModal({ project, logs: null });
    setLogsLoading(true);
    try {
      const res = await apiFetch(`/api/stacks/${encodeURIComponent(project)}/logs`);
      const data: StackLogs = await res.json();
      setLogsModal({ project, logs: data });
    } catch {
      setLogsModal({ project, logs: null });
    } finally {
      setLogsLoading(false);
    }
  };

  if (loading) {
    return (
      <Container py="xl">
        <Group justify="center">
          <Loader />
          <Text>Cargando stacks...</Text>
        </Group>
      </Container>
    );
  }

  if (stacks.length === 0) {
    return (
      <Paper shadow="sm" p="xl" withBorder>
        <Text ta="center" c="dimmed">No hay stacks Docker Compose disponibles</Text>
      </Paper>
    );
  }

  return (
    <>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between" wrap="wrap">
          <Title order={4}>🧩 Stacks Docker Compose</Title>
          <Badge size="lg" variant="light" color="cyan">{stacks.length} stacks</Badge>
        </Group>
      </Paper>

      <Accordion variant="separated">
        {stacks.map((stack) => (
          <Accordion.Item key={stack.project} value={stack.project}>
            <Accordion.Control>
              <Group justify="space-between" wrap="wrap">
                <Text fw={500}>{stack.project}</Text>
                <Group gap="xs">
                  <Badge
                    size="sm"
                    variant="light"
                    color={stack.services.some((s) => s.state === "running") ? "green" : "red"}
                  >
                    {stack.services.filter((s) => s.state === "running").length}/{stack.services.length}
                  </Badge>
                </Group>
              </Group>
            </Accordion.Control>
            <Accordion.Panel>
              <Stack gap="sm">
                <Group gap="xs" wrap="wrap">
                  <Button
                    size="xs"
                    variant="light"
                    color="blue"
                    loading={actionLoading === `${stack.project}/update`}
                    onClick={() => doAction(stack.project, "update")}
                  >
                    🔄 Update
                  </Button>
                  <Button
                    size="xs"
                    variant="light"
                    color="orange"
                    loading={actionLoading === `${stack.project}/down`}
                    onClick={() => doAction(stack.project, "down")}
                  >
                    ⏹ Down
                  </Button>
                  <Button
                    size="xs"
                    variant="light"
                    color="gray"
                    onClick={() => openLogs(stack.project)}
                  >
                    📋 Logs
                  </Button>
                </Group>

                {isMobile ? (
                  <Stack gap="xs">
                    {stack.services.map((svc) => (
                      <Paper key={svc.service} shadow="xs" p="xs" withBorder>
                        <Group justify="space-between" wrap="nowrap">
                          <Text size="sm" fw={500}>{svc.service}</Text>
                          <Badge size="sm" variant="light" color={statusColor(svc.state)}>{svc.state}</Badge>
                        </Group>
                        <Text size="xs" c="dimmed">{svc.container_name}</Text>
                        <Text size="xs" c="dimmed"><Code>{svc.image}</Code></Text>
                        {svc.status && <Text size="xs" c="dimmed">{svc.status}</Text>}
                      </Paper>
                    ))}
                  </Stack>
                ) : (
                  <Table.ScrollContainer minWidth={600}>
                    <Table striped highlightOnHover>
                      <Table.Thead>
                        <Table.Tr>
                          <Table.Th>Servicio</Table.Th>
                          <Table.Th>Contenedor</Table.Th>
                          <Table.Th>Imagen</Table.Th>
                          <Table.Th>Estado</Table.Th>
                          <Table.Th>Status</Table.Th>
                        </Table.Tr>
                      </Table.Thead>
                      <Table.Tbody>
                        {stack.services.map((svc) => (
                          <Table.Tr key={svc.service}>
                            <Table.Td><Text fw={500} size="sm">{svc.service}</Text></Table.Td>
                            <Table.Td><Text size="sm">{svc.container_name}</Text></Table.Td>
                            <Table.Td><Code>{svc.image}</Code></Table.Td>
                            <Table.Td>
                              <Badge size="sm" variant="light" color={statusColor(svc.state)}>{svc.state}</Badge>
                            </Table.Td>
                            <Table.Td><Text size="xs" c="dimmed">{svc.status}</Text></Table.Td>
                          </Table.Tr>
                        ))}
                      </Table.Tbody>
                    </Table>
                  </Table.ScrollContainer>
                )}
              </Stack>
            </Accordion.Panel>
          </Accordion.Item>
        ))}
      </Accordion>

      <Modal
        opened={!!logsModal}
        onClose={() => setLogsModal(null)}
        title={`📋 Logs: ${logsModal?.project || ""}`}
        size="xl"
        scrollAreaComponent={ScrollArea.Autosize}
      >
        {logsLoading ? (
          <Group justify="center" py="md">
            <Loader />
            <Text>Cargando logs...</Text>
          </Group>
        ) : logsModal?.logs ? (
          <Stack gap="md">
            {logsModal.logs.services.length === 0 && (
              <Text c="dimmed" ta="center">No hay servicios con logs disponibles</Text>
            )}
            {logsModal.logs.services.map((svc) => (
              <Paper key={svc.service} shadow="xs" p="sm" withBorder>
                <Group justify="space-between" mb="xs">
                  <Text fw={500} size="sm">{svc.service}</Text>
                  <Text size="xs" c="dimmed">{svc.container}</Text>
                </Group>
                {svc.lines.length === 0 ? (
                  <Text size="xs" c="dimmed" fs="italic">Sin logs</Text>
                ) : (
                  <Stack gap={2}>
                    {svc.lines.map((line, i) => (
                      <Text key={i} size="xs" style={{ fontFamily: "monospace", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
                        {line}
                      </Text>
                    ))}
                  </Stack>
                )}
              </Paper>
            ))}
          </Stack>
        ) : (
          <Text c="red">Error al cargar logs</Text>
        )}
      </Modal>
    </>
  );
}