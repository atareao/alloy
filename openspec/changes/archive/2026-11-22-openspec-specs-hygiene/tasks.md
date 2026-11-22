# Tasks

## 1. Base y pre-flight

- [x] 1.1 `source openspec/validate-workflow.sh` en verde y rama `feature/openspec-specs-hygiene` creada desde `develop`.
- [x] 1.2 Capturar la línea base: `openspec validate --strict --specs` → 2 errores + 38 warnings en 27 specs. Guardar la salida para el diff final.
- [x] 1.3 Inventario reproducible de afectadas (script reproducible con `rg`/`openspec validate`), para no depender de listas a mano.

## 2. Errores duros — specs huérfanas (2)

- [x] 2.1 `openspec/specs/backend/spec.md`: reescribir como capability `backend/state-endpoint` en `openspec/specs/backend/state-endpoint/spec.md` con `# <Capability>`, `## Purpose` real y `## Requirements`.
- [x] 2.2 `backend/state-endpoint`: convertir el contenido actual (`GET /api/state`, long-poll con `state_notify`, envelope `StateResponse`, `compute_summary`, `Cache-Control: no-cache`) en requirements con `SHALL`/`MUST` en el cuerpo y `#### Scenario:` por caso (incluido el timeout de 30 s y el primer arranque con cache vacía).
- [x] 2.3 Eliminar el fichero huérfano `openspec/specs/backend/spec.md` tras mover el contenido.
- [x] 2.4 `openspec/specs/frontend/spec.md`: reescribir como capability `frontend/state-endpoint` en `openspec/specs/frontend/state-endpoint/spec.md`, con el mismo tratamiento (Purpose real + requirements + scenarios).
- [x] 2.5 Eliminar el fichero huérfano `openspec/specs/frontend/spec.md` tras mover el contenido.
- [x] 2.6 Verificar que no queda ningún `spec.md` directamente en la raíz de `openspec/specs/<cap>/` (todos deben vivir en `openspec/specs/<cap>/<capability>/spec.md`).

## 3. Purposes de relleno (24 specs)

- [x] 3.1 `spec/auth`, `spec/auth/middleware`, `spec/auth/session`, `spec/auth/sse-ticket`.
- [x] 3.2 `spec/backend/container-pagination`, `spec/backend/update-history-lock`.
- [x] 3.3 `spec/ci/action-runtimes`, `spec/ci/cache`.
- [x] 3.4 `spec/dev/deploy-co1`, `spec/docs/readme`.
- [x] 3.5 `spec/frontend/container-list`, `spec/frontend/deps`, `spec/frontend/deps-dedupe`, `spec/frontend/lint`, `spec/frontend/sse`, `spec/frontend/test-coverage`.
- [x] 3.6 `spec/security/static-serving`, `spec/sse`.
- [x] 3.7 `spec/ui/config-page`, `spec/ui/dashboard-mobile`, `spec/ui/inspect-modal`, `spec/ui/layout`.
- [x] 3.8 `spec/updates/progress`, `spec/updates/update-health-guard`.
- [x] 3.9 Cada Purpose nuevo: 1–3 frases, específico de la capability, sin la frase de relleno de `openspec archive` y sin `TBD`/`TODO`. Editado **directamente en la spec principal** (no vía delta).

## 4. RFC 2119 — SHALL/MUST en el cuerpo (5 specs, 14 requirements)

- [x] 4.1 `spec/auth` — requirement `El middleware de auth SHALL no modificar respuestas 101 Switching Protocols`.
- [x] 4.2 `spec/sse` — requirements de `TCP_NODELAY`, código de arranque y heartbeat (3).
- [x] 4.3 `spec/ui/dashboard-mobile` — 5 requirements de layout móvil.
- [x] 4.4 `spec/ui/layout` — 3 requirements de navegación/layout.
- [x] 4.5 `spec/updates/digest-comparison` — `needs_update function` y persistencia del digest remoto (2).
- [x] 4.6 Regla: el header `### Requirement: ...` queda como descripción; la obligación `SHALL`/`MUST` pasa a la primera línea del cuerpo del requirement. Sin cambiar el significado normativo.

## 5. Verificación y cierre

- [x] 5.1 `openspec validate --strict --specs` → **0 errores y 0 warnings**.
- [x] 5.2 `openspec list --specs` cuenta las mismas capabilities que antes más `backend/state-endpoint` y `frontend/state-endpoint`; ningún requirement existente pierde su texto normativo (diff revisado).
- [x] 5.3 Auditoría de la spec resultante (contenido normativo no alterado, solo redacción/estructura).
- [x] 5.4 Documentación: `docs/openspec-specs-hygiene.md` y actualización de `HANDOFF.md`.
- [x] 5.5 Plan `plans/PLAN-030.md` referenciando el issue #204; al cerrar, renombrar a `PLAN-030-ARCHIVED.md`.
- [x] 5.6 Commit (gitmoji + conventional) **antes** de archivar; archivar el change y abrir PR a `develop` (merge commit). Cerrar #204 a mano.

## 6. Fuera de alcance (anotado, no se ejecuta aquí)

- Los `ℹ [INFO]` de requirements muy largos (>500 caracteres) no bloquean `--strict`; no se tocan.
- Unificar capabilities solapadas (`sse` vs `realtime/stream`, `frontend/sse` vs `frontend/state-endpoint`): solo se auditan, no se fusionan.
- El otro follow-up pendiente (documentar la trampa del hook SDD en `docs/pre-commit-hook.md`) va en su propio change.
