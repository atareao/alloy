import { useEffect, useState, useRef } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  ActionIcon, Badge, Button, Container, Group, Loader, Menu, Paper, Table, Text,
  Title, Tooltip, Code, Stack, Modal, Anchor, Tabs, ScrollArea, Progress, Divider,
  SimpleGrid,
} from "@mantine/core";
import type { ContainerInfo, UpdateProgress, NotifEvent, InspectData } from "../types";
import { apiFetch } from "../api";
import NotifToast from "./NotifToast";

export default function DashboardPage() {
  const [containers, setContainers] = useState<ContainerInfo[]>([]);
  const [progress, setProgress] = useState<Map<string, UpdateProgress>>(new Map());
  const [notifications, setNotifications] = useState<NotifEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [updating, setUpdating] = useState<string | null>(null);
  const [inspectName, setInspectName] = useState<string | null>(null);
  const [inspectData, setInspectData] = useState<InspectData | null>(null);
  const [inspectLoading, setInspectLoading] = useState(false);
  const [inspectError, setInspectError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [actionNotif, setActionNotif] = useState<{ container: string; action: string; error?: string } | null>(null);
  const [checkedUpdates, setCheckedUpdates] = useState<Record<string, boolean>>({});
  const [isCheckingAll, setIsCheckingAll] = useState(false);
  const [checkProgress, setCheckProgress] = useState({ current: 0, total: 0 });
  const [currentCheckImage, setCurrentCheckImage] = useState<string>("");
  const cancelCheckRef = useRef(false);
  const [checkResults, setCheckResults] = useState<{ updated: number; uptodate: number; failed: number; errors: string[] }>({ updated: 0, uptodate: 0, failed: 0, errors: [] });
  const [showCheckSummary, setShowCheckSummary] = useState(false);

  const isMobile = useMediaQuery("(max-width: 768px)");

  useEffect(() => {
    apiFetch("/api/containers")
      .then((res) => res.json())
      .then((data) => { setContainers(data); setLoading(false); })
      .catch(() => {});
  }, []);

  useEffect(() => {
    const token = localStorage.getItem("token");
    const evtSource = new EventSource(`/api/events?token=${token}`);
    evtSource.addEventListener("containers", (e) => {
      setContainers(JSON.parse(e.data).containers);
      setLoading(false);
    });
    evtSource.onerror = () => setLoading(false);
    return () => evtSource.close();
  }, []);

  useEffect(() => {
    const token = localStorage.getItem("token");
    const evtSource = new EventSource(`/api/updates?token=${token}`);
    evtSource.addEventListener("update-progress", (e) => {
      const data: UpdateProgress = JSON.parse(e.data);
      setProgress((prev) => {
        const next = new Map(prev);
        next.set(data.container, data);
        return next;
      });
      if (data.done) {
        setUpdating(null);
        setTimeout(() => setProgress((prev) => { const n = new Map(prev); n.delete(data.container); return n; }), 3000);
      }
    });
    return () => evtSource.close();
  }, []);

  useEffect(() => {
    const token = localStorage.getItem("token");
    const evtSource = new EventSource(`/api/notifications?token=${token}`);
    evtSource.addEventListener("notification", (e) => {
      setNotifications((prev) => [...prev.slice(-4), JSON.parse(e.data)]);
    });
    return () => evtSource.close();
  }, []);

  const updateContainer = async (name: string) => {
    setUpdating(name);
    try { await apiFetch(`/api/update/${encodeURIComponent(name)}`, { method: "POST" }); }
    catch { setUpdating(null); }
  };

  const updateAll = async () => {
    setUpdating("__all__");
    try {
      const res = await apiFetch("/api/update-all", { method: "POST" });
      setProgress(new Map((await res.json()).map((r: UpdateProgress) => [r.container, r])));
    } catch { /* ignore */ }
    setUpdating(null);
  };

  const checkAll = async () => {
    setIsCheckingAll(true);
    cancelCheckRef.current = false;
    setCheckResults({ updated: 0, uptodate: 0, failed: 0, errors: [] });
    setCheckProgress({ current: 0, total: containers.length });
    setCurrentCheckImage("");

    const updatedUpdates: Record<string, boolean> = {};
    let updatedCount = 0;
    let uptodateCount = 0;
    let failedCount = 0;
    const errors: string[] = [];

    for (let i = 0; i < containers.length; i++) {
      if (cancelCheckRef.current) break;

      const c = containers[i];
      const imgLabel = `${c.image}:${c.image_tag}`;
      setCurrentCheckImage(imgLabel);
      setCheckProgress(prev => ({ ...prev, current: i + 1 }));

      try {
        const res = await apiFetch(`/api/check-update/${encodeURIComponent(c.name)}`, { method: "POST" });
        if (res.ok) {
          const data = await res.json();
          const hasUpdate = data.has_update === true;
          if (hasUpdate) {
            updatedUpdates[c.name] = true;
            updatedCount++;
          } else {
            uptodateCount++;
          }
        } else {
          failedCount++;
          errors.push(`${c.name}: HTTP ${res.status}`);
        }
      } catch (e: any) {
        failedCount++;
        errors.push(`${c.name}: ${e.message || "unknown error"}`);
      }
    }

    setCheckedUpdates(prev => ({ ...prev, ...updatedUpdates }));
    setContainers(prev => prev.map(c => ({ ...c, has_update: c.has_update || !!updatedUpdates[c.name] })));
    setCheckResults({ updated: updatedCount, uptodate: uptodateCount, failed: failedCount, errors });
    setIsCheckingAll(false);
    setCurrentCheckImage("");
    setShowCheckSummary(true);
  };

  const handleCancelCheck = () => {
    cancelCheckRef.current = true;
  };

  const handleContainerAction = async (name: string, action: string) => {
    try {
      const res = await apiFetch(`/api/containers/${encodeURIComponent(name)}/${action}`, { method: "POST" });
      if (!res.ok) throw new Error((await res.text()) || `Error al ${action}`);
      setActionNotif({ container: name, action: `${action} correcto` });
      setTimeout(() => setActionNotif(null), 3000);
    } catch (e: any) {
      setActionNotif({ container: name, action: `error al ${action}`, error: e.message });
      setTimeout(() => setActionNotif(null), 3000);
    }
  };

  const handleInspect = async (name: string) => {
    setInspectName(name);
    setInspectData(null);
    setInspectLoading(true);
    setInspectError(null);
    try {
      const res = await apiFetch(`/api/containers/${encodeURIComponent(name)}/inspect`);
      if (!res.ok) throw new Error("Error al inspeccionar");
      setInspectData(await res.json());
    } catch {
      setInspectError("No se pudo obtener información del container");
    }
    setInspectLoading(false);
  };

  const handleRemove = async (name: string) => {
    setConfirmDelete(null);
    try {
      const res = await apiFetch(`/api/containers/${encodeURIComponent(name)}/remove`, { method: "POST" });
      if (!res.ok) throw new Error((await res.text()) || "Error al eliminar");
      setActionNotif({ container: name, action: "eliminado correctamente" });
      setTimeout(() => setActionNotif(null), 3000);
    } catch (e: any) {
      setActionNotif({ container: name, action: "error al eliminar", error: e.message });
      setTimeout(() => setActionNotif(null), 3000);
    }
  };

  if (loading)
    return (
      <Container py="xl">
        <Group justify="center">
          <Loader />
          <Text>Conectando...</Text>
        </Group>
      </Container>
    );

  const grouped = new Map<string, ContainerInfo[]>();
  const noStack: ContainerInfo[] = [];
  for (const c of containers) {
    if (c.compose_project) {
      const list = grouped.get(c.compose_project) || [];
      list.push(c);
      grouped.set(c.compose_project, list);
    } else {
      noStack.push(c);
    }
  }
  const sortedGroups = Array.from(grouped.entries()).sort(([a], [b]) => a.localeCompare(b));

  // ── Mobile card view ────────────────────────────────────────
  const renderMobileCard = (c: ContainerInfo) => {
    const p = progress.get(c.name);
    const isUpdating = updating === c.name || p?.done === false;
    const hasUpdate = c.has_update || checkedUpdates[c.name];
    return (
      <Paper key={c.id} shadow="sm" p="sm" withBorder>
        <Stack gap="xs">
          {/* Header: name + status + menu */}
          <Group justify="space-between" wrap="nowrap">
            <Group gap="xs" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
              <Text size="sm" fw={500} truncate>{c.name}</Text>
              <Badge
                size="sm"
                color={c.status.includes("healthy") ? "green" : c.state === "running" ? "blue" : "red"}
              >
                {c.status.includes("healthy") ? "healthy" : c.state}
              </Badge>
            </Group>
            <Menu shadow="md" width={200}>
              <Menu.Target>
                <ActionIcon variant="subtle" size="sm" aria-label="Menú">⋮</ActionIcon>
              </Menu.Target>
              <Menu.Dropdown>
                <Menu.Item leftSection="🔍" onClick={() => handleInspect(c.name)}>Inspeccionar</Menu.Item>
                <Menu.Item leftSection="▶️" onClick={() => handleContainerAction(c.name, "start")} disabled={c.state === "running"}>Iniciar</Menu.Item>
                <Menu.Item leftSection="⏹️" onClick={() => handleContainerAction(c.name, "stop")} disabled={c.state !== "running"}>Parar</Menu.Item>
                <Menu.Item leftSection="🔄" onClick={() => handleContainerAction(c.name, "restart")}>Reiniciar</Menu.Item>
                <Menu.Divider />
                <Menu.Item leftSection="🗑️" color="red" onClick={() => setConfirmDelete(c.name)}>Eliminar</Menu.Item>
              </Menu.Dropdown>
            </Menu>
          </Group>

          {/* Image + update button */}
          <Group gap="xs" wrap="nowrap">
            <Text size="xs" c="dimmed" truncate style={{ flex: 1 }}>
              {c.image}:{c.image_tag}
            </Text>
            {hasUpdate && (
              <Tooltip label="Actualizar container">
                <ActionIcon color="yellow" variant="filled" size="sm" onClick={() => updateContainer(c.name)} loading={isUpdating}>⬆</ActionIcon>
              </Tooltip>
            )}
            {c.registry_url && (
              <Tooltip label="Ver en registry">
                <ActionIcon component="a" href={c.registry_url} target="_blank" rel="noopener noreferrer" variant="subtle" size="sm">📦</ActionIcon>
              </Tooltip>
            )}
          </Group>

          {p && (
            <Group gap="xs">
              <Loader size="xs" />
              <Text size="xs" c="dimmed">{p.status}</Text>
            </Group>
          )}

          <Divider />

          {/* Details grid */}
          <SimpleGrid cols={2} spacing="xs">
            {c.compose_project && (
              <Stack gap={0}>
                <Text size="xs" c="dimmed">Stack</Text>
                <Badge size="sm" variant="light" color="grape">{c.compose_project}</Badge>
              </Stack>
            )}
            {c.ports.length > 0 && (
              <Stack gap={0}>
                <Text size="xs" c="dimmed">Puertos</Text>
                <Stack gap={2}>
                  {c.ports.map((port, i) => <Code key={i}>{port}</Code>)}
                </Stack>
              </Stack>
            )}
            {c.traefik_url && (
              <Stack gap={0}>
                <Text size="xs" c="dimmed">Traefik</Text>
                <Anchor href={c.traefik_url} target="_blank" rel="noopener noreferrer" size="xs" truncate>
                  {c.traefik_url.replace(/^https?:\/\//, "")}
                </Anchor>
              </Stack>
            )}
          </SimpleGrid>
        </Stack>
      </Paper>
    );
  };

  // ── Desktop row ─────────────────────────────────────────────
  const renderRow = (c: ContainerInfo) => {
    const p = progress.get(c.name);
    const isUpdating = updating === c.name || p?.done === false;
    const hasUpdate = c.has_update || checkedUpdates[c.name];
    return (
      <Table.Tr key={c.id}>
        <Table.Td><Text size="sm" fw={500}>{c.name}</Text></Table.Td>
        <Table.Td>
          <Group gap="xs" wrap="nowrap">
            <Text size="sm" c="dimmed">{c.image}:{c.image_tag}</Text>
            {hasUpdate && (
              <Tooltip label="Actualizar container">
                <ActionIcon color="yellow" variant="filled" size="sm" onClick={() => updateContainer(c.name)} loading={isUpdating}>⬆</ActionIcon>
              </Tooltip>
            )}
            {c.registry_url && (
              <Tooltip label="Ver en registry">
                <ActionIcon component="a" href={c.registry_url} target="_blank" rel="noopener noreferrer" variant="subtle" size="sm">📦</ActionIcon>
              </Tooltip>
            )}
          </Group>
          {p && (
            <Group gap="xs" mt="xs">
              <Loader size="xs" />
              <Text size="xs" c="dimmed">{p.status}</Text>
            </Group>
          )}
        </Table.Td>
        <Table.Td>
          {c.compose_project ? <Badge size="sm" variant="light" color="grape">{c.compose_project}</Badge> : <Text size="xs" c="dimmed">-</Text>}
        </Table.Td>
        <Table.Td>
          {c.ports.length > 0 ? (
            <Stack gap="2">{c.ports.map((port, i) => <Code key={i}>{port}</Code>)}</Stack>
          ) : <Text size="xs" c="dimmed">-</Text>}
        </Table.Td>
        <Table.Td>
          {c.traefik_url ? (
            <Anchor href={c.traefik_url} target="_blank" rel="noopener noreferrer" size="xs">🔗 {c.traefik_url.replace(/^https?:\/\//, "")}</Anchor>
          ) : <Text size="xs" c="dimmed">-</Text>}
        </Table.Td>
        <Table.Td>
          <Badge color={c.status.includes("healthy") ? "green" : c.state === "running" ? "blue" : "red"}>
            {c.status.includes("healthy") ? "healthy" : c.state}
          </Badge>
        </Table.Td>
        <Table.Td>
          <Menu shadow="md" width={200}>
            <Menu.Target><ActionIcon variant="subtle" size="sm" aria-label="Menú">⋮</ActionIcon></Menu.Target>
            <Menu.Dropdown>
              <Menu.Item leftSection="🔍" onClick={() => handleInspect(c.name)}>Inspeccionar</Menu.Item>
              <Menu.Item leftSection="▶️" onClick={() => handleContainerAction(c.name, "start")} disabled={c.state === "running"}>Iniciar</Menu.Item>
              <Menu.Item leftSection="⏹️" onClick={() => handleContainerAction(c.name, "stop")} disabled={c.state !== "running"}>Parar</Menu.Item>
              <Menu.Item leftSection="🔄" onClick={() => handleContainerAction(c.name, "restart")}>Reiniciar</Menu.Item>
              <Menu.Divider />
              <Menu.Item leftSection="🗑️" color="red" onClick={() => setConfirmDelete(c.name)}>Eliminar</Menu.Item>
            </Menu.Dropdown>
          </Menu>
        </Table.Td>
      </Table.Tr>
    );
  };

  // ── Mobile group card ───────────────────────────────────────
  const renderMobileGroup = (project: string, items: ContainerInfo[]) => (
    <Paper shadow="sm" withBorder mb="md" key={project}>
      <Group px="md" pt="sm" pb="xs">
        <Title order={4}>📦 {project}</Title>
        <Badge size="lg" variant="light" color="blue">{items.length} servicios</Badge>
      </Group>
      <Stack px="md" pb="md" gap="sm">
        {items.map(renderMobileCard)}
      </Stack>
    </Paper>
  );

  // ── Desktop group ───────────────────────────────────────────
  const renderGroup = (project: string, items: ContainerInfo[]) => (
    <Paper shadow="sm" withBorder mb="md" key={project}>
      <Group px="md" pt="sm" pb="xs">
        <Title order={4}>📦 {project}</Title>
        <Badge size="lg" variant="light" color="blue">{items.length} servicios</Badge>
      </Group>
      <Table.ScrollContainer minWidth={700}>
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Container</Table.Th>
              <Table.Th>Imagen</Table.Th>
              <Table.Th>Stack</Table.Th>
              <Table.Th>Puertos</Table.Th>
              <Table.Th>Traefik</Table.Th>
              <Table.Th>Estado</Table.Th>
              <Table.Th>Menú</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>{items.map(renderRow)}</Table.Tbody>
        </Table>
      </Table.ScrollContainer>
    </Paper>
  );

  return (
    <>
      {notifications.length > 0 && (
        <Paper mb="md" p="xs">
          <Text size="xs" c="dimmed" mb="xs">🔔 Notificaciones</Text>
          {notifications.map((n, i) => (
            <NotifToast key={i} notif={n} onDismiss={() => setNotifications((p) => p.filter((_, j) => j !== i))} />
          ))}
        </Paper>
      )}
      <Paper shadow="sm" p="md" mb="md" withBorder>
        {isCheckingAll ? (
          <Stack gap="xs">
            <Group justify="space-between">
              <Text size="sm" fw={500}>🔍 Comprobando actualizaciones...</Text>
              <Button size="xs" color="red" variant="outline" onClick={handleCancelCheck}>
                Cancelar
              </Button>
            </Group>
            <Progress value={(checkProgress.current / Math.max(checkProgress.total, 1)) * 100} animated />
            <Group justify="space-between">
              <Text size="xs" c="dimmed">
                {checkProgress.current} / {checkProgress.total} — {currentCheckImage || "iniciando..."}
              </Text>
              <Text size="xs" c={checkResults.failed > 0 ? "red" : "dimmed"}>
                ✅ {checkResults.updated} actualizaciones · ⏹️ {checkResults.uptodate} actuales{checkResults.failed > 0 ? ` · ❌ ${checkResults.failed} errores` : ""}
              </Text>
            </Group>
          </Stack>
        ) : (
          isMobile ? (
            <Stack gap="sm">
              <Text size="sm" c="dimmed">SSE en tiempo real · {containers.length} containers</Text>
              <Button onClick={checkAll} variant="filled" color="cyan" fullWidth>🔍 Check all</Button>
              <Button onClick={updateAll} loading={updating === "__all__"} variant="filled" color="yellow" fullWidth>Actualizar todo</Button>
            </Stack>
          ) : (
            <Group justify="space-between">
              <Text size="sm" c="dimmed">SSE en tiempo real · {containers.length} containers</Text>
              <Group gap="xs">
                <Tooltip label="Comprueba todos los containers contra el registry">
                  <Button onClick={checkAll} variant="filled" color="cyan">🔍 Check all</Button>
                </Tooltip>
                <Tooltip label="Actualiza todas las imágenes y reinicia containers">
                  <Button onClick={updateAll} loading={updating === "__all__"} variant="filled" color="yellow">Actualizar todo</Button>
                </Tooltip>
              </Group>
            </Group>
          )
        )}
      </Paper>

      {/* Container groups — mobile vs desktop */}
      {isMobile ? (
        <>
          {sortedGroups.map(([project, items]) => renderMobileGroup(project, items))}
          {noStack.length > 0 && (
            <Paper shadow="sm" withBorder>
              <Group px="md" pt="sm" pb="xs">
                <Title order={4}>📦 Sin stack</Title>
                <Badge size="lg" variant="light" color="gray">{noStack.length} containers</Badge>
              </Group>
              <Stack px="md" pb="md" gap="sm">
                {noStack.map(renderMobileCard)}
              </Stack>
            </Paper>
          )}
        </>
      ) : (
        <>
          {sortedGroups.map(([project, items]) => renderGroup(project, items))}
          {noStack.length > 0 && (
            <Paper shadow="sm" withBorder>
              <Group px="md" pt="sm" pb="xs">
                <Title order={4}>📦 Sin stack</Title>
                <Badge size="lg" variant="light" color="gray">{noStack.length} containers</Badge>
              </Group>
              <Table.ScrollContainer minWidth={700}>
                <Table striped highlightOnHover>
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>Container</Table.Th>
                      <Table.Th>Imagen</Table.Th>
                      <Table.Th>Stack</Table.Th>
                      <Table.Th>Puertos</Table.Th>
                      <Table.Th>Traefik</Table.Th>
                      <Table.Th>Estado</Table.Th>
                      <Table.Th>Menú</Table.Th>
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>{noStack.map(renderRow)}</Table.Tbody>
                </Table>
              </Table.ScrollContainer>
            </Paper>
          )}
        </>
      )}

      {actionNotif && (
        <Paper shadow="md" p="sm" withBorder mb="xs" style={{ position: "fixed", bottom: isMobile ? 0 : 20, right: isMobile ? 0 : 20, left: isMobile ? 0 : undefined, zIndex: 1000, background: actionNotif.error ? "#3d1f1f" : "#1f3d1f", borderColor: actionNotif.error ? "#e03131" : "#2f9e44" }}>
          <Group justify="space-between">
            <Text size="sm"><b>{actionNotif.container}</b> — {actionNotif.error ? `❌ ${actionNotif.error}` : `✅ ${actionNotif.action}`}</Text>
          </Group>
        </Paper>
      )}
      <Modal opened={inspectName !== null} onClose={() => { setInspectName(null); setInspectData(null); setInspectError(null); }} title={`🔍 Inspeccionar ${inspectName || ""}`} size={isMobile ? "100%" : "xl"}>
        {inspectLoading ? (
          <Group justify="center" py="xl"><Loader /><Text>Obteniendo información...</Text></Group>
        ) : inspectError ? <Text c="red">{inspectError}</Text> : inspectData ? (
          <Tabs defaultValue="general">
            <Tabs.List mb="sm">
              <Tabs.Tab value="general">General</Tabs.Tab>
              <Tabs.Tab value="ports">Puertos</Tabs.Tab>
              <Tabs.Tab value="volumes">Volúmenes</Tabs.Tab>
              <Tabs.Tab value="networks">Redes</Tabs.Tab>
              <Tabs.Tab value="env">ENV</Tabs.Tab>
              <Tabs.Tab value="labels">Labels</Tabs.Tab>
            </Tabs.List>
            <Tabs.Panel value="general">
              <Stack gap="xs">
                <Group><Text size="sm" fw={500} w={120}>ID:</Text><Text size="sm">{inspectData.id}</Text></Group>
                <Group><Text size="sm" fw={500} w={120}>Nombre:</Text><Text size="sm">{inspectData.name}</Text></Group>
                <Group><Text size="sm" fw={500} w={120}>Imagen:</Text><Text size="sm">{inspectData.image}</Text></Group>
                <Group><Text size="sm" fw={500} w={120}>Creado:</Text><Text size="sm">{inspectData.created}</Text></Group>
                <Group><Text size="sm" fw={500} w={120}>Estado:</Text><Badge color={inspectData.state === "running" ? "green" : "red"}>{inspectData.state}</Badge></Group>
                <Group><Text size="sm" fw={500} w={120}>Status:</Text><Text size="sm">{inspectData.status}</Text></Group>
                {inspectData.restart_policy && <Group><Text size="sm" fw={500} w={120}>Reinicio:</Text><Text size="sm">{inspectData.restart_policy}</Text></Group>}
                {inspectData.health && <Group><Text size="sm" fw={500} w={120}>Health:</Text><Badge color={inspectData.health === "healthy" ? "green" : "yellow"}>{inspectData.health}</Badge></Group>}
              </Stack>
            </Tabs.Panel>
            <Tabs.Panel value="ports">
              {inspectData.ports?.length > 0 ? (
                <Table.ScrollContainer minWidth={400}>
                  <Table striped>
                    <Table.Thead><Table.Tr><Table.Th>Puerto Privado</Table.Th><Table.Th>Puerto Público</Table.Th><Table.Th>Tipo</Table.Th></Table.Tr></Table.Thead>
                    <Table.Tbody>{inspectData.ports.map((p: any, i: number) => (
                      <Table.Tr key={i}><Table.Td>{p.private_port}</Table.Td><Table.Td>{p.public_port != null ? p.public_port : "-"}</Table.Td><Table.Td>{p.type}</Table.Td></Table.Tr>
                    ))}</Table.Tbody>
                  </Table>
                </Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin puertos expuestos</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="volumes">
              {inspectData.mounts?.length > 0 ? (
                <Table.ScrollContainer minWidth={400}>
                  <Table striped>
                    <Table.Thead><Table.Tr><Table.Th>Origen</Table.Th><Table.Th>Destino</Table.Th><Table.Th>Modo</Table.Th></Table.Tr></Table.Thead>
                    <Table.Tbody>{inspectData.mounts.map((m: any, i: number) => (
                      <Table.Tr key={i}><Table.Td><Text size="sm">{m.source}</Text></Table.Td><Table.Td><Text size="sm">{m.destination}</Text></Table.Td><Table.Td><Badge variant="light">{m.mode}</Badge></Table.Td></Table.Tr>
                    ))}</Table.Tbody>
                  </Table>
                </Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin volúmenes montados</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="networks">
              {inspectData.networks?.length > 0 ? (
                <Table.ScrollContainer minWidth={400}>
                  <Table striped>
                    <Table.Thead><Table.Tr><Table.Th>Red</Table.Th><Table.Th>IP</Table.Th><Table.Th>Gateway</Table.Th></Table.Tr></Table.Thead>
                    <Table.Tbody>{inspectData.networks.map((n: any, i: number) => (
                      <Table.Tr key={i}><Table.Td><Text size="sm">{n.name}</Text></Table.Td><Table.Td><Code>{n.ip_address}</Code></Table.Td><Table.Td><Code>{n.gateway}</Code></Table.Td></Table.Tr>
                    ))}</Table.Tbody>
                  </Table>
                </Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin redes</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="env">
              {inspectData.env?.length > 0 ? (
                <ScrollArea h={300}><Code block>{inspectData.env.map((e: string, i: number) => <div key={i}>{e}</div>)}</Code></ScrollArea>
              ) : <Text size="sm" c="dimmed" py="md">Sin variables de entorno</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="labels">
              {inspectData.labels && Object.keys(inspectData.labels).length > 0 ? (
                <ScrollArea h={300}>{Object.entries(inspectData.labels).map(([k, v]) => (
                  <Group key={k} gap="xs" mb="xs"><Text size="sm" fw={500}>{k}:</Text><Text size="sm">{v}</Text></Group>
                ))}</ScrollArea>
              ) : <Text size="sm" c="dimmed" py="md">Sin labels</Text>}
            </Tabs.Panel>
          </Tabs>
        ) : null}
      </Modal>
      <Modal opened={confirmDelete !== null} onClose={() => setConfirmDelete(null)} title="🗑️ Confirmar Eliminación" size="sm">
        <Text mb="md">¿Seguro que quieres eliminar <b>{confirmDelete}</b>? Esta acción no se puede deshacer.</Text>
        <Group justify="flex-end">
          <Button variant="default" onClick={() => setConfirmDelete(null)}>Cancelar</Button>
          <Button color="red" onClick={() => confirmDelete && handleRemove(confirmDelete)}>Eliminar</Button>
        </Group>
      </Modal>
      <Modal opened={showCheckSummary} onClose={() => setShowCheckSummary(false)} title="📋 Resumen de comprobación" size="sm">
        <Stack gap="md">
          <Paper p="md" withBorder>
            <Text size="sm" c="dimmed" mb="xs">Resultados</Text>
            <Group gap="lg">
              <Stack gap="0" align="center">
                <Text size="xl" fw={700}>{checkProgress.total}</Text>
                <Text size="xs" c="dimmed">Comprobados</Text>
              </Stack>
              <Stack gap="0" align="center">
                <Text size="xl" fw={700} c="yellow">{checkResults.updated}</Text>
                <Text size="xs" c="dimmed">Actualizaciones</Text>
              </Stack>
              <Stack gap="0" align="center">
                <Text size="xl" fw={700} c="green">{checkResults.uptodate}</Text>
                <Text size="xs" c="dimmed">Actuales</Text>
              </Stack>
              <Stack gap="0" align="center">
                <Text size="xl" fw={700} c="red">{checkResults.failed}</Text>
                <Text size="xs" c="dimmed">Errores</Text>
              </Stack>
            </Group>
          </Paper>
          {cancelCheckRef.current && (
            <Text size="sm" c="orange">⚠️ Comprobación cancelada por el usuario.</Text>
          )}
          {checkResults.errors.length > 0 && (
            <Paper p="sm" withBorder bg="red.0">
              <Text size="xs" fw={500} mb="xs" c="red">Errores:</Text>
              {checkResults.errors.map((err, i) => (
                <Text key={i} size="xs" c="red">{err}</Text>
              ))}
            </Paper>
          )}
          <Group justify="flex-end">
            <Button onClick={() => setShowCheckSummary(false)}>Cerrar</Button>
          </Group>
        </Stack>
      </Modal>
    </>
  );
}