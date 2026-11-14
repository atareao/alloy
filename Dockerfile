# ═══════════════════════════════════════════════════════════════
# Stage 1: Backend (Rust)
# ═══════════════════════════════════════════════════════════════
FROM docker.io/library/rust:alpine3.23 AS backend-builder

RUN apk add --no-cache --update \
    build-base \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

WORKDIR /build

# Cache dependencies (avoid recompiling every time)
RUN cargo init --bin --name alloy .

COPY backend/Cargo.toml backend/Cargo.lock backend/build.rs ./
RUN cargo build --release && \
    rm -rf src

COPY backend/src ./src
RUN touch src/main.rs && \
    cargo build --release && \
    strip target/release/alloy

# ═══════════════════════════════════════════════════════════════
# Stage 2: Frontend (pnpm)
# ═══════════════════════════════════════════════════════════════
FROM docker.io/library/node:24-alpine AS frontend-builder

WORKDIR /build

# pnpm no-interactivo: nunca espera confirmación por TTY
ENV CI=true

# Versión de pnpm derivada de `packageManager` (fuente única: frontend/package.json),
# sin pin duplicado en el Dockerfile. Se ignora cualquier sufijo de integridad (`+sha512…`).
COPY frontend/package.json ./
RUN PNPM_VERSION=$(node -p 'require("./package.json").packageManager.replace(/^pnpm@/, "").split("+")[0]') \
    && case "$PNPM_VERSION" in ""|*[!0-9A-Za-z.+-]*) echo "packageManager debe declarar una version semver de pnpm; valor: '$PNPM_VERSION'" >&2; exit 1;; esac \
    && npm install -g "pnpm@$PNPM_VERSION"

COPY frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY frontend/ ./
RUN pnpm run build

# ═══════════════════════════════════════════════════════════════
# Stage 3: Runtime
# ═══════════════════════════════════════════════════════════════
FROM alpine:3.23

ARG UID=1000
ARG GID=1000

RUN apk add --no-cache \
    ca-certificates \
    su-exec \
    && addgroup -g "$GID" app \
    && adduser -D -h /app -u "$UID" -G app app

WORKDIR /app
COPY --from=backend-builder /build/target/release/alloy .
COPY --from=frontend-builder /build/dist ./dist

RUN mkdir -p /app/data && chown -R app:app /app

COPY --chmod=755 docker/entrypoint.sh /usr/local/bin/entrypoint.sh
ENV PUID=${UID} PGID=${GID}
USER root
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
EXPOSE 3066
CMD ["./alloy"]
