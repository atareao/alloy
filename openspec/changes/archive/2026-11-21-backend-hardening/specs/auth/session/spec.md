# auth/session Specification (delta)

## ADDED Requirements

### Requirement: La cookie de sesión MUST marcarse `Secure`

La cookie `session` creada tras un login OIDC MUST marcarse `Secure` (además de `HttpOnly` y
`SameSite=Lax`), de modo que el navegador solo la envíe por HTTPS. Para desarrollo local servido
por HTTP plano MUST poder desactivarse explícitamente mediante la variable de entorno
`ALLOW_INSECURE_COOKIES=1` (por defecto **activado**, es decir, cookie `Secure`).

#### Scenario: Producción (por defecto)

- **Given** el servidor sin `ALLOW_INSECURE_COOKIES`
- **When** se completa el login y se emite la cookie `session`
- **Then** la cabecera `Set-Cookie` incluye `Secure`, `HttpOnly` y `SameSite=Lax`

#### Scenario: Desarrollo local en HTTP

- **Given** el servidor con `ALLOW_INSECURE_COOKIES=1`
- **When** se emite la cookie `session`
- **Then** la cabecera `Set-Cookie` incluye `HttpOnly` y `SameSite=Lax` pero NO `Secure`
