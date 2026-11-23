# frontend/deps-dedupe Specification

## Purpose
Esta capability garantiza que el lockfile resuelva una única versión de `@ant-design/cssinjs`, compatible con los rangos declarados, con una comprobación reproducible y el build y los tests en verde.

## Requirements

### Requirement: El lock SHALL resolver una única versión de `@ant-design/cssinjs`

`frontend/pnpm-lock.yaml` MUST contener una sola versión resuelta de `@ant-design/cssinjs`. MUST NOT existir más de una versión distinta con el prefijo `@ant-design/cssinjs@` en el lock.

#### Scenario: Una sola entrada en el lock

- **Given** el `pnpm-lock.yaml` del frontend
- **When** se buscan las entradas que empiezan por `@ant-design/cssinjs@`
- **Then** aparece exactamente una versión distinta
- **And** no aparecen dos versiones simultáneas

### Requirement: La versión fijada SHALL satisfacer los rangos declarados

La versión resuelta MUST satisfacer el rango que declara `antd` (`~2.1.2`) y el que declara `@ant-design/cssinjs-utils` (`^2.1.2`). El override MUST fijarse a una versión que cumpla ambos, sin forzar una versión que `antd` excluya.

El override MUST declararse en `frontend/pnpm-workspace.yaml`: pnpm ≥11 ignora el campo `pnpm` de `package.json`, así que `pnpm.overrides` ya no surte efecto.

#### Scenario: El override es compatible con ambos consumidores

- **Given** `overrides` en `frontend/pnpm-workspace.yaml` fija `@ant-design/cssinjs` a `2.1.2`
- **When** se resuelve la dependencia
- **Then** `2.1.2` satisface `~2.1.2` de `antd`
- **And** `2.1.2` satisface `^2.1.2` de `@ant-design/cssinjs-utils`

#### Scenario: No se adopta una versión excluida por antd

- **Given** la decisión documentada del change
- **When** se revisa la versión fijada
- **Then** no es `2.2.0`, que queda fuera del rango `~2.1.2` de `antd`
- **And** la alternativa `2.2.0` queda documentada como plan B

### Requirement: El build y los tests SHALL seguir en verde

Tras aplicar el override y regenerar el lock, todas las verificaciones del frontend MUST terminar con código 0: `pnpm exec tsc -b`, `pnpm run lint`, `npx vitest run` y `pnpm run build`.

#### Scenario: Verificación completa del frontend

- **Given** el lock regenerado con una única versión
- **When** se ejecutan `pnpm exec tsc -b`, `pnpm run lint`, `npx vitest run` y `pnpm run build`
- **Then** los cuatro terminan con código 0
- **And** no aparecen errores nuevos de estilos ni de tipos

### Requirement: La deduplicación SHALL tener una comprobación reproducible

MUST existir un comando reproducible documentado que demuestre la deduplicación: `pnpm why @ant-design/cssinjs` MUST mostrar una sola versión.

#### Scenario: `pnpm why` muestra una sola versión

- **Given** el lock deduplicado
- **When** se ejecuta `pnpm why @ant-design/cssinjs`
- **Then** la salida muestra una única versión
- **And** el comando queda documentado en `docs/dedupe-antd-cssinjs.md`
