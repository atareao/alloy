# Design

## Contexto

Hook actual en `.git/hooks/pre-commit` (no versionado). El contador:

```bash
INCOMPLETE=$(grep -c '\[ \]' "${change_dir}tasks.md" 2>/dev/null || echo 0)
```

`grep -c` con 0 coincidencias imprime `0` **y** sale con código 1 → `|| echo 0` añade otro `0` →
`INCOMPLETE="0\n0"` → `[ "0\n0" -gt 0 ]` → `[: 0\n0: se esperaba un entero`.

## Decisiones

### D1 — Corrección del contador

```bash
INCOMPLETE=$(grep -c '\[ \]' "${change_dir}tasks.md" 2>/dev/null || true)
INCOMPLETE=${INCOMPLETE:-0}
```

`|| true` evita el `echo 0` espurio (grep ya imprime su conteo) y `${VAR:-0}` cubre el caso de que
`grep` no imprima nada (fichero ilegible). El resultado es siempre un entero.

### D2 — Versionar el hook en `.githooks/`

`.git/hooks/` no se versiona. Se crea `.githooks/pre-commit` como copia canónica y se instala con
`git config core.hooksPath .githooks` (receta `just install-hooks`). `core.hooksPath` es config
**local** del repo (no se commitea), por eso el instalador es explícito.

### D3 — Sin cambios de comportamiento

Se preserva el contrato del hook (bloquea source staged sin proposal; el aviso de tareas es
informativo, no bloqueante). No se fusiona con `openspec/require-proposal.sh` para no alterar la
semántica de ninguno de los dos (la deduplicación queda como follow-up).

## Fuera de alcance

- Unificar el hook con `openspec/require-proposal.sh` / `validate-workflow.sh`.
- Hooks adicionales (pre-push, commit-msg).
