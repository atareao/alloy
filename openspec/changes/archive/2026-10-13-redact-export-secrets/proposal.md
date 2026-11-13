# Proposal

## Why

`GET /api/admin/export` devuelve `settings` completo, **incluidos secretos**
en claro: los tokens `telegram_token`/`matrix_token`/`valet_token` y `webhook_url` (que en
Slack/Discord y similares es una credencial *bearer* de facto). Está detrás del mismo `auth_middleware`
OIDC que el resto, así que cualquier sesión autenticada puede leerlos. Tras
`redact-notification-secrets` (que limpió `GET/PUT /api/config`), `export` es la **última
superficie** que filtra esos secretos.

## What Changes

- `export_config_h` redacta en el payload de `settings` los tres tokens **y `webhook_url`**
  (los emite como `null`), manteniendo el resto de campos y `update_history` intactos.
- `import_config_h` **no cambia**: sigue aceptando tokens si el usuario los aporta en un
  import manual.
- Consecuencia documentada: un export/import **round-trip pierde los secretos** (tokens y
  `webhook_url`); hay que volver a introducirlos tras restaurar. Es el precio de no viajar secretos en un JSON.

## Capabilities

### New Capabilities
- `admin/config-export`: exportación de la configuración sin secretos.

### Modified Capabilities
<!-- Ninguna. -->

## Impact

- `backend/src/admin.rs`: `export_config_h` (+ tests).
- `frontend/src/components/ConfigPage.tsx`: el texto de la tarjeta de export/import debe
  advertir de que los tokens no se exportan y hay que reintroducirlos (texto, sin lógica).
- Sin cambios de contrato en import ni en `/api/config`.

## Open Questions

- Alternativa rechazada: exigir re-autenticación para exportar. Más fricción y no resuelve
  que el fichero exportado quede guardado en disco con secretos.
