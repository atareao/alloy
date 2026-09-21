import type { ReactNode } from "react";
import {
  Button,
  Card,
  Flex,
  Input,
  Switch,
  Tag,
  Typography,
  Tooltip,
} from "antd";
import { SearchOutlined, AppstoreOutlined } from "@ant-design/icons";
import type { ContainerInfo } from "../types";

export interface ContainerTableProps {
  sortedGroups: [string, ContainerInfo[]][];
  noStack: ContainerInfo[];
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  stateFilter: string[];
  setStateFilter: (f: string[]) => void;
  showPendingUpdates: boolean;
  setShowPendingUpdates: (v: boolean) => void;
  availableStates: string[];
  isMobile: boolean;
  onCheckAll: () => void;
  lastCheck: string | null;
  nextCheck: string | null;
  expandedStacks: Record<string, boolean>;
  renderGroup: (project: string, items: ContainerInfo[]) => ReactNode;
  renderRow: (c: ContainerInfo) => ReactNode;
  batchPhase: "idle" | "active";
}

export default function ContainerTable({
  sortedGroups,
  noStack,
  searchQuery,
  setSearchQuery,
  stateFilter,
  setStateFilter,
  showPendingUpdates,
  setShowPendingUpdates,
  availableStates,
  isMobile,
  onCheckAll,
  lastCheck,
  nextCheck,
  expandedStacks,
  renderGroup,
  renderRow,
  batchPhase,
}: ContainerTableProps) {
  return (
    <>
      {/* Search + filters bar — hidden during batch operation */}
      {batchPhase === "idle" && (
        <Card bordered style={{ marginBottom: 16 }}>
          <Flex vertical gap="small">
            <Flex gap="middle" wrap="nowrap" align="flex-end">
              <Input
                placeholder="Buscar por nombre, imagen, stack..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                allowClear
                onClear={() => setSearchQuery("")}
                style={{ flex: 1 }}
              />
              <Tooltip title="Comprobar todos contra registry">
                <Button
                  onClick={onCheckAll}
                  type="default"
                  size="small"
                  icon={<SearchOutlined />}
                >
                  {isMobile ? "" : " Check"}
                </Button>
              </Tooltip>
            </Flex>
            <Flex gap="small" wrap="nowrap" style={{ marginLeft: 8 }}>
              {lastCheck && (
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  Última:{" "}
                  {new Date(lastCheck).toLocaleString([], {
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
              {nextCheck && (
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  Próxima:{" "}
                  {new Date(nextCheck).toLocaleString([], {
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
            </Flex>
            <Flex gap="middle" wrap="wrap" justify="space-between">
              <Flex gap="small" wrap="wrap" align="center">
                {availableStates.length > 0 && (
                  <Flex gap="small" wrap="wrap">
                    {availableStates.map((s) => (
                      <Tag
                        key={s}
                        color={stateFilter.includes(s) ? "blue" : undefined}
                        style={{ cursor: "pointer" }}
                        onClick={() => {
                          if (stateFilter.includes(s)) {
                            setStateFilter(stateFilter.filter((f) => f !== s));
                          } else {
                            setStateFilter([...stateFilter, s]);
                          }
                        }}
                      >
                        {s}
                      </Tag>
                    ))}
                  </Flex>
                )}
                <Flex align="center" gap="small">
                  <Switch
                    checked={showPendingUpdates}
                    onChange={(checked) => setShowPendingUpdates(checked)}
                    size="small"
                  />
                  <Typography.Text style={{ fontSize: 12 }}>
                    Solo pendientes de actualizar
                  </Typography.Text>
                </Flex>
              </Flex>
            </Flex>
          </Flex>
        </Card>
      )}

      {/* Stack groups — grid on mobile, list on desktop */}
      {isMobile ? (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: 8,
            background: "transparent",
          }}
        >
          {sortedGroups.map(([project, items]) => (
            <div
              key={project}
              style={
                expandedStacks[project]
                  ? { gridColumn: "1 / -1" }
                  : { aspectRatio: "1", overflow: "hidden" }
              }
            >
              {renderGroup(project, items)}
            </div>
          ))}
        </div>
      ) : (
        sortedGroups.map(([project, items]) => renderGroup(project, items))
      )}

      {/* Ungrouped containers */}
      {noStack.length > 0 &&
        (isMobile ? (
          <div
            style={{
              background: "var(--ant-color-fill-tertiary)",
              borderRadius: 6,
              padding: 2,
              marginTop: 8,
            }}
          >
            <Flex align="center" style={{ padding: "8px 16px" }}>
              <Typography.Text strong style={{ fontSize: 14 }}>
                <AppstoreOutlined /> Sin stack
              </Typography.Text>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, marginLeft: 8 }}
              >
                {noStack.length} containers
              </Typography.Text>
            </Flex>
            {noStack.map(renderRow)}
          </div>
        ) : (
          <Card bordered>
            <Flex align="center" style={{ padding: "8px 16px" }}>
              <Typography.Text strong style={{ fontSize: 14 }}>
                <AppstoreOutlined /> Sin stack
              </Typography.Text>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, marginLeft: 8 }}
              >
                {noStack.length} containers
              </Typography.Text>
            </Flex>
            {noStack.map(renderRow)}
          </Card>
        ))}
    </>
  );
}
