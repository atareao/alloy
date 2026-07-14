import { ActionIcon, Group, Paper, Text } from "@mantine/core";
import type { NotifEvent } from "../types";

export default function NotifToast({
  notif,
  onDismiss,
}: {
  notif: NotifEvent;
  onDismiss: () => void;
}) {
  return (
    <Paper
      shadow="md"
      p="sm"
      withBorder
      mb="xs"
      style={{ background: "#1c1c1c" }}
    >
      <Group justify="space-between" wrap="nowrap">
        <div style={{ flex: 1, minWidth: 0 }}>
          <Text size="sm" truncate>
            <b>{notif.container}</b> {notif.status}
          </Text>
          <Text size="xs" c="dimmed">
            {notif.timestamp}
          </Text>
        </div>
        <ActionIcon variant="subtle" color="gray" size="sm" onClick={onDismiss}>
          ✕
        </ActionIcon>
      </Group>
    </Paper>
  );
}
