# frontend/test-isolation Specification

## Purpose

Definir el contrato de aislamiento de la suite de tests del frontend: cada test MUST dejar el DOM
y el scheduler de React limpios, de forma que el teardown del entorno jsdom nunca provoque errores
no gestionados y la suite no sea intermitente.

## ADDED Requirements

### Requirement: Aislamiento del holder estático de antd

Cuando un test usa la API estática de antd (`notification` / `message` importados como función), el
holder que antd monta en `document.body` MUST quedar cerrado y desmontado al terminar el test, de
forma que el test siguiente empiece sin nodos `.ant-notification` ni `.ant-message` residuales.

#### Scenario: La notificación estática no sobrevive al test

- **Given** un test que renderiza la aplicación y dispara el flujo que llama a `notification.success(...)`
- **When** el test termina y comienza el siguiente test del mismo fichero
- **Then** `document.querySelector(".ant-notification")` es `null`
- **And** el número de hijos de `document.body` vuelve al valor previo a ese test

#### Scenario: Cierre idempotente

- **Given** un test que no ha creado ninguna notificación
- **When** termina el test y se ejecuta el cierre global
- **Then** no se lanza ninguna excepción y el DOM no queda alterado

### Requirement: Sin trabajo pendiente del scheduler de React al terminar el test

Al terminar cada test el scheduler de React MUST NOT tener tareas pendientes. El cierre global MUST
drenar tanto microtareas como macrotareas (donde React encola el *flush* de *passive effects*), de
forma que ningún callback de React se ejecute después de que el entorno jsdom sea destruido.

#### Scenario: Drenado de macrotareas

- **Given** un callback encolado como macrotarea (p. ej. vía `setImmediate`)
- **When** se invoca el helper de drenado de la suite
- **Then** el callback ya se ha ejecutado al regresar el helper

#### Scenario: Drenado del trabajo de React encolado fuera de `act`

- **Given** una actualización de estado disparada fuera de `act` (como hacen los eventos del stream SSE en los tests)
- **When** se invoca el helper de drenado de la suite
- **Then** el DOM ya refleja la actualización al regresar el helper

#### Scenario: Teardown sin errores no gestionados

- **Given** la suite completa ejecutada con un único worker
- **When** vitest destruye el entorno de cada fichero de test
- **Then** la ejecución termina con exit 0 y sin `Unhandled Errors`
- **And** no aparece ningún `ReferenceError: window is not defined`
