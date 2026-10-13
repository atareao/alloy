# Spec Delta

## Purpose

Definir que la exportación de configuración de Alloy no filtra los secretos de los canales
de notificación.

## ADDED Requirements

### Requirement: La exportación no incluye secretos


`GET /api/admin/export` MUST NOT incluir secretos en claro dentro del payload de `settings`:
los tokens `telegram_token`, `matrix_token` y `valet_token`, y también `webhook_url` (que
puede ser una credencial), MUST emitirse como `null`. El resto de campos de `settings` y
`update_history` MUST conservarse. `import_config_h` MUST seguir aceptando secretos si el
payload los aporta.

#### Scenario: Exportar con canales configurados

- **Given** tokens de Telegram, Matrix y Valet configurados y un `webhook_url` definido
- **When** se llama a `GET /api/admin/export`
- **Then** `settings.telegram_token`, `settings.matrix_token`, `settings.valet_token` y
  `settings.webhook_url` son `null`
- **And** el resto de campos de `settings` y `update_history` se conservan

#### Scenario: Round-trip documentado

- **Given** un export con los tokens a `null`
- **When** se importa ese JSON
- **Then** los canales quedan sin token y hay que reintroducirlos
