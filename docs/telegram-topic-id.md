# telegram-topic-id

## Qué es

Soporte para enrutar las notificaciones de Telegram a un **Topic** (tema) de un supergrupo,
mediante el parámetro `message_thread_id` de la API `sendMessage`. Resuelve el issue
[#89](https://github.com/atareao/alloy/issues/89): sin él, todos los mensajes caían en el
tema *General*.

## Ficheros implicados

- `backend/src/models.rs` — `Settings.telegram_topic_id`, `PublicConfig.telegram_topic_id`,
  `UpdateSettingsReq.telegram_topic_id`.
- `backend/src/db.rs` — persistencia de la clave `telegram_topic_id` en la tabla `settings`.
- `backend/src/config.rs` — lectura/escritura por `GET/PUT /api/config`.
- `backend/src/notifications.rs` — `tg_topic_id`, `telegram_payload` y el envío.
- `frontend/src/types.ts` y `frontend/src/components/ConfigPage.tsx` — campo "Topic ID (opcional)".

## Cómo funciona

1. La UI guarda el topic (solo dígitos) en `PUT /api/config`. El backend lo normaliza con
   `trim`; vacío o solo espacios ⇒ `None`; campo ausente ⇒ conserva el valor actual.
2. Al enviar, `tg_topic_id` parsea el valor a `i64` (tras `trim`). `telegram_payload` añade
   `"message_thread_id"` como **número** solo si hay topic; si no, el cuerpo es idéntico al
   anterior.
3. Si el topic guardado no es numérico, `notify_telegram` devuelve `Err` **sin** hacer HTTP y
   registra `ERROR` (fail-fast, para no enviar silenciosamente a *General*).
4. `telegram_configured` no cambia: sigue siendo `telegram_token ∧ telegram_chat_id`; el
   topic es opcional.

## Configuración

- **UI**: Configuración → Notificaciones → tarjeta **Telegram** → campo **Topic ID (opcional)**.
- **API**: `PUT /api/config` con `{"telegram_topic_id":"42"}`; `GET /api/config` lo devuelve.
- Desactivar: `""` (o vaciar el campo) lo borra. No hay variable de entorno: es un ajuste
  persistido en SQLite.

> Nota: el supergrupo debe tener los **Topics activados**; si el chat no tiene topics, deja el
> campo vacío (enviar `message_thread_id` puede devolver error de Telegram).

## Decisiones

- Campo dedicado en vez de la sintaxis `chat_id/topic_id` (design D1): explícito y precargable.
- Guardado como `String` con `trim`, enviado como número (design D2).
- Topic no numérico ⇒ fallo visible en el envío (design D3).
- Semántica de `PUT` idéntica al resto de campos (design D4). El campo vacío se envía como
  `""` (no `null`, que el backend interpreta como "conservar").
- La UI restringe la entrada a dígitos (`inputMode="numeric"` + saneo).

## Follow-ups

- Endurecer la validación a entero positivo (rango `i32`) y/o devolver `400` en el `PUT` si el
  topic no es numérico, en lugar de fallar en el envío.
- El input de Chat ID adyacente carece de nombre accesible (deuda preexistente).
- `ConfigPage.tsx` (>1200 líneas) pide extraer la tarjeta Telegram a un subcomponente.
