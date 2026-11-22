import { Card, Flex, Space, Tag, Typography } from "antd";
import type { AppConfig } from "../types";

interface AppInfoCardProps {
  configProp: AppConfig | null;
}

/** Card «ℹ️ Información de la aplicación» (pestaña Información). */
export default function AppInfoCard({ configProp }: AppInfoCardProps) {
  return (
    <Card bordered size="small">
      <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16 }}>
        ℹ️ Información de la aplicación
      </Typography.Title>
      <Space direction="vertical" size="small" style={{ width: "100%" }}>
        <Flex gap="middle" align="center">
          <Typography.Text strong style={{ minWidth: 100 }}>
            Versión
          </Typography.Text>
          <Tag color="blue">v{configProp?.version || "—"}</Tag>
        </Flex>
        <Flex gap="middle" align="center">
          <Typography.Text strong style={{ minWidth: 100 }}>
            Compilado
          </Typography.Text>
          <Typography.Text type="secondary">
            {configProp?.build_date
              ? new Date(configProp.build_date).toLocaleString([], {
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                  hour: "2-digit",
                  minute: "2-digit",
                  hour12: false,
                })
              : "—"}
          </Typography.Text>
        </Flex>
        <Flex gap="middle" align="center">
          <Typography.Text strong style={{ minWidth: 100 }}>
            Repositorio
          </Typography.Text>
          <Typography.Link
            href={configProp?.repo_url || "#"}
            target="_blank"
            rel="noopener noreferrer"
          >
            {configProp?.repo_url || "—"}
          </Typography.Link>
        </Flex>
        <Flex gap="middle" align="center">
          <Typography.Text strong style={{ minWidth: 100 }}>
            Zona horaria
          </Typography.Text>
          <Typography.Text type="secondary">
            {configProp?.timezone || "UTC"}
          </Typography.Text>
        </Flex>
        <Flex gap="middle" align="center">
          <Typography.Text strong style={{ minWidth: 100 }}>
            Puerto
          </Typography.Text>
          <Typography.Text type="secondary">
            {configProp?.port || 3066}
          </Typography.Text>
        </Flex>
        <Flex gap="middle" align="center">
          <Typography.Text strong style={{ minWidth: 100 }}>
            Auth
          </Typography.Text>
          <Tag color="green">OIDC</Tag>
        </Flex>
      </Space>
    </Card>
  );
}
