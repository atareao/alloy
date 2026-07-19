import type { ReactNode } from "react";
import {
  ActionIcon,
  Button,
  Chip,
  Group,
  Paper,
  Stack,
  Switch,
  Text,
  TextInput,
  Tooltip,
} from "@mantine/core";
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
  expandedStacks: Record<string, boolean>;
  renderGroup: (project: string, items: ContainerInfo[]) => ReactNode;
  renderRow: (c: ContainerInfo) => ReactNode;
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
  expandedStacks,
  renderGroup,
  renderRow,
}: ContainerTableProps) {
  return (
    <>
      {/* Search + filters bar */}
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Stack gap="sm">
          <Group gap="md" wrap="nowrap" align="flex-end">
            <TextInput
              placeholder="Buscar por nombre, imagen, stack..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.currentTarget.value)}
              rightSection={
                searchQuery ? (
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    onClick={() => setSearchQuery("")}
                  >
                    ✕
                  </ActionIcon>
                ) : undefined
              }
              style={{ flex: 1 }}
            />
            <Tooltip label="Comprobar todos contra registry">
              <Button
                onClick={onCheckAll}
                variant="light"
                color="cyan"
                size="sm"
              >
                {isMobile ? "🔍" : "🔍 Check"}
              </Button>
            </Tooltip>
          </Group>
          <Group gap="md" wrap="wrap" justify="space-between">
            <Group gap="xs" wrap="wrap">
              {availableStates.length > 0 && (
                <Chip.Group
                  multiple
                  value={stateFilter}
                  onChange={setStateFilter}
                >
                  <Group gap="xs" wrap="wrap">
                    {availableStates.map((s) => (
                      <Chip key={s} value={s} size="xs" variant="outline">
                        {s}
                      </Chip>
                    ))}
                  </Group>
                </Chip.Group>
              )}
              <Switch
                label="Solo pendientes de actualizar"
                checked={showPendingUpdates}
                onChange={(e) =>
                  setShowPendingUpdates(e.currentTarget.checked)
                }
                size="xs"
              />
            </Group>
          </Group>
          </Stack>
      </Paper>

      {/* Stack groups — grid on mobile, list on desktop */}
      {isMobile ? (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: "var(--mantine-spacing-sm)",
            marginBottom: "var(--mantine-spacing-md)",
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
      {noStack.length > 0 && (
        <Paper shadow="sm" withBorder>
          <Group px="md" pt="sm" pb="xs">
            <Text size="md" fw={700}>
              📦 Sin stack
            </Text>
            <Text size="xs" c="dimmed">
              {noStack.length} containers
            </Text>
          </Group>
          {noStack.map(renderRow)}
        </Paper>
      )}
    </>
  );
}
