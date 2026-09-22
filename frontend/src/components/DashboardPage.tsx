import { useState, useMemo, useEffect } from "react";
import { useMediaQuery } from "../useMediaQuery";
import {
  Button,
  Card,
  Flex,
  Modal,
  Row,
  Col,
  Tag,
  Typography,
  notification,
} from "antd";
import { DeleteOutlined, AppstoreOutlined } from "@ant-design/icons";
import type {
  ContainerInfo,
  ContainerSummary,
  BatchProgress,
  InspectData,
  UpdatePolicy,
  UpdateCheckConfig,
} from "../types";
import { apiFetch } from "../api";
import ContainerTable from "./ContainerTable";
import ContainerRow from "./ContainerRow";
import InspectModal from "./InspectModal";
import LogsModal from "./LogsModal";
import SummaryDialog from "./SummaryDialog";
import type { BatchResults } from "./BatchProgress";

// ── Props ────────────────────────────────────────────────────
interface DashboardPageProps {
  containers: ContainerInfo[];
  setContainers: React.Dispatch<React.SetStateAction<ContainerInfo[]>>;
  progress: BatchProgress;
  containersLoaded: boolean;
  // Batch state (managed in App to survive tab switches)
  batchPhase: "idle" | "active";
  checkResults: BatchResults;
  updateResults: BatchResults;
  showSummary: boolean;
  setShowSummary: (v: boolean) => void;
  checkConfig: UpdateCheckConfig | null;
  onCheckAll: () => void;
  summary: ContainerSummary;
}

