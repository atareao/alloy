# Design

## Context

`notify_telegram` exige `telegram_token` **y** `telegram_chat_id`; `notify_matrix` exige
`matrix_homeserver`, `matrix_token` **y** `matrix_room`; `notify_valet` exige `valet_url` y
`valet_token`; `notify_webhook` solo `webhook_url`. Hoy `*_configured` no refleja esos
requisitos.

## Decisiones

### 1. `*_configured` = el canal puede enviar
Se define como la conjunción de los campos obligatorios de cada canal (ver proposal). Es una
función pura derivada de `Settings`, testeable sin red.

### 2. Editabilidad de configuraciones parciales
Desde `redact-notification-secrets` el valor del token **no** viaja al navegador, así que la
UI no puede detectar "hay un token guardado" mirando el campo. Se añaden a `PublicConfig`
tres banderas **no secretas**: `telegram_token_set`, `matrix_token_set`, `valet_token_set`
(`settings.<token>.is_some()`).

El switch de cada tarjeta se inicializa con:

```
enabled = configured || <token_set> || <algún campo no secreto presente>
```

Así, una config parcial (p. ej. token guardado pero sin chat_id, o homeserver sin token)
sigue mostrando los campos y es editable, en lugar de ocultarlos. El switch "on" con canal
incompleto muestra un `Typography.Text type="warning"`: "Faltan campos para completar la
configuración".

### 3. Sin cambio en la semántica de guardado
El guardado sigue enviando los campos presentes; el backend ya distingue "ausente = no
tocar" de `""` = borrar. No se toca `UpdateSettingsReq`.

## Riesgos

- **Regresión visual**: un canal con config parcial (antes "Activado") pasa a mostrar aviso.
  Es el comportamiento correcto y deseado.
- **Tests existentes**: los que asumían `matrix_configured = homeserver` deben actualizarse.

## Migration

Sin migración de datos; es cálculo derivado. Rollback trivial (revertir código).
