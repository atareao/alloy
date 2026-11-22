# Spec Delta: backend/db-connection

## ADDED Requirements

### Requirement: Toda conexión de la pool SHALL aplicar los PRAGMAs de sesión

Cada conexión que la pool entregue a un handler, worker o test MUST tener `busy_timeout` de 5000 ms y `foreign_keys` activado, con independencia del orden en que se creen o reciclen las conexiones. Los PRAGMAs MUST aplicarse al crear la conexión dentro del manager de la pool, no solo en la conexión de inicialización, y un fallo al aplicarlos MUST impedir que esa conexión se entregue como válida.

#### Scenario: Dos conexiones distintas de la pool

- **GIVEN** una pool de tamaño 2
- **WHEN** se toman dos conexiones distintas y se consulta `PRAGMA busy_timeout` y `PRAGMA foreign_keys` en cada una
- **THEN** ambas devuelven 5000 y 1 respectivamente

#### Scenario: Conexión reciclada

- **GIVEN** una conexión devuelta a la pool y vuelta a tomar
- **WHEN** se consulta `PRAGMA busy_timeout`
- **THEN** sigue siendo 5000

### Requirement: El arranque SHALL verificar el modo journal

`init_db` MUST comprobar el valor devuelto al solicitar `PRAGMA journal_mode=WAL` y, si el modo resultante no es `wal`, MUST emitir un aviso de nivel `warn` que indique el modo activo y su consecuencia, sin abortar el arranque.

#### Scenario: Sistema de ficheros sin soporte WAL

- **GIVEN** una base de datos cuyo `PRAGMA journal_mode=WAL` devuelve `delete`
- **WHEN** se inicializa la base de datos
- **THEN** el proceso arranca, queda registrado un `warn` con el modo efectivo y la aplicación sigue operativa

#### Scenario: WAL disponible

- **GIVEN** un directorio temporal en un sistema de ficheros con soporte WAL
- **WHEN** se inicializa la base de datos
- **THEN** `PRAGMA journal_mode` devuelve `wal` y no se registra ningún aviso de degradación

### Requirement: Los fallos al persistir el histórico SHALL NOT silenciarse

Cuando una escritura de histórico falle en un camino best-effort, el error MUST registrarse con nivel de error incluyendo el contenedor y la imagen afectados; MUST NOT descartarse el `Result` con `let _ =`.

#### Scenario: Append fallido en el scheduler

- **GIVEN** una escritura de histórico que devuelve error
- **WHEN** el scheduler intenta persistir la entrada
- **THEN** el error se registra con `tracing::error!` y la operación principal del scheduler continúa