export default function DashboardPage({
  containers,
  progress,
  batchPhase,
  checkResults,
  updateResults,
  showSummary,
  setShowSummary,
  checkConfig,
  onCheckAll,
  summary,
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
  const [loadingActions, setLoadingActions] = useState<Record<string, string>>(
    {},
  );
  const [policies, setPolicies] = useState<UpdatePolicy[]>([]);
  const [expandedRows, setExpandedRows] = useState<Record<string, boolean>>({});
  const [expandedStacks, setExpandedStacks] = useState<Record<string, boolean>>(
    {},
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [stateFilter, setStateFilter] = useState<string[]>([]);
  const [showPendingUpdates, setShowPendingUpdates] = useState(false);

  // ── Computed ──────────────────────────────────────────────
  const containerInfo = useMemo(() => {
    if (!inspectName) return null;
    return containers.find((c) => c.name === inspectName) || null;
  }, [inspectName, containers]);

  // Global last/next check times — from update check config API
  const globalLastCheck = useMemo(() => {
    return checkConfig?.last_run_at ?? null;
  }, [checkConfig]);

  const globalNextCheck = useMemo(() => {
    return checkConfig?.next_run_at ?? null;
  }, [checkConfig]);

  const availableStates = useMemo(
    () => Array.from(new Set(containers.map((c) => c.state))).sort(),
    [containers],
  );

  const filteredContainers = useMemo(() => {
    return containers.filter((c) => {
      if (showPendingUpdates && !c.has_update) return false;
      if (stateFilter.length > 0 && !stateFilter.includes(c.state))
        return false;
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
    const sorted = Object.entries(grouped).sort(([a], [b]) =>
      a.localeCompare(b),
    );
    return { sortedGroups: sorted, noStack: ungrouped };
  }, [filteredContainers]);

  const statsRunning = summary.running;
  const statsStopped = summary.stopped;
  const statsUpdates = summary.with_updates;

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
    const typeMap: Record<string, "success" | "error" | "warning" | "info"> = {
      green: "success",
      red: "error",
      yellow: "warning",
    };
    const type = typeMap[color] || "info";
    notification[type]({
      message: title || "Alloy",
      description: message,
      duration: 3,
    });
  };

  // ── Effects ───────────────────────────────────────────────
  useEffect(() => {
    apiFetch("/api/update-policies")
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
      setLogTimeout(true);
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
      onRestart={(name) =>
        handleContainerAction(name, "restart", "Reiniciando...")
      }
      onRemove={(name) => setConfirmDelete(name)}
      onStackAction={handleStackAction}
    />
  );

  const renderGroup = (project: string, items: ContainerInfo[]) => {
    const running = items.filter((c) => c.state === "running").length;
    if (isMobile) {
      const isExpanded = !!expandedStacks[project];
      return (
        <Card
          key={project}
          size="small"
          style={{ height: "100%" }}
          styles={{
            body: {
              height: "100%",
              width: "100%",
              display: "flex",
              flexDirection: "column",
              padding: 0,
            },
          }}
        >
          <div
            style={{
              background: "transparent",
              cursor: "pointer",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              textAlign: "center",
              overflow: "hidden",
              flex: isExpanded ? undefined : 1,
              padding: isMobile ? 6 : 10,
            }}
            onClick={() => toggleStackExpand(project)}
          >
            <Typography.Text strong style={{ fontSize: 14 }} ellipsis>
              <AppstoreOutlined /> {project}
            </Typography.Text>
            <div style={{ marginTop: 4 }}>
              <Tag color={running === items.length ? "success" : "warning"}>
                {running}/{items.length}
              </Tag>
            </div>
          </div>
          {isExpanded && (
            <div>
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
                  onStart={(name) =>
                    handleContainerAction(name, "start", "Iniciando...")
                  }
                  onStop={(name) =>
                    handleContainerAction(name, "stop", "Parando...")
                  }
                  onRestart={(name) =>
                    handleContainerAction(name, "restart", "Reiniciando...")
                  }
                  onRemove={(name) => setConfirmDelete(name)}
                  onStackAction={handleStackAction}
                />
              ))}
            </div>
          )}
        </Card>
      );
    }
    return (
      <Card bordered size="small" key={project} style={{ marginBottom: 16 }}>
        <div
          style={{
            padding: "8px 12px",
            background: "var(--ant-color-bg-layout)",
          }}
        >
          <Flex
            gap="small"
            wrap="nowrap"
            align="center"
            style={{ minWidth: 0, flex: 1, overflow: "hidden" }}
          >
            <Typography.Text strong style={{ fontSize: 14 }} ellipsis>
              <AppstoreOutlined /> {project}
            </Typography.Text>
            <Tag color={running === items.length ? "success" : "warning"}>
              {running}/{items.length}
            </Tag>
          </Flex>
        </div>
        <table style={{ width: "100%" }}>
          <tbody>{items.map(renderRow)}</tbody>
        </table>
      </Card>
    );
  };

  // ── Main render ───────────────────────────────────────────
  return (
    <>
      {/* Stats bar */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col span={6}>
          <Card
            size="small"
            style={{ borderTop: "3px solid var(--ant-color-primary)" }}
          >
            <Typography.Text
              strong
              style={{
                textAlign: "center",
                display: "block",
                fontSize: isMobile ? 14 : 20,
              }}
            >
              {containers.length}
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ textAlign: "center", display: "block", fontSize: 12 }}
            >
              Total
            </Typography.Text>
          </Card>
        </Col>
        <Col span={6}>
          <Card
            size="small"
            style={{ borderTop: "3px solid var(--ant-color-success)" }}
          >
            <Typography.Text
              strong
              style={{
                textAlign: "center",
                display: "block",
                fontSize: isMobile ? 14 : 20,
              }}
            >
              {statsRunning}
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ textAlign: "center", display: "block", fontSize: 12 }}
            >
              Running
            </Typography.Text>
          </Card>
        </Col>
        <Col span={6}>
          <Card
            size="small"
            style={{ borderTop: "3px solid var(--ant-color-error)" }}
          >
            <Typography.Text
              strong
              style={{
                textAlign: "center",
                display: "block",
                fontSize: isMobile ? 14 : 20,
              }}
            >
              {statsStopped}
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ textAlign: "center", display: "block", fontSize: 12 }}
            >
              Stopped
            </Typography.Text>
          </Card>
        </Col>
        <Col span={6}>
          <Card
            size="small"
            style={{
              borderTop: `3px solid ${statsUpdates > 0 ? "var(--ant-color-warning)" : "var(--ant-color-text-quaternary)"}`,
            }}
          >
            <Typography.Text
              strong
              style={{
                textAlign: "center",
                display: "block",
                fontSize: isMobile ? 14 : 20,
              }}
            >
              {statsUpdates}
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ textAlign: "center", display: "block", fontSize: 12 }}
            >
              Updates
            </Typography.Text>
          </Card>
        </Col>
      </Row>

      {/* Container table (search + filters + groups) */}
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
        onCheckAll={onCheckAll}
        lastCheck={globalLastCheck}
        nextCheck={globalNextCheck}
        expandedStacks={expandedStacks}
        renderGroup={renderGroup}
        renderRow={renderRow}
        batchPhase={batchPhase}
      />

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
            ? {
                image: containerInfo.image,
                image_tag: containerInfo.image_tag,
                registry_url: containerInfo.registry_url,
              }
            : null
        }
        inspectData={inspectData}
        loading={inspectLoading}
        error={inspectError}
      />

      {/* Confirm delete modal */}
      <Modal
        open={confirmDelete !== null}
        onCancel={() => setConfirmDelete(null)}
        title={
          <>
            <DeleteOutlined /> Confirmar Eliminación
          </>
        }
        width={400}
        footer={
          <Flex justify="flex-end" gap="small">
            <Button onClick={() => setConfirmDelete(null)}>Cancelar</Button>
            <Button
              danger
              type="primary"
              onClick={() => confirmDelete && handleRemove(confirmDelete)}
            >
              Eliminar
            </Button>
          </Flex>
        }
      >
        <Typography.Text>
          ¿Seguro que quieres eliminar <b>{confirmDelete}</b>? Esta acción no se
          puede deshacer.
        </Typography.Text>
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
