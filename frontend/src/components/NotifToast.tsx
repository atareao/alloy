import { Button, Card, Typography, Flex } from "antd";
import { CloseOutlined } from "@ant-design/icons";
import type { NotifEvent } from "../types";

const { Text } = Typography;

export default function NotifToast({
  notif,
  onDismiss,
}: {
  notif: NotifEvent;
  onDismiss: () => void;
}) {
  return (
    <Card
      size="small"
      variant="outlined"
      style={{
        marginBottom: "var(--ant-margin-xs)",
        background: "var(--ant-color-bg-container)",
      }}
      styles={{ body: { padding: "var(--ant-padding-sm)" } }}
    >
      <Flex justify="space-between" wrap="nowrap" align="flex-start">
        <div style={{ flex: 1, minWidth: 0 }}>
          <Text ellipsis>
            <b>{notif.container}</b> {notif.status}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: "0.75rem" }}>
            {notif.timestamp}
          </Text>
        </div>
        <Button
          type="text"
          size="small"
          icon={<CloseOutlined />}
          onClick={onDismiss}
          style={{ flexShrink: 0 }}
        />
      </Flex>
    </Card>
  );
}