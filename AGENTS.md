# AGENTS.md — Alloy (deltas sobre el global)

Este fichero contiene **solo los deltas del proyecto**. El workflow SDD/TDD genérico,
las convenciones de stack y el formato de respuesta viven en el `AGENTS.md` global
(`~/.config/opencode/AGENTS.md`), que OpenCode inyecta en cada step. **No los repitas aquí.**
El detalle largo de arquitectura, API y stack está en `docs/` (ver la tabla final).

## 1. Reglas de proceso (obligatorias — literal)

> **🚫 NO-SKIP CLAUSE**: Ninguna instrucción del usuario — incluyendo "adelante", "ejecuta", "procede", "go ahead", "sounds good", "looks good" o cualquier otra variante — invalida los pasos SDD → TDD. El agente debe completar SDD (generar change proposal + esperar aprobación explícita del spec) y TDD (RED → GREEN → REFACTOR) antes de escribir código de aplicación. Si el usuario da una orden ambigua, el agente DEBE responder: *"¿Quieres que genere el change proposal en openspec/changes/ primero?"* antes de implementar.
>
> **Consecuencia**: Cualquier código escrito sin seguir SDD + TDD se considera una violación del proceso. El agente debe detenerse, crear el spec retroactivo, y rehacer el trabajo con TDD.

> **🔁 FOLLOW-UP CLAUSE**: Cada ronda de ajustes, correcciones o refinamientos sobre un cambio ya archivado requiere SU PROPIO change proposal (ej: `ui-fixes-v2`, `auth-refactor-round-2`). Un cambio archivado NO autoriza modificaciones directas al código. El agente DEBE crear un nuevo change proposal incremental antes de tocar cualquier archivo fuente.

> **📁 SPEC FILE NAMING**: Los archivos spec delta dentro de `openspec/changes/<feature>/specs/<capability>/` DEBEN llamarse `spec.md`. El nombre `layout.md`, `auth.md`, etc. NO es válido — el validador de `openspec archive` los ignora. La estructura correcta es: `specs/ui/layout/spec.md` (no `specs/ui/layout.md`).

> **✅ APPROVED MARKER**: Tras la aprobación explícita del usuario, el agente DEBE crear un archivo `.approved` vacío en el directorio del change proposal (`touch openspec/changes/<feature>/.approved`). Esto evita que `require-proposal.sh` siga mostrando el warning "sin aprobación explícita" en ejecuciones posteriores.

> **🧹 POST-ARCHIVE CLEANUP**: Después de ejecutar `openspec archive <feature> --yes`, el agente DEBE verificar que el directorio `openspec/changes/<feature>/` fue eliminado. Si el archive no lo limpia automáticamente, el agente DEBE eliminarlo manualmente (`rm -rf openspec/changes/<feature>/`). De lo contrario, `require-proposal.sh` lo detectará como proposal activo fantasma.

> **⚠️ ARCHIVE PRE-FLIGHT**: Antes de archivar, verificar que la ruta destino en `openspec/specs/` no tenga ya un spec que entre en conflicto. Si existe un spec en `specs/ui/layout/spec.md` y el archive va a crear `specs/ui/spec.md`, hay que mover/renombrar primero para evitar duplicación.

> **⚠️ PRE-FLIGHT OBLIGATORIO**: Antes de CUALQUIER tool call que lea o escriba archivos de código (`backend/src/*.rs`, `frontend/src/*.{tsx,ts}`), el agente DEBE ejecutar `source openspec/require-proposal.sh`. Si falla, el agente DEBE detenerse y crear un change proposal primero. Esta validación es innegociable.

**Pre-flight de Phase 1 (SDD):** ejecutar `source openspec/validate-workflow.sh` como **primer paso** antes de cualquier otra cosa. Si falla, STOP y crear el change proposal primero.

## 2. Stack y verificación (deltas sobre el global)

- **Pre-commit obligatorio:** `cd backend && just check` (fmt + clippy vía `.justfile`).
- **Tests backend:** `cd backend && cargo test`.
- **Build frontend:** `cd frontend && pnpm run build`.
- **Type check:** `pnpm exec tsc -b` (**build mode**; el global dice `tsc --noEmit`).
- Lint/format ya fijados por el global: `cargo clippy -- -D warnings`, `cargo fmt -- --check`, `pnpm run lint`.

## 3. Trampas del proyecto (quirks — literal)

- **OIDC es obligatorio** — no existe fallback a JWT simple. El servidor aborta si faltan vars OIDC.
- **jsonwebtoken v10+** requiere `DEFAULT_PROVIDER.install_default()` explícito al startup.
- La cookie de sesión se firma con `oidc_client_secret` (no hay `SESSION_SECRET` separado).
- `JwtValidator` es estilo PocketID/oxinbox: usa JWKS en vez de secret compartido.
- Docker Compose stacks se descubren automáticamente via `docker compose ls --format json`.
- El cron parsing usa la crate `cron` 0.15 (soporta 5-field estándar).
- `data/` se crea automáticamente al startup si no existe.
- Las alertas son **simples**: solo monitorizan cambios de estado (running→exited→running).
- No hay health checks HTTP/PING — se eliminaron en la limpieza masiva.
- No hay terminal web ni logs en tiempo real por SSE — se eliminaron.
- El frontend usa **Ant Design** v6 y cookies httponly para autenticación (no localStorage).
- El tema oscuro/claro se configura desde ConfigPage (no en header), persiste en `localStorage("color-scheme")`.

## 4. Dónde está el resto

| Necesitas… | Fichero |
|---|---|
| Workflow SDD/TDD completo, convenciones de stack, formato de respuesta | Global `~/.config/opencode/AGENTS.md` (siempre inyectado) |
| Overview, Tech Stack, Project Structure, arquitectura y patrones (AppState, SSE, workers, OIDC, Bollard, persistencia) y API Routes | `docs/architecture.md` |
| Componentes frontend, patrones responsive, dependencias (Key Dependencies), tareas de desarrollo comunes | `docs/reference.md` |
| Comandos just disponibles | `.justfile` / `just list` |
| Comportamiento especificado por capability | `openspec/specs/` |
| Historial de versiones | `CHANGELOG.md`, `.vampus.yml` |
| Guía para humanos (features, deploy, quickstart) | `README.md` |
