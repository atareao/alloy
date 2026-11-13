# Proposal

## Why

El canal de Telegram solo permite configurar token y `chat_id`. En un supergrupo con
**Topics** activados, Telegram exige `message_thread_id` en `sendMessage` para enrutar el
mensaje a un tema concreto; sin él todo cae en *General* (y el sufijo `chat_id/topic` no
funciona). Issue #89.

## What Changes

- Nuevo ajuste opcional `telegram_topic_id` (string), persistido en SQLite y editable por la
  UI, junto a token y chat_id.
- `notify_telegram` añade `"message_thread_id": <n>` al payload JSON **solo** cuando hay topic
  configurado y es un entero válido.
- `PublicConfig` expone `telegram_topic_id` (no es secreto) para poder precargar el campo.
- La tarjeta Telegram de `ConfigPage` incorpora el campo "Topic ID (opcional)", lo envía en
  `PUT /api/config` y lo limpia al desactivar el canal.
- Sin topic configurado, comportamiento **idéntico** al actual (sin regresión).

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `notifications`: se añade el requisito de enrutado a Topics de Telegram mediante
  `message_thread_id`, su configuración por API/UI y su persistencia.

## Impact

**Backend**
- `models.rs`: `Settings.telegram_topic_id`, `PublicConfig.telegram_topic_id`,
  `UpdateSettingsReq.telegram_topic_id`.
- `db.rs`: carga/guardado de la clave `telegram_topic_id`.
- `config.rs`: lectura en `config_handler` y escritura en `update_config_h`.
- `notifications.rs`: helper de topic y `message_thread_id` en el payload.

**Frontend**
- `types.ts`: `telegram_topic_id` en `AppConfig`.
- `ConfigPage.tsx`: estado, precarga, input y envío.

**Tests**: Rust (payload con/sin topic, persistencia, API) y frontend (campo y envío).

**Sin cambios**: `config.yaml`, rutas, export/import (el campo viaja en `Settings` sin ser
secreto).
