# PLAN-011 — Aplicar los overrides de la guarda de la política por defecto

- **Issue**: #155 (https://github.com/atareao/alloy/issues/155)
- **Change OpenSpec**: `apply-default-guard-overrides`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `fix/default-guard-overrides` desde `develop`; PR a `develop`.

## Objetivo

Que los overrides de la guarda (`max_start_attempts`, `health_window_secs`) de la **política por defecto** dejen de descartarse: persistirlos, devolverlos y aplicarlos al *fallback* de los contenedores **sin** política específica.

## Problema

- La UI envía `max_start_attempts`/`health_window_secs` en `PUT /api/update-policies/default` (`ConfigPage.tsx:521-524`, `types.ts:168-176`), pero `DefaultUpdatePolicy` (`admin.rs:160-165`) solo tiene 3 campos → **serde los descarta sin error** (no-op silencioso).
- Los contenedores **sin** política se resuelven con una `UpdatePolicy` sintética con `max_start_attempts: None, health_window_secs: None` (`scheduler.rs:263`, `handlers.rs:1409`, `handlers.rs:2192`) → caen al global e ignoran el default configurado.

## Alcance (backend)

1. `models.rs`: `Settings.default_max_start_attempts: Option<u32>`, `Settings.default_health_window_secs: Option<u64>` (`#[serde(default)]`) + en `Default`; helper `Settings::default_guard_overrides()`.
2. `admin.rs`: `DefaultUpdatePolicy` con los dos `Option`; `GET` los puebla; `PUT` los persiste (`null`/ausente = sin override).
3. Sembrar la `UpdatePolicy` de *fallback* con `settings.default_guard_overrides()` en los 3 sitios (`workers/scheduler.rs`, `updates/handlers.rs` ×2).
4. Tests: round-trip PUT/GET; precedencia (contenedor > default > global); compat de settings sin los campos.

## Fuera de alcance

- El flujo manual monovolumen `update_container_h` (usa globales por diseño documentado, no resuelve política).
- Cambios de contrato de `UpdateCheckConfig` (`/api/update-check/config`) y del frontend.

## Precedencia

`override de política de contenedor` > `override de política por defecto` > `global`.
Los overrides de la política por defecto aplican **solo** a contenedores **sin** política; un contenedor **con** política sin override propio cae al global.

## Plan TDD

### RED
- Round-trip `PUT`/`GET /api/update-policies/default` (persisten y se devuelven); `null` los limpia.
- Contenedor sin política usa el override por defecto (no el global); política por contenedor gana; política sin override → global.
- JSON de `Settings` sin los campos nuevos → `None`.

### GREEN
- Campos en `Settings` + helper; `DefaultUpdatePolicy` (GET/PUT); sembrar el fallback en los 3 sitios.

### REFACTOR / verificación
- `cd backend && just check` + `cargo test`.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Backend en verde (`cargo test`, `just check`).
- [ ] `docs/apply-default-guard-overrides.md`.
- [ ] `openspec archive apply-default-guard-overrides` + limpieza.
- [ ] PR a `develop` mergeado e issue #155 cerrado.

## Referencias

- Spec delta: `openspec/changes/apply-default-guard-overrides/specs/updates/update-health-guard/spec.md`.
- `backend/src/models.rs:238-269,315-330,381-394`; `backend/src/admin.rs:160-193`; `backend/src/updates/handlers.rs:284-293,1409,2192`; `backend/src/workers/scheduler.rs:118-126,263`.
- `frontend/src/components/ConfigPage.tsx:272-289,521-524,1440-1478`; `frontend/src/types.ts:168-176`.
