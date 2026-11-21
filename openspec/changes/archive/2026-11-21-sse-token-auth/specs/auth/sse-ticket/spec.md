## ADDED Requirements

### Requirement: El endpoint `GET /api/auth/sse-token` SHALL emitir un ticket SSE efímero

El endpoint `GET /api/auth/sse-token` MUST requerir una sesión válida (cookie o `Authorization: Bearer`) y MUST devolver JSON `{ "token": String, "expires_in": u64 }`. El `token` MUST ser un JWT firmado con el mismo secreto de sesión, con claims `sub`, `iat`, `exp` y `purpose = "sse"`, donde `exp = iat + 60`. El token MUST NOT ser el JWT de sesión. La respuesta MUST incluir `Cache-Control: no-store`.

**Contracts:**
```rust
pub struct SseTicketClaims {
    pub sub: String,
    pub purpose: String, // "sse"
    pub iat: usize,
    pub exp: usize,
}

pub struct SseTicketResponse {
    pub token: String,
    pub expires_in: u64,
}
```

#### Scenario: Emisión con sesión válida

- **GIVEN** una petición a `GET /api/auth/sse-token` con cookie de sesión válida
- **WHEN** el handler emite el ticket
- **THEN** responde `200 OK` con `{ token, expires_in: 60 }`
- **AND** el `token` decodificado tiene `purpose = "sse"` y `exp - iat = 60`
- **AND** el `token` es distinto del JWT de sesión de la cookie
- **AND** la respuesta incluye `Cache-Control: no-store`

#### Scenario: Emisión sin sesión

- **GIVEN** una petición a `GET /api/auth/sse-token` sin cookie ni Bearer válidos
- **WHEN** el middleware procesa la petición
- **THEN** responde `401 Unauthorized` con JSON de error

#### Scenario: El ticket no sirve como sesión

- **GIVEN** un ticket SSE válido
- **WHEN** se envía como cookie `session` o como `Authorization: Bearer`
- **THEN** el middleware responde `401 Unauthorized`

### Requirement: El middleware SHALL aceptar tickets SSE por query sólo en `/api/stream`

El middleware MUST extraer el parámetro `token` de la query sólo cuando la ruta sea exactamente `/api/stream`. MUST validar ese token como `SseTicketClaims` (firma, `purpose = "sse"` y `exp` vigente). MUST rechazar un JWT de sesión enviado por query. MUST NOT aceptar tickets SSE por cookie ni por `Authorization: Bearer`. La cookie de sesión y el Bearer de sesión MUST seguir funcionando con normalidad.

#### Scenario: Ticket válido en el stream

- **GIVEN** un ticket SSE vigente
- **WHEN** se abre `GET /api/stream?token=<ticket>`
- **THEN** el middleware autentica la petición y el stream responde `200 OK` con `Content-Type: text/event-stream`

#### Scenario: JWT de sesión en query

- **GIVEN** un JWT de sesión válido
- **WHEN** se envía `GET /api/stream?token=<jwt_sesion>`
- **THEN** el middleware responde `401 Unauthorized`

#### Scenario: Ticket fuera del stream

- **GIVEN** un ticket SSE vigente
- **WHEN** se envía `GET /api/containers?token=<ticket>`
- **THEN** el middleware responde `401 Unauthorized`

#### Scenario: Ticket expirado

- **GIVEN** un ticket SSE con `exp` pasado
- **WHEN** se envía `GET /api/stream?token=<ticket>`
- **THEN** el middleware responde `401 Unauthorized`

#### Scenario: Ticket en cookie

- **GIVEN** un ticket SSE vigente
- **WHEN** se envía como cookie `session`
- **THEN** el middleware responde `401 Unauthorized`

#### Scenario: La cookie sigue autenticando el stream

- **GIVEN** una sesión válida en cookie
- **WHEN** se abre `GET /api/stream` sin query token
- **THEN** el middleware autentica la petición y el stream responde `200 OK`

### Requirement: La validación de sesión SHALL rechazar tickets SSE

`SessionClaims` MUST incluir un campo opcional `purpose`. La validación de sesión MUST rechazar cualquier token cuyo `purpose` esté presente. La emisión y el refresco de sesión MUST NOT escribir `purpose`.

#### Scenario: Ticket usado como sesión

- **GIVEN** un ticket SSE vigente
- **WHEN** se decodifica como `SessionClaims` en el middleware
- **THEN** la validación falla con `401 Unauthorized`
