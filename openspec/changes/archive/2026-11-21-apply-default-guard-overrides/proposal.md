# Proposal

## Why

La UI de **política por defecto** (`ConfigPage.tsx`: sección «🛡️ Overrides de la guarda»)
envía `max_start_attempts` y `health_window_secs` al guardar
(`PUT /api/update-policies/default`), pero el struct backend `DefaultUpdatePolicy`
(`backend/src/admin.rs:160-165`) solo tiene 3 campos (`action`, `cleanup_old_image`,
`rollback_on_failure`). **serde descarta los dos overrides sin error**: el usuario configura
algo que nunca se persiste ni se devuelve (no-op silencioso).

A diferencia de la política **por contenedor** —`UpdatePolicy`, que sí tiene esos overrides
como `Option` y cuya UI dice «Vacío = usar el valor global»—, la política por defecto no los
soporta. Los contenedores **sin** política específica se resuelven con una `UpdatePolicy`
sintética con `max_start_attempts: None, health_window_secs: None`
(`scheduler.rs:263`, `updates/handlers.rs:1409`, `updates/handlers.rs:2192`), que cae
directamente al global e ignora cualquier default configurado.

## What Changes

- **Backend**
  - `models.rs` — `Settings`: nuevos campos `default_max_start_attempts: Option<u32>` y
    `default_health_window_secs: Option<u64>` (`#[serde(default)]`, compatibles con settings
    existentes). Helper `Settings::default_guard_overrides()` devuelve el par `(Option, Option)`.
  - `admin.rs` — `DefaultUpdatePolicy`: añadir los dos `Option`; `GET /api/update-policies/default`
    los devuelve desde settings; `PUT` los persiste (`null`/ausente = sin override).
  - **Aplicación**: los tres sitios que construyen la `UpdatePolicy` sintética de *fallback*
    para contenedores sin política siembran `max_start_attempts`/`health_window_secs` desde
    `settings.default_guard_overrides()`. El resto de `guard_params` (precedencia) no cambia.
- **Frontend**: sin cambios. Ya envía y carga estos campos (`types.ts:168-176`,
  `ConfigPage.tsx:288-289,521-524`); al devolverlos el `GET`, la UI los mostrará.
- **Tests**: round-trip PUT/GET de la política por defecto; y que un contenedor **sin** política
  use el override por defecto (no el global), mientras la política **por contenedor** sigue
  ganando.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `updates/update-health-guard`: extiende «Configuración de la guarda (global y por política)»
  con los overrides de la **política por defecto** y su precedencia.

## Impact

- **Backend**: `src/models.rs`, `src/admin.rs`, `src/workers/scheduler.rs`,
  `src/updates/handlers.rs` (+ tests). **Aditivo**: nuevos campos con `#[serde(default)]`;
  settings previos cargan sin migración (los campos quedan `None`).
- **API**: `GET/PUT /api/update-policies/default` amplían su cuerpo con dos campos opcionales
  (aditivo, retrocompatible). El contrato `UpdateCheckConfig` (`/api/update-check/config`) **no
  cambia**.
- **Frontend**: ninguno.

## Decisión de precedencia

`override de política de contenedor` **>** `override de política por defecto` **>** `global`.

Los overrides de la política por defecto aplican **solo a contenedores sin política
específica**. Un contenedor **con** política que no fije su override sigue cayendo al
**global** (como declara su UI: «Vacío = usar el valor global»). El flujo manual monovolumen
(`update_container_h`) queda **fuera de alcance**: usa los globales por diseño documentado.
