import { useState } from 'react'
import { useMediaQuery } from '@mantine/hooks'
import {
  Badge, Button, Group, Modal, Paper, Select, Stack, Switch, Table, Text, TextInput, Divider,
} from '@mantine/core'
import { apiFetch } from './api'
import type { UpdateCheckConfig, UpdatePolicy, UpdateAction, ContainerInfo } from './types'

interface CheckConfigPageProps {
  containers: ContainerInfo[]
  policies: UpdatePolicy[]
  setPolicies: (p: UpdatePolicy[]) => void
  config: UpdateCheckConfig | null
  setConfig: (c: UpdateCheckConfig) => void
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
];

const ACTION_OPTIONS = [
  { value: 'none', label: '❌ No hacer nada' },
  { value: 'pull', label: '⬇️ Pull imagen' },
  { value: 'pull-restart', label: '🔄 Pull + reiniciar contenedor' },
  { value: 'pull-restart-stack', label: '📦 Pull + reiniciar stack' },
];

export default function CheckConfigPage({ containers, policies, setPolicies, config, setConfig }: CheckConfigPageProps) {
  const isMobile = useMediaQuery('(max-width: 768px)')
  const [saving, setSaving] = useState(false)
  const [showPolicyModal, setShowPolicyModal] = useState(false)
  const [policyTarget, setPolicyTarget] = useState<string | null>(null)
  const [policyAction, setPolicyAction] = useState<UpdateAction>('pull')
  const [policyCleanup, setPolicyCleanup] = useState(false)
  const [policyRollback, setPolicyRollback] = useState(false)

  // Cron form
  const [cronExpr, setCronExpr] = useState(config?.cron || '0 */6 * * *')
  const [cronEnabled, setCronEnabled] = useState(config?.enabled || false)
  const [cronNotify, setCronNotify] = useState(config?.notify || false)

  const handleSaveConfig = async () => {
    setSaving(true)
    try {
      const body: UpdateCheckConfig = { cron: cronExpr, enabled: cronEnabled, notify: cronNotify }
      const res = await apiFetch('/api/update-check/config', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (res.ok) {
        const data = await res.json()
        setConfig(data)
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleSavePolicy = async () => {
    if (!policyTarget) return
    setSaving(true)
    try {
      const body = { action: policyAction, cleanup_old_image: policyCleanup, rollback_on_failure: policyRollback }
      const res = await apiFetch(`/api/update-policies/${encodeURIComponent(policyTarget)}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (res.ok) {
        const data = await res.json()
        const existing = policies.findIndex(p => p.container === policyTarget)
        if (existing >= 0) {
          const next = [...policies]
          next[existing] = data
          setPolicies(next)
        } else {
          setPolicies([...policies, data])
        }
        setShowPolicyModal(false)
        resetPolicyForm()
      }
    } catch { /* ignore */ }
    setSaving(false)
  }

  const handleDeletePolicy = async (container: string) => {
    try {
      await apiFetch(`/api/update-policies/${encodeURIComponent(container)}`, { method: 'DELETE' })
      setPolicies(policies.filter(p => p.container !== container))
    } catch { /* ignore */ }
  }

  const resetPolicyForm = () => {
    setPolicyTarget(null)
    setPolicyAction('pull')
    setPolicyCleanup(false)
    setPolicyRollback(false)
  }

  const actionLabel = (action: UpdateAction) => {
    const opt = ACTION_OPTIONS.find(o => o.value === action)
    return opt ? opt.label : action
  }

  const actionColor = (action: UpdateAction) => {
    switch (action) {
      case 'none': return 'gray'
      case 'pull': return 'blue'
      case 'pull-restart': return 'yellow'
      case 'pull-restart-stack': return 'violet'
    }
  }

  const renderPolicyCard = (p: UpdatePolicy) => (
    <Paper key={p.container} shadow="sm" p="sm" withBorder>
      <Stack gap="xs">
        <Group justify="space-between" wrap="nowrap">
          <Text size="sm" fw={500} truncate style={{ flex: 1 }}>{p.container}</Text>
          <Badge color={actionColor(p.action)} variant="light" size="sm">{actionLabel(p.action)}</Badge>
        </Group>
        <Divider />
        <Group gap="xs" wrap="wrap">
          {p.cleanup_old_image && <Badge size="sm" variant="light" color="yellow">🧹 Borrar anterior</Badge>}
          {p.rollback_on_failure && <Badge size="sm" variant="light" color="orange">🔄 Rollback</Badge>}
          {!p.cleanup_old_image && !p.rollback_on_failure && <Text size="xs" c="dimmed">Sin extras</Text>}
        </Group>
        <Button size="xs" color="red" variant="light" fullWidth onClick={() => handleDeletePolicy(p.container)}>Eliminar</Button>
      </Stack>
    </Paper>
  )

  return (
    <Stack>
      {/* ── Update Check Cron Config ─────────────────────────── */}
      <Paper shadow="sm" p="md" withBorder>
        <Text fw={500} mb="sm">⏰ Revisión de actualizaciones</Text>
        <Text size="sm" c="dimmed" mb="md">
          Programa revisiones periódicas de actualizaciones de imágenes.
          Cuando se detecte una actualización, se marcará el contenedor y
          se ejecutará la acción configurada en las políticas de abajo.
        </Text>
        <Stack gap="sm">
          <Select
            label="Frecuencia"
            data={CRON_PRESETS}
            value={cronExpr}
            onChange={(v) => v && setCronExpr(v)}
            searchable
          />
          <TextInput
            label="Expresión Cron (personalizada)"
            description="Edita directamente si los presets no se ajustan"
            placeholder="0 */6 * * *"
            value={cronExpr}
            onChange={(e) => setCronExpr(e.currentTarget.value)}
          />
          <Group>
            <Switch label="🔔 Notificar vía Telegram/Matrix" checked={cronNotify} onChange={(e) => setCronNotify(e.currentTarget.checked)} />
            <Switch label="Activada" checked={cronEnabled} onChange={(e) => setCronEnabled(e.currentTarget.checked)} />
          </Group>
          <Button onClick={handleSaveConfig} loading={saving} variant="filled" mt="sm">
            Guardar configuración
          </Button>
        </Stack>
      </Paper>

      {/* ── Update Policies ──────────────────────────────────── */}
      <Paper shadow="sm" p="md" withBorder>
        <Group justify="space-between" mb="md">
          <Text fw={500}>📋 Políticas de actualización</Text>
          <Button onClick={() => setShowPolicyModal(true)} variant="filled" size={isMobile ? 'sm' : 'md'}>
            + Añadir política
          </Button>
        </Group>
        <Text size="sm" c="dimmed" mb="md">
          Define qué acción ejecutar cuando un contenedor tenga una actualización pendiente.
        </Text>
      </Paper>

      <Modal
        opened={showPolicyModal}
        onClose={() => { setShowPolicyModal(false); resetPolicyForm() }}
        title="➕ Añadir política de actualización"
        size={isMobile ? '100%' : 'md'}
      >
        <Stack>
          <Select
            label="Contenedor / Stack"
            placeholder="Selecciona un contenedor"
            data={containers.map(c => ({ value: c.name, label: c.name }))}
            value={policyTarget}
            onChange={setPolicyTarget}
            searchable
            clearable
            required
          />
          <Select
            label="Acción"
            data={ACTION_OPTIONS}
            value={policyAction}
            onChange={(v) => v && setPolicyAction(v as UpdateAction)}
          />
          <Divider label="Opciones" labelPosition="center" />
          <Switch
            label="🧹 Borrar imagen anterior"
            description="Elimina la imagen anterior después de actualizar"
            checked={policyCleanup}
            onChange={(e) => setPolicyCleanup(e.currentTarget.checked)}
          />
          <Switch
            label="🔄 Rollback si falla"
            description="Si el contenedor no arranca tras la actualización, restaura la imagen anterior"
            checked={policyRollback}
            onChange={(e) => setPolicyRollback(e.currentTarget.checked)}
          />
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={() => { setShowPolicyModal(false); resetPolicyForm() }}>Cancelar</Button>
            <Button onClick={handleSavePolicy} loading={saving} disabled={!policyTarget}>Guardar</Button>
          </Group>
        </Stack>
      </Modal>

      {policies.length === 0 ? (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            No hay políticas configuradas. Añade políticas para definir qué hacer
            cuando un contenedor tenga una actualización disponible.
          </Text>
        </Paper>
      ) : isMobile ? (
        <Stack gap="sm">{policies.map(renderPolicyCard)}</Stack>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table.ScrollContainer minWidth={500}>
            <Table striped highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Contenedor</Table.Th>
                  <Table.Th>Acción</Table.Th>
                  <Table.Th>🧹 Borrar</Table.Th>
                  <Table.Th>🔄 Rollback</Table.Th>
                  <Table.Th>Acción</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {policies.map((p) => (
                  <Table.Tr key={p.container}>
                    <Table.Td><Text size="sm" fw={500}>{p.container}</Text></Table.Td>
                    <Table.Td><Badge variant="light" color={actionColor(p.action)}>{actionLabel(p.action)}</Badge></Table.Td>
                    <Table.Td><Text size="sm">{p.cleanup_old_image ? '✅' : '—'}</Text></Table.Td>
                    <Table.Td><Text size="sm">{p.rollback_on_failure ? '✅' : '—'}</Text></Table.Td>
                    <Table.Td>
                      <Button size="xs" color="red" variant="light" onClick={() => handleDeletePolicy(p.container)}>Eliminar</Button>
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