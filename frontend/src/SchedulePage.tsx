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
  Tooltip,
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

interface ScheduleEntry {
  id: number
  container: string
  cron: string
  action: string
  enabled: boolean
}

// ═══════════════════════════════════════════════════════════════
// Page: Schedule (planificador cron)
// ═══════════════════════════════════════════════════════════════

interface SchedulePageProps {
  containers: { name: string }[]
}

const CRON_PRESETS = [
  { value: '0 */6 * * *', label: 'Cada 6 horas' },
  { value: '0 */12 * * *', label: 'Cada 12 horas' },
  { value: '0 0 * * *', label: 'Cada día a medianoche' },
  { value: '0 6 * * *', label: 'Cada día a las 6:00' },
  { value: '0 0 * * 0', label: 'Cada domingo' },
  { value: '0 0 1 * *', label: 'Cada 1 del mes' },
  { value: '*/30 * * * *', label: 'Cada 30 minutos' },
  { value: '0 */1 * * *', label: 'Cada hora' },
]

const ACTION_OPTIONS = [
  { value: 'update', label: '🔄 Actualizar imagen' },
  { value: 'restart', label: '🔄 Reiniciar container' },
  { value: 'prune', label: '🧹 Prune system' },
  { value: 'check-update', label: '🔍 Verificar actualización' },
]

export default function SchedulePage({ containers }: SchedulePageProps) {
  const [schedules, setSchedules] = useState<ScheduleEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [showModal, setShowModal] = useState(false)
  const [saving, setSaving] = useState(false)

  // New schedule form
  const [newContainer, setNewContainer] = useState<string | null>(null)
  const [newCron, setNewCron] = useState('0 */6 * * *')
  const [newAction, setNewAction] = useState<string | null>('update')
  const [newEnabled, setNewEnabled] = useState(true)

  const loadSchedules = useCallback(async () => {
    setLoading(true)
    try {
      const res = await apiFetch('/api/schedule')
      setSchedules(await res.json())
    } catch { /* ignore */ }
    setLoading(false)
  }, [])

  useEffect(() => { loadSchedules() }, [loadSchedules])

  const handleCreate = async () => {
    if (!newContainer || !newAction) return
    setSaving(true)
    try {
      const res = await apiFetch('/api/schedule', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          container: newContainer,
          cron: newCron,
          action: newAction,
          enabled: newEnabled,
        }),
      })
      if (res.ok) {
        await loadSchedules()
        setShowModal(false)
        resetForm()
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleDelete = async (id: number) => {
    try {
      await apiFetch(`/api/schedule/${id}`, { method: 'DELETE' })
      setSchedules((prev) => prev.filter((s) => s.id !== id))
    } catch { /* ignore */ }
  }

  const resetForm = () => {
    setNewContainer(null)
    setNewCron('0 */6 * * *')
    setNewAction('update')
    setNewEnabled(true)
  }

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>

  const actionColor = (action: string) => {
    switch (action) {
      case 'update': return 'blue'
      case 'restart': return 'yellow'
      case 'prune': return 'red'
      case 'check-update': return 'violet'
      default: return 'gray'
    }
  }

  const actionLabel = (action: string) => {
    const opt = ACTION_OPTIONS.find((o) => o.value === action)
    return opt ? opt.label : action
  }

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            ⏰ Planificador Cron · {schedules.length} tareas
          </Text>
          <Button onClick={() => setShowModal(true)} variant="filled">
            + Nueva tarea
          </Button>
        </Group>
      </Paper>

      <Modal
        opened={showModal}
        onClose={() => { setShowModal(false); resetForm() }}
        title="➕ Nueva tarea programada"
        size="md"
      >
        <Stack>
          <Select
            label="Container"
            placeholder="Selecciona un container"
            data={containers.map((c) => ({ value: c.name, label: c.name }))}
            value={newContainer}
            onChange={setNewContainer}
            searchable
            required
          />
          <Select
            label="Acción"
            data={ACTION_OPTIONS}
            value={newAction}
            onChange={setNewAction}
          />
          <Select
            label="Frecuencia (Cron)"
            data={CRON_PRESETS}
            value={newCron}
            onChange={(v) => v && setNewCron(v)}
            searchable
          />
          <TextInput
            label="Expresión Cron (personalizada)"
            description="Edita directamente la expresión si los presets no se ajustan"
            placeholder="0 */6 * * *"
            value={newCron}
            onChange={(e) => setNewCron(e.currentTarget.value)}
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
              Crear tarea
            </Button>
          </Group>
        </Stack>
      </Modal>

      {schedules.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay tareas programadas. Añade tareas cron para automatizar
            actualizaciones, reinicios y limpieza de Docker.
          </Text>
        </Paper>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table striped highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Container</Table.Th>
                <Table.Th>Acción</Table.Th>
                <Table.Th>Expresión Cron</Table.Th>
                <Table.Th>Estado</Table.Th>
                <Table.Th>Acción</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {schedules.map((sched) => (
                <Table.Tr key={sched.id}>
                  <Table.Td>
                    <Text size="sm" fw={500}>{sched.container}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={actionColor(sched.action)} variant="light">
                      {actionLabel(sched.action)}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Tooltip label={sched.cron}>
                      <Text size="xs" style={{ fontFamily: 'monospace' }}>
                        {sched.cron}
                      </Text>
                    </Tooltip>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={sched.enabled ? 'green' : 'gray'}>
                      {sched.enabled ? 'Activa' : 'Inactiva'}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Button
                      size="xs"
                      color="red"
                      variant="light"
                      onClick={() => handleDelete(sched.id)}
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