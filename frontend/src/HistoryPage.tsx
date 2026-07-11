import { useCallback, useEffect, useState } from 'react'
import {
  Badge,
  Button,
  Group,
  Loader,
  Modal,
  Paper,
  Stack,
  Table,
  Text,
} from '@mantine/core'

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

function apiFetch(path: string, opts?: RequestInit) {
  const token = localStorage.getItem('token')
  return fetch(path, {
    ...opts,
    headers: { ...opts?.headers, 'Authorization': `Bearer ${token}` },
  })
}

// ═══════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════

interface HistoryEntry {
  container: string
  image: string
  old_digest: string
  new_digest: string
  timestamp: string
  status: string
  duration_ms: number
}

// ═══════════════════════════════════════════════════════════════
// Page: History (histórico de updates)
// ═══════════════════════════════════════════════════════════════

export default function HistoryPage() {
  const [history, setHistory] = useState<HistoryEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [clearing, setClearing] = useState(false)
  const [confirmClear, setConfirmClear] = useState(false)

  const loadHistory = useCallback(async () => {
    setLoading(true)
    try {
      const res = await apiFetch('/api/history')
      setHistory(await res.json())
    } catch { /* ignore */ }
    setLoading(false)
  }, [])

  useEffect(() => { loadHistory() }, [loadHistory])

  const handleClear = async () => {
    setClearing(true)
    try {
      await apiFetch('/api/history', { method: 'DELETE' })
      setHistory([])
    } catch { /* ignore */ }
    setClearing(false)
    setConfirmClear(false)
  }

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>

  const statusColor = (status: string) => {
    switch (status) {
      case 'success': return 'green'
      case 'failed': return 'red'
      case 'skipped': return 'yellow'
      default: return 'gray'
    }
  }

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
    return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`
  }

  const formatDate = (ts: string) => {
    try {
      return new Date(ts).toLocaleString()
    } catch {
      return ts
    }
  }

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            📜 Histórico de actualizaciones · {history.length} entradas
          </Text>
          {history.length > 0 && (
            <Button
              onClick={() => setConfirmClear(true)}
              variant="filled"
              color="red"
              size="xs"
            >
              🗑️ Limpiar historial
            </Button>
          )}
        </Group>
      </Paper>

      <Modal
        opened={confirmClear}
        onClose={() => setConfirmClear(false)}
        title="🗑️ Limpiar historial"
        size="sm"
      >
        <Text size="sm" mb="md">
          ¿Estás seguro de que deseas eliminar todo el historial de actualizaciones?
          Esta acción no se puede deshacer.
        </Text>
        <Group justify="flex-end">
          <Button variant="default" onClick={() => setConfirmClear(false)}>
            Cancelar
          </Button>
          <Button color="red" onClick={handleClear} loading={clearing}>
            Eliminar todo
          </Button>
        </Group>
      </Modal>

      {history.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay historial de actualizaciones. Cuando se actualice un container,
            aparecerá aquí.
          </Text>
        </Paper>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table striped highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Container</Table.Th>
                <Table.Th>Imagen</Table.Th>
                <Table.Th>Versión anterior</Table.Th>
                <Table.Th>Nueva versión</Table.Th>
                <Table.Th>Estado</Table.Th>
                <Table.Th>Duración</Table.Th>
                <Table.Th>Fecha</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {history.map((entry, i) => (
                <Table.Tr key={i}>
                  <Table.Td>
                    <Text size="sm" fw={500}>{entry.container}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{entry.image}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed" style={{ fontFamily: 'monospace' }}>
                      {entry.old_digest?.length > 20
                        ? entry.old_digest.substring(0, 20) + '...'
                        : entry.old_digest || '-'}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed" style={{ fontFamily: 'monospace' }}>
                      {entry.new_digest?.length > 20
                        ? entry.new_digest.substring(0, 20) + '...'
                        : entry.new_digest || '-'}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={statusColor(entry.status)}>
                      {entry.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{formatDuration(entry.duration_ms)}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{formatDate(entry.timestamp)}</Text>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        </Paper>
      )}
    </Stack>
  )
}