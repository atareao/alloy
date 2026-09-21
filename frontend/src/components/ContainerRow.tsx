import {
  Button,
  Card,
  Flex,
  Spin,
  Tag,
  Tooltip,
  Typography,
  message,
} from "antd";
import { LinkOutlined, UpOutlined, DownOutlined } from "@ant-design/icons";
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
  const prog = progress.get(c.name);
  const isUpdating = prog && !prog.done;
  const isDone = prog && prog.done;

  const progressColor = isUpdating
    ? "yellow"
    : isDone && prog?.error
      ? "red"
      : isDone
        ? "green"
        : undefined;

  const progressLabel = prog
    ? prog.done
      ? prog.error
        ? "❌"
        : "✅"
      : "🔄"
    : undefined;

  const showToast = (msg: string, color: string) => {
    if (color === "green") message.success(msg, 3);
    else if (color === "red") message.error(msg, 3);
    else message.info(msg, 3);
  };

  const statusColor = (c: ContainerInfo) =>
    c.status.includes("healthy")
      ? "green"
      : c.state === "running"
        ? "blue"
        : "red";

  const statusDot = (c: ContainerInfo) => {
    const color = statusColor(c);
    const dotColor =
      color === "green"
        ? "var(--ant-color-success)"
        : color === "blue"
          ? "var(--ant-color-primary)"
          : "var(--ant-color-error)";
    return (
      <div
        style={{
          width: 10,
          height: 10,
          borderRadius: "50%",
          backgroundColor: dotColor,
          flexShrink: 0,
        }}
      />
    );
  };

  return isMobile ? (
    <div
      style={{
        background: "var(--ant-color-bg-container)",
        borderRadius: 6,
        marginBottom: 1,
        borderBottom: "1px solid var(--ant-color-border)",
      }}
    >
      {/* Main row — clickable header */}
      <div
        onClick={() => onToggleExpand(c.name)}
        style={{
          cursor: "pointer",
          padding: isMobile ? 2 : 8,
          background: "var(--ant-color-bg-elevated)",
          borderRadius: 6,
        }}
      >
        <Flex justify="space-between" wrap="nowrap" align="center">
          <Flex
            gap="small"
            wrap="nowrap"
            align="center"
            style={{ flex: 1, minWidth: 0, overflow: "hidden" }}
          >
            {statusDot(c)}
            {hasUpdate && (
              <Tag
                color="yellow"
                style={{ fontSize: 10, lineHeight: "14px", padding: "0 4px" }}
              >
                !
              </Tag>
            )}
            <Typography.Text
              strong
              ellipsis
              style={{ minWidth: 60, fontSize: 14 }}
            >
              {isMobile
                ? c.name.length > 12
                  ? c.name.slice(0, 9) + "..."
                  : c.name
                : c.name}
            </Typography.Text>
            {c.updating && <Spin size="small" />}
            <Typography.Text
              type="secondary"
              ellipsis
              style={{ minWidth: 60, fontSize: 12 }}
            >
              {isMobile
                ? c.status.length > 20
                  ? c.status.slice(0, 17) + "..."
                  : c.status
                : c.status}
            </Typography.Text>
            {prog && (
              <Tooltip title={prog.status}>
                <Tag color={progressColor} style={{ flexShrink: 0 }}>
                  {isUpdating ? (
                    <Flex gap={4} wrap="nowrap" align="center">
                      <Spin size="small" />
                      <Typography.Text style={{ fontSize: 12 }}>
                        {prog.status.slice(0, 20)}
                      </Typography.Text>
                    </Flex>
                  ) : (
                    progressLabel
                  )}
                </Tag>
              </Tooltip>
            )}
            {c.traefik_url && isMobile ? (
              <Button
                href={c.traefik_url}
                target="_blank"
                rel="noopener noreferrer"
                type="default"
                size="small"
                icon={<LinkOutlined />}
                onClick={(e) => e.stopPropagation()}
              />
            ) : c.traefik_url ? (
              <Typography.Link
                href={c.traefik_url}
                target="_blank"
                rel="noopener noreferrer"
                ellipsis
                style={{ maxWidth: 180 }}
                onClick={(e) => e.stopPropagation()}
              >
                <LinkOutlined /> {c.traefik_url.replace(/^https?:\/\//, "")}
              </Typography.Link>
            ) : null}
          </Flex>
          {!isMobile && (
            <Button
              type="text"
              size="small"
              icon={expanded ? <UpOutlined /> : <DownOutlined />}
              style={{ flexShrink: 0 }}
            />
          )}
        </Flex>
      </div>

      {/* Expanded actions panel */}
      {expanded && (
        <div
          style={{
            background: "var(--ant-color-bg-elevated)",
            borderRadius: 6,
            margin: "0 8px 8px 8px",
          }}
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
        </div>
      )}
    </div>
  ) : (
    <Card
      bordered
      size="small"
      styles={{
        body: {
          padding: 0,
        },
      }}
    >
      {/* Main row — clickable header */}
      <div
        onClick={() => onToggleExpand(c.name)}
        style={{ cursor: "pointer", padding: 8 }}
      >
        <Flex justify="space-between" wrap="nowrap" align="center">
          <Flex
            gap="small"
            wrap="nowrap"
            align="center"
            style={{ flex: 1, minWidth: 0, overflow: "hidden" }}
          >
            {statusDot(c)}
            {hasUpdate && (
              <Tag
                color="yellow"
                style={{ fontSize: 10, lineHeight: "14px", padding: "0 4px" }}
              >
                !
              </Tag>
            )}
            <Typography.Text
              strong
              ellipsis
              style={{ minWidth: 60, fontSize: 14 }}
            >
              {isMobile
                ? c.name.length > 12
                  ? c.name.slice(0, 9) + "..."
                  : c.name
                : c.name}
            </Typography.Text>
            {c.updating && <Spin size="small" />}
            <Typography.Text
              type="secondary"
              ellipsis
              style={{ minWidth: 60, fontSize: 12 }}
            >
              {isMobile
                ? c.status.length > 20
                  ? c.status.slice(0, 17) + "..."
                  : c.status
                : c.status}
            </Typography.Text>
            {prog && (
              <Tooltip title={prog.status}>
                <Tag color={progressColor} style={{ flexShrink: 0 }}>
                  {isUpdating ? (
                    <Flex gap={4} wrap="nowrap" align="center">
                      <Spin size="small" />
                      <Typography.Text style={{ fontSize: 12 }}>
                        {prog.status.slice(0, 20)}
                      </Typography.Text>
                    </Flex>
                  ) : (
                    progressLabel
                  )}
                </Tag>
              </Tooltip>
            )}
            {c.traefik_url && isMobile ? (
              <Button
                href={c.traefik_url}
                target="_blank"
                rel="noopener noreferrer"
                type="default"
                size="small"
                icon={<LinkOutlined />}
                onClick={(e) => e.stopPropagation()}
              />
            ) : c.traefik_url ? (
              <Typography.Link
                href={c.traefik_url}
                target="_blank"
                rel="noopener noreferrer"
                ellipsis
                style={{ maxWidth: 180 }}
                onClick={(e) => e.stopPropagation()}
              >
                <LinkOutlined /> {c.traefik_url.replace(/^https?:\/\//, "")}
              </Typography.Link>
            ) : null}
          </Flex>
          {!isMobile && (
            <Button
              type="text"
              size="small"
              icon={expanded ? <UpOutlined /> : <DownOutlined />}
              style={{ flexShrink: 0 }}
            />
          )}
        </Flex>
      </div>

      {/* Expanded actions panel */}
      {expanded && (
        <Card
          bordered
          size="small"
          style={{
            background: "var(--ant-color-bg-elevated)",
            margin: "0 8px 8px 8px",
          }}
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
        </Card>
      )}
    </Card>
  );
}
