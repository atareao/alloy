import { Divider, Flex, Modal, Space, Spin, Table, Tabs, Tag, Typography } from "antd";
import type { InspectData } from "../types";

const { Text, Link } = Typography;

interface InspectModalProps {
  opened: boolean;
  onClose: () => void;
  containerName: string | null;
  containerInfo: {
    image: string;
    image_tag: string;
    registry_url: string;
  } | null;
  inspectData: InspectData | null;
  loading: boolean;
  error: string | null;
}

export default function InspectModal({
  opened,
  onClose,
  containerName,
  containerInfo,
  inspectData,
  loading,
  error,
}: InspectModalProps) {
  const portColumns = [
    { title: "Puerto Privado", dataIndex: "private_port", key: "private_port" },
    {
      title: "Puerto Público",
      dataIndex: "public_port",
      key: "public_port",
      render: (val: number | null) => (val != null ? val : "-"),
    },
    { title: "Tipo", dataIndex: "type", key: "type" },
  ];

  const volumeColumns = [
    { title: "Origen", dataIndex: "source", key: "source" },
    { title: "Destino", dataIndex: "destination", key: "destination" },
    {
      title: "Modo",
      dataIndex: "mode",
      key: "mode",
      render: (mode: string) => <Tag>{mode}</Tag>,
    },
  ];

  const networkColumns = [
    { title: "Red", dataIndex: "name", key: "name" },
    {
      title: "IP",
      dataIndex: "ip_address",
      key: "ip_address",
      render: (ip: string) => (
        <code style={{ fontSize: 12 }}>{ip}</code>
      ),
    },
    {
      title: "Gateway",
      dataIndex: "gateway",
      key: "gateway",
      render: (gw: string) => (
        <code style={{ fontSize: 12 }}>{gw}</code>
      ),
    },
  ];

  const generalTab = (
    <Space direction="vertical" size="small" style={{ width: "100%" }}>
      {inspectData ? (
        <>
          <Flex align="center" gap={8}>
            <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
              ID:
            </Text>
            <Text style={{ fontFamily: "monospace", fontSize: 11 }}>
              {inspectData.id}
            </Text>
          </Flex>
          <Flex align="center" gap={8}>
            <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
              Nombre:
            </Text>
            <Text style={{ fontSize: 14 }}>{inspectData.name}</Text>
          </Flex>
          <Flex align="center" gap={8}>
            <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
              Imagen:
            </Text>
            <Text style={{ fontSize: 14 }}>{inspectData.image}</Text>
          </Flex>
          <Flex align="center" gap={8}>
            <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
              Creado:
            </Text>
            <Text style={{ fontSize: 14 }}>{inspectData.created}</Text>
          </Flex>
          <Flex align="center" gap={8}>
            <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
              Estado:
            </Text>
            <Tag color={inspectData.state === "running" ? "success" : "error"}>
              {inspectData.state}
            </Tag>
          </Flex>
          <Flex align="center" gap={8}>
            <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
              Status:
            </Text>
            <Text style={{ fontSize: 14 }}>{inspectData.status}</Text>
          </Flex>
          {inspectData.restart_policy && (
            <Flex align="center" gap={8}>
              <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
                Reinicio:
              </Text>
              <Text style={{ fontSize: 14 }}>{inspectData.restart_policy}</Text>
            </Flex>
          )}
          {inspectData.health && (
            <Flex align="center" gap={8}>
              <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
                Health:
              </Text>
              <Tag
                color={inspectData.health === "healthy" ? "success" : "warning"}
              >
                {inspectData.health}
              </Tag>
            </Flex>
          )}
          <Divider style={{ margin: "8px 0" }} />
          {containerInfo && (
            <>
              <Text style={{ fontSize: 14, fontWeight: 500 }}>
                Información adicional
              </Text>
              <Flex align="center" gap={8}>
                <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
                  Imagen:
                </Text>
                <Text style={{ fontSize: 14 }}>{containerInfo.image}</Text>
              </Flex>
              <Flex align="center" gap={8}>
                <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
                  Tag:
                </Text>
                <Text style={{ fontSize: 14 }}>{containerInfo.image_tag}</Text>
              </Flex>
              {containerInfo.registry_url && (
                <Flex align="center" gap={8}>
                  <Text style={{ fontSize: 14, fontWeight: 500, width: 140 }}>
                    Registry:
                  </Text>
                  <Link
                    href={containerInfo.registry_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ fontSize: 14 }}
                    ellipsis
                  >
                    Ver en registry
                  </Link>
                </Flex>
              )}
            </>
          )}
        </>
      ) : (
        <Text type="secondary" style={{ fontSize: 14, padding: "16px 0" }}>
          Selecciona un container para inspeccionar
        </Text>
      )}
    </Space>
  );

  const portsTab = inspectData?.ports?.length ? (
    <Table
      dataSource={inspectData.ports}
      columns={portColumns}
      rowKey={(_, index) => `row-${index}`}
      pagination={false}
      size="small"
      scroll={{ x: "max-content" }}
    />
  ) : (
    <Text type="secondary" style={{ fontSize: 14, padding: "16px 0", display: "block" }}>
      Sin puertos expuestos
    </Text>
  );

  const volumesTab = inspectData?.mounts?.length ? (
    <Table
      dataSource={inspectData.mounts}
      columns={volumeColumns}
      rowKey={(_, index) => `row-${index}`}
      pagination={false}
      size="small"
      scroll={{ x: "max-content" }}
    />
  ) : (
    <Text type="secondary" style={{ fontSize: 14, padding: "16px 0", display: "block" }}>
      Sin volúmenes montados
    </Text>
  );

  const networksTab = inspectData?.networks?.length ? (
    <Table
      dataSource={inspectData.networks}
      columns={networkColumns}
      rowKey={(_, index) => `row-${index}`}
      pagination={false}
      size="small"
      scroll={{ x: "max-content" }}
    />
  ) : (
    <Text type="secondary" style={{ fontSize: 14, padding: "16px 0", display: "block" }}>
      Sin redes
    </Text>
  );

  const envTab = inspectData?.env?.length ? (
    <div style={{ maxHeight: 300, overflow: "auto" }}>
      <pre
        style={{
          backgroundColor: "transparent",
          color: "inherit",
          padding: 0,
          margin: 0,
          fontFamily:
            "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        }}
      >
        {inspectData.env.map((e, i) => (
          <div key={i}>{e}</div>
        ))}
      </pre>
    </div>
  ) : (
    <Text type="secondary" style={{ fontSize: 14, padding: "16px 0", display: "block" }}>
      Sin variables de entorno
    </Text>
  );

  const labelsTab =
    inspectData?.labels && Object.keys(inspectData.labels).length > 0 ? (
      <div style={{ maxHeight: 300, overflow: "auto" }}>
        {Object.entries(inspectData.labels).map(([k, v]) => (
          <Flex key={k} gap={4} style={{ marginBottom: 4 }}>
            <Text style={{ fontSize: 14, fontWeight: 500 }}>{k}:</Text>
            <Text style={{ fontSize: 14 }}>{v}</Text>
          </Flex>
        ))}
      </div>
    ) : (
      <Text type="secondary" style={{ fontSize: 14, padding: "16px 0", display: "block" }}>
        Sin labels
      </Text>
    );

  const tabItems = [
    { key: "general", label: "General", children: generalTab },
    { key: "ports", label: "Puertos", children: portsTab },
    { key: "volumes", label: "Volúmenes", children: volumesTab },
    { key: "networks", label: "Redes", children: networksTab },
    { key: "env", label: "ENV", children: envTab },
    { key: "labels", label: "Labels", children: labelsTab },
  ];

  return (
    <Modal
      open={opened}
      onCancel={onClose}
      title={`🔍 ${containerName || ""}`}
      width={960}
      footer={null}
    >
      {loading ? (
        <Flex justify="center" align="center" style={{ padding: "24px 0" }}>
          <Spin />
          <Text style={{ marginLeft: 8 }}>Obteniendo información...</Text>
        </Flex>
      ) : error ? (
        <Text type="danger">{error}</Text>
      ) : (
        <Tabs items={tabItems} />
      )}
    </Modal>
  );
}