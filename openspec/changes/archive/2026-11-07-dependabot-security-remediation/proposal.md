# Proposal

## Why

GitHub Dependabot reporta **44 alertas de seguridad abiertas** en `atareao/alloy`, todas por
dependencias **transitivas** (ninguna es una dependencia directa en uso de producción):

- **21× undici** (npm) — actual `7.30.0`, parche `7.30.1`. Tireado por `jsdom` (devDep, entorno
  de test de vitest). Severidad high/medium/low: TLS bypass, cache poisoning, DoS, CRLF injection.
- **10× vite** (npm) — actual `8.1.5`, parche `8.1.6`. Incluye 3 avisos de `launch-editor`.
- **4× @vitest/mocker** (npm) — actual `4.1.11`, parche `4.1.12` (vía `vitest`).
- **3× postcss** (npm) — actual `8.5.15`, parche `8.5.23` (vía `vite`).
- **2× source-map-js** (npm) — actual `1.2.1`, parche `1.2.2` (vía `postcss`/`css-tree`).
- **4× rustls** (cargo) — actual `0.23.42`, parche `0.23.46` (vía `bollard`/`reqwest`).

Sin cambios funcionales esperados: solo se actualizan versiones a las parcheadas. Afecta a los
dos lockfiles: `frontend/package-lock.json` (CI/local, `npm`) y `frontend/pnpm-lock.yaml`
(stage `frontend-builder` del Dockerfile, `pnpm`), más `backend/Cargo.lock`.

## What Changes

- **Frontend (npm)**: `npm audit fix` para subir undici→7.30.1, vite→8.1.6, vitest→4.1.12
  (arrastra @vitest/mocker), postcss→8.5.23 y source-map-js→1.2.2. Regenerar
  `package-lock.json`.
- **Frontend (pnpm)**: regenerar `pnpm-lock.yaml` con las mismas versiones parcheadas para que
  el stage Docker quede consistente (`pnpm install` + `pnpm run build --frozen-lockfile` OK).
- **Backend (cargo)**: `cargo update -p rustls` para subir a `0.23.46`. Regenerar `Cargo.lock`.
- Sin cambios de código de aplicación, de API ni de contrato. No se eliminan avisos salvo los
  que tengan versión parcheada publicada.

## Capabilities

### New Capabilities
- `security/dependency-hygiene`: los lockfiles del proyecto no contienen dependencias con
  vulnerabilidades conocidas de severidad low+ con parche disponible.

### Modified Capabilities
<!-- Ninguna. -->

## Impact

- `frontend/package-lock.json`, `frontend/pnpm-lock.yaml`, `backend/Cargo.lock`.
- `frontend/package.json` solo si `npm audit fix` exigiera ampliar algún rango (esperado: no).
- Riesgo: bumps minor/patch; verificar `npm run build`, `npx tsc -b`, `npx vitest run`,
  `cargo test` y `cargo clippy` en verde sin regresiones.
- Verificación externa: `gh api repos/atareao/alloy/dependabot/alerts` sin alertas `open`.

## Open Questions

- `pnpm-lock.yaml` y `package-lock.json` coexisten; se mantienen ambos (CI usa npm, Docker usa
  pnpm). Alternativa descartada: unificar en un solo gestor (fuera del alcance de este fix).
