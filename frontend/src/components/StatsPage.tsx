import { useEffect, useState } from "react";
import { Stack, Paper, Group, Badge, Table, Text, Loader, Container } from "@mantine/core";
import type { ContainerStats } from "../types";
import { apiFetch } from "../api";
import { useSSE } from "../useSSE";

export default function StatsPage() {
  const [stats, setStats] = useState<ContainerStats[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch("/api/containers")
      .then((res) => res.json())
      .then(async (containers: any[]) => {
        const running = containers.filter((c) => c.state === "running" || c.state === "created");
        if (running.length === 0) { setLoading(false); return; }
        const results = await Promise.allSettled(
          running.map((c) =>
            apiFetch(`/api/stats/${encodeURIComponent(c.name)}`).then(async (r) => {
              if (!r.ok) return null;
              const data = await r.json();
              if (typeof data.cpu_percent !== "number") return null;
              return data as ContainerStats;
            }),
          ),
        );
        const initialStats: ContainerStats[] = [];
        for (const r of results) {
          if (r.status === "fulfilled" && r.value) initialStats.push(r.value);
        }
        if (initialStats.length > 0) setStats(initialStats);
        setLoading(false);
      })
      .catch(() => {});
  }, []);

  useSSE("/api/stats-events", "stats", (data) => {
    setStats(JSON.parse(data));
    setLoading(false);
  }, () => setLoading(false));

  if (loading)
    return (
      <Container py="xl">
        <Group justify="center"><Loader /><Text>Conectando...</Text></Group>
      </Container>
    );

  if (stats.length === 0)
    return (
      <Paper shadow="sm" p="xl" withBorder>
        <Text ta="center" c="dimmed">No hay containers en ejecución para mostrar estadísticas</Text>
      </Paper>
    );

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">📊 Estadísticas en vivo · Actualizado cada 3s</Text>
          <Badge size="lg" variant="light" color="green">{stats.length} containers activos</Badge>
        </Group>
      </Paper>
      <Paper shadow="sm" withBorder>
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Container</Table.Th>
              <Table.Th>CPU %</Table.Th>
              <Table.Th>Memoria (uso/límite)</Table.Th>
              <Table.Th>Red (RX / TX)</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {stats.map((s) => (
              <Table.Tr key={s.name}>
                <Table.Td><Text size="sm" fw={500}>{s.name}</Text></Table.Td>
                <Table.Td><Badge color={s.cpu_percent > 80 ? "red" : s.cpu_percent > 50 ? "yellow" : "blue"}>{s.cpu_percent.toFixed(1)}%</Badge></Table.Td>
                <Table.Td>
                  <Text size="sm">{s.memory_usage_mb.toFixed(1)} MB / {s.memory_limit_mb.toFixed(1)} MB</Text>
                  <Text size="xs" c="dimmed">({s.memory_limit_mb > 0 ? ((s.memory_usage_mb / s.memory_limit_mb) * 100).toFixed(1) : "0"}%)</Text>
                </Table.Td>
                <Table.Td>
                  <Text size="xs" c="dimmed">RX: {s.network_rx_kb.toFixed(1)} KB</Text>
                  <Text size="xs" c="dimmed">TX: {s.network_tx_kb.toFixed(1)} KB</Text>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Paper>
    </Stack>
  );
}