# backend/update-history-lock Specification

## Purpose
Esta capability fija el orden correcto de adquisición de locks en el historial de actualizaciones: ningún guard de `update_history` debe sobrevivir al `await` de la pool, y el historial en memoria debe publicarse antes de persistir. Evita interbloqueos con los handlers que toman la conexión antes que el estado.

## Requirements

### Requirement: Ningún guard de `update_history` SHALL cruzar el `await` de la pool

Todo helper que publique o modifique el historial de actualizaciones MUST soltar el guard de `update_history.lock().await` antes de esperar una conexión con `db_pool.get().await`. El guard MUST NOT mantenerse vivo durante ese `await`, para evitar un ciclo con los handlers que adquieren la conexión antes que el estado (`export_config_h`, `import_config_h`).

Aplica a: `backend/src/stacks.rs::append_stack_history`, `backend/src/updates/history.rs::delete_history_h` y cualquier helper equivalente. `backend/src/updates/handlers.rs::push_history_entry` ya cumple el contrato y MUST conservarse como referencia.

#### Scenario: `append_stack_history` no retiene el guard durante la pool

- **Given** una pool de BD con una única conexión, ocupada por otro cliente
- **When** `append_stack_history` publica una entrada y espera una conexión para persistirla
- **Then** otro lector concurrente de `update_history.lock()` obtiene el guard sin esperar a que la pool entregue la conexión

#### Scenario: `delete_history_h` no retiene el guard durante la pool

- **Given** una pool de BD con una única conexión, ocupada por otro cliente
- **When** `delete_history_h` limpia el historial y espera una conexión para persistir el borrado
- **Then** otro lector concurrente de `update_history.lock()` obtiene el guard sin esperar a que la pool entregue la conexión

### Requirement: El historial en memoria SHALL publicarse antes de persistir

Los helpers MUST publicar el cambio en el historial en memoria (dentro de la sección crítica) antes de intentar la persistencia. Si la persistencia falla, el cambio en memoria MUST mantenerse y el error MUST registrarse sin abortar la operación principal.

#### Scenario: Fallo de persistencia no revierte el historial en memoria

- **Given** una pool de BD no disponible
- **When** un helper publica una entrada en el historial
- **Then** la entrada permanece en memoria
- **AND** el error de persistencia se registra
- **AND** la operación principal no se aborta

### Requirement: El código muerto con el patrón inseguro SHALL eliminarse

`backend/src/updates/common.rs::record_update_entry` MUST eliminarse, porque conserva el patrón inseguro y no tiene call-sites. Si en el futuro se necesita, MUST implementarse con el mismo contrato de orden.

#### Scenario: Sin código muerto con el patrón inseguro

- **WHEN** se busca el patrón `update_history.lock().await` seguido de `db_pool.get().await` fuera de `push_history_entry`
- **THEN** no aparece en `backend/src/updates/common.rs`

### Requirement: La suite SHALL seguir verde

Tras el cambio, `cargo test` y `just check` MUST terminar con código 0. El recuento de tests MUST ser mayor o igual a 539.

#### Scenario: Verificación completa

- **GIVEN** los cambios aplicados
- **WHEN** se ejecutan `cargo test` y `just check`
- **THEN** ambos terminan con código 0
- **AND** hay al menos 539 tests
