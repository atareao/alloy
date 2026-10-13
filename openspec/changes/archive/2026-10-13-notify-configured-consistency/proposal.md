# Proposal

## Why

El indicador `*_configured` de `GET /api/config` es inconsistente entre canales y no refleja
si el canal puede realmente enviar:

- `telegram_configured = telegram_token.is_some()` (ignora el `chat_id`, que el notificador
  **sí** exige).
- `matrix_configured = matrix_homeserver.is_some()` (ignora token y room, que el notificador
  **sí** exige).
- `valet_configured = valet_url && valet_token` (el único que exige los campos necesarios).
- `webhook_configured = webhook_url.is_some()` (correcto, solo necesita URL).

Resultado: la UI muestra un canal "Activado" aunque le falten campos y el envío vaya a
fallar con error. Detectado en el review de `redact-notification-secrets`.

## What Changes

- Alinear `*_configured` con los campos que cada notificador **exige** para enviar:
  - `telegram_configured = telegram_token && telegram_chat_id`
  - `matrix_configured = matrix_homeserver && matrix_token && matrix_room`
  - `valet_configured = valet_url && valet_token` (sin cambios)
  - `webhook_configured = webhook_url` (sin cambios)
- `PublicConfig` expone banderas **no secretas** `telegram_token_set`/`matrix_token_set`/
  `valet_token_set` (si hay un token guardado), ya que tras `redact-notification-secrets` el
  valor no viaja al navegador.
- **UI** (para que una config parcial siga siendo editable): el switch de cada tarjeta se
  inicializa desde `*_configured` **o** desde "algún campo del canal presente" (incluida la
  bandera `*_token_set`); si el switch está activado pero el canal NO está completo, se
  muestra un aviso "Faltan campos" que evita el estado engañoso.
- Sin cambios en la semántica de envío de los notificadores (ya hacen early-return).

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `notifications`: el indicador de "configurado" refleja la capacidad real de envío.

## Impact

- `backend/src/config.rs`: `config_handler` (derivación de los tres `*_configured`) + tests.
- `frontend/src/components/ConfigPage.tsx`: inicialización de switches + aviso de campos
  incompletos.
- `frontend/src/components/ConfigPage.test.tsx`: tests de derivación y del aviso.
- `models.rs`: `PublicConfig` gana las banderas no secretas `*_token_set` (el resto del contrato no cambia).

## Open Questions

- **Editabilidad**: se opta por "switch on si `configured` o si hay algún campo", para no
  dejar inaccesibles (campos ocultos) las configuraciones parciales preexistentes.
