# updates/update-check Delta

## ADDED Requirements

### Requirement: El fin de batch expone contadores reales

Cuando termina un batch de check (`check_and_apply_all`), el evento SSE de fin
(`update-progress` con `done = true`) MUST transportar los contadores reales acumulados:
`checked` (comprobados), `updatable` (con actualización disponible), `uptodate` (al día),
`updated` (aplicados con éxito), `failed` (fallos al aplicar) y `errors` (errores al comprobar).
MUST NOT emitir el evento de fin con todos los contadores a cero cuando se procesaron
contenedores. La limpieza posterior de la caché de progreso MUST NOT sobrescribir el evento de
fin con ceros.

#### Scenario: Evento de fin con contadores reales

- **Given** un batch que procesó 10 contenedores, 3 con actualización disponible, 0 errores y 0 fallos
- **When** finaliza el batch
- **Then** el evento `update-progress` con `done = true` lleva `checked = 10`, `updatable = 3`, `uptodate = 7`, `updated`/`failed`/`errors` coherentes

#### Scenario: Batch sin contenedores

- **Given** un batch sin contenedores que procesar
- **When** finaliza el batch
- **Then** el evento de fin lleva todos los contadores a 0 (es el único caso legítimo de ceros)

#### Scenario: Limpieza de caché sin pisar el resumen

- **Given** un batch que acaba con contadores reales
- **When** se limpia la caché de progreso (`reset_progress_cache`)
- **Then** el evento de fin con contadores reales no se reemite con ceros
