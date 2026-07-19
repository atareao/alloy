import { useState, useRef, useMemo, useEffect } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  Badge,
  Button,
  Collapse,
  Group,
  Modal,
  Paper,
  SimpleGrid,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { showNotification } from "@mantine/notifications";
import type {
  ContainerInfo,
  UpdateProgress,
  InspectData,
  UpdatePolicy,
} from "../types";
import { apiFetch } from "../api";
import ContainerTable from "./ContainerTable";
import ContainerRow from "./ContainerRow";
import BatchProgress from "./BatchProgress";
import InspectModal from "./InspectModal";
import LogsModal from "./LogsModal";
import SummaryDialog from "./SummaryDialog";

// ── Props ────────────────────────────────────────────────────
interface DashboardPageProps {
  containers: ContainerInfo[];
  setContainers: React.Dispatch<React.SetStateAction<ContainerInfo[]>>;
  progress: Map<string, UpdateProgress>;
  containersLoaded: boolean;
}

// ── Types ────────────────────────────────────────────────────
type CheckAllPhase = "idle" | "checking" | "updating";

interface CheckAllResults {
  total: number;
  updated: number;
  uptodate: number;
  failed: number;
  done: number;
  errors: string[];
}

export default function DashboardPage({
  containers,
  setContainers,
  progress,
}: DashboardPageProps) {
  const isMobile = useMediaQuery("(max-width: 768px)");

  // ── State ─────────────────────────────────────────────────
  const [inspectName, setInspectName] = useState<string | null>(null);
  const [inspectData, setInspectData] = useState<InspectData | null>(null);
  const [inspectLoading, setInspectLoading] = useState(false);
  const [inspectError, setInspectError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [logsContainer, setLogsContainer] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [logSearch, setLogSearch] = useState("");
  const [logWrap, setLogWrap] = useState(false);
  const [logError, setLogError] = useState<string | null>(null);
  const [logTimeout, setLogTimeout] = useState(false);
  const [loadingActions, setLoadingActions] = useState<Record<string, string>>({});
  const [batchPhase, setBatchPhase] = useState<CheckAllPhase>("idle");
  const [batchProgress, setBatchProgress] = useState({ current: 0, total: 0 });
  const [batchCurrentItem, setBatchCurrentItem] = useState("");
  const cancelBatchRef = useRef(false);
  const pendingTotalRef = useRef(0);
  const [checkResults, setCheckResults] = useState<CheckAllResults>({
    total: 0, updated: 0, uptodate: 0, failed: 0, done: 0, errors: [],
  });
  const [updateResults, setUpdateResults] = useState<CheckAllResults>({
    total: 0, updated: 0, uptodate: 0, done: 0, failed: 0, errors: [],
  });
  const [showSummary, setShowSummary] = useState(false);
  const [policies, setPolicies] = useState<UpdatePolicy[]>([]);
  const [expandedRows, setExpandedRows] = useState<Record<string, boolean>>({});
  const [expandedStacks, setExpandedStacks] = useState<Record<string, boolean>>({});
  const [searchQuery, setSearchQuery] = useState("");
  const [stateFilter, setStateFilter] = useState<string[]>([]);
  const [showPendingUpdates, setShowPendingUpdates] = useState(false);

  // ── Computed ──────────────────────────────────────────────
  const containerInfo = useMemo(() => {
    if (!inspectName) return null;
    return containers.find((c) => c.name === inspectName) || null;
  }, [inspectName, containers]);

  const availableStates = useMemo(
    () => Array.from(new Set(containers.map((c) => c.state))).sort(),
    [containers],
  );

  const filteredContainers = useMemo(() => {
    return containers.filter((c) => {
      if (showPendingUpdates && !c.has_update) return false;
      if (stateFilter.length > 0 && !stateFilter.includes(c.state)) return false;
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        return (
          c.name.toLowerCase().includes(q) ||
          c.image.toLowerCase().includes(q) ||
          (c.compose_project || "").toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [containers, searchQuery, stateFilter, showPendingUpdates]);

  const { sortedGroups, noStack } = useMemo(() => {
    const grouped: Record<string, ContainerInfo[]> = {};
    const ungrouped: ContainerInfo[] = [];
    for (const c of filteredContainers) {
      if (c.compose_project) {
        (grouped[c.compose_project] ||= []).push(c);
      } else {
        ungrouped.push(c);
      }
    }
    const sorted = Object.entries(grouped).sort(([a], [b]) => a.localeCompare(b));
    return { sortedGroups: sorted, noStack: ungrouped };
  }, [filteredContainers]);

  const statsRunning = containers.filter((c) => c.state === "running").length;
  const statsStopped = containers.filter((c) => c.state !== "running").length;
  const statsUpdates = containers.filter((c) => c.has_update).length;

  // ── Helpers ───────────────────────────────────────────────
  const getPolicy = (name: string): UpdatePolicy | undefined =>
    policies.find((p) => p.container === name);

  const toggleExpand = (name: string) => {
    setExpandedRows((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  const toggleStackExpand = (project: string) => {
    setExpandedStacks((prev) => ({ ...prev, [project]: !prev[project] }));
  };

  const showToast = (message: string, color: string, title?: string) => {
    showNotification({
      title: title || "Alloy",
      message,
      color,
      autoClose: 3000,
      style: { borderLeft: `4px solid var(--mantine-color-${color}-6)` },
    });
  };

  // ── Effects ───────────────────────────────────────────────
  useEffect(() => {
    if (batchPhase === "idle") return;
    let doneCount = 0;
    let currentItem = "";
    progress.forEach((p) => {
      if (batchPhase === "updating") {
        const isUpdate =
          p.status.startsWith("🔄") ||
          p.status.startsWith("✅ actualizado") ||
          p.status.startsWith("✅ pulled") ||
          p.status.startsWith("✅ stack") ||
          p.status.startsWith("⚠️") ||
          p.status.startsWith("📥") ||
          p.status.startsWith("✅ Updated");
        if (!isUpdate) return;
      }
      if (p.done) doneCount++;
      else if (currentItem === "") currentItem = p.container;
    });
    setBatchProgress((prev) =>
      doneCount !== prev.current ? { ...prev, current: doneCount } : prev,
    );
    if (currentItem) setBatchCurrentItem(currentItem);
    if (
      batchPhase === "updating" &&
      doneCount > 0 &&
      doneCount >= pendingTotalRef.current
    ) {
      setTimeout(() => {
        setBatchPhase("idle");
        setShowSummary(true);
      }, 1500);
    }
  }, [progress, batchPhase]);

  useEffect(() => {
    fetch("/api/update-policies", { credentials: "include" })
      .then((res) => res.json())
      .then((data: UpdatePolicy[]) => setPolicies(data))
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!logsContainer) return;
    setLogs([]);
    setLogError(null);
    setLogTimeout(false);
    const timeoutId = setTimeout(() => {
      if (logs.length === 0) setLogTimeout(true);
    }, 5000);
    const evtSource = new EventSource(
      `/api/containers/${encodeURIComponent(logsContainer)}/logs`,
      { withCredentials: true },
    );
    evtSource.addEventListener("log", (e: Event) => {
      setLogs((prev) => [...prev, (e as MessageEvent).data].slice(-500));
    });
    evtSource.addEventListener("error", (e: Event) => {
      setLogError((e as MessageEvent).data || "Error del servidor");
      evtSource.close();
    });
    evtSource.onerror = () => {
      setLogError("Conexión perdida");
      evtSource.close();
    };
    return () => {
      clearTimeout(timeoutId);
      evtSource.close();
    };
  }, [logsContainer]);

  // ── Handlers ──────────────────────────────────────────────
  const handleContainerAction = async (
    name: string,
    action: string,
    label: string,
  ) => {
    setLoadingActions((prev) => ({ ...prev, [name]: label }));
    try {
      const res = await apiFetch(
        `/api/containers/${encodeURIComponent(name)}/${action}`,
        { method: "POST" },
      );
      if (!res.ok) throw new Error((await res.text()) || `Error al ${action}`);
      showToast(`${action} correcto ✅`, "green");
    } catch (e: any) {
      showToast(`error al ${action}: ${e.message}`, "red");
    } finally {
      setLoadingActions((prev) => {
        const next = { ...prev };
        delete next[name];
        return next;
      });
    }
  };

  const handleInspect = async (name: string) => {
    setInspectName(name);
    setInspectData(null);
    setInspectLoading(true);
    setInspectError(null);
    try {
      const res = await apiFetch(
        `/api/containers/${encodeURIComponent(name)}/inspect`,
      );
      if (!res.ok) throw new Error("Error al inspeccionar");
      setInspectData(await res.json());
    } catch {
      setInspectError("No se pudo obtener información del container");
    }
    setInspectLoading(false);
  };

  const handleLogs = (name: string) => {
    setLogsContainer(name);
    setLogs([]);
  };

  const handleRemove = async (name: string) => {
    setConfirmDelete(null);
    try {
      const res = await apiFetch(
        `/api/containers/${encodeURIComponent(name)}/remove`,
        { method: "POST" },
      );
      if (!res.ok) throw new Error((await res.text()) || "Error al eliminar");
      showToast(`🗑️ ${name} — eliminado ✅`, "green");
    } catch (e: any) {
      showToast(`🗑️ ${name} — error: ${e.message}`, "red");
    }
  };

  const handleStackAction = async (
    project: string,
    items: ContainerInfo[],
    action: string,
    label: string,
  ) => {
    setLoadingActions((prev) => ({ ...prev, [project]: label }));
    for (const c of items) {
      try {
        await apiFetch(
          `/api/containers/${encodeURIComponent(c.name)}/${action}`,
          { method: "POST" },
        );
      } catch {
        /* ignore */
      }
    }
    const labels: Record<string, string> = {
      start: "iniciados",
      stop: "parados",
      restart: "reiniciados",
    };
    showToast(`📦 ${project} — todos ${labels[action] || action} ✅`, "green");
    setLoadingActions((prev) => {
      const next = { ...prev };
      delete next[project];
      return next;
    });
  };

  const checkAll = async () => {
    cancelBatchRef.current = false;
    setBatchPhase("checking");
    setCheckResults({ total: 0, updated: 0, uptodate: 0, failed: 0, done: 0, errors: [] });
    setUpdateResults({ total: 0, updated: 0, uptodate: 0, done: 0, failed: 0, errors: [] });
    setBatchProgress({ current: 0, total: containers.length });
    setBatchCurrentItem("🔍 Verificando...");
    setShowSummary(false);
    let updatedCount = 0;
    let uptodateCount = 0;
    let failedCount = 0;
    const errors: string[] = [];
    try {
      const res = await apiFetch("/api/check-all", { method: "POST" });
      if (res.ok) {
        const updated: ContainerInfo[] = await res.json();
        setContainers((prev) =>
          prev.map((c) => updated.find((u) => u.name === c.name) || c),
        );
        updatedCount = updated.filter((c) => c.has_update).length;
        uptodateCount = updated.filter((c) => !c.has_update).length;
      } else {
        failedCount = containers.length;
        errors.push(`HTTP ${res.status}`);
      }
    } catch (e: any) {
      failedCount = containers.length;
      errors.push(`${e.message || "unknown error"}`);
    }
    setCheckResults({ total: containers.length, updated: updatedCount, uptodate: uptodateCount, failed: failedCount, done: 0, errors });
    setBatchProgress({ current: containers.length, total: containers.length });
    setBatchCurrentItem("");
    if (updatedCount > 0) {
      setBatchPhase("updating");
      pendingTotalRef.current = updatedCount;
      setBatchProgress({ current: 0, total: updatedCount });
      setBatchCurrentItem("⬆️ Aplicando políticas...");
    } else {
      setTimeout(() => {
        setBatchPhase("idle");
        setShowSummary(true);
      }, 500);
    }
  };

  // ── Render props ──────────────────────────────────────────
  const renderRow = (c: ContainerInfo) => (
    <ContainerRow
      container={c}
      isMobile={isMobile}
      expanded={!!expandedRows[c.name]}
      onToggleExpand={toggleExpand}
      progress={progress}
      getPolicy={getPolicy}
      setPolicies={setPolicies}
      loadingActions={loadingActions}
      batchPhase={batchPhase}
      containers={containers}
      onInspect={handleInspect}
      onLogs={handleLogs}
      onStart={(name) => handleContainerAction(name, "start", "Iniciando...")}
      onStop={(name) => handleContainerAction(name, "stop", "Parando...")}
      onRestart={(name) => handleContainerAction(name, "restart", "Reiniciando...")}
      onRemove={(name) => setConfirmDelete(name)}
      onStackAction={handleStackAction}
    />
  );

  const renderGroup = (project: string, items: ContainerInfo[]) => {
    const running = items.filter((c) => c.state === "running").length;
    if (isMobile) {
      const isExpanded = !!expandedStacks[project];
      return (
        <Paper
          shadow="sm"
          withBorder
          key={project}
          style={{
            height: "100%",
            width: "100%",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <Paper
            p="xs"
            style={{
              background: "var(--mantine-color-dark-6)",
              cursor: "pointer",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              textAlign: "center",
              overflow: "hidden",
              flex: isExpanded ? undefined : 1,
            }}
            onClick={() => toggleStackExpand(project)}
          >
            <Text size="sm" fw={700} truncate ta="center" mb={4} w="100%">
              📦 {project}
            </Text>
            <Badge
              size="sm"
              variant="light"
              color={running === items.length ? "green" : "yellow"}
            >
              {running}/{items.length}
            </Badge>
          </Paper>
          <Collapse expanded={isExpanded}>
            <div style={{ overflowX: "auto", width: "100%" }}>
              <Table>
                <Table.Tbody>
                  {items.map((c) => (
                  <ContainerRow
                  key={c.id}
                  container={c}
                  isMobile={isMobile}
                  expanded={!!expandedRows[c.name]}
                  onToggleExpand={toggleExpand}
                  progress={progress}
                  getPolicy={getPolicy}
                  setPolicies={setPolicies}
                  loadingActions={loadingActions}
                  batchPhase={batchPhase}
                  containers={containers}
                  onInspect={handleInspect}
                  onLogs={handleLogs}
                  onStart={(name) => handleContainerAction(name, "start", "Iniciando...")}
                  onStop={(name) => handleContainerAction(name, "stop", "Parando...")}
                  onRestart={(name) => handleContainerAction(name, "restart", "Reiniciando...")}
                  onRemove={(name) => setConfirmDelete(name)}
                  onStackAction={handleStackAction}
                />
              ))}
              </Table.Tbody>
            </Table>
            </div>
          </Collapse>
        </Paper>
      );
    }
    return (
      <Paper shadow="sm" withBorder mb="md" key={project}>
        <Stack gap={0}>
          <Paper p="sm" style={{ background: "var(--mantine-color-dark-6)" }}>
            <Group gap="xs" wrap="nowrap" style={{ minWidth: 0, flex: 1, overflow: "hidden" }}>
              <Text size="md" fw={700} truncate>📦 {project}</Text>
              <Badge size="sm" variant="light" color={running === items.length ? "green" : "yellow"}>
                {running}/{items.length}
              </Badge>
            </Group>
          </Paper>
          <Table>
            <Table.Tbody>{items.map(renderRow)}</Table.Tbody>
          </Table>
        </Stack>
      </Paper>
    );
  };

  // ── Main render ───────────────────────────────────────────
  return (
    <>
      {/* Stats bar */}
      <SimpleGrid cols={{ base: 4 }} mb="md">
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder
          style={{ borderTop: "3px solid var(--mantine-color-blue-6)" }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{containers.length}</Text>
          <Text ta="center" size="xs" c="dimmed">Total</Text>
        </Paper>
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder
          style={{ borderTop: "3px solid var(--mantine-color-green-6)" }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{statsRunning}</Text>
          <Text ta="center" size="xs" c="dimmed">Running</Text>
        </Paper>
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder
          style={{ borderTop: "3px solid var(--mantine-color-red-6)" }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{statsStopped}</Text>
          <Text ta="center" size="xs" c="dimmed">Stopped</Text>
        </Paper>
        <Paper shadow="sm" p={isMobile ? "xs" : "sm"} withBorder
          style={{ borderTop: `3px solid var(--mantine-color-${statsUpdates > 0 ? "yellow" : "gray"}-6)` }}>
          <Text ta="center" size={isMobile ? "sm" : "xl"} fw={700}>{statsUpdates}</Text>
          <Text ta="center" size="xs" c="dimmed">Updates</Text>
        </Paper>
      </SimpleGrid>

      {/* Batch progress */}
      <BatchProgress
        phase={batchPhase}
        batchProgress={batchProgress}
        batchCurrentItem={batchCurrentItem}
        checkResults={checkResults}
        updateResults={updateResults}
        onCancel={() => { cancelBatchRef.current = true; }}
      />

      {/* Container table (search + filters + groups) */}
      {batchPhase === "idle" && (
        <ContainerTable
          sortedGroups={sortedGroups}
          noStack={noStack}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          stateFilter={stateFilter}
          setStateFilter={setStateFilter}
          showPendingUpdates={showPendingUpdates}
          setShowPendingUpdates={setShowPendingUpdates}
          availableStates={availableStates}
          isMobile={isMobile}
          onCheckAll={checkAll}
          expandedStacks={expandedStacks}
          renderGroup={renderGroup}
          renderRow={renderRow}
        />
      )}

      {/* Inspect modal */}
      <InspectModal
        opened={inspectName !== null}
        onClose={() => {
          setInspectName(null);
          setInspectData(null);
          setInspectError(null);
        }}
        containerName={inspectName}
        containerInfo={
          inspectName && containerInfo
            ? { image: containerInfo.image, image_tag: containerInfo.image_tag, registry_url: containerInfo.registry_url }
            : null
        }
        inspectData={inspectData}
        loading={inspectLoading}
        error={inspectError}
      />

      {/* Confirm delete modal */}
      <Modal
        opened={confirmDelete !== null}
        onClose={() => setConfirmDelete(null)}
        title="🗑️ Confirmar Eliminación"
        size="sm"
      >
        <Text mb="md">
          ¿Seguro que quieres eliminar <b>{confirmDelete}</b>? Esta acción no se
          puede deshacer.
        </Text>
        <Group justify="flex-end">
          <Button variant="default" onClick={() => setConfirmDelete(null)}>
            Cancelar
          </Button>
          <Button
            color="red"
            onClick={() => confirmDelete && handleRemove(confirmDelete)}
          >
            Eliminar
          </Button>
        </Group>
      </Modal>

      {/* Summary dialog */}
      <SummaryDialog
        opened={showSummary}
        onClose={() => setShowSummary(false)}
        checkResults={checkResults}
        updateResults={updateResults}
        phase={batchPhase}
      />

      {/* Logs modal */}
      <LogsModal
        opened={logsContainer !== null}
        onClose={() => {
          setLogsContainer(null);
          setLogs([]);
          setLogError(null);
          setLogTimeout(false);
        }}
        containerName={logsContainer}
        logs={logs}
        logSearch={logSearch}
        setLogSearch={setLogSearch}
        logWrap={logWrap}
        setLogWrap={setLogWrap}
        logError={logError}
        logTimeout={logTimeout}
      />
    </>
  );
}