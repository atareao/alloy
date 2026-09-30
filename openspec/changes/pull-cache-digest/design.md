# Design

## Context

El pipeline de updates persiste en SQLite `last_remote_digest` (config digest de la imagen
que el contenedor **ejecuta**) y `has_update`. Con la política `Pull` (descargar sin
recrear), el pull baja la imagen al caché local pero el contenedor sigue en la imagen
vieja: `last_remote_digest` no avanza y `has_update` permanece `true`. En cada ciclo de
`check_and_apply_all` se vuelve a detectar el update y se repite el pull de la misma
imagen, sin efecto útil.

## Goals / Non-Goals

**Goals:**
- Registrar el digest de la imagen ya descargada al caché (`last_pulled_digest`).
- Omitir el pull cuando el remoto coincide con lo ya cacheado.
- Mantener intacta la semántica de `has_update` y `last_remote_digest`.

**Non-Goals:**
- Cambiar la semántica de `has_update` (sigue indicando imagen en ejecución vs remota).
- Cambios de UI/frontend.
- Recrear automáticamente contenedores con política `Pull`.

## Decisions

### D1: Columna `last_pulled_digest` separada de `last_remote_digest`
Se añade `last_pulled_digest TEXT NOT NULL DEFAULT ''` a `containers`. Representa el
**caché**, no la ejecución. Alternativa descartada: reutilizar `last_remote_digest` — rompe
la semántica "imagen en ejecución" y haría que `has_update` mintiera.

### D2: Skip del pull en la rama `Pull` de `apply_single_policy`
Si `policy.action == Pull` y `p.remote_digest == last_pulled_digest`, se omite la descarga
y se registra un log informativo. Alternativa descartada: filtrar en `check_and_apply_all`
— duplica la lógica de política y no cubre los caminos manuales.

### D3: Persistir `last_pulled_digest` tras pull exitoso
En la rama `Pull`, tras `pull_image` OK, se persiste `last_pulled_digest = remote_digest`.
En `PullRestart`/`PullRestartStack` también se actualiza (el pull ocurrió), aunque ahí
`last_remote_digest` ya avanza y el skip no aplica. Un pull fallido no lo modifica.

### D4: Helper puro `should_skip_pull`
`fn should_skip_pull(action: &UpdateAction, remote: Option<&str>, last_pulled: Option<&str>) -> bool`
aislado para test unitario sin Docker. Devuelve `true` solo si `action == Pull`, ambos
digests presentes y no vacíos, e iguales.

### D5: Migración idempotente
`ALTER TABLE containers ADD COLUMN last_pulled_digest TEXT NOT NULL DEFAULT ''` con el
mismo patrón tolerante a "duplicate column" que `last_remote_digest`.

### D6: El skip es un no-op
Cuando `should_skip_pull` es true, la rama `Pull` NO debe marcar `success = true` (que dispararía notificación "actualizado y reiniciado", entrada de historial y prune). En su lugar registra un log informativo, actualiza el progreso y termina sin efectos secundarios. Alternativa descartada: mantener `success = true` y añadir un flag `skipped` — más complejo y deja el bloque de éxito lleno de condicionales.

## Risks / Trade-offs

- **Contenedor recreado externamente**: si el usuario recrea el contenedor a mano, el
  caché y la ejecución pueden divergir; `last_pulled_digest` solo evita re-descargar, no
  afecta a la detección de updates.
- **Primer ciclo**: `last_pulled_digest` vacío → se hace un pull inicial (comportamiento
  actual). A partir de ahí, sin pulls redundantes.
- **UI**: con `Pull`, el badge "update disponible" seguirá visible hasta que el contenedor
  se recree. Es correcto (la imagen está cacheada pero no en ejecución); fuera de alcance.
