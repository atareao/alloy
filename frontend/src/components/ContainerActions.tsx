import { Button, Flex, Space, Spin, Switch, Typography } from "antd";
import {
  SearchOutlined,
  ReloadOutlined,
  StopOutlined,
  PlayCircleOutlined,
  FileTextOutlined,
  DeleteOutlined,
} from "@ant-design/icons";
import type { ContainerInfo, UpdateProgress, UpdatePolicy } from "../types";
import { apiFetch } from "../api";
import PolicyActionButton from "./PolicyActionButton";

interface ContainerActionsProps {
  container: ContainerInfo;
  isMobile: boolean;
  progress: Map<string, UpdateProgress>;
  batchPhase: string;
  loadingActions: Record<string, string>;
  containers: ContainerInfo[];
  getPolicy: (name: string) => UpdatePolicy | undefined;
  setPolicies: React.Dispatch<React.SetStateAction<UpdatePolicy[]>>;
  onInspect: (name: string) => void;
  onLogs: (name: string) => void;
  onStart: (name: string) => void;
  onStop: (name: string) => void;
  onRestart: (name: string) => void;
  onRemove: (name: string) => void;
  onStackAction: (
    project: string,
    items: ContainerInfo[],
    action: string,
    label: string,
  ) => void;
  showToast: (message: string, color: string) => void;
}

