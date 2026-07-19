import {
  ActionIcon,
  Anchor,
  Badge,
  Button,
  Collapse,
  Group,
  Loader,
  Paper,
  Table,
  Text,
} from "@mantine/core";
import { showNotification } from "@mantine/notifications";
import type { ContainerInfo, UpdateProgress, UpdatePolicy } from "../types";
import ContainerActions from "./ContainerActions";

interface ContainerRowProps {
  container: ContainerInfo;
  isMobile: boolean;
  expanded: boolean;
  onToggleExpand: (name: string) => void;
  progress: Map<string, UpdateProgress>;
  getPolicy: (name: string) => UpdatePolicy | undefined;
  setPolicies: React.Dispatch<React.SetStateAction<UpdatePolicy[]>>;
  loadingActions: Record<string, string>;
  batchPhase: string;
  containers: ContainerInfo[];
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
}

export default function ContainerRow({
  container,
  isMobile,
  expanded,
  onToggleExpand,
  progress,
  getPolicy,
  setPolicies,
  loadingActions,
  batchPhase,
  containers,
  onInspect,
  onLogs,
  onStart,
  onStop,
  onRestart,
  onRemove,
  onStackAction,
}: ContainerRowProps) {
  const c = container;
  const hasUpdate = c.has_update;
  const showToast = (message: string, color: string) => {
    showNotification({
      title: "Alloy",
      message,
      color,
      autoClose: 3000,
      style: { borderLeft: `4px solid var(--mantine-color-${color}-6)` },
    });
  };

  const statusColor = (c: ContainerInfo) =>
    c.status.includes("healthy")
      ? "green"
      : c.state === "running"
        ? "blue"
        : "red";

  const statusDot = (c: ContainerInfo) => {
    const color = statusColor(c);
    return (
      <div
        style={{
          width: 10,
          height: 10,
          borderRadius: "50%",
          backgroundColor: `var(--mantine-color-${color}-6)`,
          flexShrink: 0,
        }}
      />
    );
  };

  return (
    <Table.Tr key={c.id}>
      <Table.Td
        style={{ padding: 0, border: "none" }}
      >
        <Paper p="sm" style={{ background: "transparent" }}>
          <Group
            justify="space-between"
            wrap="nowrap"
            style={{ flex: 1, cursor: "pointer" }}
            onClick={() => onToggleExpand(c.name)}
          >
            <Group
              gap="xs"
              wrap="nowrap"
              style={{ flex: 1, minWidth: 0, overflow: "hidden" }}
            >
              {statusDot(c)}
              {hasUpdate && (
                <Badge size="xs" variant="filled" color="yellow" circle>
                  !
                </Badge>
              )}
              <Text size="sm" fw={500} truncate style={{ minWidth: 60 }}>
                {isMobile
                  ? c.name.length > 12
                    ? c.name.slice(0, 9) + "..."
                    : c.name
                  : c.name}
              </Text>
              {c.updating && <Loader size="xs" />}
              <Text size="xs" c="dimmed" truncate style={{ minWidth: 60 }}>
                {isMobile
                  ? c.status.length > 20
                    ? c.status.slice(0, 17) + "..."
                    : c.status
                  : c.status}
              </Text>
              {c.traefik_url && isMobile ? (
                <Button
                  component="a"
                  href={c.traefik_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  variant="light"
                  color="blue"
                  size="sm"
                  onClick={(e) => e.stopPropagation()}
                >
                  🔗
                </Button>
              ) : c.traefik_url ? (
                <Anchor
                  href={c.traefik_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  size="xs"
                  truncate
                  style={{ maxWidth: 180 }}
                  onClick={(e) => e.stopPropagation()}
                >
                  🔗 {c.traefik_url.replace(/^https?:\/\//, "")}
                </Anchor>
              ) : null}
            </Group>
            {!isMobile && (
              <ActionIcon
                variant="subtle"
                color="gray"
                size="sm"
                style={{ flexShrink: 0 }}
              >
                {expanded ? "▲" : "▼"}
              </ActionIcon>
            )}
          </Group>
        </Paper>
        <Collapse expanded={expanded}>
          <Paper
            p="sm"
            withBorder
            mx="sm"
            mb="sm"
            style={{ background: "var(--mantine-color-dark-6)" }}
          >
            <ContainerActions
              container={c}
              isMobile={isMobile}
              progress={progress}
              batchPhase={batchPhase}
              loadingActions={loadingActions}
              containers={containers}
              getPolicy={getPolicy}
              setPolicies={setPolicies}
              onInspect={onInspect}
              onLogs={onLogs}
              onStart={onStart}
              onStop={onStop}
              onRestart={onRestart}
              onRemove={onRemove}
              onStackAction={onStackAction}
              showToast={showToast}
            />
          </Paper>
        </Collapse>
      </Table.Td>
    </Table.Tr>
  );
}