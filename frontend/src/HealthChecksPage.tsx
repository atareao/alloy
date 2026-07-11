import { useCallback, useEffect, useState } from 'react'
import {
  Badge,
  Button,
  Group,
  Loader,
  Modal,
  NumberInput,
  Paper,
  Select,
  Stack,
  Switch,
  Table,
  Text,
  TextInput,
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

interface HealthCheck {
  id: number
  type: string
  target: string
  interval_secs: number
  container: string
  enabled: boolean
  status: string
  latency_ms: number | null
  last_checked: string | null
}

// ═══════════════════════════════════════════════════════════════
// Page: Health Checks
// ═══════════════════════════════════════════════════════════════

interface HealthChecksPageProps {
  containers: { name: string }[]
}

export default function HealthChecksPage({ containers }: HealthChecksPageProps) {
  const [checks, setChecks] = useState<HealthCheck[]>([])
  const [loading, setLoading] = useState(true)
  const [showModal, setShowModal] = useState(false)
  const [saving, setSaving] = useState(false)

  // New check form
  const [newType, setNewType] = useState<string | null>('http')
  const [newTarget, setNewTarget] = useState('')
  const [newInterval, setNewInterval] = useState<number>(30)
  const [newContainer, setNewContainer] = useState<string | null>(null)
  const [newEnabled, setNewEnabled] = useState(true)

  const loadChecks = useCallback(async () => {
    setLoading(true)
    try {
      const res = await apiFetch('/api/health-checks')
      setChecks(await res.json())
    } catch { /* ignore */ }
    setLoading(false)
  }, [])

  useEffect(() => { loadChecks() }, [loadChecks])

  const handleCreate = async () => {
    if (!newType || !newTarget) return
    setSaving(true)
    try {
      const res = await apiFetch('/api/health-checks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: newType,
          target: newTarget,
          interval_secs: newInterval,
          container: newContainer || '',
          enabled: newEnabled,
        }),
      })
      if (res.ok) {
        await loadChecks()
        setShowModal(false)
        resetForm()
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleDelete = async (id: number) => {
    try {
      await apiFetch(`/api/health-checks/${id}`, { method: 'DELETE' })
      setChecks((prev) => prev.filter((c) => c.id !== id))
    } catch { /* ignore */ }
  }

  const resetForm = () => {
    setNewType('http')
    setNewTarget('')
    setNewInterval(30)
    setNewContainer(null)
    setNewEnabled(true)
  }

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>

  const statusColor = (status: string) => {
    switch (status) {
      case 'healthy': return 'green'
      case 'unhealthy': return 'red'
      case 'pending': return 'yellow'
      default: return 'gray'
    }
  }

  const formatLatency = (ms: number | null) => {
    if (ms === null || ms === undefined) return '-'
    if (ms < 1000) return `${ms.toFixed(0)}ms`
    return `${(ms / 1000).toFixed(2)}s`
  }

  const formatLastChecked = (ts: string | null) => {
    if (!ts) return 'Nunca'
    try {
      return new Date(ts).toLocaleString()
    } catch {
      return ts
    }
  }

  const friendlyType = (type: string) => {
    switch (type) {
      case 'http': return '🌐 HTTP'
      case 'ping': return '📡 PING'
      default: return type
    }
  }

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            ❤️ Health Checks · {checks.length} checks
          </Text>
          <Button onClick={() => setShowModal(true)} variant="filled">
            + Nuevo health check
          </Button>
        </Group>
      </Paper>

      <Modal
        opened={showModal}
        onClose={() => { setShowModal(false); resetForm() }}
        title="➕ Nuevo Health Check"
        size="md"
      >
        <Stack>
          <Select
            label="Tipo"
            data={[
              { value: 'http', label: '🌐 HTTP' },
              { value: 'ping', label: '📡 PING' },
            ]}
            value={newType}
            onChange={setNewType}
          />
          <TextInput
            label="Target"
            placeholder={newType === 'http' ? 'https://ejemplo.com/health' : '8.8.8.8'}
            value={newTarget}
            onChange={(e) => setNewTarget(e.currentTarget.value)}
            required
          />
          <NumberInput
            label="Intervalo (segundos)"
            value={newInterval}
            onChange={(v) => setNewInterval(Number(v) || 30)}
            min={5}
            max={3600}
          />
          {newType === 'http' && (
            <Select
              label="Container (opcional)"
              placeholder="Ninguno"
              data={[
                { value: '', label: 'Ninguno' },
                ...containers.map((c) => ({ value: c.name, label: c.name })),
              ]}
              value={newContainer}
              onChange={setNewContainer}
              searchable
              clearable
            />
          )}
          <Switch
            label="Activado"
            checked={newEnabled}
            onChange={(e) => setNewEnabled(e.currentTarget.checked)}
          />
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={() => { setShowModal(false); resetForm() }}>
              Cancelar
            </Button>
            <Button onClick={handleCreate} loading={saving}>
              Crear health check
            </Button>
          </Group>
        </Stack>
      </Modal>

      {checks.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay health checks configurados. Añade checks HTTP o PING para
            monitorizar la disponibilidad de tus servicios.
          </Text>
        </Paper>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table striped highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Tipo</Table.Th>
                <Table.Th>Target</Table.Th>
                <Table.Th>Intervalo</Table.Th>
                <Table.Th>Container</Table.Th>
                <Table.Th>Estado</Table.Th>
                <Table.Th>Latencia</Table.Th>
                <Table.Th>Último check</Table.Th>
                <Table.Th>Activo</Table.Th>
                <Table.Th>Acción</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {checks.map((check) => (
                <Table.Tr key={check.id}>
                  <Table.Td>
                    <Badge variant="light" color="teal">
                      {friendlyType(check.type)}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" style={{ fontFamily: 'monospace' }}>
                      {check.target}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{check.interval_secs}s</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{check.container || '-'}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={statusColor(check.status)}>
                      {check.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{formatLatency(check.latency_ms)}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" c="dimmed">{formatLastChecked(check.last_checked)}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={check.enabled ? 'green' : 'gray'} size="sm">
                      {check.enabled ? '✅' : '❌'}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Button
                      size="xs"
                      color="red"
                      variant="light"
                      onClick={() => handleDelete(check.id)}
                    >
                      Eliminar
                    </Button>
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