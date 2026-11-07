# Tasks

## 0. Baseline (RED)

- [x] 0.1 Confirmar baseline: `cargo test` (backend) y `npx vitest run` (frontend) en verde.
- [x] 0.2 Registrar `npm audit` en rojo (6 vulns: 2 moderate, 4 high) y las 44 alertas Dependabot.

## 1. Backend — cargo

- [x] 1.1 `cargo update -p rustls` → `Cargo.lock` con rustls `>= 0.23.46`.
- [x] 1.2 `cargo test` verde + `cargo clippy --all-targets --all-features -- -D warnings`.

## 2. Frontend — npm

- [x] 2.1 `npm audit fix` en `frontend/` → `package-lock.json` con undici/vite/vitest/postcss/source-map-js parcheados.
- [x] 2.2 `npm audit` limpio (0 vulnerabilities).
- [x] 2.3 `npx tsc -b` + `npx vitest run` + `npm run build` en verde.

## 3. Frontend — pnpm (Docker)

- [x] 3.1 Regenerar `pnpm-lock.yaml` con las versiones parcheadas (import/install).
- [x] 3.2 Verificar `pnpm install --frozen-lockfile` y `pnpm run build` OK.

## 4. Verificación

- [x] 4.1 `gh api repos/atareao/alloy/dependabot/alerts` sin alertas `open` de los paquetes afectados.
- [x] 4.2 Revisión del diff de lockfiles: solo bumps de versión, sin cambios de código fuente.
- [x] 4.3 Revisión con `rust-reviewer` (backend) del cambio de `Cargo.lock`.

## 5. Archive

- [ ] 5.1 `openspec archive dependabot-security-remediation --yes` + limpieza del directorio.
