# Spec Delta

## ADDED Requirements

### Requirement: Enrutado de Telegram a un Topic

El ajuste `telegram_topic_id` (opcional) MUST permitir enviar las notificaciones de Telegram
a un tema concreto de un supergrupo. Cuando esté configurado, `notify_telegram` MUST incluir
`"message_thread_id"` como **número** en el cuerpo de `sendMessage`. Cuando no esté
configurado, el cuerpo MUST ser idéntico al actual (sin `message_thread_id`). La presencia o
ausencia del topic MUST NOT alterar `telegram_configured`, que sigue siendo
`telegram_token ∧ telegram_chat_id`.

#### Scenario: Envío a un topic concreto

- **Given** `telegram_token`, `telegram_chat_id` y `telegram_topic_id = "42"`
- **When** se ejecuta `notify_telegram`
- **Then** el cuerpo de `sendMessage` incluye `"message_thread_id": 42` (numérico)
- **And** el mensaje se envía al chat indicado

#### Scenario: Sin topic configurado

- **Given** `telegram_token` y `telegram_chat_id` pero `telegram_topic_id` ausente
- **When** se ejecuta `notify_telegram`
- **Then** el cuerpo NO contiene la clave `message_thread_id`
- **And** el comportamiento es idéntico al actual

#### Scenario: Topic inválido

- **Given** `telegram_topic_id` con un valor no numérico no vacío
- **When** se ejecuta `notify_telegram`
- **Then** devuelve `Err` sin emitir petición HTTP
- **And** se registra un `ERROR` indicando topic inválido

### Requirement: Configuración y persistencia del Topic de Telegram

`telegram_topic_id` MUST persistirse en SQLite y editarse vía `GET/PUT /api/config`.
`PublicConfig` MUST exponer `telegram_topic_id` (no es secreto; no requiere indicador
`*_set`). En `PUT /api/config`: campo **ausente** MUST conservar el valor actual; cadena
vacía o solo espacios MUST desactivarlo (`None`); un valor no vacío MUST guardarse
normalizado con `trim`. `GET /api/config` MUST devolver el topic guardado para precargar la UI.

#### Scenario: Guardar un topic

- **Given** un usuario con sesión que envía `{"telegram_topic_id":"42"}`
- **When** se llama a `PUT /api/config`
- **Then** el topic se guarda en SQLite como `"42"`
- **And** `GET /api/config` devuelve `telegram_topic_id: "42"`

#### Scenario: Desactivar el topic

- **Given** un topic guardado
- **When** se hace `PUT /api/config` con `{"telegram_topic_id":""}`
- **Then** el topic pasa a `None` y `GET /api/config` devuelve `null`

#### Scenario: Campo ausente lo conserva

- **Given** un topic guardado y una petición sin el campo `telegram_topic_id`
- **When** se llama a `PUT /api/config`
- **Then** el topic existente se conserva

### Requirement: El Topic se edita desde la tarjeta de Telegram

La tarjeta Telegram de la UI MUST ofrecer un campo "Topic ID (opcional)" precargado con el
valor guardado, MUST enviar `telegram_topic_id` en `PUT /api/config` al guardar, y MUST
enviar cadena vacía (`""`) cuando el switch del canal se apaga.

#### Scenario: Precarga y guardado desde la UI

- **Given** una config con `telegram_topic_id = "42"`
- **When** se renderiza y se guarda la tarjeta Telegram sin cambios
- **Then** el campo muestra `42` y el `PUT` incluye `telegram_topic_id: "42"`

#### Scenario: Vaciar el campo borra el topic

- **Given** la tarjeta Telegram con un topic guardado y el switch encendido
- **When** el usuario vacía el campo y guarda
- **Then** el `PUT` incluye `telegram_topic_id: ""`
- **And** el topic pasa a `None` (el campo vacío NO se envía como `null`, que el backend
  interpretaría como "conservar")

#### Scenario: Apagar el canal limpia el topic

- **Given** la tarjeta Telegram con un topic escrito
- **When** se apaga el switch y se guarda
- **Then** el `PUT` incluye `telegram_topic_id: ""`
