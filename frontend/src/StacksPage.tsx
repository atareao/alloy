import { useCallback, useEffect, useState } from 'react'
import {
  Badge,
  Button,
  Group,
  Loader,
  Paper,
  Stack,
  Table,
  Text,
  Title,
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

interface ServiceInfo {
  service: string
  container_name: string
  image: string
  status: string
}

interface StackInfo {
  project: string
  services: ServiceInfo[]
}

// ═══════════════════════════════════════════════════════════════
// Page: Stacks (docker-compose)
// ═══════════════════════════════════════════════════════════════

export default function StacksPage() {
  const [stacks, setStacks] = useState<StackInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [updatingProject, setUpdatingProject] = useState<string | null>(null)

  const loadStacks = useCallback(async () => {
    setLoading(true)
    try {
      const res = await apiFetch('/api/stacks')
      setStacks(await res.json())
    } catch { /* ignore */ }
    setLoading(false)
  }, [])

  useEffect(() => { loadStacks() }, [loadStacks])

  const handleUpdate = useCallback(async (project: string) => {
    if (updatingProject) return // already updating
    setUpdatingProject(project)
    try {
      await apiFetch(`/api/stacks/${encodeURIComponent(project)}/update`, { method: 'POST' })
    } catch { /* ignore */ }
    // Reload stacks after update completes
    await loadStacks()
    setUpdatingProject(null)
  }, [updatingProject, loadStacks])

  if (loading) return <Group justify="center" py="xl"><Loader /></Group>

  if (stacks.length === 0) return (
    <Paper shadow="sm" p="xl" withBorder>
      <Text ta="center" c="dimmed">No se encontraron stacks de docker-compose</Text>
    </Paper>
  )

  const statusColor = (status: string) => {
    if (status.includes('Up') || status.includes('healthy')) return 'green'
    if (status.includes('running')) return 'blue'
    if (status.includes('paused')) return 'yellow'
    return 'red'
  }

  return (
    <Stack>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Group justify="space-between">
          <Text size="sm" c="dimmed">🧩 {stacks.length} stacks de docker-compose</Text>
          <Badge size="lg" variant="light" color="violet">
            {stacks.reduce((acc, s) => acc + s.services.length, 0)} servicios
          </Badge>
        </Group>
      </Paper>

      {stacks.map((stack) => (
        <Paper key={stack.project} shadow="sm" withBorder>
          <Stack gap={0}>
            <Paper p="sm" style={{ background: 'var(--mantine-color-dark-6)' }}>
              <Group justify="space-between">
                <Title order={4}>
                  📦 {stack.project}
                </Title>
                <Tooltip label={updatingProject === stack.project ? 'Actualizando...' : 'Actualizar este stack (pull + up -d)'}>
                  <Button
                    size="xs"
                    variant="light"
                    color="cyan"
                    loading={updatingProject === stack.project}
                    disabled={updatingProject !== null}
                    onClick={() => handleUpdate(stack.project)}
                  >
                    {updatingProject === stack.project ? 'Actualizando...' : 'Actualizar stack'}
                  </Button>
                </Tooltip>
              </Group>
            </Paper>
            <Table striped highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Servicio</Table.Th>
                  <Table.Th>Container</Table.Th>
                  <Table.Th>Imagen</Table.Th>
                  <Table.Th>Estado</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {stack.services.map((svc) => (
                  <Table.Tr key={svc.service}>
                    <Table.Td>
                      <Text size="sm" fw={500}>{svc.service}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="xs" c="dimmed">{svc.container_name}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="xs" c="dimmed">{svc.image}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Badge color={statusColor(svc.status)}>
                        {svc.status}
                      </Badge>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </Stack>
        </Paper>
      ))}
    </Stack>
  )
}
