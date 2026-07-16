import { useState, StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { MantineProvider } from '@mantine/core'
import { Notifications } from '@mantine/notifications'
import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'
import App from './App.tsx'
import ErrorBoundary from './components/ErrorBoundary.tsx'

function Root() {
  const [colorScheme, setColorScheme] = useState<'dark' | 'light'>(
    () => (localStorage.getItem('color-scheme') as 'dark' | 'light') || 'dark'
  )

  return (
    <StrictMode>
      <MantineProvider forceColorScheme={colorScheme}>
        <Notifications position="bottom-right" zIndex={1000} />
          <ErrorBoundary>
            <App colorScheme={colorScheme} setColorScheme={setColorScheme} />
          </ErrorBoundary>
        
      </MantineProvider>
    </StrictMode>
  )
}

createRoot(document.getElementById('root')!).render(<Root />)