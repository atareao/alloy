import { useEffect, useState, useRef } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  ActionIcon, Badge, Button, Container, Group, Loader, Menu, Paper, Table, Text,
  Title, Tooltip, Code, Stack, Modal, Anchor, Tabs, ScrollArea, Progress, Divider,
  SimpleGrid,
} from "@mantine/core";
import type { ContainerInfo, UpdateProgress, NotifEvent, InspectData } from "../types";
import { apiFetch, truncate } from "../api";
import NotifToast from "./NotifToast";

// ── Types ────────────────────────────────────────────────────
type CheckAllPhase = 'idle' | 'checking' | 'updating';

interface CheckAllResults {
  total: number;
  updated: number;    // containers with available update
  uptodate: number;   // already up-to-date
  failed: number;     // check failed
  errors: string[];
}

interface UpdateAllResults {
  done: number;
  failed: number;
  errors: string[];
}

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
  const [singleCheckLoading, setSingleCheckLoading] = useState<string | null>(null);

  // Check all / Update all state
  const [batchPhase, setBatchPhase] = useState<CheckAllPhase>('idle');
  const [batchProgress, setBatchProgress] = useState({ current: 0, total: 0 });
  const [batchCurrentItem, setBatchCurrentItem] = useState("");
  const cancelBatchRef = useRef(false);
  const [checkResults, setCheckResults] = useState<CheckAllResults>({ total: 0, updated: 0, uptodate: 0, failed: 0, errors: [] });
  const [updateResults, setUpdateResults] = useState<UpdateAllResults>({ done: 0, failed: 0, errors: [] });
  const [showSummary, setShowSummary] = useState(false);

  const isMobile = useMediaQuery("(max-width: 768px)");

  // ── Data fetching ──────────────────────────────────────────

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

  // ── Single container actions ───────────────────────────────

  const checkSingleContainer = async (name: string) => {
    setSingleCheckLoading(name);
    try {
      const res = await apiFetch(`/api/check-update/${encodeURIComponent(name)}`, { method: "POST" });
      if (res.ok) {
        const data = await res.json();
        const hasUpdate = data.has_update === true;
        setCheckedUpdates(prev => ({ ...prev, [name]: hasUpdate }));
        setContainers(prev => prev.map(c => c.name === name ? { ...c, has_update: hasUpdate } : c));
        setActionNotif({
          container: name,
          action: hasUpdate ? "actualización disponible ⬆️" : "está actualizado ✅",
        });
        setTimeout(() => setActionNotif(null), 3000);
      } else {
        setActionNotif({ container: name, action: "error al comprobar", error: `HTTP ${res.status}` });
        setTimeout(() => setActionNotif(null), 4000);
      }
    } catch (e: any) {
      setActionNotif({ container: name, action: "error al comprobar", error: e.message });
      setTimeout(() => setActionNotif(null), 4000);
    }
    setSingleCheckLoading(null);
  };

  const updateSingleContainer = async (name: string) => {
    setUpdating(name);
    try {
      await apiFetch(`/api/update/${encodeURIComponent(name)}`, { method: "POST" });
      setCheckedUpdates(prev => { const n = { ...prev }; delete n[name]; return n; });
      setContainers(prev => prev.map(c => c.name === name ? { ...c, has_update: false } : c));
      setActionNotif({ container: name, action: "actualizado ✅" });
      setTimeout(() => setActionNotif(null), 3000);
    } catch {
      setActionNotif({ container: name, action: "error al actualizar", error: "fallo en la petición" });
      setTimeout(() => setActionNotif(null), 4000);
    }
    setUpdating(null);
  };

  // ── Check all ──────────────────────────────────────────────

  const checkAll = async () => {
    const initialContainers = containers;
    cancelBatchRef.current = false;
    setBatchPhase('checking');
    setCheckResults({ total: 0, updated: 0, uptodate: 0, failed: 0, errors: [] });
    setUpdateResults({ done: 0, failed: 0, errors: [] });
    setBatchProgress({ current: 0, total: initialContainers.length });
    setBatchCurrentItem("");
    setShowSummary(false);

    const updatedUpdates: Record<string, boolean> = {};
    let updatedCount = 0;
    let uptodateCount = 0;
    let failedCount = 0;
    const errors: string[] = [];

    for (let i = 0; i < initialContainers.length; i++) {
      if (cancelBatchRef.current) break;

      const c = initialContainers[i];
      const imgLabel = truncate(`${c.image}:${c.image_tag}`);
      setBatchCurrentItem(`🔍 ${c.name} — ${imgLabel}`);
      setBatchProgress(prev => ({ ...prev, current: i + 1 }));

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
    setContainers(prev => prev.map(c => ({ ...c, has_update: !!updatedUpdates[c.name] })));
    setCheckResults({ total: initialContainers.length, updated: updatedCount, uptodate: uptodateCount, failed: failedCount, errors });
    setBatchProgress({ current: initialContainers.length, total: initialContainers.length });

    setTimeout(() => {
      setBatchPhase('idle');
      setShowSummary(true);
    }, 500);
  };

  // ── Update all (check first, then update) ──────────────────

  const updateAll = async () => {
    const initialContainers = containers;
    cancelBatchRef.current = false;
    setBatchPhase('checking');
    setCheckResults({ total: 0, updated: 0, uptodate: 0, failed: 0, errors: [] });
    setUpdateResults({ done: 0, failed: 0, errors: [] });
    setBatchProgress({ current: 0, total: initialContainers.length });
    setBatchCurrentItem("");
    setShowSummary(false);

    // ── Phase 1: Check all ───────────────────────────────────
    const containersToUpdate: ContainerInfo[] = [];
    let checkUpdated = 0;
    let checkUptodate = 0;
    let checkFailed = 0;
    const checkErrors: string[] = [];

    for (let i = 0; i < initialContainers.length; i++) {
      if (cancelBatchRef.current) break;

      const c = initialContainers[i];
      setBatchCurrentItem(`🔍 ${c.name} — ${truncate(`${c.image}:${c.image_tag}`)}`);
      setBatchProgress(prev => ({ ...prev, current: i + 1 }));

      try {
        const res = await apiFetch(`/api/check-update/${encodeURIComponent(c.name)}`, { method: "POST" });
        if (res.ok) {
          const data = await res.json();
          const hasUpdate = data.has_update === true;
          if (hasUpdate) {
            containersToUpdate.push(c);
            checkUpdated++;
          } else {
            checkUptodate++;
          }
        } else {
          checkFailed++;
          checkErrors.push(`${c.name}: HTTP ${res.status}`);
        }
      } catch (e: any) {
        checkFailed++;
        checkErrors.push(`${c.name}: ${e.message || "unknown error"}`);
      }
    }

    const updatedUpdates: Record<string, boolean> = {};
    containersToUpdate.forEach(c => { updatedUpdates[c.name] = true; });
    setCheckedUpdates(prev => ({ ...prev, ...updatedUpdates }));
    setContainers(prev => prev.map(c => ({ ...c, has_update: !!updatedUpdates[c.name] })));
    setCheckResults({ total: initialContainers.length, updated: checkUpdated, uptodate: checkUptodate, failed: checkFailed, errors: checkErrors });

    // ── Phase 2: Update only those that need it ──────────────
    if (containersToUpdate.length > 0 && !cancelBatchRef.current) {
      setBatchPhase('updating');
      setBatchProgress({ current: 0, total: containersToUpdate.length });
      let updateDone = 0;
      let updateFailed = 0;
      const updateErrors: string[] = [];
      const succeededNames: string[] = [];

      for (let i = 0; i < containersToUpdate.length; i++) {
        if (cancelBatchRef.current) break;

        const c = containersToUpdate[i];
        setBatchCurrentItem(`⬆️ ${c.name} — ${truncate(`${c.image}:${c.image_tag}`)}`);
        setBatchProgress(prev => ({ ...prev, current: i + 1 }));

        try {
          const res = await apiFetch(`/api/update/${encodeURIComponent(c.name)}`, { method: "POST" });
          if (res.ok) {
            updateDone++;
            succeededNames.push(c.name);
          } else {
            updateFailed++;
            updateErrors.push(`${c.name}: HTTP ${res.status}`);
          }
        } catch (e: any) {
          updateFailed++;
          updateErrors.push(`${c.name}: ${e.message || "unknown error"}`);
        }
      }

      setUpdateResults({ done: updateDone, failed: updateFailed, errors: updateErrors });

      // ✅ Clear update flags only for successfully updated containers
      setCheckedUpdates(prev => {
        const n = { ...prev };
        succeededNames.forEach(name => { n[name] = false; });
        return n;
      });
      setContainers(prev => prev.map(c =>
        succeededNames.includes(c.name) ? { ...c, has_update: false } : c
      ));
    }

    setTimeout(() => {
      setBatchPhase('idle');
      setShowSummary(true);
    }, 500);
  };

  const handleCancelBatch = () => {
    cancelBatchRef.current = true;
  };

  // ── Container lifecycle ────────────────────────────────────

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

  // ── Derived data ───────────────────────────────────────────

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

  const isBusy = batchPhase !== 'idle';

  // ── Container menu (shared between mobile & desktop) ────────
  const renderMenu = (c: ContainerInfo) => {
    const isSingleChecking = singleCheckLoading === c.name;
    const isSingleUpdating = updating === c.name;
    const hasUpdate = c.has_update || checkedUpdates[c.name];
    return (
      <Menu shadow="md" width={220}>
        <Menu.Target>
          <ActionIcon variant="subtle" size="sm" aria-label="Menú">⋮</ActionIcon>
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Item leftSection="🔍" onClick={() => handleInspect(c.name)}>Inspeccionar</Menu.Item>
          <Menu.Divider />
          <Menu.Item
            leftSection={isSingleChecking ? <Loader size="xs" /> : "🔍"}
            onClick={() => checkSingleContainer(c.name)}
            disabled={isSingleChecking || isBusy}
          >
            {isSingleChecking ? 'Comprobando...' : 'Check update'}
          </Menu.Item>
          <Menu.Item
            leftSection={isSingleUpdating ? <Loader size="xs" /> : (hasUpdate ? "⬆️" : "⬆️")}
            onClick={() => updateSingleContainer(c.name)}
            disabled={isSingleUpdating || isBusy}
          >
            {isSingleUpdating ? 'Actualizando...' : 'Actualizar'}
          </Menu.Item>
          <Menu.Divider />
          <Menu.Item leftSection="▶️" onClick={() => handleContainerAction(c.name, "start")} disabled={c.state === "running"}>Iniciar</Menu.Item>
          <Menu.Item leftSection="⏹️" onClick={() => handleContainerAction(c.name, "stop")} disabled={c.state !== "running"}>Parar</Menu.Item>
          <Menu.Item leftSection="🔄" onClick={() => handleContainerAction(c.name, "restart")}>Reiniciar</Menu.Item>
          <Menu.Divider />
          <Menu.Item leftSection="🗑️" color="red" onClick={() => setConfirmDelete(c.name)}>Eliminar</Menu.Item>
        </Menu.Dropdown>
      </Menu>
    );
  };

  // ── Mobile card view ────────────────────────────────────────
  const renderMobileCard = (c: ContainerInfo) => {
    const p = progress.get(c.name);
    const isSingleUpdating = updating === c.name || p?.done === false;
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
            {renderMenu(c)}
          </Group>

          {/* Image + update button */}
          <Group gap="xs" wrap="nowrap">
            <Text size="xs" c="dimmed" truncate style={{ flex: 1 }}>
              {truncate(`${c.image}:${c.image_tag}`)}
            </Text>
            {hasUpdate && (
              <Tooltip label="Actualizar container">
                <ActionIcon color="yellow" variant="filled" size="sm" onClick={() => updateSingleContainer(c.name)} loading={isSingleUpdating}>⬆</ActionIcon>
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
    const isSingleUpdating = updating === c.name || p?.done === false;
    const hasUpdate = c.has_update || checkedUpdates[c.name];
    return (
      <Table.Tr key={c.id}>
        <Table.Td><Text size="sm" fw={500}>{c.name}</Text></Table.Td>
        <Table.Td>
          <Group gap="xs" wrap="nowrap">
            <Text size="sm" c="dimmed">{truncate(`${c.image}:${c.image_tag}`)}</Text>
            {hasUpdate && (
              <Tooltip label="Actualizar container">
                <ActionIcon color="yellow" variant="filled" size="sm" onClick={() => updateSingleContainer(c.name)} loading={isSingleUpdating}>⬆</ActionIcon>
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
          {renderMenu(c)}
        </Table.Td>
      </Table.Tr>
    );
  };

  // ── Group renderers ─────────────────────────────────────────

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

  // ── Batch progress bar ──────────────────────────────────────
  const renderBatchProgress = () => {
    const isUpdatePhase = batchPhase === 'updating';
    const total = isUpdatePhase
      ? checkResults.updated  // only updating containers that need it
      : batchProgress.total;
    const pct = total > 0 ? (batchProgress.current / total) * 100 : 0;

    return (
      <Stack gap="xs">
        <Group justify="space-between">
          <Text size="sm" fw={500}>
            {isUpdatePhase ? '⬆️ Actualizando containers...' : '🔍 Comprobando actualizaciones...'}
          </Text>
          <Group gap="xs">
            {!isUpdatePhase && (
              <Text size="xs" c="dimmed" mr="sm">
                ✅ {checkResults.updated} upd · ⏹️ {checkResults.uptodate} ok{checkResults.failed > 0 ? ` · ❌ ${checkResults.failed}` : ''}
              </Text>
            )}
            <Button size="xs" color="red" variant="outline" onClick={handleCancelBatch}>
              Cancelar
            </Button>
          </Group>
        </Group>
        <Progress value={pct} animated color={isUpdatePhase ? 'yellow' : 'cyan'} />
        <Group justify="space-between">
          <Text size="xs" c="dimmed">
            {batchProgress.current} / {total} — {batchCurrentItem || "iniciando..."}
          </Text>
          {isUpdatePhase && (
            <Text size="xs" c="dimmed">
              ✅ {updateResults.done} hechos{updateResults.failed > 0 ? ` · ❌ ${updateResults.failed} errores` : ''}
            </Text>
          )}
        </Group>
      </Stack>
    );
  };

  // ── Main render ─────────────────────────────────────────────

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

      {/* Batch progress bar (checking or updating) */}
      {batchPhase !== 'idle' && (
        <Paper shadow="sm" p="md" mb="md" withBorder>
          {renderBatchProgress()}
        </Paper>
      )}

      {/* Action buttons — only visible when idle */}
      {batchPhase === 'idle' && (
        <Paper shadow="sm" p="md" mb="md" withBorder>
          {isMobile ? (
            <Stack gap="sm">
              <Text size="sm" c="dimmed">SSE en tiempo real · {containers.length} containers</Text>
              <Button onClick={checkAll} variant="filled" color="cyan" fullWidth>🔍 Check all</Button>
              <Button onClick={updateAll} variant="filled" color="yellow" fullWidth>⬆️ Actualizar todo</Button>
            </Stack>
          ) : (
            <Group justify="space-between">
              <Text size="sm" c="dimmed">SSE en tiempo real · {containers.length} containers</Text>
              <Group gap="xs">
                <Tooltip label="Comprueba todos los containers contra el registry">
                  <Button onClick={checkAll} variant="filled" color="cyan">🔍 Check all</Button>
                </Tooltip>
                <Tooltip label="Comprueba y actualiza solo los que tengan update disponible">
                  <Button onClick={updateAll} variant="filled" color="yellow">⬆️ Actualizar todo</Button>
                </Tooltip>
              </Group>
            </Group>
          )}
        </Paper>
      )}

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

      {/* Toast notification */}
      {actionNotif && (
        <Paper shadow="md" p="sm" withBorder mb="xs" style={{ position: "fixed", bottom: isMobile ? 0 : 20, right: isMobile ? 0 : 20, left: isMobile ? 0 : undefined, zIndex: 1000, background: actionNotif.error ? "#3d1f1f" : "#1f3d1f", borderColor: actionNotif.error ? "#e03131" : "#2f9e44" }}>
          <Group justify="space-between">
            <Text size="sm"><b>{actionNotif.container}</b> — {actionNotif.error ? `❌ ${actionNotif.error}` : `✅ ${actionNotif.action}`}</Text>
          </Group>
        </Paper>
      )}

      {/* Inspect modal */}
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
                <Table.ScrollContainer minWidth={400}><Table striped><Table.Thead><Table.Tr><Table.Th>Puerto Privado</Table.Th><Table.Th>Puerto Público</Table.Th><Table.Th>Tipo</Table.Th></Table.Tr></Table.Thead><Table.Tbody>{inspectData.ports.map((p: any, i: number) => (
                  <Table.Tr key={i}><Table.Td>{p.private_port}</Table.Td><Table.Td>{p.public_port != null ? p.public_port : "-"}</Table.Td><Table.Td>{p.type}</Table.Td></Table.Tr>
                ))}</Table.Tbody></Table></Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin puertos expuestos</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="volumes">
              {inspectData.mounts?.length > 0 ? (
                <Table.ScrollContainer minWidth={400}><Table striped><Table.Thead><Table.Tr><Table.Th>Origen</Table.Th><Table.Th>Destino</Table.Th><Table.Th>Modo</Table.Th></Table.Tr></Table.Thead><Table.Tbody>{inspectData.mounts.map((m: any, i: number) => (
                  <Table.Tr key={i}><Table.Td><Text size="sm">{m.source}</Text></Table.Td><Table.Td><Text size="sm">{m.destination}</Text></Table.Td><Table.Td><Badge variant="light">{m.mode}</Badge></Table.Td></Table.Tr>
                ))}</Table.Tbody></Table></Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin volúmenes montados</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="networks">
              {inspectData.networks?.length > 0 ? (
                <Table.ScrollContainer minWidth={400}><Table striped><Table.Thead><Table.Tr><Table.Th>Red</Table.Th><Table.Th>IP</Table.Th><Table.Th>Gateway</Table.Th></Table.Tr></Table.Thead><Table.Tbody>{inspectData.networks.map((n: any, i: number) => (
                  <Table.Tr key={i}><Table.Td><Text size="sm">{n.name}</Text></Table.Td><Table.Td><Code>{n.ip_address}</Code></Table.Td><Table.Td><Code>{n.gateway}</Code></Table.Td></Table.Tr>
                ))}</Table.Tbody></Table></Table.ScrollContainer>
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

      {/* Confirm delete modal */}
      <Modal opened={confirmDelete !== null} onClose={() => setConfirmDelete(null)} title="🗑️ Confirmar Eliminación" size="sm">
        <Text mb="md">¿Seguro que quieres eliminar <b>{confirmDelete}</b>? Esta acción no se puede deshacer.</Text>
        <Group justify="flex-end">
          <Button variant="default" onClick={() => setConfirmDelete(null)}>Cancelar</Button>
          <Button color="red" onClick={() => confirmDelete && handleRemove(confirmDelete)}>Eliminar</Button>
        </Group>
      </Modal>

      {/* Summary dialog (after check-all or update-all) */}
      <Modal
        opened={showSummary}
        onClose={() => setShowSummary(false)}
        title={updateResults.done > 0 || updateResults.failed > 0 ? "📋 Resumen de actualización" : "📋 Resumen de comprobación"}
        size="sm"
      >
        <Stack gap="md">
          {/* Check results */}
          <Paper p="md" withBorder>
            <Text size="sm" c="dimmed" mb="xs">🔍 Comprobación</Text>
            <Group gap="lg">
              <Stack gap="0" align="center">
                <Text size="xl" fw={700}>{checkResults.total}</Text>
                <Text size="xs" c="dimmed">Comprobados</Text>
              </Stack>
              <Stack gap="0" align="center">
                <Text size="xl" fw={700} c="yellow">{checkResults.updated}</Text>
                <Text size="xs" c="dimmed">Actualizables</Text>
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

          {/* Update results (only if update-all ran) */}
          {(updateResults.done > 0 || updateResults.failed > 0) && (
            <Paper p="md" withBorder>
              <Text size="sm" c="dimmed" mb="xs">⬆️ Actualización</Text>
              <Group gap="lg">
                <Stack gap="0" align="center">
                  <Text size="xl" fw={700} c="green">{updateResults.done}</Text>
                  <Text size="xs" c="dimmed">Actualizados</Text>
                </Stack>
                <Stack gap="0" align="center">
                  <Text size="xl" fw={700} c="red">{updateResults.failed}</Text>
                  <Text size="xs" c="dimmed">Fallos</Text>
                </Stack>
              </Group>
            </Paper>
          )}

          {cancelBatchRef.current && (
            <Text size="sm" c="orange">⚠️ Operación cancelada por el usuario.</Text>
          )}

          {/* Errors */}
          {[...checkResults.errors, ...updateResults.errors].length > 0 && (
            <Paper p="sm" withBorder bg="red.0">
              <Text size="xs" fw={500} mb="xs" c="red">Errores:</Text>
              {[...checkResults.errors, ...updateResults.errors].map((err, i) => (
                <Text key={i} size="xs" c="red">{err}</Text>
              ))}
            </Paper>
          )}

          <Group justify="flex-end">
            <Button onClick={() => setShowSummary(false)}>Cerrar</Button>
          </Group>
        </Stack>
      </Modal>
    </>
  );
}
