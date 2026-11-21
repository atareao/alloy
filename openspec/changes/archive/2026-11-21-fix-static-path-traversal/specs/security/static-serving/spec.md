# security/static-serving Delta

## ADDED Requirements

### Requirement: Servido de estáticos confinado a `dist/`

El handler de estáticos (`frontend_handler`) MUST servir únicamente ficheros que, una vez resueltos, estén
**dentro** del directorio `dist/`. MUST resolver la ruta de forma segura (canonicalizando y comprobando que
el resultado queda bajo `dist/`) y MUST rechazar cualquier petición cuyo camino intente salir de `dist/`
(segmentos `..`, rutas absolutas o sus equivalentes percent-encoded). Las peticiones cuya ruta no
corresponda a un fichero existente dentro de `dist/` MUST caer al *fallback* SPA (servir `dist/index.html`),
preservando el enrutado del cliente.

#### Scenario: Sirve un asset existente dentro de dist/

- **Given** el fichero `dist/assets/app.js` existe
- **When** el cliente pide `/assets/app.js`
- **Then** se sirve ese fichero con su `content-type`

#### Scenario: La raíz sirve el index

- **Given** el fichero `dist/index.html` existe
- **When** el cliente pide `/`
- **Then** se sirve `dist/index.html`

#### Scenario: Un path traversal con `..` se rechaza

- **Given** un fichero existente **fuera** de `dist/` (p. ej. `../Cargo.toml` respecto a `dist/`)
- **When** el cliente pide `/../Cargo.toml` o `/../../etc/passwd`
- **Then** NO se sirve ese fichero externo
- **And** su contenido no aparece en la respuesta (se aplica el *fallback* SPA)

#### Scenario: Un path traversal percent-encoded se rechaza

- **Given** un fichero existente fuera de `dist/`
- **When** el cliente pide `/%2e%2e/%2e%2e/etc/passwd`
- **Then** NO se sirve ese fichero externo

#### Scenario: Una ruta de cliente desconocida cae al fallback SPA

- **Given** una ruta de cliente sin fichero asociado (p. ej. `/dashboard`)
- **When** el cliente la pide
- **Then** se sirve `dist/index.html` (fallback SPA)
