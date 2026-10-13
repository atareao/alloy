# Proposal

## Why

El review de `valet-notifier` (capability `notifications`) señaló que la tarjeta **Valet**
de `ConfigPage` no expone nombres accesibles: el `<Switch>` no tiene `aria-label` y los
inputs de URL/token solo tienen `placeholder`, sin etiqueta programática asociada. Un
usuario de lector de pantalla no puede identificar el switch ni los campos.

Es una deuda de accesibilidad de bajo riesgo y solo de frontend, heredada del patrón
existente de Telegram/Matrix pero introducida en la tarjeta nueva.

## What Changes

- Añadir `aria-label` al `<Switch>` de Valet ("Activar Valet").
- Añadir `aria-label` a los inputs de URL ("URL de Valet") y de token ("Ingest key de
  Valet").
- Sin cambios de comportamiento ni de contrato de API.

## Capabilities

### New Capabilities
- `ui/a11y`: accesibilidad de la tarjeta de configuración de Valet.

### Modified Capabilities
<!-- Ninguna. -->

## Impact

- `frontend/src/components/ConfigPage.tsx`: tres atributos `aria-label` en la tarjeta Valet.
- `frontend/src/components/ConfigPage.test.tsx`: un test que verifique que el switch y los
  campos son accesibles por su nombre.
- Sin cambios en backend.

## Open Questions

- **Alcance**: se limita a la tarjeta Valet. Las tarjetas Telegram/Matrix tienen el mismo
  defecto (pre-existente); arreglarlas es un cambio aparte si se desea.
