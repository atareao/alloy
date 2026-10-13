# Spec Delta

## ADDED Requirements

### Requirement: El fan-out no retiene el lock de ajustes

`notify_all_state` MUST leer los ajustes una sola vez y MUST liberar el `Mutex<Settings>`
antes de iniciar las llamadas de red de los canales. Mientras un fan-out está en curso,
`GET/PUT /api/config` y el endpoint de test MUST poder adquirir el lock sin esperar a que
termine el envío.

#### Scenario: La config no se bloquea durante el fan-out

- **Given** un fan-out de notificación en curso (una llamada de canal en vuelo)
- **When** se intenta adquirir el lock de `Settings`
- **Then** la adquisición tiene éxito sin esperar a que termine el canal
