# BatchProgress — Progress Bar Fix

## MODIFIED Requirements

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