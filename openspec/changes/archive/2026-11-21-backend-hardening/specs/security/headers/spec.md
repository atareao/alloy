# security/headers Specification

## Purpose

Definir el contrato de cabeceras de seguridad: todas las respuestas del servidor MUST incluir un
conjunto base de cabeceras que mitiguen clickjacking, sniffing de tipos, fugas de Referer y, cuando
aplique, downgrade HTTPS; y una CSP compatible con la SPA (React + Ant Design).

## ADDED Requirements

### Requirement: Cabeceras de seguridad en todas las respuestas

Todas las respuestas HTTP MUST incluir:

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Content-Security-Policy` que permita la SPA (scripts/estilos propios, `style-src 'unsafe-inline'`
  por Ant Design, `img-src`/`font-src`/`connect-src` acordes) y que **no** permita `frame-ancestors`
  distintos de `'none'`
- `Strict-Transport-Security` cuando la petición llegue por HTTPS

Las cabeceras MUST NOT romper la carga de la SPA ni de los endpoints de API/SSE.

#### Scenario: Respuesta de API

- **Given** una petición autenticada a `/api/...`
- **When** el servidor responde
- **Then** la respuesta incluye `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY` y
  `Referrer-Policy: strict-origin-when-cross-origin`

#### Scenario: Carga de la SPA

- **Given** una petición a una ruta del frontend servida desde `dist/`
- **When** el navegador aplica la CSP
- **Then** los scripts, estilos y fuentes de la SPA cargan sin ser bloqueados

#### Scenario: HSTS bajo HTTPS

- **Given** una petición servida por HTTPS
- **When** el servidor responde
- **Then** la respuesta incluye `Strict-Transport-Security`
