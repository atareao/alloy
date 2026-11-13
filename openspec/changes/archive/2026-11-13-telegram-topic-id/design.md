# Design

## Context

- `Settings` (`models.rs:197`) guarda `telegram_token`/`telegram_chat_id` como `Option<String>`
  y se persiste clave-valor en SQLite (`db.rs`).
- `notify_telegram` (`notifications.rs:29`) construye el cuerpo con `serde_json::json!`
  (línea 37) y hace `POST` a `api.telegram.org/bot{token}/sendMessage` (línea 39).
- `GET/PUT /api/config` (`config.rs`) y la tarjeta Telegram (`ConfigPage.tsx:355`) manejan los
  campos; el patrón de `PUT` es "ausente conserva / vacío borra".
- No hay validación del `chat_id` (solo vacío ⇒ `None`).

## Goals / Non-Goals

**Goals:**

- Enrutar la notificación a un Topic del supergrupo vía `message_thread_id`.
- Configuración por API y UI, persistida en SQLite, con precarga.
- Cero regresión cuando no se configura topic.

**Non-Goals:**

- No se soporta la sintaxis `chat_id/topic_id` en el campo chat_id (se descarta por ambigua).
- No se toca Valet/Matrix/webhook ni el fan-out.
- No se valida el `chat_id` (fuera de alcance).

## Decisions

### D1 — Campo dedicado `telegram_topic_id`, no sintaxis `chat/topic`

El campo separado es explícito, se puede precargar y no rompe `chat_id` existentes.
*Alternativa considerada*: parsear `chat_id/thread` (propuesta en el issue). Rechazada:
ambigua (chat_ids pueden contener `/`? no, pero complica la UI y el saneamiento).

### D2 — Almacenar como `String` (trim) y enviar como número

Se guarda `Option<String>` igual que `chat_id` (evita cambiar DTOs/serde), se normaliza con
`trim` al guardar, y al construir el payload se parsea a `i64` porque la API de Telegram exige
`message_thread_id` **numérico**. `None`/vacío ⇒ la clave se omite.

### D3 — Topic inválido hace fallar el envío (fail-fast)

Si el topic está presente pero no parsea a `i64`, `notify_telegram` devuelve `Err` sin HTTP y
registra `ERROR`. Silenciarlo enviaría el mensaje a *General* (el bug que se corrige), así que
se prefiere un fallo visible en el botón Test. La UI restringe la entrada a dígitos.

### D4 — Semántica de `PUT` idéntica al resto de campos

`telegram_topic_id` sigue el patrón existente: ausente ⇒ conserva, `""`/solo espacios ⇒
`None`, valor ⇒ `Some(trim(v))`. No añade indicador `*_set` en `PublicConfig` porque no es
secreto; se expone el valor directamente.

### D5 — Persistencia en la tabla `settings`

Nueva clave `telegram_topic_id` en `load_settings`/`save_settings`, con el mismo tratamiento
de `None` ⇒ `DELETE` que el resto. El export/import lo arrastra automáticamente por ser parte
de `Settings`.

## Risks / Trade-offs

- **Topic inválido bloquea el canal**: es intencionado (D3). Mitigación: validación de entrada
  en UI y mensaje de error claro en el Test.
- **Supergrupo sin Topics**: si el chat no tiene topics, enviar `message_thread_id` puede
  devolver error de Telegram. El usuario simplemente deja el campo vacío; se documenta.
- **Tipos de `message_thread_id`**: se envía número (no string) para cumplir la API; cubierto
  por tests del payload.
