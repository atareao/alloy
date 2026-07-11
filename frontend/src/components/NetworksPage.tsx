import { useEffect, useState } from "react";
import { Stack, Paper, Group, Badge, Table, Text, Loader } from "@mantine/core";
import type { NetworkInfo } from "../types";
import { apiFetch } from "../api";

export default function NetworksPage() {
  const [networks, setNetworks] = useState<NetworkInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch("/api/networks")
      .then((r) => r.json())
      .then(setNetworks)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  if (loading) return (<Group justify="center" py="xl"><Loader /></Group>);

  if (networks.length === 0)
    return (
      <Paper shadow="sm" p="xl" withBorder><Text ta="center" c="dimmed">No hay redes Docker</Text></Paper>
    );

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Text size="sm" c="dimmed">🌐 {networks.length} redes</Text>
      </Paper>
      <Paper shadow="sm" withBorder>
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Nombre</Table.Th>
              <Table.Th>Driver</Table.Th>
              <Table.Th>Scope</Table.Th>
              <Table.Th>Subnet</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {networks.map((n) => (
              <Table.Tr key={n.name}>
                <Table.Td><Text size="sm" fw={500}>{n.name}</Text></Table.Td>
                <Table.Td><Badge variant="light">{n.driver}</Badge></Table.Td>
                <Table.Td><Text size="sm">{n.scope}</Text></Table.Td>
                <Table.Td><Text size="xs" c="dimmed">{n.subnet || "-"}</Text></Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Paper>
    </Stack>
  );
}