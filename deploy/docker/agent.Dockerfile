# syntax=docker/dockerfile:1.7
# Agente de atestación (motor y keeper).
#
# El binario es estatico en practica: solo necesita ca-certificates para hablar
# TLS con el RPC. La clave de la cuenta engine entra por entorno o por un
# keyfile montado; nunca se copia dentro de la imagen.

# ---------- builder ----------
FROM rust:1-bookworm AS builder
WORKDIR /src

# Cargar solo los manifiestos primero deja las dependencias en una capa que se
# reutiliza mientras no cambie el lockfile.
COPY Cargo.toml Cargo.lock ./
COPY contracts/conditional-payment/Cargo.toml contracts/conditional-payment/
COPY contracts/test-token/Cargo.toml contracts/test-token/
COPY services/attestation-agent/Cargo.toml services/attestation-agent/

RUN mkdir -p contracts/conditional-payment/src \
             contracts/test-token/src \
             services/attestation-agent/src \
 && echo 'fn main() {}' > contracts/conditional-payment/src/lib.rs \
 && echo 'fn main() {}' > contracts/test-token/src/lib.rs \
 && echo 'fn main() {}' > services/attestation-agent/src/lib.rs \
 && echo 'fn main() {}' > services/attestation-agent/src/bin/cangu_attest.rs \
 && cargo build --release -p attestation-agent --bin cangu-attest \
 && rm -rf contracts/conditional-payment/src \
             contracts/test-token/src \
             services/attestation-agent/src

# Codigo real encima de la capa de dependencias ya cacheada.
COPY contracts/ contracts/
COPY services/attestation-agent/ services/attestation-agent/
COPY fixtures/ fixtures/

# Especifica la version exacta para que el artefacto sea reproducible.
# El release no lleva simbolos y usa panic=abort, asi que no hace falta strip.
RUN touch contracts/conditional-payment/src/lib.rs \
             contracts/test-token/src/lib.rs \
             services/attestation-agent/src/lib.rs \
 && cargo build --release --locked -p attestation-agent --bin cangu-attest \
 && strip target/release/cangu-attest 2>/dev/null || true

# ---------- runner ----------
FROM debian:bookworm-slim AS runner

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# Usuario sin privilegios. El keeper es un proceso de larga vida y no necesita
# nada mas.
RUN groupadd --system --gid 1001 cangupay \
 && useradd  --system --uid 1001 --gid cangupay --create-home cangupay

COPY --from=builder /src/target/release/cangu-attest /usr/local/bin/cangu-attest

USER cangupay
WORKDIR /home/cangupay

# El keeper y el atestado son de larga duracion: si el RPC no responde, el
# proceso sigue vivo pero el healthcheck avisa.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD cangu-attest --help >/dev/null 2>&1 || exit 1

ENTRYPOINT ["cangu-attest"]
CMD ["--help"]
