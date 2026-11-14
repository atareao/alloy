# Design

## Contexto

`su-exec` fija los grupos suplementarios a `{gid}` (verificado: `su-exec 1000:1000 id -G` → `1000`),
de modo que los grupos de `--group-add` se pierden. Antes de #83 la imagen no tenía entrypoint
(`USER app`), y ahí Docker aplicaba `group_add` al proceso directamente.

## Decisiones

### D1 — Retirar el entrypoint en vez de arreglarlo

Alternativas evaluadas: (a) sustituir `su-exec` por `setpriv` conservando los grupos añadidos;
(b) retirar el entrypoint y volver a `USER app`. Se elige **(b)** porque la única razón del
entrypoint era el reajuste de identidad en runtime, y esa flexibilidad ya está cubierta por
`ARG UID/GID` en build y por `UserNS=keep-id`; mantener un proceso que arranca como `root` es
superficie de error innecesaria. Menos código, menos modos de fallo.

### D2 — Se conserva la identidad configurable en build

`ARG UID=1000` / `ARG GID=1000` se mantienen: es la vía documentada para hosts cuyo usuario no es
1000, sin coste en runtime y sin privilegios.

### D3 — `USER app` restaura el comportamiento correcto de `group_add`

Sin entrypoint, el runtime aplica al proceso los grupos suplementarios del contenedor. El Quadlet
(`GroupAdd=136`) y los compose (`group_add`) vuelven a funcionar sin workarounds.

## Fuera de alcance

- Reintroducir configurabilidad de identidad en runtime (explícitamente descartada).
