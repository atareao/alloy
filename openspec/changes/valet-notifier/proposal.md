# Proposal

## Why

Alloy ya notifica cambios de estado de contenedores a Telegram, Matrix y un webhook
genérico. El usuario mantiene **Valet**, un asistente personal con una API de ingesta de
notificaciones (`POST /api/notifications`, autenticada con una *ingest key* `val_…`).
Añadir un notificador hacia Valet permite que los eventos de contenedores (arranque,
parada, muerte, pausa…) lleguen a la bandeja de Valet y, desde ahí, a sus notificaciones
WebPush — sin tener que reescribir el webhook genérico ni mantener un script puente.

La documentación de la API está en `/data/rust/valet/docs/api-notifications.md`.

## What Changes

- Nuevo canal de notificación **Valet** en el backend, con dos ajustes: `valet_url`
  (base URL, p. ej. `https://valet.example.com`) y `valet_token` (ingest key `val_…`).
- `notify_valet(settings, container, status)` hace `POST {valet_url}/api/notifications`
  con `Authorization: Bearer {valet_token}` y cuerpo JSON
  `{message, origin:"alloy", author:"alloy", level, metadata}`.
- Mapeo determinista de `level`: `info` por defecto; `warning`/`critical` para estados de
  parada/muerte/error.
- Se integra en `notify_all` (fan-out best-effort, en paralelo, sin bloquear a los demás).
- Nuevo canal `"valet"` en `POST /api/notifications/test`.
- Ajustes persistidos en SQLite (`db::save_settings`/`load_settings`) y expuestos en
  `GET/PUT /api/config` (`valet_configured`, `valet_url`, `valet_token`), siguiendo el
  mismo patrón que Telegram/Matrix.
- UI: nueva tarjeta **Valet** en `ConfigPage` (pestaña Notificaciones) con switch,
  campos URL + token, botón *Guardar* y botón *Test*.

## Capabilities

### New Capabilities
- `notifications`: canales de notificación salientes de Alloy (Telegram, Matrix, webhook
  y, ahora, Valet), su configuración, mapeo de niveles y comportamiento best-effort.

### Modified Capabilities
<!-- Ninguna. Telegram/Matrix/webhook no cambian de comportamiento. -->

## Impact

- `backend/src/notifications.rs`: nueva `notify_valet`, helpers de nivel/URL, integración
  en `notify_all`, canal `"valet"` en el endpoint de test, tests.
- `backend/src/models.rs`: campos `valet_url`/`valet_token` en `Settings` y
  `UpdateSettingsReq`; campos `valet_url`/`valet_token`/`valet_configured` en `PublicConfig`.
- `backend/src/config.rs`: `config_handler` + `update_config_h` (mapeo y limpieza de
  `valet_url`/`valet_token`).
- `backend/src/db.rs`: mapa de ajustes (`load_settings`/`save_settings`).
- `frontend/src/types.ts`: campos Valet en `AppConfig`.
- `frontend/src/components/ConfigPage.tsx`: tarjeta Valet + `saveValet`.
- No se toca el schema SQLite (los ajustes van en la tabla clave-valor existente).
- Fuera de alcance: gestión de ingest keys en Valet (`/api/ingest-keys`), lectura de la
  bandeja, y WebPush — Alloy solo **produce** notificaciones.

## Open Questions

- **Nivel (`level`)**: se propone un mapeo fijo por estado. Si se quiere configurable, es
  un campo extra (`valet_level_map`); se deja fuera salvo petición.
- **Exposición del token**: `PublicConfig` ya expone `telegram_token`/`matrix_token`;
  por consistencia se exponen `valet_token`, aunque un diseño más estricto lo redactaría.
