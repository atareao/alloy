# updates Specification

## Purpose
TBD - created by archiving change update-history-retention. Update Purpose after archive.

## Requirements

### Requirement: Retención configurable por días

El sistema SHALL exponer una política de retención de `update_history` expresada en días, persistida en la configuración del proyecto. El valor SHALL interpretarse así:

- `0` → retención ilimitada (no se poda automáticamente).
- `N > 0` → se eliminan los registros con `created_at` anterior a `now - N días`.

El valor por defecto SHALL ser `30`.

#### Contract: Rust types

```rust
/// Configuración de retención de histórico de actualizaciones.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateHistoryRetention {
    /// Días de retención. `0` significa retención ilimitada.
    pub retention_days: u32,
}

impl Default for UpdateHistoryRetention {
    fn default() -> Self {
        Self { retention_days: 30 }
    }
}

/// Resultado de una operación de poda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneResult {
    /// Número de filas eliminadas.
    pub deleted: u64,
    /// Días de retención aplicados (0 = ilimitada, deleted siempre 0).
    pub retention_days: u32,
    /// Duración de la operación en milisegundos.
    pub duration_ms: u64,
}
```

#### Contract: Repositorio

```rust
/// Elimina de `update_history` los registros más antiguos que `retention_days`.
/// Devuelve el número de filas eliminadas.
/// Si `retention_days == 0`, no elimina nada y devuelve 0.
pub async fn prune_update_history(pool: &SqlitePool, retention_days: u32) -> Result<u64, DbError>;
```

#### Contract: API endpoints

| Método | Ruta | Body | Respuesta | Errores |
|---|---|---|---|---|
| `GET` | `/api/updates/retention` | — | `200` `UpdateHistoryRetention` | `401` no autenticado |
| `PUT` | `/api/updates/retention` | `UpdateHistoryRetention` | `200` `UpdateHistoryRetention` | `400` body inválido, `401` |
| `POST` | `/api/updates/retention/prune` | — | `200` `PruneResult` | `401` |

#### Contract: DB schema

```sql
-- Migración idempotente (solo índice, no cambia columnas).
CREATE INDEX IF NOT EXISTS idx_update_history_created_at
    ON update_history (created_at);
```

La sentencia de poda SHALL ser:

```sql
DELETE FROM update_history WHERE created_at < datetime('now', ?1);
-- ?1 = format!("-{} days", retention_days)
```

#### Scenario: Lectura de la política por defecto

- **GIVEN** una instalación nueva sin valor de retención guardado
- **WHEN** el cliente hace `GET /api/updates/retention`
- **THEN** la respuesta es `200` con `{ "retention_days": 30 }`

#### Scenario: Actualización válida de la política

- **GIVEN** un usuario autenticado
- **WHEN** hace `PUT /api/updates/retention` con `{ "retention_days": 7 }`
- **THEN** la respuesta es `200` con `{ "retention_days": 7 }`
- **AND** el valor queda persistido en la configuración
- **AND** la siguiente poda automática usa `7` días

#### Scenario: Valor de retención ilimitada

- **GIVEN** el usuario configura `retention_days = 0`
- **WHEN** se ejecuta la poda automática o manual
- **THEN** no se elimina ningún registro
- **AND** `PruneResult.deleted == 0`
- **AND** `update_history` conserva todas sus filas

### Requirement: Poda automática periódica

El sistema SHALL ejecutar una poda automática de `update_history` desde el worker de fondo existente, con un intervalo configurable independiente de la retención.

La poda SHALL:

- Ejecutarse sin bloquear las operaciones de actualización en curso.
- Registrar un log estructurado con `deleted`, `duration_ms` y `retention_days`.
- No abortar el worker si una ejecución falla; el error se loguea y se reintenta en el siguiente ciclo.

#### Scenario: Poda elimina registros fuera de retención

- **GIVEN** `retention_days = 30` y existen filas con `created_at` de 45 días atrás
- **WHEN** el worker ejecuta la poda
- **THEN** las filas de 45 días se eliminan
- **AND** las filas de los últimos 30 días permanecen intactas
- **AND** se emite un log `deleted = <n>`

#### Scenario: Poda es idempotente

- **GIVEN** una poda ya ejecutada que eliminó todas las filas fuera de retención
- **WHEN** se ejecuta la poda de nuevo inmediatamente
- **THEN** `deleted == 0`
- **AND** no se produce ningún error

#### Scenario: Fallo de base de datos no rompe el worker

- **GIVEN** la base de datos devuelve un error temporal durante la poda
- **WHEN** el worker procesa el ciclo
- **THEN** el error se loguea como `WARN`/`ERROR`
- **AND** el worker continúa vivo
- **AND** la siguiente iteración vuelve a intentar la poda

### Requirement: Poda manual bajo demanda

El sistema SHALL permitir lanzar una poda inmediata mediante `POST /api/updates/retention/prune`, usando la política vigente.

#### Scenario: Poda manual con retención activa

- **GIVEN** `retention_days = 7` y hay filas de hace 10 días
- **WHEN** el usuario hace `POST /api/updates/retention/prune`
- **THEN** la respuesta es `200` con `deleted >= 1`, `retention_days = 7` y `duration_ms >= 0`
- **AND** las filas de hace 10 días ya no existen

#### Scenario: Poda manual sin sesión

- **GIVEN** una petición sin cookie de sesión válida
- **WHEN** se hace `POST /api/updates/retention/prune`
- **THEN** la respuesta es `401`
- **AND** no se elimina ninguna fila

### Requirement: Validación de entrada

El sistema SHALL rechazar valores de retención no válidos con `400` y un mensaje de error descriptivo.

#### Scenario: Body inválido en actualización

- **GIVEN** un usuario autenticado
- **WHEN** hace `PUT /api/updates/retention` con `{ "retention_days": -1 }` o un body no JSON
- **THEN** la respuesta es `400`
- **AND** la configuración persistida no cambia

#### Scenario: Valor extremadamente grande es aceptado

- **GIVEN** un usuario autenticado
- **WHEN** hace `PUT /api/updates/retention` con `{ "retention_days": 36500 }`
- **THEN** la respuesta es `200`
- **AND** la poda resultante no elimina filas dentro de ese rango

### Requirement: No interferencia con operaciones críticas

La poda SHALL ejecutarse en lotes o con un `DELETE` acotado por índice, de forma que no bloquee las escrituras de nuevas entradas de `update_history` ni las actualizaciones de contenedores.

#### Scenario: Actualización concurrente durante la poda

- **GIVEN** una poda en curso sobre un histórico grande
- **WHEN** se registra un nuevo evento de actualización
- **THEN** la inserción se completa correctamente
- **AND** el nuevo registro no es eliminado por esa poda

#### Scenario: Histórico vacío

- **GIVEN** `update_history` está vacía
- **WHEN** se ejecuta la poda (automática o manual)
- **THEN** `deleted == 0`
- **AND** no se produce ningún error
