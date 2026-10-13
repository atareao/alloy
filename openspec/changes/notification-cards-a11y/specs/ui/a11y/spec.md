# Spec Delta

## ADDED Requirements

### Requirement: Controles accesibles de las tarjetas Telegram y Matrix

Los `<Switch>` de activación y los campos de token de las tarjetas Telegram y Matrix MUST
exponer un nombre accesible (`aria-label`), con el mismo criterio que la tarjeta Valet.

#### Scenario: Switch de Telegram con nombre accesible

- **Given** la tarjeta Telegram renderizada
- **When** un lector de pantalla enfoca el switch
- **Then** se anuncia con el nombre "Activar Telegram"

#### Scenario: Switch de Matrix con nombre accesible

- **Given** la tarjeta Matrix renderizada
- **When** un lector de pantalla enfoca el switch
- **Then** se anuncia con el nombre "Activar Matrix"

#### Scenario: Campos de token con nombre accesible

- **Given** las tarjetas Telegram y Matrix activadas
- **When** un lector de pantalla enfoca el campo de token
- **Then** se anuncia "Token del bot de Telegram" y "Access token de Matrix" respectivamente
