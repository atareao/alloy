#!/usr/bin/env sh
# Verificación de identidad del contenedor Alloy y de la propagación de grupos
# suplementarios (--group-add / group_add / GroupAdd).
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DOCKERFILE="$ROOT/Dockerfile"
SCRIPT="$ROOT/docker/entrypoint.sh"
PROBE="${TMPDIR:-/tmp}/alloy-identity-probe.Dockerfile"
IMAGE="alloy-identity-probe"
GID_PROBE="${GID_PROBE:-999}"

if [ "$GID_PROBE" = "1000" ]; then
  echo "GID_PROBE no puede ser 1000 (es el GID propio de app; el test sería un falso positivo)" >&2
  exit 2
fi

fails=0
ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; fails=$((fails+1)); }

# --- stage de runtime del Dockerfile (de "FROM alpine:3.23" al final) ---
RUNTIME=$(awk '/^FROM alpine:3\.23$/{f=1} f' "$DOCKERFILE")

echo "== Estructura del Dockerfile (stage runtime) =="
printf '%s\n' "$RUNTIME" | grep -q '^USER app$' && ok "USER app presente" || fail "falta 'USER app'"
printf '%s\n' "$RUNTIME" | grep -q 'ENTRYPOINT' && fail "ENTRYPOINT presente" || ok "sin ENTRYPOINT"
printf '%s\n' "$RUNTIME" | grep -q 'su-exec' && fail "su-exec presente" || ok "sin su-exec"
printf '%s\n' "$RUNTIME" | grep -qE 'ENV +PUID|ENV +PGID|PUID=|PGID=' && fail "ENV PUID/PGID presente" || ok "sin ENV PUID/PGID"
printf '%s\n' "$RUNTIME" | grep -q 'entrypoint.sh' && fail "referencia a entrypoint.sh" || ok "sin referencia a entrypoint.sh"

echo "== Ficheros y docs =="
[ -e "$SCRIPT" ] && fail "docker/entrypoint.sh existe" || ok "docker/entrypoint.sh no existe"
for f in README.md alloy.container.example; do
  if grep -qE 'su-exec|PUID|PGID' "$ROOT/$f"; then fail "$f menciona su-exec/PUID/PGID"; else ok "$f limpio"; fi
done
# El doc explica el mecanismo retirado y puede nombrarlo, pero no debe ofrecer PUID/PGID como API.
if grep -qE 'PUID|PGID' "$ROOT/docs/container-identity.md"; then fail "docs/container-identity.md menciona PUID/PGID"; else ok "docs/container-identity.md limpio"; fi

echo "== Funcional: grupos suplementarios y acceso al socket simulado =="
awk '/^FROM alpine:3\.23$/{f=1} f' "$DOCKERFILE" > "$PROBE"
sed -i 's#^COPY --from=backend-builder .*#RUN mkdir -p /app/dist \&\& touch /app/alloy#' "$PROBE"
sed -i '/^COPY --from=frontend-builder /d' "$PROBE"
# El probe se crea como root (necesario para chgrp) y vuelve a `USER app` para reproducir el runtime.
printf 'USER root\nRUN touch /tmp/mock.sock && chgrp %s /tmp/mock.sock && chmod 660 /tmp/mock.sock\nUSER app\n' "$GID_PROBE" >> "$PROBE"

if podman build -q -f "$PROBE" -t "$IMAGE" "$ROOT" >/dev/null 2>&1; then
  OUT=$(podman run --rm --group-add "$GID_PROBE" "$IMAGE" sh -c \
    'echo GROUPS=$(id -G); echo UID=$(id -u); cat /tmp/mock.sock >/dev/null 2>&1 && echo SOCKET_OK || echo SOCKET_DENIED')
  echo "$OUT" | sed 's/^/    /'
  echo "$OUT" | grep -qE "GROUPS=.*\b$GID_PROBE\b" && ok "gid $GID_PROBE en grupos" || fail "gid $GID_PROBE ausente de los grupos"
  echo "$OUT" | grep -q 'SOCKET_OK' && ok "socket simulado accesible" || fail "socket simulado NO accesible"
  echo "$OUT" | grep -q 'UID=0$' && fail "proceso corre como root" || ok "proceso no es root"
else
  fail "no se pudo construir el probe"
fi

echo
if [ "$fails" -eq 0 ]; then echo "OK: identidad del contenedor correcta"; exit 0; fi
echo "FALLOS: $fails"; exit 1
