# Proposal

## Why

`PUT /api/update-check/config` (`backend/src/admin.rs:53-87`) escribe el cron **sin validarlo**:

- `admin.rs:59` persiste `body.cron` tal cual; `admin.rs:74` calcula `next_run_at` con
  `compute_next_run` (devuelve `None` si el cron es inválido). No hay rama de error.
- Un cron inválido queda **guardado** y el scheduler lo ignora en silencio (`tracing::warn!` en
  `workers/scheduler.rs:62-65`). El operador no recibe feedback y la config queda rota.

La UI usa y el storage espera cron de **5 campos** (`0 6 * * *`), pero la crate `cron` 0.15 requiere
6 campos (segundos primero), por lo que el código normaliza prefijando `"0 "` (duplicado en
`admin.rs:42`, `timezone.rs:52`, `scheduler.rs:59`).

## What Changes

- **Nuevo helper reutilizable** `pub fn is_valid_cron(expr: &str) -> bool` en `backend/src/timezone.rs`
  (junto a `next_cron_time`), punto único de validación: exige **exactamente 5 campos** y valida
  parseando `"0 {expr}"` con `cron`.
- **`put_update_check_config_h`** (`admin.rs`): pasa a
  `Result<Json<UpdateCheckConfig>, AppError>`; valida `body.cron` **antes** de mutar el estado y
  devuelve `AppError::BadRequest` (400, `{"error": msg}`) si el cron es inválido, vacío o no tiene 5
  campos.
- **Frontend** (`frontend/src/components/ConfigPage.tsx`): en `saveUpdateCheck` (`:288-331`), leer el
  body del error del backend y mostrarlo en el `Alert` (hoy muestra un mensaje genérico; replicar el
  patrón de `:518-524`).

## Capabilities

### Modified Capabilities

- `updates/update-check`: el PUT de la configuración valida la expresión cron y rechaza con 400 las
  entradas inválidas/vacías/que no tengan 5 campos; la UI propaga el error.

### New Capabilities

- Ninguna.

## Impact

- **Backend**: `timezone.rs` (nuevo `is_valid_cron`), `admin.rs` (firma del PUT + validación).
- **Frontend**: `ConfigPage.tsx` (`saveUpdateCheck`) + tests.
- **API**: nuevo caso **400** en `PUT /api/update-check/config` con `{"error": msg}` (aditivo; el
  camino válido no cambia).
- **Datos**: sin migración.
- **Dependencias**: ninguna (`cron 0.15` ya presente).

## Addendum (alcance ampliado, aprobado)

Además del PUT, se valida el cron en **`POST /api/admin/import`** (`admin.rs:218-230`), que hoy
persiste un `Settings` completo sin validar: era la única vía restante que aceptaba un cron inválido.
Solo se valida si `update_check_cron` viene **presente y no vacío** (`None`/`""` = sin override, se
acepta). El import pasa a `Result<Json<Value>, AppError>` y delega en `is_valid_cron`.
