# backend/error-handling Specification

## Purpose
Definir que ningún handler de producción aborta el proceso por un fallo de infraestructura de
datos. Los handlers cuya **respuesta depende** de la conexión MUST traducir el fallo a `AppError`
(HTTP 500); los usos **best-effort** (escrituras laterales de un batch, p. ej. timestamps) MUST NOT
panickear y MUST registrar el fallo, sin tumbar la respuesta.

## Requirements

### Requirement: Los handlers no MUST abortar ante un fallo de pool o de lock

Los handlers no-test no MUST usar `unwrap()` sobre `db_pool.get().await` ni sobre `conn.lock()`.

- Cuando la **respuesta** del handler requiere la conexión (leer o persistir estado) y la pool falla
  (cerrada/agotada/error) o el `Mutex` de la conexión está envenenado, el handler MUST devolver
  `AppError::Internal` (HTTP 500 con `{"error": ...}`) y el proceso MUST permanecer en marcha.
- Cuando el uso de la conexión es **best-effort** (escrituras laterales cuyo error no debe cambiar
  la respuesta, p. ej. actualizar timestamps al final de un batch), el handler MUST NOT panickear y
  MUST registrar el fallo (`tracing::warn!`/`error!`), continuando con la respuesta normal.

La conversión MUST centralizarse en un helper reutilizable (`AppError::from_pool`), no duplicarse.

#### Scenario: Fallo de pool en un handler dependiente de la respuesta

- **Given** una pool de BD que falla al entregar conexión (cerrada o con error)
- **When** llega una petición a un handler cuya respuesta usa la conexión (p. ej. `PUT /api/config`)
- **Then** la respuesta es HTTP 500 con JSON `{"error": ...}`
- **And** el proceso no panickea ni termina

#### Scenario: Mutex de conexión envenenado

- **Given** una conexión cuyo `Mutex` ha quedado envenenado
- **When** un handler intenta `conn.lock()`
- **Then** el handler responde 500 si lo necesita para su respuesta, o registra el fallo y continúa si es best-effort
- **And** el proceso sigue en marcha

#### Scenario: Escritura best-effort no tumba la respuesta

- **Given** una pool que falla al final de un batch de comprobación
- **When** el handler intenta persistir timestamps (best-effort)
- **Then** registra el fallo y devuelve la respuesta normal (lista de contenedores)
- **And** el proceso no panickea

#### Scenario: Camino feliz sin cambios

- **Given** una pool sana
- **When** un handler persiste o lee estado
- **Then** el comportamiento y el JSON de la API son idénticos a los actuales
