# Spec Delta

## ADDED Requirements

### Requirement: Los secretos de notificación no se exponen por la API

`GET /api/config` y la respuesta de `PUT /api/config` MUST NOT incluir los secretos de los
canales de notificación en claro: `telegram_token`, `matrix_token` y `valet_token` MUST estar
ausentes del payload. La API MUST exponer únicamente indicadores booleanos `*_configured`.
`PUT /api/config` MUST distinguir "campo de token ausente" (conservar el actual) de cadena
vacía (borrarlo), y la UI MUST permitir guardar sin reenviar el token cuando el campo se deja
vacío.

#### Scenario: Lectura de la config sin secretos

- **Given** canales configurados con sus tokens
- **When** se llama a `GET /api/config`
- **Then** la respuesta NO contiene `telegram_token`, `matrix_token` ni `valet_token`
- **And** sí contiene `telegram_configured`, `matrix_configured` y `valet_configured` en `true`

#### Scenario: Guardar sin reenviar el token lo conserva

- **Given** un canal activado con token guardado y el campo de token vacío en la UI
- **When** se hace `PUT /api/config` sin el campo de token
- **Then** el token existente se conserva y el canal sigue configurado

#### Scenario: Escribir un token lo reemplaza

- **Given** un canal con token guardado
- **When** se hace `PUT /api/config` con un token nuevo no vacío
- **Then** el token se reemplaza por el nuevo

#### Scenario: Desactivar borra el token

- **Given** un canal activado
- **When** se hace `PUT /api/config` con el token a `""`
- **Then** el token pasa a `None` y `*_configured` pasa a `false`
