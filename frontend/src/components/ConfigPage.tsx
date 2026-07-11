import { useEffect, useState } from "react";
import { Stack, Paper, Group, Badge, Text, Loader, Title, SimpleGrid, Divider } from "@mantine/core";
import type { AppConfig, DockerInfo } from "../types";
import { apiFetch } from "../api";

export default function ConfigPage() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [dockerInfo, setDockerInfo] = useState<DockerInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      apiFetch("/api/config").then((r) => r.json()),
      apiFetch("/api/docker-info").then((r) => r.json()).catch(() => null),
    ])
      .then(([cfg, info]) => { setConfig(cfg); setDockerInfo(info); })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  if (loading) return (<Group justify="center" py="xl"><Loader /></Group>);

  return (
    <Stack>
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">🐳 Información del Daemon</Title>
        {dockerInfo ? (
          <Stack>
            <SimpleGrid cols={{ base: 1, sm: 2 }}>
              <div><Text size="xs" c="dimmed">Versión</Text><Text size="sm" fw={500}>{dockerInfo.version}</Text></div>
              <div><Text size="xs" c="dimmed">Sistema Operativo</Text><Text size="sm" fw={500}>{dockerInfo.os}</Text></div>
              <div><Text size="xs" c="dimmed">Arquitectura</Text><Text size="sm" fw={500}>{dockerInfo.arch}</Text></div>
              <div><Text size="xs" c="dimmed">Imágenes</Text><Text size="sm" fw={500}>{dockerInfo.images}</Text></div>
            </SimpleGrid>
            <Divider />
            <Text size="sm" fw={500} mb="xs">Containers</Text>
            <SimpleGrid cols={{ base: 2, sm: 4 }}>
              <div><Text size="xs" c="dimmed">Total</Text><Text size="sm" fw={500}>{dockerInfo.containers_total}</Text></div>
              <div><Text size="xs" c="dimmed">Running</Text><Badge color="green">{dockerInfo.containers_running}</Badge></div>
              <div><Text size="xs" c="dimmed">Paused</Text><Badge color="yellow">{dockerInfo.containers_paused}</Badge></div>
              <div><Text size="xs" c="dimmed">Stopped</Text><Badge color="gray">{dockerInfo.containers_stopped}</Badge></div>
            </SimpleGrid>
          </Stack>
        ) : (
          <Text size="sm" c="dimmed">No se pudo obtener información del daemon</Text>
        )}
      </Paper>
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">🔐 Autenticación</Title>
        <Text size="sm">JWT configurado: <Badge color="green">✅</Badge></Text>
        <Text size="xs" c="dimmed" mt="xs">Usa el token para autenticarte en todas las peticiones</Text>
      </Paper>
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">📡 Conexiones</Title>
        <Group><Text size="sm">Telegram:</Text><Badge color={config?.telegram_configured ? "green" : "gray"}>{config?.telegram_configured ? "✅ Conectado" : "❌ No configurado"}</Badge></Group>
        <Group mt="xs"><Text size="sm">Matrix:</Text><Badge color={config?.matrix_configured ? "green" : "gray"}>{config?.matrix_configured ? "✅ Conectado" : "❌ No configurado"}</Badge></Group>
      </Paper>
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">🤖 Auto-update</Title>
        <Group><Text size="sm">Estado:</Text><Badge color={config?.auto_update_enabled ? "green" : "gray"}>{config?.auto_update_enabled ? "✅ Activado" : "❌ Desactivado"}</Badge></Group>
        {config?.auto_update_enabled && <Text size="sm" mt="xs">Intervalo: cada <b>{config?.auto_update_interval_hours}h</b></Text>}
      </Paper>
      <Paper shadow="sm" p="md" withBorder>
        <Title order={4} mb="md">⚙️ General</Title>
        <Text size="sm">Puerto: <b>{config?.port}</b></Text>
        {config?.allowed_containers ? (
          <Text size="sm" mt="xs">Containers monitorizados: <b>{config.allowed_containers.join(", ")}</b></Text>
        ) : (
          <Text size="sm" mt="xs">Containers monitorizados: <b>Todos</b></Text>
        )}
      </Paper>
    </Stack>
  );
}