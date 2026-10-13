# Proposal

## Why

La tarjeta Valet ya expone nombres accesibles (`aria-label`) en su switch y campos, pero las
tarjetas **Telegram** y **Matrix** siguen sin ellos (deuda señalada en el review de
`redact-notification-secrets`). Un lector de pantalla no puede identificar esos controles.

## What Changes

- Añadir `aria-label` a los `<Switch>` de Telegram ("Activar Telegram") y Matrix
  ("Activar Matrix").
- Añadir `aria-label` a los inputs de token de ambas tarjetas ("Token del bot de Telegram",
  "Access token de Matrix").
- Sin cambios de comportamiento.

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `ui/a11y`: nombres accesibles en las tarjetas de notificación (además de Valet).

## Impact

- `frontend/src/components/ConfigPage.tsx`: cuatro `aria-label`.
- `frontend/src/components/ConfigPage.test.tsx`: tests por nombre accesible.

## Open Questions

- **Alcance**: se limitan a switch + input de token de Telegram/Matrix (los controles cuyo
  propósito no es evidente). Los inputs de homeserver/room/chat_id son autocontenidos con su
  etiqueta visible; se pueden añadir en un cambio aparte si se desea.
