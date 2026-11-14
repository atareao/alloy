# dev/pre-commit-hook Specification

## Purpose

Definir el contrato del hook `pre-commit` de Alloy: bloquea commits que toquen código fuente sin un
change proposal activo, avisa de tareas incompletas sin fallar, y es instalable y reproducible desde
el propio repositorio (no solo desde `.git/hooks/`).

## ADDED Requirements

### Requirement: El hook está versionado e instalable

El hook `pre-commit` MUST existir como fichero **versionado** en el repositorio (`.githooks/pre-commit`)
y MUST poder instalarse con un comando documentado (`just install-hooks`, que configura
`core.hooksPath`). Ningún comportamiento del hook MUST depender de un fichero no versionado.

#### Scenario: Instalación reproducible

- **Given** un clon fresco del repositorio sin `core.hooksPath` configurado
- **When** se ejecuta `just install-hooks`
- **Then** `core.hooksPath` apunta a `.githooks` y el hook queda activo

### Requirement: El aviso de tareas incompletas no falla con cero pendientes

El hook MUST avisar del número de tareas sin completar de cada change activo, y con **cero** tareas
incompletas MUST NOT producir ningún error (en particular, MUST NOT intentar comparar un valor no
numérico). El conteo MUST ser un entero válido aunque `grep` no encuentre coincidencias.

#### Scenario: Change con todas las tareas completadas

- **Given** un `tasks.md` sin ninguna tarea pendiente
- **When** se ejecuta el hook sobre cambios de código staged
- **Then** no se imprime ningún error de comparación entera
- **And** no se muestra el aviso de tareas incompletas

#### Scenario: Change con tareas pendientes

- **Given** un `tasks.md` con N tareas pendientes (N > 0)
- **When** se ejecuta el hook
- **Then** muestra el aviso indicando exactamente N

### Requirement: El hook bloquea código fuente sin proposal

El hook MUST rechazar (exit != 0) el commit de cambios staged en ficheros fuente
(`*.rs`, `*.ts`, `*.tsx`, `*.js`, `*.jsx`) cuando no haya ningún change proposal activo, y MUST
permitirlo cuando sí lo haya. El aviso de tareas incompletas MUST ser informativo: no MUST bloquear
el commit por sí solo.

#### Scenario: Código staged sin proposal

- **Given** cambios staged en un fichero `.rs` y ningún change proposal activo
- **When** se ejecuta el hook
- **Then** el commit se rechaza con un mensaje que indica crear un proposal

#### Scenario: Tareas incompletas no bloquean

- **Given** un change con tareas pendientes y código fuente staged
- **When** se ejecuta el hook
- **Then** el hook avisa pero termina con éxito (exit 0)
