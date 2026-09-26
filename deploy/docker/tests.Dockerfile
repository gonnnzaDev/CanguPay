# syntax=docker/dockerfile:1.7
# Imagen de pruebas: Rust + Python + Node en un solo sitio.
#
# Cubre la bateria completa que hoy vive en scripts/test_roles_workflow.sh:
#   - reglas del motor y canonicalizacion (pytest)
#   - manifiesto de fixtures y reglas de evidencia sintetica
#   - contrato: roles y workflow (cargo test)
#   - agente Rust, incluidos los golden
#   - frontend: typecheck, lint y tests (node)
#
# No es una imagen de produccion: lleva toolchain completo a proposito.

FROM rust:1-bookworm AS base
WORKDIR /src

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      gnupg \
      git \
      python3 \
      python3-pip \
      python3-venv \
      unzip \
 && curl -fsSL https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal \
 && . "$HOME/.cargo/env" \
 && rustup target add wasm32v1-none \
 && rm -rf /var/lib/apt/lists/*

# Node para las pruebas del frontend.
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

# pytest vive en un virtualenv para no ensuciar el sistema.
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Fixtures primero: si el manifiesto cambia, solo se invalida lo de Python.
COPY scripts/verify_fixtures.py /src/scripts/verify_fixtures.py
COPY services/attestation-agent/ /src/services/attestation-agent/
RUN pip install --no-cache-dir pytest

# Manifiestos de Rust para cachear la resolucion de dependencias.
COPY Cargo.toml Cargo.lock /src/
COPY contracts/ /src/contracts/

# El codigo de las pruebas de Python.
COPY services/attestation-agent/tests /src/services/attestation-agent/tests

ENV PYTHONPATH=/src/services/attestation-agent
ENV PYTHONDONTWRITEBYTECODE=1

# Verificacion de humo al construir: si la imagen no puede ni listar tests, no
# tiene sentido publicarla.
RUN python3 -m pytest services/attestation-agent/tests --collect-only -q >/dev/null \
 && cargo metadata --no-deps --format-version 1 >/dev/null

CMD ["bash"]
