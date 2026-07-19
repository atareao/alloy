import {
  Anchor,
  Badge,
  Code,
  Divider,
  Group,
  Loader,
  Modal,
  ScrollArea,
  Stack,
  Table,
  Tabs,
  Text,
} from "@mantine/core";
import type { InspectData } from "../types";

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
  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={`🔍 ${containerName || ""}`}
      size="xl"
    >
      {loading ? (
        <Group justify="center" py="xl">
          <Loader />
          <Text>Obteniendo información...</Text>
        </Group>
      ) : error ? (
        <Text c="red">{error}</Text>
      ) : (
        <Tabs defaultValue="general">
          <Tabs.List mb="sm">
            <Tabs.Tab value="general">General</Tabs.Tab>
            <Tabs.Tab value="ports">Puertos</Tabs.Tab>
            <Tabs.Tab value="volumes">Volúmenes</Tabs.Tab>
            <Tabs.Tab value="networks">Redes</Tabs.Tab>
            <Tabs.Tab value="env">ENV</Tabs.Tab>
            <Tabs.Tab value="labels">Labels</Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="general">
            {inspectData ? (
              <Stack gap="xs">
                <Group>
                  <Text size="sm" fw={500} w={140}>
                    ID:
                  </Text>
                  <Text
                    size="sm"
                    style={{ fontFamily: "monospace", fontSize: 11 }}
                  >
                    {inspectData.id}
                  </Text>
                </Group>
                <Group>
                  <Text size="sm" fw={500} w={140}>
                    Nombre:
                  </Text>
                  <Text size="sm">{inspectData.name}</Text>
                </Group>
                <Group>
                  <Text size="sm" fw={500} w={140}>
                    Imagen:
                  </Text>
                  <Text size="sm">{inspectData.image}</Text>
                </Group>
                <Group>
                  <Text size="sm" fw={500} w={140}>
                    Creado:
                  </Text>
                  <Text size="sm">{inspectData.created}</Text>
                </Group>
                <Group>
                  <Text size="sm" fw={500} w={140}>
                    Estado:
                  </Text>
                  <Badge
                    color={inspectData.state === "running" ? "green" : "red"}
                  >
                    {inspectData.state}
                  </Badge>
                </Group>
                <Group>
                  <Text size="sm" fw={500} w={140}>
                    Status:
                  </Text>
                  <Text size="sm">{inspectData.status}</Text>
                </Group>
                {inspectData.restart_policy && (
                  <Group>
                    <Text size="sm" fw={500} w={140}>
                      Reinicio:
                    </Text>
                    <Text size="sm">{inspectData.restart_policy}</Text>
                  </Group>
                )}
                {inspectData.health && (
                  <Group>
                    <Text size="sm" fw={500} w={140}>
                      Health:
                    </Text>
                    <Badge
                      color={
                        inspectData.health === "healthy" ? "green" : "yellow"
                      }
                    >
                      {inspectData.health}
                    </Badge>
                  </Group>
                )}
                <Divider my="xs" />
                {containerInfo && (
                  <>
                    <Text size="sm" fw={500}>
                      Información adicional
                    </Text>
                    <Group>
                      <Text size="sm" fw={500} w={140}>
                        Imagen:
                      </Text>
                      <Text size="sm">{containerInfo.image}</Text>
                    </Group>
                    <Group>
                      <Text size="sm" fw={500} w={140}>
                        Tag:
                      </Text>
                      <Text size="sm">{containerInfo.image_tag}</Text>
                    </Group>
                    {containerInfo.registry_url && (
                      <Group>
                        <Text size="sm" fw={500} w={140}>
                          Registry:
                        </Text>
                        <Anchor
                          href={containerInfo.registry_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          size="sm"
                          truncate
                        >
                          Ver en registry
                        </Anchor>
                      </Group>
                    )}
                  </>
                )}
              </Stack>
            ) : (
              <Text size="sm" c="dimmed" py="md">
                Selecciona un container para inspeccionar
              </Text>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="ports">
            {inspectData?.ports?.length ? (
              <Table.ScrollContainer minWidth={400}>
                <Table striped>
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>Puerto Privado</Table.Th>
                      <Table.Th>Puerto Público</Table.Th>
                      <Table.Th>Tipo</Table.Th>
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {inspectData.ports.map((p, i) => (
                      <Table.Tr key={i}>
                        <Table.Td>{p.private_port}</Table.Td>
                        <Table.Td>
                          {p.public_port != null ? p.public_port : "-"}
                        </Table.Td>
                        <Table.Td>{p.type}</Table.Td>
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              </Table.ScrollContainer>
            ) : (
              <Text size="sm" c="dimmed" py="md">
                Sin puertos expuestos
              </Text>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="volumes">
            {inspectData?.mounts?.length ? (
              <Table.ScrollContainer minWidth={400}>
                <Table striped>
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>Origen</Table.Th>
                      <Table.Th>Destino</Table.Th>
                      <Table.Th>Modo</Table.Th>
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {inspectData.mounts.map((m, i) => (
                      <Table.Tr key={i}>
                        <Table.Td>
                          <Text size="sm">{m.source}</Text>
                        </Table.Td>
                        <Table.Td>
                          <Text size="sm">{m.destination}</Text>
                        </Table.Td>
                        <Table.Td>
                          <Badge variant="light">{m.mode}</Badge>
                        </Table.Td>
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              </Table.ScrollContainer>
            ) : (
              <Text size="sm" c="dimmed" py="md">
                Sin volúmenes montados
              </Text>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="networks">
            {inspectData?.networks?.length ? (
              <Table.ScrollContainer minWidth={400}>
                <Table striped>
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>Red</Table.Th>
                      <Table.Th>IP</Table.Th>
                      <Table.Th>Gateway</Table.Th>
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {inspectData.networks.map((n, i) => (
                      <Table.Tr key={i}>
                        <Table.Td>
                          <Text size="sm">{n.name}</Text>
                        </Table.Td>
                        <Table.Td>
                          <Code>{n.ip_address}</Code>
                        </Table.Td>
                        <Table.Td>
                          <Code>{n.gateway}</Code>
                        </Table.Td>
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              </Table.ScrollContainer>
            ) : (
              <Text size="sm" c="dimmed" py="md">
                Sin redes
              </Text>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="env">
            {inspectData?.env?.length ? (
              <ScrollArea h={300}>
                <Code block>
                  {inspectData.env.map((e, i) => (
                    <div key={i}>{e}</div>
                  ))}
                </Code>
              </ScrollArea>
            ) : (
              <Text size="sm" c="dimmed" py="md">
                Sin variables de entorno
              </Text>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="labels">
            {inspectData?.labels &&
            Object.keys(inspectData.labels).length > 0 ? (
              <ScrollArea h={300}>
                {Object.entries(inspectData.labels).map(([k, v]) => (
                  <Group key={k} gap="xs" mb="xs">
                    <Text size="sm" fw={500}>
                      {k}:
                    </Text>
                    <Text size="sm">{v}</Text>
                  </Group>
                ))}
              </ScrollArea>
            ) : (
              <Text size="sm" c="dimmed" py="md">
                Sin labels
              </Text>
            )}
          </Tabs.Panel>
        </Tabs>
      )}
    </Modal>
  );
}