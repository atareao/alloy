import { useCallback, useEffect, useState } from "react";
import { Stack, Paper, Group, Select, NumberInput, Button, Loader, ScrollArea, Code, Text } from "@mantine/core";
import type { ContainerInfo } from "../types";
import { apiFetch } from "../api";

export default function LogsPage({ containers }: { containers: ContainerInfo[] }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [tail, setTail] = useState<number>(50);
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  const loadLogs = useCallback(async () => {
    if (!selected) return;
    setLoading(true);
    try {
      const res = await apiFetch(`/api/logs/${encodeURIComponent(selected)}?tail=${tail}`);
      setLogs(await res.json());
    } catch {
      setLogs(["Error loading logs"]);
    }
    setLoading(false);
  }, [selected, tail]);

  useEffect(() => { if (selected) loadLogs(); }, [selected, loadLogs]);

  return (
    <Stack>
      <Paper shadow="sm" p="md" withBorder>
        <Group>
          <Select label="Container" placeholder="Selecciona un container"
            data={containers.map((c) => ({ value: c.name, label: c.name }))}
            value={selected} onChange={setSelected} searchable style={{ flex: 1 }} />
          <NumberInput label="Líneas" value={tail} onChange={(v) => setTail(Number(v) || 50)} min={10} max={500} w={100} />
          <Button onClick={loadLogs} loading={loading} mt="xl" variant="light">Recargar</Button>
        </Group>
      </Paper>
      <Paper shadow="sm" withBorder p="md">
        {!selected ? (
          <Text c="dimmed" ta="center" py="xl">Selecciona un container para ver sus logs</Text>
        ) : loading ? (
          <Group justify="center" py="xl"><Loader /></Group>
        ) : (
          <ScrollArea h={500}>
            <Code block>{logs.map((line, i) => <div key={i}>{line}</div>)}</Code>
          </ScrollArea>
        )}
      </Paper>
    </Stack>
  );
}