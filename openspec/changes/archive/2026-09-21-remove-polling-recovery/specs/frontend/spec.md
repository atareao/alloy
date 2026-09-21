# Spec: Remove polling, add page-reload recovery

## REMOVED Requirements

### Requirement: Frontend SHALL poll /api/check-progress every 2s during batch

The frontend SHALL NOT poll `GET /api/check-progress` every 2 seconds during active batch operations. SSE auto-reconnect and the broadcast channel buffer make this polling redundant.

**Contracts (removed):**
```typescript
// ELIMINADO: Polling fallback for progress
useEffect(() => {
    if (!authenticated || batchPhaseRef.current !== "active") return;
    const interval = setInterval(async () => {
        const res = await fetch("/api/check-progress", { credentials: "include" });
        // ... update progress map, detect __batch__ sentinel
    }, 2000);
    return () => clearInterval(interval);
}, [authenticated, batchPhase]);
```

#### Scenario: No polling requests during batch
- **Given** una operación batch activa
- **When** el frontend está en `batchPhase = "active"`
- **Then** NO SHALL hacer peticiones periódicas a `/api/check-progress`
- **And** el progreso SHALL llegar exclusivamente vía SSE `/api/updates`

## ADDED Requirements

### Requirement: Frontend SHALL recover batch progress on page reload

On mount, the frontend SHALL call `GET /api/check-progress` **once**. If there are entries with `done === false`, it SHALL set `batchPhase = "active"` and populate the progress map so the `BatchProgress` card appears.

**Contracts:**
```typescript
// AÑADIDO: Recovery on page reload
useEffect(() => {
    if (!authenticated) return;
    fetch("/api/check-progress", { credentials: "include" })
      .then((res) => (res.ok ? res.json() : null))
      .then((data: Record<string, UpdateProgress> | null) => {
        if (!data) return;
        const entries = Object.values(data);
        const hasActive = entries.some((e) => !e.done);
        if (!hasActive) return;
        setProgress((prev) => {
          const next = new Map(prev);
          for (const entry of entries) next.set(entry.container, entry);
          return next;
        });
        setBatchPhase("active");
        const best = entries.reduce((a, b) => (a.checked > b.checked ? a : b));
        if (best.total > 0) {
          setBatchProgress({ current: best.checked, total: best.total });
        }
      })
      .catch(() => {});
  }, [authenticated]);
```

#### Scenario: Recarga durante actualización activa
- **Given** el usuario inició un "Check All" que está actualizando contenedores
- **When** recarga la página (F5)
- **Then** al montar, se llama `GET /api/check-progress` una vez
- **And** si hay entries con `done === false`, se setea `batchPhase = "active"`
- **And** se muestra el `BatchProgress` card con el progreso actual
- **And** el SSE `/api/updates` reconecta y recibe eventos posteriores

#### Scenario: Recarga sin actualización en curso
- **Given** no hay ninguna operación batch activa
- **When** el usuario recarga la página
- **Then** `GET /api/check-progress` devuelve `{}` vacío
- **And** `batchPhase` permanece en `"idle"`
- **And** no se muestra ningún progress card

#### Scenario: Actualización ya completada al recargar
- **Given** el batch terminó pero la página se recarga antes de ver el resultado
- **When** se monta la página
- **Then** `GET /api/check-progress` devuelve entries con `done === true`
- **And** `hasActive` es `false`
- **And** no se activa el batchPhase
- **And** el state worker (SSE `/api/events`) ya tiene los contenedores actualizados

#### Scenario: Error de red al recuperar progreso
- **Given** hay una actualización activa
- **When** la página se recarga y `GET /api/check-progress` falla (red)
- **Then** el `.catch()` ignora el error silenciosamente
- **And** `batchPhase` permanece en `"idle"`
- **And** el SSE `/api/updates` reconecta y recibe eventos posteriores
- **And** el state worker refrescará los contenedores en ~30s