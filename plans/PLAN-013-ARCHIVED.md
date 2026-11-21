# PLAN-013 — Descomponer ConfigPage.tsx (#157)

- **Issue**: #157
- **Change OpenSpec**: `decompose-config-page` (capability `ui/config-page`)
- **Rama**: `feature/decompose-config-page` (desde `develop`)
- **Tipo**: refactor puro (sin cambio de comportamiento)

## Objetivo

Reducir `frontend/src/components/ConfigPage.tsx` (1698 líneas) a un orquestador delgado (~60–100 líneas) extrayendo:
un módulo puro de validación cron, dos hooks de estado y ocho componentes Card, **sin cambiar el comportamiento observable**.

## Alcance

- Nuevo módulo: `frontend/src/cronValidation.ts`.
- Nuevos hooks (planos en `src/`, convención real del repo): `useConfigPage.ts`, `useUpdateCheckConfig.ts`.
- Nuevos componentes en `frontend/src/components/`: `TelegramCard`, `MatrixCard`, `ValetCard`, `UpdateCheckCard`, `DefaultPolicyCard`, `AppInfoCard`, `ThemeCard`, `ExportImportCard`.
- `ConfigPage.tsx` pasa a orquestador.

## Red de seguridad (caracterización)

- `frontend/src/components/ConfigPage.test.tsx` (~2396 líneas) **NO se modifica**: debe pasar 100% en verde.
- Baseline: `npx vitest run` → 135 OK; `pnpm exec tsc -b` limpio.

## Tareas

Ver `openspec/changes/decompose-config-page/tasks.md`.

## Criterios de aceptación

- [x] `ConfigPage.tsx` sin handlers `save*` ni estado de formulario por dominio (orquestador).
- [x] Validación cron en módulo puro sin React.
- [x] Estado compartido (`saving`, `testing`, `error`, `success`, `activeTab`) y auto-ocultado a 3 s con cleanup en `useConfigPage`.
- [x] 8 Cards extraídas; textos visibles, `aria-label` y placeholders intactos.
- [x] `ConfigPage.test.tsx` pasa sin cambios.
- [x] `vitest run`, `tsc -b`, `lint`, `build` limpios.
- [x] `docs/decompose-config-page.md` creado.

## Follow-ups (fuera de alcance)

- Desduplicar el bloque guard-overrides con `PolicyActionButton.tsx`.
- Valorar tests de comportamiento nuevos para los handlers no cubiertos (Test de Telegram/Matrix, Export/Import real).
