import { useState, useRef, useMemo, useEffect } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  ActionIcon, Badge, Button, Container, Collapse, Group, Loader, Paper, Table, Text,
  Title, Tooltip, Code, Stack, Modal, Anchor, Tabs, ScrollArea, Progress, Divider,
  SimpleGrid, TextInput, Chip, Switch,
} from "@mantine/core";
import { showNotification } from "@mantine/notifications";
import type { ContainerInfo, UpdateProgress, NotifEvent, InspectData, AppConfig, UpdatePolicy } from "../types";
import { apiFetch, truncate } from "../api";
import NotifToast from "./NotifToast";
import PolicyActionButton from "./PolicyActionButton";

// ── Props ────────────────────────────────────────────────────
interface DashboardPageProps {
  containers: ContainerInfo[];
  setContainers: React.Dispatch<React.SetStateAction<ContainerInfo[]>>;
  progress: Map<string, UpdateProgress>;
  notifications: NotifEvent[];
  setNotifications: React.Dispatch<React.SetStateAction<NotifEvent[]>>;
  containersLoaded: boolean;
  config: AppConfig | null;
}

// ── Types ────────────────────────────────────────────────────
type CheckAllPhase = 'idle' | 'checking' | 'updating';

interface CheckAllResults {
  total: number;
  updated: number;
  uptodate: number;
  failed: number;
  errors: string[];
}

interface UpdateAllResults {
  done: number;
  failed: number;
  errors: string[];
}

