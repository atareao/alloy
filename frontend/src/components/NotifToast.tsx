import { useEffect } from "react";
import { Group, Paper, Text } from "@mantine/core";
import type { NotifEvent } from "../types";

export default function NotifToast({
  notif,
  onDismiss,
}: {
  notif: NotifEvent;
  onDismiss: () => void;
}) {
  useEffect(() => {
    const t = setTimeout(onDismiss, 4000);
    return () => clearTimeout(t);
  }, [onDismiss]);

  return (
    <Paper
      shadow="md"
      p="sm"
      withBorder
      mb="xs"
      style={{ background: "#1c1c1c" }}
    >
      <Group justify="space-between">
        <Text size="sm">
          <b>{notif.container}</b> {notif.status}
        </Text>
        <Text size="xs" c="dimmed">
          {notif.timestamp}
        </Text>
      </Group>
    </Paper>
  );
}
