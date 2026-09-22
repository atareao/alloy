# Design

## Solución

En `auth_middleware` (backend/src/auth.rs), después de ejecutar `next.run(req).await` y obtener la respuesta, verificar si el `Content-Type` de la respuesta es `text/event-stream`. Si lo es, saltar toda modificación de headers (sliding session).

### Cambio concreto

```rust
pub async fn auth_middleware(...) -> Result<Response, Response> {
    // ... existing auth check code ...

    let mut response = next.run(req).await;

    // SSE responses must not have their headers modified
    // to avoid buffering the stream
    let is_sse = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("text/event-stream"))
        .unwrap_or(false);

    if is_sse {
        return Ok(response);
    }

    // Sliding session (only for non-SSE responses)
    let now = Utc::now().timestamp() as usize;
    let sliding_threshold = (idle_timeout_secs as usize) / 2;
    if now.saturating_sub(claims.last_active) > sliding_threshold {
        // ... existing sliding session code ...
    }

    Ok(response)
}
```

### Por qué funciona

El `Sse` response de Axum establece `Content-Type: text/event-stream` en los headers de la respuesta. Al detectar este header antes de modificar cualquier otro header, evitamos que el middleware bufferée el stream. La autenticación sigue funcionando porque la verificación de la cookie ocurre antes de ejecutar la request.

### Riesgos

- Ninguno: el cambio es mínimo y localizado. Solo afecta a respuestas SSE.
- Las respuestas SSE no tendrán sliding session, pero esto es aceptable porque las conexiones SSE son largas y el sliding session no es relevante para ellas.