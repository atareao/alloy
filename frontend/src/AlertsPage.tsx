import { useState } from 'react'
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
  Divider,
} from '@mantine/core'
import { apiFetch } from './api'
import type { AlertRule, AppConfig } from './types'

interface AlertPageProps {
  containers: { name: string }[]
  alerts: AlertRule[]
  setAlerts: (a: AlertRule[]) => void
  config: AppConfig | null
}

const NOTIFY_OPTIONS = [
  { value: 'telegram', label: '📱 Telegram' },
  { value: 'matrix', label: '💬 Matrix' },
]

export default function AlertsPage({ containers, alerts, setAlerts, config }: AlertPageProps) {
  const isMobile = useMediaQuery('(max-width: 768px)')
  const [showModal, setShowModal] = useState(false)
  const [saving, setSaving] = useState(false)

  const [newContainer, setNewContainer] = useState<string | null>(null)
  const [newEnabled, setNewEnabled] = useState(true)
  const [newNotify, setNewNotify] = useState<string[]>([])

  const availableChannels = NOTIFY_OPTIONS.filter((opt) => {
    if (!config) return false
    if (opt.value === 'telegram') return config.telegram_configured
    if (opt.value === 'matrix') return config.matrix_configured
    return false
  })

  const hasNotificationChannels = availableChannels.length > 0

  const handleCreate = async () => {
    if (!newContainer) return
    setSaving(true)
    try {
      const res = await apiFetch('/api/alerts', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          container: newContainer,
          enabled: newEnabled,
          notify_via: newNotify,
        }),
      })
      if (res.ok) {
        const data = await res.json()
        setAlerts([...alerts, data])
        setShowModal(false)
        setNewContainer(null)
        setNewEnabled(true)
        setNewNotify([])
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleDelete = async (id: string) => {
    try {
      await apiFetch(`/api/alerts/${id}`, { method: 'DELETE' })
      setAlerts(alerts.filter((a) => a.id !== id))
    } catch { /* ignore */ }
  }

  // ── Mobile card ─────────────────────────────────────────────
  const renderMobileCard = (alert: AlertRule) => (
    <Paper key={alert.id} shadow="sm" p="sm" withBorder>
      <Stack gap="xs">
        <Group justify="space-between" wrap="nowrap">
          <Text size="sm" fw={500} truncate style={{ flex: 1 }}>{alert.container}</Text>
          <Badge size="sm" color={alert.enabled ? 'green' : 'gray'}>
            {alert.enabled ? 'Monitoreando' : 'Inactivo'}
          </Badge>
        </Group>
        <Divider />
        <Group gap="xs">
          <Text size="xs" c="dimmed">Notificar vía:</Text>
          {alert.notify_via.length > 0 ? (
            <Group gap={4}>
              {alert.notify_via.map((ch) => (
                <Badge key={ch} variant="light" color="blue" size="sm">
                  {ch === 'telegram' ? '📱 Telegram' : '💬 Matrix'}
                </Badge>
              ))}
            </Group>
          ) : (
            <Text size="xs" c="dimmed">—</Text>
          )}
        </Group>
        <Button
          size="xs"
          color="red"
          variant="light"
          fullWidth
          onClick={() => handleDelete(alert.id)}
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
            🔔 Alertas de estado · {alerts.length} monitoreados
          </Text>
          <Button onClick={() => setShowModal(true)} variant="filled" size={isMobile ? 'sm' : 'md'}>
            + Monitorear container
          </Button>
        </Group>
      </Paper>

      <Modal
        opened={showModal}
        onClose={() => { setShowModal(false); setNewContainer(null); setNewEnabled(true); setNewNotify([]) }}
        title="➕ Monitorear container"
        size={isMobile ? '100%' : 'md'}
      >
        <Stack>
          <Text size="sm" c="dimmed">
            Recibirás una notificación cuando el container cambie a un estado anómalo
            (exited, dead, paused, restarting) y cuando se recupere (vuelva a running).
          </Text>
          <Select
            label="Container"
            placeholder="Selecciona un container"
            data={containers.map((c) => ({ value: c.name, label: c.name }))}
            value={newContainer}
            onChange={setNewContainer}
            searchable
            clearable
          />
          {hasNotificationChannels ? (
            <Select
              label="Notificar vía"
              placeholder="Selecciona canales"
              data={availableChannels}
              value={newNotify.length === 1 ? newNotify[0] : null}
              onChange={(val) => setNewNotify(val ? [val] : [])}
              clearable
            />
          ) : (
            <Text size="sm" c="orange">
              ⚠️ No hay canales de notificación configurados.
              Para recibir alertas, configura Telegram o Matrix en las variables de entorno.
            </Text>
          )}
          <Switch
            label="Activado"
            checked={newEnabled}
            onChange={(e) => setNewEnabled(e.currentTarget.checked)}
          />
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={() => { setShowModal(false); setNewContainer(null); setNewEnabled(true); setNewNotify([]) }}>
              Cancelar
            </Button>
            <Button onClick={handleCreate} loading={saving} disabled={!newContainer}>
              Crear alerta
            </Button>
          </Group>
        </Stack>
      </Modal>

      {alerts.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay containers monitoreados. Añade uno para recibir notificaciones
            cuando cambie de estado.
          </Text>
        </Paper>
      ) : isMobile ? (
        <Stack gap="sm">
          {alerts.map(renderMobileCard)}
        </Stack>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table.ScrollContainer minWidth={450}>
            <Table striped highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Container</Table.Th>
                  <Table.Th>Notificar vía</Table.Th>
                  <Table.Th>Estado</Table.Th>
                  <Table.Th>Acción</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {alerts.map((alert) => (
                  <Table.Tr key={alert.id}>
                    <Table.Td>
                      <Text size="sm">{alert.container}</Text>
                    </Table.Td>
                    <Table.Td>
                      {alert.notify_via.length > 0 ? (
                        <Group gap={4}>
                          {alert.notify_via.map((ch) => (
                            <Badge key={ch} variant="light" color="blue" size="sm">
                              {ch === 'telegram' ? '📱 Telegram' : '💬 Matrix'}
                            </Badge>
                          ))}
                        </Group>
                      ) : (
                        <Text size="xs" c="dimmed">—</Text>
                      )}
                    </Table.Td>
                    <Table.Td>
                      <Badge color={alert.enabled ? 'green' : 'gray'}>
                        {alert.enabled ? 'Monitoreando' : 'Inactivo'}
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
          </Table.ScrollContainer>
        </Paper>
      )}
    </Stack>
  )
}