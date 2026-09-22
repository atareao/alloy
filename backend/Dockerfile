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

COPY ./Cargo.toml ./Cargo.lock ./build.rs ./
RUN cargo build --release && \
    rm -rf src

COPY ./src ./src
RUN touch src/main.rs && \
    cargo build --release && \
    strip target/release/alloy

# ═══════════════════════════════════════════════════════════════
# Stage 2: Runtime
# ═══════════════════════════════════════════════════════════════
FROM alpine:3.23

RUN apk add --no-cache ca-certificates && \
    adduser -D -h /app -u 1000 app

WORKDIR /app
COPY --from=backend-builder /build/target/release/alloy .

RUN mkdir -p /app/data && chown -R app:app /app

USER app
EXPOSE 3066
CMD ["./alloy"]