export default function ContainerActions({
  container,
  isMobile,
  progress,
  batchPhase,
  loadingActions,
  containers,
  getPolicy,
  setPolicies,
  onInspect,
  onLogs,
  onStart,
  onStop,
  onRestart,
  onRemove,
  onStackAction,
  showToast,
}: ContainerActionsProps) {
  const c = container;
  const p = progress.get(c.name);
  const busy = batchPhase !== "idle" || !!p;
  const btnSize = isMobile ? "small" : "small";
  const policy = getPolicy(c.name);
  const policyAction = policy?.action || "pull-restart";
  const policyLabels: Record<string, string> = {
    none: "❌ No hacer nada",
    pull: "⬇️ Solo pull",
    "pull-restart": "🔄 Pull + reiniciar",
    "pull-restart-stack": "📦 Pull + reiniciar stack",
  };

  const stackContainers = c.compose_project
    ? containers.filter((cc) => cc.compose_project === c.compose_project)
    : [];
  const isMultiStack = stackContainers.length > 1;

  return (
    <Space direction="vertical" size="small" style={{ width: "100%" }}>
      <Flex gap={isMobile ? 8 : 6} wrap="wrap" align="center">
        <Button
          size={btnSize}
          type="default"
          icon={<SearchOutlined />}
          onClick={() => onInspect(c.name)}
        >
          Inspeccionar
        </Button>
        <Button
          size={btnSize}
          type="default"
          icon={
            loadingActions[c.name] === "Reiniciando..." ? undefined : (
              <ReloadOutlined />
            )
          }
          onClick={() => onRestart(c.name)}
          loading={loadingActions[c.name] === "Reiniciando..."}
          disabled={busy && loadingActions[c.name] !== "Reiniciando..."}
        >
          {loadingActions[c.name] === "Reiniciando..."
            ? "Reiniciando..."
            : "Reiniciar"}
        </Button>
        {c.state === "running" ? (
          <Button
            size={btnSize}
            danger
            icon={
              loadingActions[c.name] === "Parando..." ? undefined : (
                <StopOutlined />
              )
            }
            onClick={() => onStop(c.name)}
            loading={loadingActions[c.name] === "Parando..."}
            disabled={busy && loadingActions[c.name] !== "Parando..."}
          >
            {loadingActions[c.name] === "Parando..." ? "Parando..." : "Parar"}
          </Button>
        ) : (
          <Button
            size={btnSize}
            type="primary"
            icon={
              loadingActions[c.name] === "Iniciando..." ? undefined : (
                <PlayCircleOutlined />
              )
            }
            onClick={() => onStart(c.name)}
            loading={loadingActions[c.name] === "Iniciando..."}
            disabled={busy && loadingActions[c.name] !== "Iniciando..."}
          >
            {loadingActions[c.name] === "Iniciando..."
              ? "Iniciando..."
              : "Iniciar"}
          </Button>
        )}
        {isMultiStack && (
          <>
            <Button
              size={btnSize}
              danger
              icon={
                loadingActions[c.compose_project!] ===
                "Parando todos..." ? undefined : (
                  <StopOutlined />
                )
              }
              onClick={() =>
                onStackAction(
                  c.compose_project!,
                  stackContainers,
                  "stop",
                  "Parando todos...",
                )
              }
              loading={
                loadingActions[c.compose_project!] === "Parando todos..."
              }
              disabled={
                (busy &&
                  loadingActions[c.compose_project!] !== "Parando todos...") ||
                !stackContainers.some((sc) => sc.state === "running")
              }
            >
              {loadingActions[c.compose_project!] === "Parando todos..."
                ? "Parando todos..."
                : "Parar todos"}
            </Button>
            <Button
              size={btnSize}
              type="default"
              icon={
                loadingActions[c.compose_project!] ===
                "Reiniciando todos..." ? undefined : (
                  <ReloadOutlined />
                )
              }
              onClick={() =>
                onStackAction(
                  c.compose_project!,
                  stackContainers,
                  "restart",
                  "Reiniciando todos...",
                )
              }
              loading={
                loadingActions[c.compose_project!] === "Reiniciando todos..."
              }
              disabled={
                busy &&
                loadingActions[c.compose_project!] !== "Reiniciando todos..."
              }
            >
              {loadingActions[c.compose_project!] === "Reiniciando todos..."
                ? "Reiniciando todos..."
                : "Reiniciar todos"}
            </Button>
          </>
        )}
        <Button
          size={btnSize}
          type="default"
          icon={<FileTextOutlined />}
          onClick={() => onLogs(c.name)}
          disabled={busy}
        >
          Logs
        </Button>
        <Button
          size={btnSize}
          type="default"
          icon={<DeleteOutlined />}
          onClick={() => onRemove(c.name)}
          disabled={busy}
        >
          Eliminar
        </Button>
        <Flex align="center" gap="small">
          <Switch
            size="small"
            checked={getPolicy(c.name)?.notify_events ?? true}
            disabled={busy}
            onChange={async (checked) => {
              const current = getPolicy(c.name);
              const newVal = checked;
              try {
                const res = await apiFetch(
                  `/api/update-policies/${encodeURIComponent(c.name)}`,
                  {
                    method: "PUT",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({
                      action: current?.action || "pull-restart",
                      cleanup_old_image: current?.cleanup_old_image || false,
                      rollback_on_failure:
                        current?.rollback_on_failure || false,
                      notify_events: newVal,
                    }),
                  },
                );
                if (res.ok) {
                  const updated: UpdatePolicy = await res.json();
                  setPolicies((prev) => {
                    const next = prev.filter((p) => p.container !== c.name);
                    next.push(updated);
                    return next;
                  });
                  showToast(
                    `🔔 Notificaciones ${newVal ? "activadas" : "desactivadas"} para ${c.name}`,
                    newVal ? "green" : "gray",
                  );
                }
              } catch {
                showToast(`🔔 Error al cambiar notificaciones`, "red");
              }
            }}
          />
          <Typography.Text style={{ fontSize: 12 }}>
            Notificar eventos
          </Typography.Text>
        </Flex>
      </Flex>
      {p && (
        <Flex gap="small" align="center">
          <Spin size="small" />
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {p.status}
          </Typography.Text>
        </Flex>
      )}
      <Flex gap="small" wrap="wrap" justify="flex-start" align="center">
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {policyLabels[policyAction] || policyAction}
        </Typography.Text>
        <PolicyActionButton
          containerName={c.name}
          getPolicy={getPolicy}
          setPolicies={setPolicies}
          busy={busy}
          showToast={showToast}
          size={btnSize}
        />
      </Flex>
      {c.last_check && (
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          Última revisión:{" "}
          {new Date(c.last_check).toLocaleString([], {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
            hour12: false,
          })}
        </Typography.Text>
      )}
      {c.next_check && (
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          Próxima revisión:{" "}
          {new Date(c.next_check).toLocaleString([], {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
            hour12: false,
          })}
        </Typography.Text>
      )}
    </Space>
  );
}
