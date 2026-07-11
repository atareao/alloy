import { useCallback, useEffect, useState } from 'react'
import {
  Badge,
  Button,
  Group,
  Loader,
  Modal,
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

interface AlertRule {
  id: number
  type: string
  container: string
  threshold: string
  enabled: boolean
  notify_via: string
}

// ═══════════════════════════════════════════════════════════════
// Page: Alerts (alertas custom)
// ═══════════════════════════════════════════════════════════════

interface AlertsPageProps {
  containers: { name: string }[]
}

export default function AlertsPage({ containers }: AlertsPageProps) {
  const [alerts, setAlerts] = useState<AlertRule[]>([])
  const [loading, setLoading] = useState(true)
  const [showModal, setShowModal] = useState(false)
  const [saving, setSaving] = useState(false)

  // New alert form state
  const [newType, setNewType] = useState<string | null>('resource')
  const [newContainer, setNewContainer] = useState<string | null>(null)
  const [newThreshold, setNewThreshold] = useState('')
  const [newNotifyVia, setNewNotifyVia] = useState<string | null>('telegram')
  const [newEnabled, setNewEnabled] = useState(true)

  const loadAlerts = useCallback(async () => {
    setLoading(true)
    try {
      const res = await apiFetch('/api/alerts')
      setAlerts(await res.json())
    } catch { /* ignore */ }
    setLoading(false)
  }, [])

  useEffect(() => { loadAlerts() }, [loadAlerts])

  const handleCreate = async () => {
    if (!newType) return
    setSaving(true)
    try {
      const res = await apiFetch('/api/alerts', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: newType,
          container: newContainer || '',
          threshold: newThreshold,
          enabled: newEnabled,
          notify_via: newNotifyVia || 'telegram',
        }),
      })
      if (res.ok) {
        await loadAlerts()
        setShowModal(false)
        resetForm()
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleDelete = async (id: number) => {
    try {
      await apiFetch(`/api/alerts/${id}`, { method: 'DELETE' })
      setAlerts((prev) => prev.filter((a) => a.id !== id))
    } catch { /* ignore */ }
  }

  const resetForm = () => {
    setNewType('resource')
    setNewContainer(null)
    setNewThreshold('')
    setNewNotifyVia('telegram')
    setNewEnabled(true)
  }

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>

  const typeColor = (type: string) => {
    switch (type) {
      case 'resource': return 'orange'
      case 'status': return 'blue'
      case 'custom': return 'grape'
      default: return 'gray'
    }
  }

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            🔔 Alertas · {alerts.length} reglas
          </Text>
          <Button onClick={() => setShowModal(true)} variant="filled">
            + Nueva alerta
          </Button>
        </Group>
      </Paper>

      <Modal
        opened={showModal}
        onClose={() => { setShowModal(false); resetForm() }}
        title="➕ Nueva alerta"
        size="md"
      >
        <Stack>
          <Select
            label="Tipo"
            data={[
              { value: 'resource', label: '💾 Recurso (CPU/RAM)' },
              { value: 'status', label: '🔄 Estado' },
              { value: 'custom', label: '⚙️ Personalizada' },
            ]}
            value={newType}
            onChange={setNewType}
          />
          <Select
            label="Container"
            placeholder="Todos los containers"
            data={[
              { value: '', label: 'Todos los containers' },
              ...containers.map((c) => ({ value: c.name, label: c.name })),
            ]}
            value={newContainer}
            onChange={setNewContainer}
            searchable
            clearable
          />
          <TextInput
            label="Umbral"
            placeholder="p.ej. cpu>80, memory>500mb"
            value={newThreshold}
            onChange={(e) => setNewThreshold(e.currentTarget.value)}
          />
          <Select
            label="Notificar vía"
            data={[
              { value: 'telegram', label: '📱 Telegram' },
              { value: 'matrix', label: '💬 Matrix' },
              { value: 'both', label: 'Ambos' },
            ]}
            value={newNotifyVia}
            onChange={setNewNotifyVia}
          />
          <Switch
            label="Activada"
            checked={newEnabled}
            onChange={(e) => setNewEnabled(e.currentTarget.checked)}
          />
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={() => { setShowModal(false); resetForm() }}>
              Cancelar
            </Button>
            <Button onClick={handleCreate} loading={saving}>
              Crear alerta
            </Button>
          </Group>
        </Stack>
      </Modal>

      {alerts.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay alertas configuradas. Crea una alerta para recibir notificaciones
            cuando un container supere un umbral o cambie de estado.
          </Text>
        </Paper>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table striped highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Tipo</Table.Th>
                <Table.Th>Container</Table.Th>
                <Table.Th>Umbral</Table.Th>
                <Table.Th>Notificar vía</Table.Th>
                <Table.Th>Estado</Table.Th>
                <Table.Th>Acción</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {alerts.map((alert) => (
                <Table.Tr key={alert.id}>
                  <Table.Td>
                    <Badge color={typeColor(alert.type)}>{alert.type}</Badge>
                  </Table.Td>
                  <Table.Td>
                    <Text size="sm">{alert.container || 'Todos'}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="xs" style={{ fontFamily: 'monospace' }}>
                      {alert.threshold}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge variant="light" color="blue">{alert.notify_via}</Badge>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={alert.enabled ? 'green' : 'gray'}>
                      {alert.enabled ? 'Activa' : 'Inactiva'}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Button
                      size="xs"
                      color="red"
                      variant="light"
                      onClick={() => handleDelete(alert.id)}
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