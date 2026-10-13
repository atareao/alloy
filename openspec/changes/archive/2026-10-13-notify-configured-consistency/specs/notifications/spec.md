# Spec Delta

## ADDED Requirements

### Requirement: El indicador "configurado" refleja la capacidad real de envío

`GET /api/config` MUST calcular cada `*_configured` como la conjunción de los campos que el
notificador correspondiente exige: `telegram_configured = telegram_token ∧ telegram_chat_id`;
`matrix_configured = matrix_homeserver ∧ matrix_token ∧ matrix_room`;
`valet_configured = valet_url ∧ valet_token`; `webhook_configured = webhook_url`. Una
configuración parcial MUST marcar el canal como NO configurado. `PublicConfig` MUST exponer
además banderas no secretas `telegram_token_set`, `matrix_token_set` y `valet_token_set`
(indicando si hay un token guardado, sin revelar su valor).

#### Scenario: Canal parcial no está configurado

- **Given** `matrix_homeserver` definido pero `matrix_token` o `matrix_room` ausentes
- **When** se llama a `GET /api/config`
- **Then** `matrix_configured` es `false`

#### Scenario: Canal completo está configurado

- **Given** `matrix_homeserver`, `matrix_token` y `matrix_room` definidos
- **When** se llama a `GET /api/config`
- **Then** `matrix_configured` es `true`

#### Scenario: Telegram necesita chat_id

- **Given** `telegram_token` definido pero `telegram_chat_id` ausente
- **When** se llama a `GET /api/config`
- **Then** `telegram_configured` es `false`

### Requirement: La UI permite editar configuraciones parciales

La tarjeta de cada canal MUST inicializar su switch activado si el canal está configurado,
**o** si hay un token guardado (`*_token_set`), **o** si alguno de sus campos no secretos
está presente; de modo que una configuración parcial siga siendo editable. MUST mostrar un
aviso cuando el switch está activado pero el canal no está completo.

#### Scenario: Config parcial editable con aviso

- **Given** una tarjeta con algún campo o bandera `*_token_set` presente pero `*_configured=false`
- **When** se renderiza
- **Then** el switch está activado, los campos son visibles y se muestra el aviso
  "Faltan campos para completar la configuración"

#### Scenario: Config completa sin aviso

- **Given** una tarjeta con el canal completo (`*_configured=true`)
- **When** se renderiza
- **Then** no se muestra el aviso
