import {
  Button,
  Group,
  Loader,
  Stack,
  Switch,
  Text,
} from "@mantine/core";
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
  const btnSize = isMobile ? "sm" : "compact-sm";
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
    <Stack gap="xs">
      <Group gap={isMobile ? 8 : 6} wrap="wrap">
        <Button
          size={btnSize}
          variant="light"
          color="gray"
          leftSection="🔍"
          onClick={() => onInspect(c.name)}
        >
          Inspeccionar
        </Button>
        <Button
          size={btnSize}
          variant="light"
          color="orange"
          leftSection={
            loadingActions[c.name] === "Reiniciando..." ? undefined : "🔄"
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
            variant="light"
            color="red"
            leftSection={
              loadingActions[c.name] === "Parando..." ? undefined : "⏹"
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
            variant="light"
            color="green"
            leftSection={
              loadingActions[c.name] === "Iniciando..." ? undefined : "▶"
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
              variant="light"
              color="red"
              leftSection={
                loadingActions[c.compose_project!] === "Parando todos..."
                  ? undefined
                  : "⏹"
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
              variant="light"
              color="orange"
              leftSection={
                loadingActions[c.compose_project!] === "Reiniciando todos..."
                  ? undefined
                  : "🔄"
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
          variant="light"
          color="grape"
          leftSection="📋"
          onClick={() => onLogs(c.name)}
          disabled={busy}
        >
          Logs
        </Button>
        <Button
          size={btnSize}
          variant="light"
          color="gray"
          leftSection="🗑"
          onClick={() => onRemove(c.name)}
          disabled={busy}
        >
          Eliminar
        </Button>
        <Switch
          size="xs"
          label="Notificar eventos"
          checked={getPolicy(c.name)?.notify_events ?? true}
          disabled={busy}
          onChange={async () => {
            const current = getPolicy(c.name);
            const newVal = !(current?.notify_events ?? true);
            try {
              const res = await apiFetch(
                `/api/update-policies/${encodeURIComponent(c.name)}`,
                {
                  method: "PUT",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                    action: current?.action || "pull-restart",
                    cleanup_old_image: current?.cleanup_old_image || false,
                    rollback_on_failure: current?.rollback_on_failure || false,
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
      </Group>
      {p && (
        <Group gap="xs">
          <Loader size="xs" />
          <Text size="xs" c="dimmed">
            {p.status}
          </Text>
        </Group>
      )}
      <Group gap="xs" wrap="wrap" justify="flex-start">
        <Text size="xs" c="dimmed">
          {policyLabels[policyAction] || policyAction}
        </Text>
        <PolicyActionButton
          containerName={c.name}
          getPolicy={getPolicy}
          setPolicies={setPolicies}
          busy={busy}
          showToast={showToast}
          size={btnSize}
        />
      </Group>
      {c.last_check && (
        <Text size="xs" c="dimmed">
          Última revisión: {new Date(c.last_check).toLocaleString([], {
            year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit',
            hour12: false
          })}
        </Text>
      )}
      {c.next_check && (
        <Text size="xs" c="dimmed">
          Próxima revisión: {new Date(c.next_check).toLocaleString([], {
            year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit',
            hour12: false
          })}
        </Text>
      )}
    </Stack>
  );
}