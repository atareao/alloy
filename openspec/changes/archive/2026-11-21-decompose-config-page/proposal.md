# Proposal — decompose-config-page (#157)

## Why

`frontend/src/components/ConfigPage.tsx` tiene **1698 líneas** en un solo fichero: mezcla 5 dominios de configuración (Telegram, Matrix, Valet, update-check, política por defecto), validación/normalización de cron, export/import y componentes de tema. Eso hace la página difícil de leer, revisar y testear de forma aislada, y provoca duplicación (p. ej. el bloque de overrides de la guarda se repite con `PolicyActionButton.tsx`). Es deuda técnica registrada como **#157**.

Es un **refactor puro**: el objetivo es reducir el fichero a un orquestador delgado **sin cambiar el comportamiento observable**. La suite `ConfigPage.test.tsx` (~2396 líneas) es la red de seguridad y **no debe modificarse**.

## What Changes

- Extraer los helpers de cron (líneas ~27–170) a un **módulo puro** `frontend/src/cronValidation.ts` (mismo patrón que el ya existente `cronPresets.ts`).
- Crear hooks para el estado compartido y el específico:
  - `frontend/src/useConfigPage.ts` — `saving`, `testing`, `error`, `success`, `activeTab` y `showSuccess` (auto-ocultado a 3 s **con cleanup**).
  - `frontend/src/useUpdateCheckConfig.ts` — fetch + estado de update-check (incl. conversión ms↔s de `check_interval_ms` y la guarda global).
- Extraer cada `<Card>` a su propio componente en `frontend/src/components/`:
  `TelegramCard`, `MatrixCard`, `ValetCard`, `UpdateCheckCard`, `DefaultPolicyCard`, `AppInfoCard`, `ThemeCard`, `ExportImportCard`.
- `ConfigPage.tsx` queda como **orquestador** (~60–100 líneas): `Tabs` + `Alerts` top-level + composición de las Cards.

## Capabilities

### New Capabilities

- `ui/config-page`: contrato de composición y comportamiento observable de la página de Configuración — qué orquesta `ConfigPage`, dónde vive cada pieza (módulo puro, hooks, Cards), y las invariantes observables que el refactor NO puede romper.

### Modified Capabilities

- Ninguna.

## Impact

- **Frontend (código)**: `src/components/ConfigPage.tsx` (adelgaza), +8 componentes en `src/components/`, +2 hooks y +1 módulo en `src/`, siguiendo la convención real del repo (**plano en `src/`**, no hay `src/hooks/`).
- **Tests**: `ConfigPage.test.tsx` **sin cambios**; debe pasar 100% en verde tras el refactor. No se añaden tests de comportamiento nuevos (caracterización).
- **Sin cambios** en backend, API, tipos (`AppConfig`, etc.), endpoints o dependencias.
- **Fuera de alcance**: desduplicar el bloque guard-overrides con `PolicyActionButton`, añadir react-query, o cambiar textos/UI. Son follow-ups.

## Non-goals

- Cambiar comportamiento, textos visibles, `aria-label`, placeholders o endpoints.
- Introducir estado global nuevo (Context) — el estado se eleva mediante hooks/props.
- Tocar `cronPresets.ts` (ya extraído).
