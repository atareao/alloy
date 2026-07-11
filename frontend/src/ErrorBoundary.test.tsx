import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MantineProvider } from '@mantine/core'
import ErrorBoundary from './components/ErrorBoundary'

function Wrapper({ children }: { children: React.ReactNode }) {
  return <MantineProvider>{children}</MantineProvider>
}

describe('ErrorBoundary', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  it('renders children when no error', () => {
    render(
      <Wrapper><ErrorBoundary><div>Hello</div></ErrorBoundary></Wrapper>,
    )
    expect(screen.getByText('Hello')).toBeInTheDocument()
  })

  it('renders fallback UI when child throws', () => {
    const Bomb = () => { throw new Error('boom!') }

    render(
      <Wrapper>
        <ErrorBoundary>
          <Bomb />
        </ErrorBoundary>
      </Wrapper>,
    )

    expect(screen.getByText('⚠️ Algo salió mal')).toBeInTheDocument()
    expect(screen.getByText('boom!')).toBeInTheDocument()
    expect(screen.getByText('Recargar página')).toBeInTheDocument()
  })

  it('renders "Error desconocido" when error has no message', () => {
    const Bomb = () => { throw new Error() }

    render(
      <Wrapper>
        <ErrorBoundary>
          <Bomb />
        </ErrorBoundary>
      </Wrapper>,
    )

    expect(screen.getByText('Error desconocido')).toBeInTheDocument()
  })
})