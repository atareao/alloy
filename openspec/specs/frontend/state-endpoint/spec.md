# frontend/state-endpoint Specification

## Purpose

Capability del frontend que consume el endpoint unificado de estado `GET /api/state`
a través del hook `useStatePoll`, recibiendo un `StateResponse` con contenedores,
resumen (`ContainerSummary`) y progreso en una sola respuesta. Sustituye el polling
periódico de `/api/check-progress` por un flujo reactivo basado en el estado recibido.

## Requirements

### Requirement: `useStatePoll` SHALL entregar el `StateResponse` completo

El hook `useStatePoll` SHALL exponer la callback `onState: (StateResponse) => void` y
parsear la respuesta como `StateResponse` (con `containers`, `summary` y `progress`),
en lugar de como `ContainerInfo[]`.

#### Scenario: Estado y progreso en una sola respuesta

- **GIVEN** el frontend llama a `useStatePoll`
- **WHEN** llega la respuesta
- **THEN** se invoca `onState` con un `StateResponse` que contiene containers, summary y progress
- **AND** se actualiza el mapa de progreso desde `state.progress`
- **AND** se inicia inmediatamente una nueva petición

### Requirement: `App.tsx` SHALL consumir summary y progress del estado recibido

`App.tsx` SHALL actualizar contenedores, resumen y progreso desde el `StateResponse`
recibido, SHALL usar `state.progress` en lugar del polling periódico, y SHALL detectar
la finalización de un batch mediante la entrada `__batch__` en `state.progress`.

#### Scenario: Batch completado detectado vía estado

- **GIVEN** el frontend recibe un `StateResponse`
- **WHEN** `state.progress` contiene `__batch__` con `done: true`
- **THEN** la fase del batch pasa a `"idle"`
- **AND** se muestra el diálogo de resumen
- **AND** se refrescan el historial y la configuración

### Requirement: `DashboardPage.tsx` SHALL usar el `summary` recibido como prop

`DashboardPage.tsx` SHALL aceptar la prop `summary: ContainerSummary` y usar
directamente `summary.running`, `summary.stopped` y `summary.with_updates` en lugar
de calcular las estadísticas localmente.

#### Scenario: Estadísticas del dashboard desde el resumen

- **GIVEN** `DashboardPage` recibe la prop `summary`
- **WHEN** renderiza las estadísticas
- **THEN** usa `summary.running`, `summary.stopped` y `summary.with_updates` directamente
- **AND** no realiza el cálculo local de `statsRunning`, `statsStopped` ni `statsUpdates`

### Requirement: El polling SHALL reintentar con backoff exponencial ante fallo de red

Cuando una petición de `useStatePoll` falla, el hook SHALL esperar con backoff
exponencial (1s, 2s, 4s, ... hasta un máximo de 30s) y, al alcanzar el máximo de
reintentos (10), SHALL invocar `onError`.

#### Scenario: Fallo de red durante el polling

- **GIVEN** el frontend está haciendo polling vía `useStatePoll`
- **WHEN** una petición falla
- **THEN** el hook espera con backoff exponencial (1s, 2s, 4s, ... hasta 30s)
- **WHEN** el backoff alcanza el máximo de reintentos (10)
- **THEN** se invoca `onError`
