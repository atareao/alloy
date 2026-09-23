# ui/batch-progress Specification

## Purpose
Tarjeta de progreso para operaciones por lotes (check/update de contenedores) que muestra barra de progreso, contador con contenedor y acción actual, y resumen en vivo de resultados.

## Requirements

### Requirement: Barra de progreso con porcentaje real

El sistema SHALL mostrar una barra de progreso con el porcentaje calculado como `(current / total) * 100`, donde `current` es el número de contenedores procesados (done=true) y `total` es el número total de contenedores en el lote.

El sistema SHALL sincronizar `batchProgress.current` y `batchProgress.total` con `progress.checked` y `progress.total` respectivamente en cada actualización de state polling, para que la barra de progreso refleje el progreso real.

**Cambio:** En `App.tsx`, el callback de `useStatePoll` ahora ejecuta `setBatchProgress({ current: bp.checked, total: bp.total })` después de `setProgress(bp)`.

#### Scenario: Progreso avanza al completar cada contenedor
- **WHEN** un contenedor termina su proceso (check o update) con `done: true`
- **THEN** el contador `current` se incrementa en 1
- **AND** la barra de progreso refleja el nuevo porcentaje

#### Scenario: Progreso parcial durante batch activo

**Given** un batch activo (batchPhase === "active")  
**When** el state polling recibe `progress.checked = 5` y `progress.total = 10`  
**Then** `batchProgress.current` se actualiza a 5  
**And** `batchProgress.total` se actualiza a 10  
**And** la barra de progreso muestra 50%

#### Scenario: Progreso completo

**Given** un batch activo  
**When** el state polling recibe `progress.checked = 10` y `progress.total = 10`  
**Then** `batchProgress.current` se actualiza a 10  
**And** la barra de progreso muestra 100%

#### Scenario: Cancelación

**Given** un batch activo  
**When** el usuario cancela  
**Then** `batchProgress` se resetea a `{ current: 0, total: 0 }`  
**And** la barra de progreso desaparece (phase → "idle")

#### Scenario: Sin batch activo

**Given** batchPhase === "idle"  
**When** el state polling recibe cualquier progreso  
**Then** `batchProgress` no se modifica  
**And** BatchProgress no se renderiza

### Requirement: Una sola fase (sin transición checking → updating)
El sistema SHALL usar un único estado `"active"` para todo el proceso de verificación y actualización. No hay separación entre "checking" y "updating". El `batchPhase` pasa de `"idle"` a `"active"` al iniciar y vuelve a `"idle"` al completar.

#### Scenario: Fase única durante todo el proceso
- **WHEN** el usuario hace clic en "Check"
- **THEN** `batchPhase` cambia a `"active"`
- **AND** el título de la card es `"🔄 Revisando y actualizando containers..."` (único, no cambia)
- **AND** el total de la barra de progreso es el número total de contenedores
- **WHEN** el proceso termina (todos los contenedores tienen `done: true`)
- **THEN** `batchPhase` vuelve a `"idle"`

### Requirement: Texto de progreso sin "iniciando..."
El sistema SHALL mostrar el texto de estado del contenedor actual con el formato `{current} / {total} — {status}`. NO debe mostrar "iniciando..." en ningún momento. Si no hay datos de progreso, debe mostrar `"🔍 Verificando..."`.

#### Scenario: Muestra contenedor siendo verificado
- **WHEN** hay contenedores en progreso
- **THEN** el texto muestra `X / N — 🔍 Verificando <container>:<tag>...`

#### Scenario: Muestra contenedor siendo actualizado
- **WHEN** un contenedor está siendo actualizado (pull + restart)
- **THEN** el texto muestra `X / N — 🔄 actualizando <container>...`

#### Scenario: Muestra "Verificando..." si no hay datos
- **WHEN** la fase es active y no hay entries en el progress Map
- **THEN** el texto muestra `X / N — 🔍 Verificando...` (no "iniciando...")

#### Scenario: Muestra "Completado" al terminar
- **WHEN** todos los contenedores tienen `done: true`
- **THEN** el texto muestra `N / N — ✅ Completado`

### Requirement: Total constante durante toda la operación
El sistema SHALL mantener el mismo `total` en la barra de progreso durante todo el proceso, desde que se inicia hasta que termina. El total es siempre el número total de contenedores.

#### Scenario: Total no cambia durante la operación
- **WHEN** la operación está en progreso
- **THEN** el `total` en la barra de progreso se mantiene constante
- **AND** el contador `current` se incrementa a medida que los contenedores completan su proceso

### Requirement: Resumen en vivo de resultados
El sistema SHALL mostrar un resumen debajo de la barra con: total de contenedores, revisados (done=true), actualizados (done=true, sin error, status contiene "actualizado"/"pulled"/"Updated"), errores (error != null o status empieza con ❌/⚠️), y pendientes (done=false).

#### Scenario: Resumen se actualiza en tiempo real
- **WHEN** un contenedor cambia su estado en el progress Map
- **THEN** los contadores del resumen se actualizan en el mismo render

#### Scenario: Errores se contabilizan en revisados y errores
- **WHEN** un contenedor termina con error
- **THEN** aparece en el contador de revisados
- **AND** aparece en el contador de errores
- **AND** NO aparece en el contador de actualizados

### Requirement: Sin lista scrollable de entradas individuales
El sistema SHALL NOT mostrar una lista scrollable con entradas individuales de cada contenedor. Toda la información de progreso se muestra exclusivamente mediante la barra de progreso, el contador con acción actual, y el resumen en vivo.

#### Scenario: No hay lista de entradas
- **WHEN** se renderiza el componente BatchProgress
- **THEN** no hay ningún elemento con `overflow: auto` o `maxHeight` para listar entradas individuales

### Requirement: BatchProgress reemplaza la card de búsqueda durante la operación
El sistema SHALL ocultar la card de búsqueda/filtros de ContainerTable durante la fase `active`, y mostrarla de nuevo al volver a `idle`. El componente BatchProgress ocupa el espacio visual de la card de búsqueda.

#### Scenario: Card de búsqueda oculta durante active
- **WHEN** `batchPhase` es `"active"`
- **THEN** la card de búsqueda/filtros NO se renderiza
- **AND** BatchProgress se muestra en su lugar

#### Scenario: Card de búsqueda reaparece al terminar
- **WHEN** `batchPhase` vuelve a `"idle"`
- **THEN** la card de búsqueda/filtros se renderiza normalmente
