import { useCallback, useEffect, useState } from "react";
import { Stack, Paper, Group, Badge, Table, Text, Loader, Button, Modal, Divider } from "@mantine/core";
import type { VolumeInfo, PruneResult } from "../types";
import { apiFetch, formatBytes } from "../api";

export default function VolumesPage() {
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [pruning, setPruning] = useState(false);
  const [pruneResult, setPruneResult] = useState<PruneResult | null>(null);
  const [showResult, setShowResult] = useState(false);

  const loadVolumes = useCallback(async () => {
    setLoading(true);
    try { setVolumes(await (await apiFetch("/api/volumes")).json()); }
    catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadVolumes(); }, [loadVolumes]);

  const handlePrune = async () => {
    setPruning(true);
    setPruneResult(null);
    try {
      const result: PruneResult = await (await apiFetch("/api/prune", { method: "POST" })).json();
      setPruneResult(result);
      setShowResult(true);
      loadVolumes();
    } catch { /* ignore */ }
    setPruning(false);
  };

  if (loading) return (<Group justify="center" py="xl"><Loader /></Group>);

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">🗄️ {volumes.length} volúmenes</Text>
          <Button onClick={handlePrune} loading={pruning} variant="filled" color="red">🗑️ Prune</Button>
        </Group>
      </Paper>
      <Modal opened={showResult} onClose={() => setShowResult(false)} title="🧹 Resultado del Prune" size="md">
        {pruneResult && (
          <Stack>
            <Group justify="space-between"><Text size="sm">Containers eliminados:</Text><Badge color="red">{pruneResult.containers_pruned}</Badge></Group>
            <Group justify="space-between"><Text size="sm">Imágenes eliminadas:</Text><Badge color="orange">{pruneResult.images_pruned}</Badge></Group>
            <Group justify="space-between"><Text size="sm">Redes eliminadas:</Text><Badge color="violet">{pruneResult.networks_pruned}</Badge></Group>
            <Group justify="space-between"><Text size="sm">Volúmenes eliminados:</Text><Badge color="grape">{pruneResult.volumes_pruned}</Badge></Group>
            <Divider />
            <Group justify="space-between"><Text size="sm" fw={500}>Espacio recuperado:</Text><Badge size="lg" color="green">{formatBytes(pruneResult.space_reclaimed_bytes)}</Badge></Group>
          </Stack>
        )}
      </Modal>
      {volumes.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder><Text ta="center" c="dimmed">No hay volúmenes Docker</Text></Paper>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table striped highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Nombre</Table.Th>
                <Table.Th>Driver</Table.Th>
                <Table.Th>Mountpoint</Table.Th>
                <Table.Th>Tamaño</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {volumes.map((v) => (
                <Table.Tr key={v.name}>
                  <Table.Td><Text size="sm" fw={500}>{v.name}</Text></Table.Td>
                  <Table.Td><Badge variant="light">{v.driver}</Badge></Table.Td>
                  <Table.Td><Text size="xs" c="dimmed">{v.mountpoint}</Text></Table.Td>
                  <Table.Td><Text size="sm">{v.size !== null ? formatBytes(v.size) : "-"}</Text></Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        </Paper>
      )}
    </Stack>
  );
}