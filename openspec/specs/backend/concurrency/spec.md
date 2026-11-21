# backend/concurrency Specification

## Purpose
Definir la garantía de concurrencia de los handlers que persisten `Settings`: ninguno MUST
retener el mutex de `Settings` mientras espera a que la pool de BD le entregue una conexión. Un
guard de `settings.lock()` no MUST cruzar un `await` de adquisición de conexión. (El orden frente
al mutex de `update_history` en `backend/src/updates/handlers.rs` queda fuera de esta capability;
se trata en el issue #170.)

## Requirements

### Requirement: Ningún guard de `Settings` cruza el `await` de la pool

Todo handler que persiste `Settings` MUST haber adquirido la conexión de la pool
(`db_pool.get().await`) **antes** de tomar `settings.lock().await`, o bien MUST soltar el guard
antes de esperar la conexión. En ningún caso un guard de `settings.lock().await` MUST mantenerse
vivo durante un `await` de adquisición de conexión. El orden de adquisición de la conexión
respecto al estado MUST ser uniforme (conexión antes que estado) en los handlers afectados.

Aplica a: `config.rs::update_config_h`; `admin.rs::put_update_check_config_h`,
`put_default_update_policy_h`, `export_config_h`, `import_config_h`.

#### Scenario: Pool saturada no bloquea a otros lectores de `Settings`

- **Given** una pool de BD con una única conexión, ocupada por otro cliente
- **When** un handler de persistencia de configuración (`PUT /api/config`, `PUT` de update
  check config, `PUT` de default update policy, export o import) empieza a ejecutarse
- **Then** otro lector concurrente de `settings.lock()` obtiene el guard sin esperar a que el
  handler termine de adquirir la conexión

#### Scenario: La mutación sigue siendo atómica y persistida

- **Given** una pool sana y un handler `PUT /api/config` con un payload válido
- **When** el handler completa
- **Then** el estado en memoria refleja los campos enviados
- **And** `db::save_settings` se invoca una vez con ese estado

#### Scenario: Pool no disponible no deja estado a medias

- **Given** una pool que no puede entregar conexión
- **When** un handler de persistencia se ejecuta
- **Then** el estado en memoria no refleja una mutación a medias (el handler falla antes de
  mutar o la mutación y el guardado quedan consistentes)
