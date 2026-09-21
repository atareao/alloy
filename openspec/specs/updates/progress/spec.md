# updates/progress Specification

## Purpose
TBD - created by archiving change fix-update-progress-counters. Update Purpose after archive.

## Requirements

### Requirement: Backend apply_single_policy SHALL pass correct counters

The `apply_single_policy` function MUST receive and forward the actual `total`, `checked`, `updated`, `errors` counter values from `check_and_apply_all` instead of passing `0`.

**Contracts:**
```rust
async fn apply_single_policy(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    update_tx: &broadcast::Sender<UpdateProgress>,
    progress_cache: &Arc<Mutex<HashMap<String, UpdateProgress>>>,
    notif_tx: &broadcast::Sender<NotifEvent>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: &DbPool,
    _tx: &broadcast::Sender<StateEvent>,
    p: &PendingUpdate,
    policy: &UpdatePolicy,
    any_success: &mut bool,
    total: u32,
    checked: u32,
    updated: u32,
    errors: u32,
)
```

#### Scenario: apply_single_policy sends correct counters
- **Given** un contenedor con `has_update=true` y política `PullRestart`
- **When** `check_and_apply_all` llama a `apply_single_policy`
- **Then** todas las llamadas a `update_progress` dentro de `apply_single_policy` MUST usar los valores `total`, `checked`, `updated`, `errors` recibidos como parámetros, no `0`

### Requirement: Frontend SHALL log SSE events to console

The frontend SSE handler for `update-progress` events MUST include a `console.log` statement to aid debugging.

#### Scenario: Console log en SSE handler
- **Given** el frontend recibe un evento SSE `update-progress`
- **When** el manejador procesa el evento
- **Then** MUST ejecutar `console.log("SSE update-progress:", data)` con los datos del evento

### Requirement: Frontend BatchProgress SHALL handle checked=0 correctly

The `checked` variable in `BatchProgress` MUST use an explicit null/undefined check instead of the `??` operator to correctly display `0` when the backend reports zero checked containers.

**Contracts:**
```typescript
const checked = latestProgress !== null && latestProgress !== undefined
    ? latestProgress.checked
    : current;
```

#### Scenario: Frontend muestra checked=0 correctamente
- **Given** un progreso donde el último entry tiene `checked=0`
- **When** `BatchProgress` calcula `checked` para el resumen
- **Then** MUST mostrar `0 revisados` (no `current` del `batchProgress`)
