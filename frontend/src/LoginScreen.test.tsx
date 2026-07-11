import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { MantineProvider } from '@mantine/core'
import LoginScreen from './components/LoginScreen'

function Wrapper({ children }: { children: React.ReactNode }) {
  return <MantineProvider>{children}</MantineProvider>
}

describe('LoginScreen', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    // Mock window.location.href
    Object.defineProperty(window, 'location', {
      value: { href: '' },
      writable: true,
    })
  })

  it('renders login button', () => {
    render(<Wrapper><LoginScreen /></Wrapper>)

    expect(screen.getByText('🐳 Cabina de Mando')).toBeInTheDocument()
    expect(screen.getByText('🔑 Iniciar sesión con OIDC')).toBeInTheDocument()
  })

  it('redirects to OIDC login on button click', () => {
    render(<Wrapper><LoginScreen /></Wrapper>)

    fireEvent.click(screen.getByText('🔑 Iniciar sesión con OIDC'))

    expect(window.location.href).toBe('/api/auth/login')
  })
})