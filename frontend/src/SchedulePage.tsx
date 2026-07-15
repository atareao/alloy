import { useState, useMemo } from 'react'
import { useMediaQuery } from '@mantine/hooks'
import {
  Badge,
  Button,
  Group,
  Modal,
  Paper,
  Select,
  Stack,
  Switch,
  Table,
  Text,
  TextInput,
  Tooltip,
  Divider,
  SegmentedControl,
} from '@mantine/core'
import { apiFetch } from './api'
import type { ScheduleEntry } from './types'

interface ContainerInfo {
  name: string
  compose_project?: string
}

interface SchedulePageProps {
  containers: ContainerInfo[]
  schedules: ScheduleEntry[]
  setSchedules: (s: ScheduleEntry[]) => void
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

const CONTAINER_ACTIONS = [
  { value: 'update', label: '🔄 Update + restart' },
  { value: 'restart', label: '🔄 Reiniciar' },
  { value: 'check-update', label: '🔍 Check update' },
]

const STACK_ACTIONS = [
  { value: 'update', label: '⬆️ Actualizar stack' },
  { value: 'check-update', label: '🔍 Check updates' },
]

export default function SchedulePage({ containers, schedules, setSchedules }: SchedulePageProps) {
  const isMobile = useMediaQuery('(max-width: 768px)')
  const [showModal, setShowModal] = useState(false)
  const [saving, setSaving] = useState(false)

  // New schedule form
  const [targetType, setTargetType] = useState<'container' | 'stack'>('container')
  const [newTarget, setNewTarget] = useState<string | null>(null)
  const [newCron, setNewCron] = useState('0 */6 * * *')
  const [newAction, setNewAction] = useState<string | null>('update')
  const [newEnabled, setNewEnabled] = useState(true)
  const [newNotify, setNewNotify] = useState(false)
  const [newCleanup, setNewCleanup] = useState<string>('none')

  // Derive unique stack names from containers
  const stacks = useMemo(() => {
    const seen = new Set<string>()
    const result: string[] = []
    for (const c of containers) {
      if (c.compose_project && !seen.has(c.compose_project)) {
        seen.add(c.compose_project)
        result.push(c.compose_project)
      }
    }
    return result.sort()
  }, [containers])

  const handleCreate = async () => {
    if (!newTarget || !newAction) return
    setSaving(true)
    try {
      const body: Record<string, any> = {
        container: newTarget,
        target_type: targetType,
        cron: newCron,
        action: newAction,
        enabled: newEnabled,
        notify: newNotify,
        cleanup: newCleanup,
      }
      const res = await apiFetch('/api/schedule', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (res.ok) {
        const data = await res.json()
        setSchedules([...schedules, data])
        setShowModal(false)
        resetForm()
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleDelete = async (id: string) => {
    try {
      await apiFetch(`/api/schedule/${id}`, { method: 'DELETE' })
      setSchedules(schedules.filter((s) => s.id !== id))
    } catch { /* ignore */ }
  }

  const resetForm = () => {
    setTargetType('container')
    setNewTarget(null)
    setNewCron('0 */6 * * *')
    setNewAction('update')
    setNewEnabled(true)
    setNewNotify(false)
    setNewCleanup('none')
  }

  const actionColor = (action: string) => {
    switch (action) {
      case 'update': return 'blue'
      case 'restart': return 'yellow'
      case 'check-update': return 'violet'
      case 'prune': return 'red'
      default: return 'gray'
    }
  }

  const actionLabel = (action: string) => {
    const allOpts = [...CONTAINER_ACTIONS, ...STACK_ACTIONS]
    const opt = allOpts.find((o) => o.value === action)
    return opt ? opt.label : action
  }

  const targetLabel = (entry: ScheduleEntry) => {
    const icon = entry.target_type === 'stack' ? '📦' : '📦'
    return `${icon} ${entry.container}`
  }

  // ── Mobile card ─────────────────────────────────────────────
  const renderMobileCard = (sched: ScheduleEntry) => (
    <Paper key={sched.id} shadow="sm" p="sm" withBorder>
      <Stack gap="xs">
        <Group justify="space-between" wrap="nowrap">
          <Text size="sm" fw={500} truncate style={{ flex: 1 }}>
            {targetLabel(sched)}
          </Text>
          <Badge size="sm" color={sched.enabled ? 'green' : 'gray'}>
            {sched.enabled ? 'Activa' : 'Inactiva'}
          </Badge>
        </Group>
        <Divider />
        <Stack gap={2}>
          <Group gap="xs">
            <Text size="xs" c="dimmed">Tipo:</Text>
            <Badge size="sm" variant="light" color={sched.target_type === 'stack' ? 'grape' : 'blue'}>
              {sched.target_type === 'stack' ? '📦 Stack' : '📦 Container'}
            </Badge>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">Acción:</Text>
            <Badge color={actionColor(sched.action)} variant="light" size="sm">
              {actionLabel(sched.action)}
            </Badge>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">Cron:</Text>
            <Text size="xs" style={{ fontFamily: 'monospace' }}>{sched.cron}</Text>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">Notificar:</Text>
            <Text size="xs">{sched.notify ? '✅ Sí' : '❌ No'}</Text>
          </Group>
          <Group gap="xs">
            <Text size="xs" c="dimmed">Limpiar:</Text>
            <Badge size="sm" variant="light" color={sched.cleanup === 'rollback' ? 'orange' : sched.cleanup === 'delete-old' ? 'yellow' : 'gray'}>
              {sched.cleanup === 'rollback' ? '🔄 Rollback' : sched.cleanup === 'delete-old' ? '🧹 Borrar' : '—'}
            </Badge>
          </Group>
        </Stack>
        <Button
          size="xs"
          color="red"
          variant="light"
          fullWidth
          onClick={() => handleDelete(sched.id)}
        >
          Eliminar
        </Button>
      </Stack>
    </Paper>
  )

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            ⏰ Planificador Cron · {schedules.length} tareas
          </Text>
          <Button onClick={() => setShowModal(true)} variant="filled" size={isMobile ? 'sm' : 'md'}>
            + Nueva tarea
          </Button>
        </Group>
      </Paper>

      <Modal
        opened={showModal}
        onClose={() => { setShowModal(false); resetForm() }}
        title="➕ Nueva tarea programada"
        size={isMobile ? '100%' : 'md'}
      >
        <Stack>
          <SegmentedControl
            value={targetType}
            onChange={(v) => { setTargetType(v as 'container' | 'stack'); setNewTarget(null); setNewAction('update') }}
            data={[
              { value: 'container', label: '📦 Container' },
              { value: 'stack', label: '📦 Stack' },
            ]}
            fullWidth
          />

          {targetType === 'container' ? (
            <Select
              label="Container"
              placeholder="Selecciona un container"
              data={[
                { value: '*', label: '🌟 Todos los containers' },
                ...containers.map((c) => ({ value: c.name, label: c.name })),
              ]}
              value={newTarget}
              onChange={setNewTarget}
              searchable
              required
            />
          ) : (
            <Select
              label="Stack"
              placeholder="Selecciona un stack"
              data={
                stacks.length > 0
                  ? stacks.map((s) => ({ value: s, label: s }))
                  : [{ value: '', label: 'No hay stacks disponibles', disabled: true }]
              }
              value={newTarget}
              onChange={setNewTarget}
              searchable
              required
            />
          )}

          <Select
            label="Acción"
            data={targetType === 'container' ? CONTAINER_ACTIONS : STACK_ACTIONS}
            value={newAction}
            onChange={setNewAction}
          />

          {newAction === 'update' && (
            <Select
              label="🧹 Limpieza / Rollback"
              description="Qué hacer con la imagen anterior tras actualizar"
              data={[
                { value: 'none', label: 'Solo actualizar' },
                { value: 'delete-old', label: '+ Borrar imagen anterior' },
                { value: 'rollback', label: '+ Rollback si falla' },
              ]}
              value={newCleanup}
              onChange={(v) => v && setNewCleanup(v)}
            />
          )}

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
            label="🔔 Notificar vía Telegram/Matrix"
            checked={newNotify}
            onChange={(e) => setNewNotify(e.currentTarget.checked)}
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
      ) : isMobile ? (
        <Stack gap="sm">
          {schedules.map(renderMobileCard)}
        </Stack>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table.ScrollContainer minWidth={600}>
            <Table striped highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Target</Table.Th>
                  <Table.Th>Tipo</Table.Th>
                  <Table.Th>Acción</Table.Th>
                  <Table.Th>Exp. Cron</Table.Th>
                  <Table.Th>Limpiar</Table.Th>
                  <Table.Th>Notificar</Table.Th>
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
                      <Badge size="sm" variant="light" color={sched.target_type === 'stack' ? 'grape' : 'blue'}>
                        {sched.target_type === 'stack' ? 'Stack' : 'Container'}
                      </Badge>
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
                      <Badge size="sm" variant="light" color={sched.cleanup === 'rollback' ? 'orange' : sched.cleanup === 'delete-old' ? 'yellow' : 'gray'}>
                        {sched.cleanup === 'rollback' ? '🔄 Rollback' : sched.cleanup === 'delete-old' ? '🧹' : '—'}
                      </Badge>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">{sched.notify ? '✅' : '—'}</Text>
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
          </Table.ScrollContainer>
        </Paper>
      )}
    </Stack>
  )
}