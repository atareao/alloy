import { useEffect, useRef, useState } from 'react'
import {
  Button,
  Group,
  Loader,
  Paper,
  Select,
  Stack,
  Text,
  TextInput,
} from '@mantine/core'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'

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
// Page: Terminal (xterm.js)
// ═══════════════════════════════════════════════════════════════

interface TerminalPageProps {
  containers: { name: string }[]
}

export default function TerminalPage({ containers }: TerminalPageProps) {
  const terminalRef = useRef<HTMLDivElement>(null)
  const termInstance = useRef<Terminal | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)
  const eventSourceRef = useRef<EventSource | null>(null)
  const [selected, setSelected] = useState<string | null>(null)
  const [connected, setConnected] = useState(false)
  const [loading, setLoading] = useState(false)
  const [inputLine, setInputLine] = useState('')

  // Initialize terminal
  useEffect(() => {
    if (!terminalRef.current || termInstance.current) return

    const terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'bar',
      fontSize: 13,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: { background: '#1a1b1e', foreground: '#d4d4d4' },
      allowProposedApi: true,
    })

    const fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)
    fitAddonRef.current = fitAddon

    terminal.open(terminalRef.current)
    fitAddon.fit()
    termInstance.current = terminal

    // Handle resize
    const onResize = () => {
      try { fitAddon.fit() } catch { /* ignore */ }
    }
    window.addEventListener('resize', onResize)

    // Handle user input in terminal
    terminal.onData((data) => {
      if (!selected) return
      // Send each keystroke as input to the backend
      apiFetch(`/api/terminal/${encodeURIComponent(selected)}/input`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ input: data }),
      }).catch(() => {})
    })

    return () => {
      window.removeEventListener('resize', onResize)
      terminal.dispose()
      termInstance.current = null
    }
  }, [selected])

  // Fit terminal when container changes or on mount
  useEffect(() => {
    if (!fitAddonRef.current) return
    const timer = setTimeout(() => {
      try { fitAddonRef.current?.fit() } catch { /* ignore */ }
    }, 100)
    return () => clearTimeout(timer)
  }, [connected])

  // Connect / disconnect SSE when container selection changes
  useEffect(() => {
    // Close previous connection
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
      eventSourceRef.current = null
    }
    if (termInstance.current) {
      termInstance.current.clear()
    }
    setConnected(false)

    if (!selected) return

    const token = localStorage.getItem('token')
    const es = new EventSource(`/api/terminal/${encodeURIComponent(selected)}?token=${token}`)
    eventSourceRef.current = es
    setLoading(true)

    es.addEventListener('output', (e: MessageEvent) => {
      setLoading(false)
      setConnected(true)
      if (termInstance.current) {
        termInstance.current.write(e.data)
      }
    })

    es.onerror = () => {
      setLoading(false)
      setConnected(false)
    }

    return () => {
      es.close()
      eventSourceRef.current = null
      setConnected(false)
    }
  }, [selected])

  const handleSendLine = async () => {
    if (!selected || !inputLine) return
    try {
      await apiFetch(`/api/terminal/${encodeURIComponent(selected)}/input`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ input: inputLine + '\n' }),
      })
    } catch { /* ignore */ }
    setInputLine('')
  }

  const handleDisconnect = () => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
      eventSourceRef.current = null
    }
    setConnected(false)
    if (termInstance.current) {
      termInstance.current.clear()
    }
  }

  return (
    <Stack>
      <Paper shadow="sm" p="md" withBorder>
        <Group>
          <Select
            label="Container"
            placeholder="Selecciona un container"
            data={containers.map((c) => ({ value: c.name, label: c.name }))}
            value={selected}
            onChange={setSelected}
            searchable
            style={{ flex: 1 }}
          />
          {connected && (
            <Button onClick={handleDisconnect} variant="light" color="red" mt="xl">
              Desconectar
            </Button>
          )}
          {loading && (
            <Group gap="xs" mt="xl">
              <Loader size="xs" />
              <Text size="xs" c="dimmed">Conectando...</Text>
            </Group>
          )}
          {connected && (
            <Text size="xs" c="green" mt="xl">● Conectado</Text>
          )}
        </Group>
      </Paper>

      <Paper shadow="sm" withBorder p="xs">
        <div
          ref={terminalRef}
          style={{
            width: '100%',
            height: '500px',
            borderRadius: '4px',
            overflow: 'hidden',
          }}
        />
        {!selected && (
          <Text c="dimmed" ta="center" py="xl">
            Selecciona un container para abrir una terminal interactiva
          </Text>
        )}
        {selected && !connected && !loading && (
          <Text c="dimmed" ta="center" py="sm" size="xs">
            Esperando conexión SSE... Si el container no está en ejecución, la terminal no se conectará.
          </Text>
        )}
      </Paper>

      <Paper shadow="sm" p="md" withBorder>
        <Group>
          <TextInput
            placeholder="Escribe un comando y pulsa Enviar..."
            value={inputLine}
            onChange={(e) => setInputLine(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSendLine()}
            disabled={!connected}
            style={{ flex: 1 }}
          />
          <Button onClick={handleSendLine} disabled={!connected} variant="light">
            Enviar
          </Button>
        </Group>
      </Paper>
    </Stack>
  )
}