# Spec Delta

## Purpose

Tarjeta de progreso para operaciones por lotes (check/update de contenedores) que muestra barra de progreso, contador con contenedor y acción actual, y resumen en vivo de resultados.

## ADDED Requirements

### Requirement: Barra de progreso con porcentaje real
El sistema SHALL mostrar una barra de progreso con el porcentaje calculado como `(current / total) * 100`, donde `current` es el número de contenedores procesados (done=true) y `total` es el número total de contenedores en el lote.

#### Scenario: Progreso avanza al completar cada contenedor
- **WHEN** un contenedor termina su proceso (check o update) con `done: true`
- **THEN** el contador `current` se incrementa en 1
- **AND** la barra de progreso refleja el nuevo porcentaje

### Requirement: Contador con contenedor y acción actual
El sistema SHALL mostrar un texto en formato `{current} / {total} — {status}` donde `status` es el texto del primer contenedor en progreso (primer entry del Map con `done: false`), o `"✅ Completado"` si todos han terminado, o `"iniciando..."` si no hay datos.

#### Scenario: Muestra contenedor siendo verificado
- **WHEN** hay contenedores en progreso durante la fase checking
- **THEN** el texto muestra `X / N — 🔍 Verificando <container>:<tag>...`

#### Scenario: Muestra contenedor siendo actualizado
- **WHEN** hay contenedores en progreso durante la fase updating
- **THEN** el texto muestra `X / N — 🔄 actualizando <container>...`

#### Scenario: Muestra "Completado" al terminar
- **WHEN** todos los contenedores tienen `done: true`
- **THEN** el texto muestra `N / N — ✅ Completado`

### Requirement: Resumen en vivo de resultados
El sistema SHALL mostrar un resumen debajo de la barra con: total de contenedores, revisados (done=true), actualizados (done=true, sin error, status contiene "actualizado"/"pulled"/"Updated"), errores (error != null o status empieza con ❌/⚠️), y pendientes (done=false).

#### Scenario: Resumen se actualiza en tiempo real
- **WHEN** un contenedor cambia su estado en el progress Map
- **THEN** los contadores del resumen se actualizan en el mismo render

#### Scenario: Errores se contabilizan en revisados y errores
- **WHEN** un contenedor termina con error
- **THEN** aparece en el contador de revisados
- **AND** aparece en el contador de errores
- **AND** NO aparece en el contador de actualizados

### Requirement: Sin lista scrollable de entradas individuales
El sistema SHALL NOT mostrar una lista scrollable con entradas individuales de cada contenedor. Toda la información de progreso se muestra exclusivamente mediante la barra de progreso, el contador con acción actual, y el resumen en vivo.

#### Scenario: No hay lista de entradas
- **WHEN** se renderiza el componente BatchProgress
- **THEN** no hay ningún elemento con `overflow: auto` o `maxHeight` para listar entradas individuales

### Requirement: Total siempre refleja todos los contenedores
El sistema SHALL usar `batchProgress.total` como total en todas las fases (checking y updating), sin cambiar a `checkResults.updated` durante la fase updating.

#### Scenario: Total no cambia entre fases
- **WHEN** la fase cambia de checking a updating
- **THEN** el total en la barra de progreso y el resumen se mantiene igual