export default function DashboardPage({
  containers,
  setContainers,
  progress,
  notifications,
  setNotifications,
  containersLoaded,
  config,
}: DashboardPageProps) {
  const [inspectName, setInspectName] = useState<string | null>(null);
  const [inspectData, setInspectData] = useState<InspectData | null>(null);
  const [inspectLoading, setInspectLoading] = useState(false);
  const [inspectError, setInspectError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  // Check all / Update all state
  const [batchPhase, setBatchPhase] = useState<CheckAllPhase>('idle');
  const [batchProgress, setBatchProgress] = useState({ current: 0, total: 0 });
  const [batchCurrentItem, setBatchCurrentItem] = useState("");
  const cancelBatchRef = useRef(false);
  const [checkResults, setCheckResults] = useState<CheckAllResults>({ total: 0, updated: 0, uptodate: 0, failed: 0, errors: [] });
  const [updateResults, setUpdateResults] = useState<UpdateAllResults>({ done: 0, failed: 0, errors: [] });
  const [showSummary, setShowSummary] = useState(false);

  // Update policies
  const [policies, setPolicies] = useState<UpdatePolicy[]>([]);
  useEffect(() => {
    fetch("/api/update-policies", { credentials: "include" })
      .then(res => res.json())
      .then((data: UpdatePolicy[]) => setPolicies(data))
      .catch(() => {});
  }, []);

  const getPolicy = (name: string): UpdatePolicy | undefined =>
    policies.find(p => p.container === name);

  // Collapse state per container
  const [expandedRows, setExpandedRows] = useState<Record<string, boolean>>({});

  // ── Stack collapse state ────────────────────────────────────
  const [expandedStacks, setExpandedStacks] = useState<Record<string, boolean>>({});

  const isMobile = useMediaQuery("(max-width: 768px)");

  const toggleExpand = (name: string) => {
    setExpandedRows(prev => ({ ...prev, [name]: !prev[name] }));
  };

  // ── Search & sort ──────────────────────────────────────────
  const [searchQuery, setSearchQuery] = useState("");

  // ── State & update filters ────────────────────────────────
  const [stateFilter, setStateFilter] = useState<string[]>([]);
  const [showPendingUpdates, setShowPendingUpdates] = useState(false);

  const availableStates = useMemo(() => {
    const states = new Set(containers.map(c => c.state));
    return Array.from(states).sort();
  }, [containers]);

  const monitoringEnabled = config?.telegram_configured || config?.matrix_configured || config?.webhook_configured;

  const toggleContainerMonitor = async (name: string, monitored: boolean) => {
    try {
      const res = await apiFetch(`/api/containers/${encodeURIComponent(name)}/monitor`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ monitored }),
      });
      if (res.ok) {
        const updated: ContainerInfo[] = await res.json();
        setContainers(prev => prev.map(c => updated.find(u => u.name === c.name) || c));
      }
    } catch { /* ignore */ }
  };

  const toggleAllMonitor = async (monitored: boolean) => {
    try {
      const res = await apiFetch("/api/containers/monitor-all", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ monitored }),
      });
      if (res.ok) {
        const updated: ContainerInfo[] = await res.json();
        setContainers(prev => prev.map(c => updated.find(u => u.name === c.name) || c));
      }
    } catch { /* ignore */ }
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
    setContainers(prev => prev.map(c => ({ ...c, has_update: !!updatedUpdates[c.name] })));
    setCheckResults({ total: initialContainers.length, updated: checkUpdated, uptodate: checkUptodate, failed: checkFailed, errors: checkErrors });

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
      showToast(`${action} correcto ✅`, "green");
    } catch (e: any) {
      showToast(`error al ${action}: ${e.message}`, "red");
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
      showToast(`🗑️ ${name} — eliminado ✅`, "green");
    } catch (e: any) {
      showToast(`🗑️ ${name} — error: ${e.message}`, "red");
    }
  };

  // ── Stack-level actions ────────────────────────────────────

  const handleStackAction = async (project: string, items: ContainerInfo[], action: string) => {
    for (const c of items) {
      try {
        await apiFetch(`/api/containers/${encodeURIComponent(c.name)}/${action}`, { method: "POST" });
      } catch { /* ignore */ }
    }
    const labels: Record<string, string> = { start: "iniciados", stop: "parados", restart: "reiniciados" };
    showToast(`📦 ${project} — todos ${labels[action] || action} ✅`, "green");
  };

  // ── Derived data ───────────────────────────────────────────

  if (!containersLoaded)
    return (
      <Container py="xl">
        <Group justify="center">
          <Loader />
          <Text>Conectando...</Text>
        </Group>
      </Container>
    );

  const q = searchQuery.toLowerCase().trim();
  const filteredContainers = containers.filter((c) => {
    if (q && !(
      c.name.toLowerCase().includes(q) ||
      c.image.toLowerCase().includes(q) ||
      (c.compose_project || "").toLowerCase().includes(q) ||
      c.ports.some((p) => p.toLowerCase().includes(q)) ||
      c.state.toLowerCase().includes(q)
    )) return false;

    if (stateFilter.length > 0 && !stateFilter.includes(c.state)) return false;

    if (showPendingUpdates && !c.has_update) return false;

    return true;
  });

  const grouped = new Map<string, ContainerInfo[]>();
  const noStack: ContainerInfo[] = [];
  for (const c of filteredContainers) {
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

  // ── Inspect a container from context info ──────────────────
  const containerInfo = inspectName ? containers.find(c => c.name === inspectName) : null;

  // ── Status helpers ───────────────────────────────────────────
  const statusColor = (c: ContainerInfo) =>
    c.status.includes("healthy") ? "green" : c.state === "running" ? "blue" : "red";

  const statusDot = (c: ContainerInfo) => {
    const color = statusColor(c);
    return (
      <div
        style={{
          width: 10,
          height: 10,
          borderRadius: '50%',
          backgroundColor: `var(--mantine-color-${color}-6)`,
          flexShrink: 0,
        }}
      />
    );
  };

  

  // ── Expanded action buttons (icon + text) ────────────────────
  const renderActions = (c: ContainerInfo) => {
    const p = progress.get(c.name);
    const busy = isBusy;
    const btnSize = isMobile ? "sm" : "compact-sm";
    const policy = getPolicy(c.name);
    const policyAction = policy?.action || 'pull-restart';
    const policyLabels: Record<string, string> = {
      'none': '❌ No hacer nada',
      'pull': '⬇️ Solo pull',
      'pull-restart': '🔄 Pull + reiniciar',
      'pull-restart-stack': '📦 Pull + reiniciar stack',
    };

    const stackContainers = c.compose_project ? containers.filter(cc => cc.compose_project === c.compose_project) : [];
    const isMultiStack = stackContainers.length > 1;

    return (
      <Stack gap="xs">
        <Group gap={isMobile ? 8 : 6} wrap="wrap">
          <Button
            size={btnSize}
            variant="light"
            color="gray"
            leftSection="🔍"
            onClick={() => handleInspect(c.name)}
          >
            Inspeccionar
          </Button>
          <Button
            size={btnSize}
            variant="light"
            color="cyan"
            leftSection="🔍"
            onClick={async () => {
              try {
                const res = await apiFetch(`/api/check-update/${encodeURIComponent(c.name)}`, { method: "POST" });
                if (res.ok) {
                  const data = await res.json();
                  setContainers(prev => prev.map(pc => pc.name === c.name ? { ...pc, has_update: data.has_update === true } : pc));
                  showToast(data.has_update ? `📦 ${c.name} — actualización disponible ⬆️` : `✅ ${c.name} — ya está actualizado`, data.has_update ? "yellow" : "green");
                } else {
                  showToast(`🔍 ${c.name} — error al revisar`, "red");
                }
              } catch {
                showToast(`🔍 ${c.name} — error de conexión`, "red");
              }
            }}
            disabled={busy}
          >
            Revisar
          </Button>
          <Button
            size={btnSize}
            variant="light"
            color="orange"
            leftSection="🔄"
            onClick={() => handleContainerAction(c.name, "restart")}
            disabled={busy}
          >
            Reiniciar
          </Button>
          {c.state === "running" ? (
            <Button
              size={btnSize}
              variant="light"
              color="red"
              leftSection="⏹"
              onClick={() => handleContainerAction(c.name, "stop")}
              disabled={busy}
            >
              Parar
            </Button>
          ) : (
            <Button
              size={btnSize}
              variant="light"
              color="green"
              leftSection="▶"
              onClick={() => handleContainerAction(c.name, "start")}
              disabled={busy}
            >
              Iniciar
            </Button>
          )}
          {isMultiStack && (
            <>
              <Button
                size={btnSize}
                variant="light"
                color="red"
                leftSection="⏹"
                onClick={() => handleStackAction(c.compose_project!, stackContainers, "stop")}
                disabled={busy || !stackContainers.some(sc => sc.state === "running")}
              >
                Parar todos
              </Button>
              <Button
                size={btnSize}
                variant="light"
                color="orange"
                leftSection="🔄"
                onClick={() => handleStackAction(c.compose_project!, stackContainers, "restart")}
                disabled={busy}
              >
                Reiniciar todos
              </Button>
            </>
          )}
          <Button
            size={btnSize}
            variant="light"
            color="gray"
            leftSection="🗑"
            onClick={() => setConfirmDelete(c.name)}
            disabled={busy}
          >
            Eliminar
          </Button>
        </Group>
        {p && (
          <Group gap="xs">
            <Loader size="xs" />
            <Text size="xs" c="dimmed">{p.status}</Text>
          </Group>
        )}
        <Group gap="xs" justify="space-between" wrap="nowrap">
          <Text size="xs" c="dimmed">
            Política: {policyLabels[policyAction] || policyAction}
          </Text>
          <PolicyActionButton
            containerName={c.name}
            getPolicy={getPolicy}
            setPolicies={setPolicies}
            busy={busy}
            showToast={showToast}
          />
        </Group>
      </Stack>
    );
  };

  // ── Container card/row header ────────────────────────────────
  const renderHeader = (c: ContainerInfo) => {
    const hasUpdate = c.has_update;
    const isExpanded = !!expandedRows[c.name];
    return (
      <Group justify="space-between" wrap="nowrap" style={{ flex: 1, cursor: 'pointer' }} onClick={() => toggleExpand(c.name)}>
        <Group gap="xs" wrap="nowrap" style={{ flex: 1, minWidth: 0, overflow: 'hidden' }}>
          {statusDot(c)}
          {hasUpdate && <Badge size="xs" variant="filled" color="yellow" circle>!</Badge>}
          <div onClick={(e) => e.stopPropagation()}>
            <Switch
              size="xs"
              checked={c.monitored}
              disabled={!monitoringEnabled}
              onChange={(e) => { toggleContainerMonitor(c.name, e.currentTarget.checked); }}
              style={{ flexShrink: 0 }}
            />
          </div>
          <Text size="sm" fw={500} truncate style={{ minWidth: 60 }}>{c.name}</Text>
          <Text size="xs" c="dimmed" truncate style={{ minWidth: 60 }}>{c.status}</Text>
          {c.traefik_url && (
            <Anchor href={c.traefik_url} target="_blank" rel="noopener noreferrer" size="xs" truncate style={{ maxWidth: 180 }} onClick={(e) => e.stopPropagation()}>
              🔗 {c.traefik_url.replace(/^https?:\/\//, "")}
            </Anchor>
          )}
        </Group>
        <ActionIcon variant="subtle" color="gray" size="sm" style={{ flexShrink: 0 }}>
          {isExpanded ? "▲" : "▼"}
        </ActionIcon>
      </Group>
    );
  };

  // ── Mobile card view ────────────────────────────────────────
  const renderMobileCard = (c: ContainerInfo) => {
    const isExpanded = !!expandedRows[c.name];
    return (
      <Paper
        key={c.id}
        shadow="sm"
        p="sm"
        withBorder
      >
        <Stack gap={6}>
          {renderHeader(c)}
          <Collapse expanded={isExpanded}>
            <Paper p="sm" withBorder mt="xs" style={{ background: 'var(--mantine-color-dark-6)' }}>
              {renderActions(c)}
            </Paper>
          </Collapse>
        </Stack>
      </Paper>
    );
  };

  // ── Desktop row ─────────────────────────────────────────────
  const renderRow = (c: ContainerInfo) => {
    const isExpanded = !!expandedRows[c.name];
    return (
      <Table.Tr key={c.id}>
        <Table.Td colSpan={isExpanded ? 1 : 1} style={{ padding: 0, border: 'none' }}>
          <Stack gap={0}>
            <Paper p="sm" style={{ background: 'transparent' }}>
              {renderHeader(c)}
            </Paper>
            <Collapse expanded={isExpanded}>
              <Paper p="sm" withBorder mx="sm" mb="sm" style={{ background: 'var(--mantine-color-dark-6)' }}>
                {renderActions(c)}
              </Paper>
            </Collapse>
          </Stack>
        </Table.Td>
      </Table.Tr>
    );
  };

  // ── Group helpers ──────────────────────────────────────────
  const groupStats = (items: ContainerInfo[]) => {
    const running = items.filter(c => c.state === "running").length;
    return { running, total: items.length };
  };

  const toggleStackExpand = (project: string) => {
    setExpandedStacks(prev => ({ ...prev, [project]: !prev[project] }));
  };

  // ── Stack header ────────────────────────────────────────────
  const renderStackHeader = (project: string, items: ContainerInfo[]) => {
    const { running, total } = groupStats(items);
    const isExpanded = !!expandedStacks[project];

    return (
      <Stack gap="xs">
        <Group justify="space-between" wrap="nowrap" style={{ cursor: 'pointer' }} onClick={() => toggleStackExpand(project)}>
          <Group gap="xs" wrap="nowrap" style={{ minWidth: 0, flex: 1, overflow: 'hidden' }}>
            <Text size="md" fw={700} truncate>📦 {project}</Text>
            <Badge size="sm" variant="light" color={running === total ? "green" : "yellow"}>{running}/{total}</Badge>
          </Group>
          <ActionIcon variant="subtle" color="gray" size="sm" style={{ flexShrink: 0 }}>
            {isExpanded ? "▲" : "▼"}
          </ActionIcon>
        </Group>
      </Stack>
    );
  };

  // ── Group renderers ─────────────────────────────────────────

  const renderMobileGroup = (project: string, items: ContainerInfo[]) => {
    return (
      <Paper shadow="sm" withBorder mb="md" key={project}>
        <Stack px="md" py="sm" gap="sm">
          {renderStackHeader(project, items)}
          <Stack px="md" pb="md" gap="sm">
            {items.map(renderMobileCard)}
          </Stack>
        </Stack>
      </Paper>
    );
  };

  const renderGroup = (project: string, items: ContainerInfo[]) => {
    return (
      <Paper shadow="sm" withBorder mb="md" key={project}>
        <Stack gap={0}>
          <Paper p="sm" style={{ background: 'var(--mantine-color-dark-6)' }}>
            {renderStackHeader(project, items)}
          </Paper>
          <Table>
            <Table.Tbody>{items.map(renderRow)}</Table.Tbody>
          </Table>
        </Stack>
      </Paper>
    );
  };

// ── Batch progress bar ──────────────────────────────────────
  const renderBatchProgress = () => {
    const isUpdatePhase = batchPhase === 'updating';
    const total = isUpdatePhase
      ? checkResults.updated
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

  // ── Stats ──────────────────────────────────────────────────
  const statsRunning = containers.filter(c => c.state === "running").length;
  const statsStopped = containers.filter(c => c.state !== "running").length;
  const statsUpdates = containers.filter(c => c.has_update).length;

  const showToast = (message: string, color: string, title?: string) => {
    showNotification({
      title: title || "Alloy",
      message,
      color,
      autoClose: 3000,
      style: { borderLeft: `4px solid var(--mantine-color-${color}-6)` },
    });
  };

  return (
    <>
      {notifications.length > 0 && (
        <Paper mb="md" p="xs">
          <Text size="xs" c="dimmed" mb="xs">🔔 Notificaciones</Text>
          {notifications.slice(0, 4).map((n, i) => (
            <NotifToast key={i} notif={n} onDismiss={() => setNotifications((p) => p.filter((_, j) => j !== i))} />
          ))}
        </Paper>
      )}

      {/* Stats bar */}
      <SimpleGrid cols={{ base: 4 }} mb="md">
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder style={{ borderTop: '3px solid var(--mantine-color-blue-6)' }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{containers.length}</Text>
          <Text ta="center" size="xs" c="dimmed">Total</Text>
        </Paper>
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder style={{ borderTop: '3px solid var(--mantine-color-green-6)' }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{statsRunning}</Text>
          <Text ta="center" size="xs" c="dimmed">Running</Text>
        </Paper>
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder style={{ borderTop: '3px solid var(--mantine-color-red-6)' }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{statsStopped}</Text>
          <Text ta="center" size="xs" c="dimmed">Stopped</Text>
        </Paper>
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder style={{ borderTop: `3px solid var(--mantine-color-${statsUpdates > 0 ? 'yellow' : 'gray'}-6)` }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{statsUpdates}</Text>
          <Text ta="center" size="xs" c="dimmed">Updates</Text>
        </Paper>
      </SimpleGrid>

      {/* Batch progress bar (checking or updating) */}
      {batchPhase !== 'idle' && (
        <Paper shadow="sm" p="md" mb="md" withBorder>
          {renderBatchProgress()}
        </Paper>
      )}

      {/* Action buttons + search — only visible when idle */}
      {batchPhase === 'idle' && (
        <Paper shadow="sm" p="md" mb="md" withBorder>
          {isMobile ? (
            <Stack gap="sm">
              <TextInput
                placeholder="Buscar..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.currentTarget.value)}
                rightSection={searchQuery ? (
                  <ActionIcon variant="subtle" size="sm" onClick={() => setSearchQuery("")}>✕</ActionIcon>
                ) : undefined}
              />
              {availableStates.length > 0 && (
                <Chip.Group multiple value={stateFilter} onChange={setStateFilter}>
                  <Group gap="xs" wrap="wrap">
                    {availableStates.map(s => (
                      <Chip key={s} value={s} size="xs" variant="outline">{s}</Chip>
                    ))}
                  </Group>
                </Chip.Group>
              )}
              <Group gap="xs" wrap="wrap">
                <Switch
                  label="Solo updates"
                  checked={showPendingUpdates}
                  onChange={(e) => setShowPendingUpdates(e.currentTarget.checked)}
                  size="xs"
                />
                <Button onClick={checkAll} variant="light" color="cyan" size="xs" flex={1}>🔍 Check</Button>
                <Button onClick={updateAll} variant="light" color="yellow" size="xs" flex={1}>⬆️ Update</Button>
                {monitoringEnabled && (
                  <Button onClick={() => toggleAllMonitor(!containers.every(c => c.monitored))} variant="light" color="green" size="xs" flex={1}>{containers.every(c => c.monitored) ? '🔕 Desmon. todos' : '🔔 Mon. todos'}</Button>
                )}
              </Group>
            </Stack>
          ) : (
            <Stack gap="sm">
              <Group gap="md" wrap="nowrap" align="flex-end">
                <TextInput
                  placeholder="Buscar por nombre, imagen, stack..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.currentTarget.value)}
                  rightSection={searchQuery ? (
                    <ActionIcon variant="subtle" size="sm" onClick={() => setSearchQuery("")}>✕</ActionIcon>
                  ) : undefined}
                  style={{ flex: 1 }}
                />
                <Tooltip label="Comprobar todos contra registry">
                  <Button onClick={checkAll} variant="light" color="cyan" size="sm">🔍 Check</Button>
                </Tooltip>
                <Tooltip label="Actualizar todos los pendientes">
                  <Button onClick={updateAll} variant="light" color="yellow" size="sm">⬆️ Update</Button>
                </Tooltip>
                {monitoringEnabled && (
                  <Tooltip label={containers.every(c => c.monitored) ? "Desmonitorizar todos los contenedores" : "Monitorizar todos los contenedores"}>
                    <Button onClick={() => toggleAllMonitor(!containers.every(c => c.monitored))} variant="light" color="green" size="sm">{containers.every(c => c.monitored) ? '🔕 Desmon. todos' : '🔔 Mon. todos'}</Button>
                  </Tooltip>
                )}
              </Group>
              <Group gap="md" wrap="wrap" justify="space-between">
                <Group gap="xs" wrap="wrap">
                  {availableStates.length > 0 && (
                    <Chip.Group multiple value={stateFilter} onChange={setStateFilter}>
                      <Group gap="xs" wrap="wrap">
                        {availableStates.map(s => (
                          <Chip key={s} value={s} size="xs" variant="outline">{s}</Chip>
                        ))}
                      </Group>
                    </Chip.Group>
                  )}
                  <Switch
                    label="Solo pendientes de actualizar"
                    checked={showPendingUpdates}
                    onChange={(e) => setShowPendingUpdates(e.currentTarget.checked)}
                    size="xs"
                  />
                </Group>
                <Text size="xs" c="dimmed">{filteredContainers.length} / {containers.length} containers</Text>
              </Group>
            </Stack>
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
              <Table>
                <Table.Tbody>{noStack.map(renderRow)}</Table.Tbody>
              </Table>
            </Paper>
          )}
        </>
      )}

      {/* Inspect modal — enriched with image, ports, traefik, registry */}
      <Modal opened={inspectName !== null} onClose={() => { setInspectName(null); setInspectData(null); setInspectError(null); }} title={`🔍 ${inspectName || ""}`} size={isMobile ? "100%" : "xl"}>
        {inspectLoading ? (
          <Group justify="center" py="xl"><Loader /><Text>Obteniendo información...</Text></Group>
        ) : inspectError ? <Text c="red">{inspectError}</Text> : (
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
              {inspectData ? (
                <Stack gap="xs">
                  <Group><Text size="sm" fw={500} w={140}>ID:</Text><Text size="sm" style={{ fontFamily: "monospace", fontSize: 11 }}>{inspectData.id}</Text></Group>
                  <Group><Text size="sm" fw={500} w={140}>Nombre:</Text><Text size="sm">{inspectData.name}</Text></Group>
                  <Group><Text size="sm" fw={500} w={140}>Imagen:</Text><Text size="sm">{inspectData.image}</Text></Group>
                  <Group><Text size="sm" fw={500} w={140}>Creado:</Text><Text size="sm">{inspectData.created}</Text></Group>
                  <Group><Text size="sm" fw={500} w={140}>Estado:</Text><Badge color={inspectData.state === "running" ? "green" : "red"}>{inspectData.state}</Badge></Group>
                  <Group><Text size="sm" fw={500} w={140}>Status:</Text><Text size="sm">{inspectData.status}</Text></Group>
                  {inspectData.restart_policy && <Group><Text size="sm" fw={500} w={140}>Reinicio:</Text><Text size="sm">{inspectData.restart_policy}</Text></Group>}
                  {inspectData.health && <Group><Text size="sm" fw={500} w={140}>Health:</Text><Badge color={inspectData.health === "healthy" ? "green" : "yellow"}>{inspectData.health}</Badge></Group>}
                  <Divider my="xs" />
                  {containerInfo && (
                    <>
                      <Text size="sm" fw={500}>Información adicional</Text>
                      {containerInfo.ports.length > 0 && (
                        <Group><Text size="sm" fw={500} w={140}>Puertos:</Text><Group gap={4}>{containerInfo.ports.map((port, i) => <Code key={i} style={{ fontSize: 10 }}>{port}</Code>)}</Group></Group>
                      )}
                      {containerInfo.traefik_url && (
                        <Group><Text size="sm" fw={500} w={140}>Traefik:</Text><Anchor href={containerInfo.traefik_url} target="_blank" rel="noopener noreferrer" size="sm" truncate>{containerInfo.traefik_url}</Anchor></Group>
                      )}
                      {containerInfo.registry_url && (
                        <Group><Text size="sm" fw={500} w={140}>Registry:</Text><Anchor href={containerInfo.registry_url} target="_blank" rel="noopener noreferrer" size="sm" truncate>Ver en registry</Anchor></Group>
                      )}
                      <Group><Text size="sm" fw={500} w={140}>Tag:</Text><Text size="sm">{containerInfo.image_tag}</Text></Group>
                      <Group><Text size="sm" fw={500} w={140}>Size:</Text><Text size="sm">{containerInfo.size_mb > 0 ? `${containerInfo.size_mb} MB` : "-"}</Text></Group>
                    </>
                  )}
                </Stack>
              ) : (
                <Text size="sm" c="dimmed" py="md">Selecciona un container para inspeccionar</Text>
              )}
            </Tabs.Panel>
            <Tabs.Panel value="ports">
              {inspectData?.ports?.length ? (
                <Table.ScrollContainer minWidth={400}><Table striped><Table.Thead><Table.Tr><Table.Th>Puerto Privado</Table.Th><Table.Th>Puerto Público</Table.Th><Table.Th>Tipo</Table.Th></Table.Tr></Table.Thead><Table.Tbody>{inspectData.ports.map((p: any, i: number) => (
                  <Table.Tr key={i}><Table.Td>{p.private_port}</Table.Td><Table.Td>{p.public_port != null ? p.public_port : "-"}</Table.Td><Table.Td>{p.type}</Table.Td></Table.Tr>
                ))}</Table.Tbody></Table></Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin puertos expuestos</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="volumes">
              {inspectData?.mounts?.length ? (
                <Table.ScrollContainer minWidth={400}><Table striped><Table.Thead><Table.Tr><Table.Th>Origen</Table.Th><Table.Th>Destino</Table.Th><Table.Th>Modo</Table.Th></Table.Tr></Table.Thead><Table.Tbody>{inspectData.mounts.map((m: any, i: number) => (
                  <Table.Tr key={i}><Table.Td><Text size="sm">{m.source}</Text></Table.Td><Table.Td><Text size="sm">{m.destination}</Text></Table.Td><Table.Td><Badge variant="light">{m.mode}</Badge></Table.Td></Table.Tr>
                ))}</Table.Tbody></Table></Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin volúmenes montados</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="networks">
              {inspectData?.networks?.length ? (
                <Table.ScrollContainer minWidth={400}><Table striped><Table.Thead><Table.Tr><Table.Th>Red</Table.Th><Table.Th>IP</Table.Th><Table.Th>Gateway</Table.Th></Table.Tr></Table.Thead><Table.Tbody>{inspectData.networks.map((n: any, i: number) => (
                  <Table.Tr key={i}><Table.Td><Text size="sm">{n.name}</Text></Table.Td><Table.Td><Code>{n.ip_address}</Code></Table.Td><Table.Td><Code>{n.gateway}</Code></Table.Td></Table.Tr>
                ))}</Table.Tbody></Table></Table.ScrollContainer>
              ) : <Text size="sm" c="dimmed" py="md">Sin redes</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="env">
              {inspectData?.env?.length ? (
                <ScrollArea h={300}><Code block>{inspectData.env.map((e: string, i: number) => <div key={i}>{e}</div>)}</Code></ScrollArea>
              ) : <Text size="sm" c="dimmed" py="md">Sin variables de entorno</Text>}
            </Tabs.Panel>
            <Tabs.Panel value="labels">
              {inspectData?.labels && Object.keys(inspectData.labels).length > 0 ? (
                <ScrollArea h={300}>{Object.entries(inspectData.labels).map(([k, v]) => (
                  <Group key={k} gap="xs" mb="xs"><Text size="sm" fw={500}>{k}:</Text><Text size="sm">{v}</Text></Group>
                ))}</ScrollArea>
              ) : <Text size="sm" c="dimmed" py="md">Sin labels</Text>}
            </Tabs.Panel>
          </Tabs>
        )}
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