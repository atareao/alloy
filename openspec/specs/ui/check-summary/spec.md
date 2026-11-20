# ui/check-summary Specification

## Purpose
Define el diálogo de resumen que aparece tras un batch de comprobación/actualización: debe mostrar recuentos reales (no ceros) de contenedores comprobados, actualizables, actuales y errores, y —si se aplicaron actualizaciones— actualizados y fallos.

## Requirements

### Requirement: Resumen post-batch con contadores reales

Tras un check manual (`POST /api/check-all`), el `SummaryDialog` MUST mostrar recuentos reales:
**Comprobados** (contenedores revisados), **Actualizables** (con actualización disponible),
**Actuales** (sin actualización) y **Errores** (fallos al comprobar o aplicar). Si se aplicaron
actualizaciones, MUST mostrar además una tarjeta de **Actualización** con **Actualizados**
(aplicados con éxito) y **Fallos** (aplicaciones fallidas). El diálogo MUST NOT mostrar ceros
cuando el batch procesó contenedores.

#### Scenario: Batch sin actualizaciones

- **Given** un batch que comprobó 10 contenedores, todos al día y sin errores
- **When** termina el check manual
- **Then** el resumen muestra Comprobados=10, Actualizables=0, Actuales=10 y Errores=0

#### Scenario: Batch con contenedores actualizables

- **Given** un batch de 10 contenedores en el que 3 tienen actualización disponible y no hay errores ni actualizaciones aplicadas
- **When** termina el check manual
- **Then** el resumen muestra Comprobados=10, Actualizables=3, Actuales=7 y Errores=0

#### Scenario: Batch con errores de comprobación

- **Given** un batch de 10 contenedores en el que la consulta de digest de 2 falla y 3 tienen actualización disponible
- **When** termina el check manual
- **Then** el resumen muestra Comprobados=10, Actualizables=3, Actuales=5 y Errores=2

#### Scenario: Batch con actualizaciones aplicadas

- **Given** un batch con 3 contenedores actualizables en el que 2 se aplican con éxito y 1 falla
- **When** termina el check manual
- **Then** la tarjeta de Actualización muestra Actualizados=2 y Fallos=1

#### Scenario: Batch cancelado

- **Given** un usuario que cancela el batch tras procesar varios contenedores
- **When** termina el batch
- **Then** los contadores de Comprobados/Actualizables/Actuales/Errores reflejan solo lo procesado hasta la cancelación (no ceros)
