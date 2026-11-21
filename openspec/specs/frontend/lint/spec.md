# frontend/lint Specification

## Purpose
TBD - created by archiving change lint-set-state-in-effect. Update Purpose after archive.

## Requirements

### Requirement: El lint SHALL quedar sin warnings

`pnpm run lint` MUST terminar con código 0 y MUST NOT emitir ningún warning. La regla `react(set-state-in-effect)` MUST NOT dispararse en `TelegramCard.tsx`, `ValetCard.tsx`, `MatrixCard.tsx`, `DashboardPage.tsx` ni `App.tsx`.

#### Scenario: Lint limpio

- **GIVEN** el frontend con los cambios aplicados
- **WHEN** se ejecuta `pnpm run lint`
- **THEN** termina con código 0
- **AND** no emite ningún warning

### Requirement: El script de lint SHALL fallar con warnings

El script `lint` de `frontend/package.json` MUST ejecutar `oxlint --deny-warnings`, de forma que cualquier warning futura haga fallar el comando.

#### Scenario: Warning futura rompe el lint

- **GIVEN** el script `lint` con `--deny-warnings`
- **WHEN** aparece un warning nuevo de cualquier regla
- **THEN** `pnpm run lint` termina con código distinto de 0

### Requirement: La sincronización desde props SHALL preservar el comportamiento

La sustitución del `useEffect` por el patrón de ajuste durante el render MUST preservar el comportamiento observable de los tres `*Card` y del modal de logs:

- Al recibir `configProp`, los campos de formulario MUST precargarse con los valores de la configuración.
- El token MUST vaciarse al recibir una nueva configuración (incluido tras guardar).
- El switch MUST activarse si la configuración parcial indica que el canal está configurado.
- El modal de logs MUST abrir un `EventSource` con la URL del contenedor seleccionado y resetear logs, error y timeout al abrirlo.

#### Scenario: Precarga desde la configuración

- **GIVEN** un `*Card` con `configProp` no nulo
- **WHEN** se renderiza
- **THEN** los campos muestran los valores de `configProp`
- **AND** el token queda vacío

#### Scenario: Switch activado por configuración parcial

- **GIVEN** una configuración con `*_configured` o `*_token_set` o el campo identificativo presente
- **WHEN** se renderiza el `*Card`
- **THEN** el switch aparece activado

#### Scenario: Logs del contenedor

- **GIVEN** el dashboard con un contenedor seleccionado para logs
- **WHEN** se abre el modal de logs
- **THEN** se crea un `EventSource` con `/api/containers/<nombre>/logs`
- **AND** los logs, el error y el timeout se resetean

### Requirement: El falso positivo asíncrono SHALL documentarse

El warning de `App.tsx` sobre `fetchCheckConfig` MUST documentarse con un comentario justificativo y un `oxlint-disable-next-line react/set-state-in-effect`, porque el `setState` ocurre después de `await` y no de forma síncrona en el efecto.

#### Scenario: Disable justificado

- **GIVEN** el `useEffect` que invoca `fetchCheckConfig`
- **WHEN** se revisa el código
- **THEN** existe un `oxlint-disable-next-line react/set-state-in-effect` con un comentario que explica que el `setState` es posterior a `await`

### Requirement: La suite SHALL seguir verde

Tras el refactor, `pnpm run test`, `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` MUST terminar con código 0, y el recuento de tests MUST ser mayor o igual a 190.

#### Scenario: Verificación completa

- **GIVEN** los cambios aplicados
- **WHEN** se ejecutan `pnpm run test`, `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build`
- **THEN** los cuatro terminan con código 0
- **AND** hay al menos 190 tests
