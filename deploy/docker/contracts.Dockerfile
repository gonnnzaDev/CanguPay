# syntax=docker/dockerfile:1.7
# Contratos Soroban: compila los WASM que se despliegan.
#
# El repo exige `stellar contract build`: compilar con cargo a pelo produce un
# WASM que no coincide con el canonico y que el CLI no quiere subir. Asi que aqui
# se instala el CLI de Stellar y se delega en el.
#
# Coste: instalar stellar-cli con cargo compila una cadena de dependencias muy
# grande. Va en una etapa propia para que se cachee y solo se pague una vez.
# Para builds rapidos en serie, monta un binario ya compilado con
# STELLAR_CLI_FROM_BIN=1 (ver scripts/docker/build.sh).

# ---------- cli: instalar stellar-cli una sola vez ----------
FROM rust:1-bookworm AS cli
ARG STELLAR_CLI_VERSION=25.2.0
RUN cargo install --locked --version "$STELLAR_CLI_VERSION" stellar-cli \
 && cargo install --list | grep -q '^stellar-cli ' \
 && stellar --version

# ---------- builder: compilar los dos contratos ----------
FROM rust:1-bookworm AS builder
WORKDIR /src

COPY --from=cli /usr/local/cargo/bin/stellar /usr/local/bin/stellar

# Para Soroban con SDK 28 el destino es wasm32v1-none, no wasm32-unknown-unknown.
RUN rustup target add wasm32v1-none

COPY Cargo.toml Cargo.lock ./
COPY contracts/ contracts/
COPY fixtures/ fixtures/

# El CLI compila el paquete desde el workspace. --release coincide con el
# perfil del workspace: opt-level z, LTO, panic abort.
RUN stellar contract build --package conditional-payment --release \
 && stellar contract build --package test-token --release

# Comprobar que el contrato cabe en el limite de la red. Un WASM que no cabe
# se despliega bien en local y falla en la red, asi que se avisa aqui.
RUN set -eu; \
    for w in /src/target/wasm32v1-none/release/*.wasm; do \
      printf '%s: %s bytes\n' "$(basename "$w")" "$(stat -c%s "$w")"; \
      test "$(stat -c%s "$w")" -le 65536 \
        || echo "AVISO: $(basename "$w") supera 64 KiB y no sera desplegable"; \
    done

# ---------- runner: solo los artefactos ----------
FROM debian:bookworm-slim AS runner
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates jq \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 1001 --create-home cangupay

WORKDIR /artifacts
COPY --from=builder /src/target/wasm32v1-none/release/*.wasm /artifacts/
COPY --from=builder /src/target/wasm32v1-none/release/*.json /artifacts/

USER cangupay
# Por defecto imprime el hash de cada WASM, que es lo que se compara con el
# que devuelve la red al desplegar.
CMD ["sh", "-c", "for w in /artifacts/*.wasm; do printf '%s  %s\n' \"$(sha256sum \"$w\" | cut -d' ' -f1)\" \"$(basename \"$w\")\"; done"]
