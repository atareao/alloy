# Proposal

## Why

El linter (oxlint) reporta dos warnings pre-existentes que deben corregirse para mantener el código limpio y sin ruido de linter.

## What Changes

1. **main.tsx**: Añadir `export` a la función `Root` para que Fast Refresh de React funcione correctamente (warning `react(only-export-components)`)
2. **DashboardPage.tsx**: Reemplazar `logs.length` por `0` en el timeout del useEffect de logs, ya que `setLogs([])` se ejecuta justo antes, eliminando la dependencia faltante (warning `react-hooks(exhaustive-deps)`)

## Capabilities

No hay cambios de comportamiento — solo lint fixes. Se usa `skip_specs: true`.

## Impact

- `frontend/src/main.tsx`: 1 línea (añadir `export`)
- `frontend/src/components/DashboardPage.tsx`: 1 línea (`logs.length` → `0`)
- Sin cambios en tests, backend, o comportamiento observable