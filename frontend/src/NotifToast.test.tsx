import { describe, it, expect, vi } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { MantineProvider } from '@mantine/core'
import NotifToast from './components/NotifToast'
import type { NotifEvent } from './types'

function Wrapper({ children }: { children: React.ReactNode }) {
  return <MantineProvider>{children}</MantineProvider>
}

describe('NotifToast', () => {
  it('renders container and status', () => {
    const notif: NotifEvent = { container: 'nginx', status: 'restarted', timestamp: '2024-01-01T00:00:00Z' }

    render(
      <Wrapper><NotifToast notif={notif} onDismiss={vi.fn()} /></Wrapper>,
    )

    expect(screen.getByText(/nginx/)).toBeInTheDocument()
    expect(screen.getByText(/restarted/)).toBeInTheDocument()
    expect(screen.getByText('2024-01-01T00:00:00Z')).toBeInTheDocument()
  })

  it('calls onDismiss after 4 seconds', async () => {
    vi.useFakeTimers()
    const onDismiss = vi.fn()
    const notif: NotifEvent = { container: 'test', status: 'ok', timestamp: '' }

    render(
      <Wrapper><NotifToast notif={notif} onDismiss={onDismiss} /></Wrapper>,
    )

    expect(onDismiss).not.toHaveBeenCalled()

    act(() => { vi.advanceTimersByTime(4000) })

    expect(onDismiss).toHaveBeenCalledTimes(1)

    vi.useRealTimers()
  })
})