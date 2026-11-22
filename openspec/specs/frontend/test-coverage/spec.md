# frontend/test-coverage Specification

## Purpose
Esta capability define la cobertura de tests del frontend: ubicación colocada de los tests por componente y cobertura de las acciones de contenedor, los filtros y el renderizado de `ContainerRow`, `ContainerTable`, `InspectModal` y `DashboardPage`.

## Requirements

### Requirement: El test de ErrorBoundary SHALL estar colocado

El test de `ErrorBoundary` MUST vivir en `frontend/src/components/ErrorBoundary.test.tsx` y MUST importar el componente con `./ErrorBoundary`.

#### Scenario: Test colocado

- **GIVEN** el árbol de `frontend/src`
- **WHEN** se busca el test de `ErrorBoundary`
- **THEN** existe en `frontend/src/components/ErrorBoundary.test.tsx`
- **AND** no existe en `frontend/src/ErrorBoundary.test.tsx`
- **AND** el import es `./ErrorBoundary`

### Requirement: Cada componente SHALL tener un test colocado

Los componentes `DashboardPage`, `ContainerRow`, `ContainerTable`, `ContainerActions` e `InspectModal` MUST tener un archivo `*.test.tsx` en el mismo directorio que el componente.

#### Scenario: Tests colocados

- **GIVEN** cada componente
- **WHEN** se busca su test
- **THEN** existe `frontend/src/components/<Componente>.test.tsx`

### Requirement: Las acciones de contenedor SHALL estar cubiertas

Los tests MUST verificar que `ContainerActions` invoca los callbacks `onInspect`, `onStart`, `onStop`, `onRestart`, `onLogs` y `onRemove` con el nombre del contenedor. MUST cubrir el estado `running` (botón Parar) y no-running (botón Iniciar). MUST cubrir el modo batch (botones deshabilitados) y las acciones de stack cuando hay varios contenedores del mismo `compose_project`.

#### Scenario: Acción de parar

- **GIVEN** un contenedor en estado `running`
- **WHEN** el usuario pulsa "Parar"
- **THEN** se llama `onStop` con el nombre del contenedor

#### Scenario: Acción de iniciar

- **GIVEN** un contenedor en estado `exited`
- **WHEN** el usuario pulsa "Iniciar"
- **THEN** se llama `onStart` con el nombre del contenedor

#### Scenario: Acción de eliminar

- **GIVEN** un contenedor
- **WHEN** el usuario pulsa "Eliminar"
- **THEN** se llama `onRemove` con el nombre del contenedor

#### Scenario: Modo batch

- **GIVEN** `batchPhase = "active"`
- **WHEN** se renderiza `ContainerActions`
- **THEN** los botones de acción están deshabilitados

### Requirement: DashboardPage SHALL cubrir filtros y acciones

Los tests de `DashboardPage` MUST cubrir el renderizado de la tabla, el filtrado por búsqueda y estado, el filtro de actualizaciones pendientes y la invocación de `apiFetch` en las acciones de contenedor. MUST cubrir la apertura/cierre del modal de inspección y la confirmación de borrado.

#### Scenario: Filtro por búsqueda

- **GIVEN** una lista con dos contenedores
- **WHEN** el usuario escribe el nombre de uno en la búsqueda
- **THEN** solo se muestra ese contenedor

#### Scenario: Acción de reinicio

- **GIVEN** un contenedor visible
- **WHEN** el usuario pulsa "Reiniciar"
- **THEN** se llama `apiFetch` con `/api/containers/<name>/restart` y método `POST`

#### Scenario: Confirmación de borrado

- **GIVEN** un contenedor visible
- **WHEN** el usuario pulsa "Eliminar" y confirma
- **THEN** se llama `apiFetch` con `/api/containers/<name>/remove` y método `POST`

### Requirement: ContainerRow, ContainerTable e InspectModal SHALL cubrir su renderizado

- `ContainerRow` MUST cubrir nombre, estado, badge de actualización, expansión y callbacks de acciones. La imagen se muestra en `InspectModal` (`containerInfo.image`), no en la fila.
- `ContainerTable` MUST cubrir búsqueda, filtros de estado, "check all", agrupación por stack y ocultación de la barra en modo batch.
- `InspectModal` MUST cubrir estados loading, error, vacío y datos, pestañas y cierre.

#### Scenario: InspectModal en error

- **GIVEN** `error = "No se pudo obtener información"`
- **WHEN** se renderiza `InspectModal`
- **THEN** se muestra el mensaje de error y el botón de cierre funciona

### Requirement: La suite SHALL seguir verde

Tras añadir los tests, `npx vitest run` MUST pasar, y `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` MUST seguir limpios.

#### Scenario: Verificación frontend

- **WHEN** se ejecutan los comandos de verificación
- **THEN** todos terminan con código 0
