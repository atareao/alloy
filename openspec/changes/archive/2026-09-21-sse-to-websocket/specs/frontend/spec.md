# Spec Delta: Frontend WebSocket hook

## ADDED Requirements

### Requirement: El frontend SHALL tener un hook `useWS` que reemplaza `useSSE`

El frontend DEBE proporcionar un hook `useWS(path, eventType, onMessage, onError)`
que:
1. Obtiene el token de sesión vía `GET /api/auth/sse-token` (endpoint existente)
2. Conecta vía `new WebSocket(\`wss://.../api/ws?token=\${token}\`)`
3. Escucha mensajes JSON, filtra por `eventType` (campo `type`), y llama a
   `onMessage` con el contenido de `data`
4. Reintenta con backoff exponencial en caso de error/cierre (misma lógica
   que `useSSE.ts`)
5. Renueva el token si la conexión falla por autenticación

#### Scenario: Componente se monta con WebSocket

- **GIVEN** un componente que usa `useWS("/api/ws", "containers", handler)`
- **WHEN** se monta el componente
- **THEN** se solicita el token vía `GET /api/auth/sse-token`
- **AND** se abre `new WebSocket("wss://dominio/api/ws?token=<token>")`
- **AND** los mensajes con `type: "containers"` se pasan al handler

#### Scenario: Componente se desmonta

- **GIVEN** un componente activo con conexión WebSocket
- **WHEN** el componente se desmonta
- **THEN** se cierra el WebSocket
- **AND** se limpian los recursos

#### Scenario: Conexión WebSocket se cierra inesperadamente

- **GIVEN** una conexión WebSocket activa
- **WHEN** la conexión se cierra (error o cierre del servidor)
- **THEN** se reintenta con backoff exponencial (1s, 2s, 4s, ... hasta 30s)
- **AND** si el error fue 401, se renueva el token antes de reconectar

### Requirement: App.tsx y DashboardPage.tsx SHALL usar useWS

Los componentes que actualmente crean `EventSource` directo DEBEN migrar a
usar `useWS` con los tipos de evento correspondientes.

#### Scenario: App.tsx recibe eventos de notificaciones

- **GIVEN** App.tsx montado
- **WHEN** se recibe un mensaje WebSocket con `type: "notification"`
- **THEN** se muestra el toast de notificación (mismo comportamiento que hoy)

#### Scenario: DashboardPage.tsx recibe eventos de contenedores

- **GIVEN** DashboardPage.tsx montado
- **WHEN** se recibe un mensaje WebSocket con `type: "containers"`
- **THEN** se actualiza la lista de contenedores (mismo comportamiento que hoy)

#### Scenario: DashboardPage.tsx recibe eventos de progreso

- **GIVEN** DashboardPage.tsx montado
- **WHEN** se recibe un mensaje WebSocket con `type: "update-progress"`
- **THEN** se actualiza la barra de progreso (mismo comportamiento que hoy)