# Proposal

## Why

`notify_all_state` (`backend/src/notifications.rs`) mantiene el `Mutex<Settings>` **durante
todas las llamadas de red** del fan-out (`notify_telegram`, `notify_matrix`,
`notify_webhook`, `notify_valet`), cada una con timeout de cliente de hasta ~30 s. Mientras
tanto, `GET/PUT /api/config` y el endpoint de test se bloquean esperando el mismo lock.

Es una deuda pre-existente (antes con 3 canales), pero añadir Valet la agrava. El fan-out
solo necesita **leer** los ajustes una vez.

## What Changes

- `notify_all_state` clona `Settings` bajo el lock y **suelta el guard** antes de lanzar el
  `tokio::join!` de los cuatro canales.
- Sin cambios de contrato ni de comportamiento visible (salvo que `/api/config` deja de
  bloquearse durante el envío).

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `notifications`: el fan-out no retiene el lock de ajustes durante las llamadas de red.

## Impact

- `backend/src/notifications.rs`: `notify_all_state` (única función a tocar).
- Test que verifique que el lock se libera durante el fan-out (p. ej. intentar
  `settings.try_lock()` desde el canal o un test que compruebe que otro `lock().await`
  progresa mientras un canal duerme).
- Sin cambios en frontend ni en otras capacidades.

## Open Questions

<!-- Ninguna. -->
