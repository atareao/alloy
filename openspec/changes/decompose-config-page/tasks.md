# Tasks — decompose-config-page (#157)

> Refactor puro: **no se modifica** `ConfigPage.test.tsx`. TDD = red de seguridad (caracterización):
> cada paso termina con `npx vitest run` + `tsc -b` en verde. Si algo se pone rojo, se revierte el paso.

## 1. Preparación (SDD → TDD)

- [ ] 1.1 Pre-flight `source openspec/validate-workflow.sh` y rama `feature/decompose-config-page` desde `develop`.
- [ ] 1.2 Baseline verde antes de tocar nada: `npx vitest run` (135 OK) y `pnpm exec tsc -b` en `frontend/`.
- [ ] 1.3 Confirmar específicamente la suite de caracterización: `npx vitest run src/components/ConfigPage.test.tsx` → verde.

## 2. Extracción de módulo puro (sin React)

- [ ] 2.1 Crear `frontend/src/cronValidation.ts` moviendo líneas ~27–170 (`CronFieldSpec`, `CRON_FIELDS`, `isNumericToken`, `isNameToken`, `isInRange`, `isValidCronBase`, `isValidCronElement`, `isValidCronField`, `normalizeCron`, `isValidCronClient`, `cronErrorFor`).
- [ ] 2.2 `ConfigPage` importa del módulo; suite + `tsc -b` verdes.

## 3. Hooks de estado

- [ ] 3.1 Crear `frontend/src/useConfigPage.ts` (estado compartido + `showSuccess` con cleanup del timer a 3 s).
- [ ] 3.2 Crear `frontend/src/useUpdateCheckConfig.ts` (fetch/estado de update-check + conversión ms↔s + guarda global).
- [ ] 3.3 `ConfigPage` consume ambos hooks; suite + `tsc -b` verdes.

## 4. Extracción de Cards (una por paso, sin tocar tests)

- [ ] 4.1 `TelegramCard.tsx` (📱 Telegram + test de notificación como handler nombrado).
- [ ] 4.2 `MatrixCard.tsx` (💬 Matrix + test).
- [ ] 4.3 `ValetCard.tsx` (🪐 Valet + test).
- [ ] 4.4 `UpdateCheckCard.tsx` (⏰ Revisión de actualizaciones: cron/presets, notify, timeouts, guarda; usar `useUpdateCheckConfig`).
- [ ] 4.5 `DefaultPolicyCard.tsx` (📋 Política por defecto: acción, cleanup, rollback, overrides).
- [ ] 4.6 `AppInfoCard.tsx` (ℹ️ Información de la aplicación).
- [ ] 4.7 `ThemeCard.tsx` (🌙/☀️ Tema; props `colorScheme`/`setColorScheme` + `localStorage`).
- [ ] 4.8 `ExportImportCard.tsx` (📦 Exportar/Importar; preservar `revokeObjectURL` y reload).
- [ ] 4.9 `ConfigPage.tsx` queda como orquestador (Tabs + Alerts + composición); suite + `tsc -b` verdes.

## 5. Verificación final

- [ ] 5.1 `npx vitest run` → verde (mismo total que baseline).
- [ ] 5.2 `pnpm exec tsc -b`, `pnpm run lint`, `pnpm run build` → sin errores/warnings nuevos.
- [ ] 5.3 Confirmar por `rg` que `ConfigPage.tsx` ya no contiene handlers `save*` ni `useState` de formulario por dominio.

## 6. Docs y cierre

- [ ] 6.1 `docs/decompose-config-page.md` (qué es, ficheros, decisiones, follow-ups).
- [ ] 6.2 PR a `develop` (#157) y, tras merge, `openspec archive decompose-config-page` + `plans/PLAN-013.md` → `PLAN-013-ARCHIVED.md`.
