# Spec Delta: SSE TCP_NODELAY per-connection

## ADDED Requirements

### Requirement: El servidor SHALL aplicar TCP_NODELAY a cada conexión TCP aceptada

El servidor HTTP DEBE deshabilitar el algoritmo de Nagle (`TCP_NODELAY = true`) en
cada socket de conexión individual aceptado por el listener, no solo en el socket
de escucha, para garantizar que los chunks de streaming (SSE) se envíen sin
retraso de agrupación del kernel.

#### Scenario: Nueva conexión de cliente SSE

- **GIVEN** el servidor Alloy está escuchando en el puerto configurado
- **WHEN** un cliente abre una conexión (p.ej. `GET /api/events`, `GET /api/stream`)
- **THEN** el `TcpStream` aceptado para esa conexión tiene `TCP_NODELAY` habilitado
- **AND** los eventos emitidos por el `broadcast::Sender` correspondiente se escriben
  en el socket sin esperar a que el buffer de Nagle se llene o expire el timer

#### Scenario: Fallo al establecer TCP_NODELAY en una conexión

- **GIVEN** una conexión aceptada donde `set_nodelay(true)` devuelve `Err`
  (caso raro, p.ej. socket ya cerrado)
- **WHEN** se intenta aplicar la opción
- **THEN** el servidor registra un `tracing::trace!`/`warn!` con el error
- **AND** continúa sirviendo la conexión con el comportamiento por defecto
  (no debe abortar ni hacer panic)

### Requirement: El código de arranque NO SHALL depender de nodelay en el socket de escucha

El código NO DEBE asumir que `set_nodelay` en el socket de escucha (`TcpSocket`)
se propaga a las conexiones aceptadas, ya que esto es falso en Linux/BSD.

#### Scenario: Revisión de código

- **GIVEN** el bloque de arranque en `main.rs`
- **WHEN** se revisa el manejo de `TCP_NODELAY`
- **THEN** la única llamada relevante a `set_nodelay(true)` que afecta el
  comportamiento de streaming está en el `tap_io` aplicado sobre el listener
  ya convertido (`TcpListener` de tokio), no sobre el `TcpSocket` previo al bind
