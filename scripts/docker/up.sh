#!/usr/bin/env bash
# Levanta la pila segun el destino.
#
#   scripts/docker/up.sh web        frontend contra la red que diga el .env
#   scripts/docker/up.sh web -d     lo mismo, en segundo plano
#   scripts/docker/up.sh localnet   nodo Stellar local
#   scripts/docker/up.sh agent      keeper contra el contrato configurado
#   scripts/docker/up.sh contracts  compila los WASM e imprime sus hashes
#   scripts/docker/up.sh server     pila de servidor (web + keeper)
#   scripts/docker/up.sh down       para todo
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

log()  { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33maviso:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if docker compose version >/dev/null 2>&1; then
  dc() { docker compose "$@"; }
elif command -v docker-compose >/dev/null 2>&1; then
  dc() { docker-compose "$@"; }
else
  die "no hay docker compose"
fi
docker info >/dev/null 2>&1 || die "el daemon de docker no responde"

LOCAL=( -f deploy/compose.local.yml )
SERVER=( -f deploy/compose.server.yml )

# -d / --detach se puede pasar en cualquier posicion.
DETACH=()
for a in "$@"; do
  [[ "$a" == "-d" || "$a" == "--detach" ]] && DETACH=(-d)
done

exigir_env() {
  [[ -f .env ]] || die "falta .env en la raiz. Copia deploy/.env.example y completalo."
}

exigir_agente() {
  exigir_env
  local faltan=()
  for v in CANGUPA_CONTRACT_ID CANGUPA_EXPECTED_AMOUNT CANGUPA_EXPECTED_CURRENCY; do
    # shellcheck disable=SC1091
    grep -qE "^${v}=.+" .env || faltan+=("$v")
  done
  if [[ ${#faltan[@]} -gt 0 ]]; then
    die "faltan en .env: ${faltan[*]}"
  fi
  # La clave de la engine llega de entorno o de un keyfile montado.
  if ! grep -qE '^CANGUPA_ENGINE_SECRET=.+' .env \
     && ! grep -qE '^CANGUPA_ENGINE_KEYFILE=.+' .env; then
    die "falta la clave de la engine: define CANGUPA_ENGINE_SECRET o CANGUPA_ENGINE_KEYFILE en .env"
  fi
}

OBJETO="${1:-help}"
[[ -z "${DETACH[*]}" ]] || shift || true

case "$OBJETO" in
  web)
    exigir_env
    log "levantando el frontend"
    dc "${LOCAL[@]}" up "${DETACH[@]}" --build web
    warn "recuerda: NEXT_PUBLIC_* se compila dentro de la imagen."
    warn "cambiar el contrato en el .env exige reconstruir, no reiniciar."
    log "web en http://localhost:${WEB_PORT:-3000}"
    ;;

  localnet)
    log "levantando el nodo Stellar local (solo desarrollo)"
    dc "${LOCAL[@]}" --profile localnet up "${DETACH[@]}" stellar
    log "  rpc      http://localhost:8000"
    log "  horizon  http://localhost:8080"
    log "  passphrase: Local Sandbox Stellar Network ; September 2022"
    warn "para usar este nodo, ponlo en el .env:"
    warn "  CANGUPA_RPC_URL=http://localhost:8000"
    warn "  CANGUPA_NETWORK=\"Local Sandbox Stellar Network ; September 2022\""
    ;;

  contracts)
    log "compilando los contratos (la primera vez instala stellar-cli y tarda)"
    mkdir -p target/wasm32v1-none/release
    dc "${LOCAL[@]}" --profile build run --rm contracts
    log "artefactos"
    ls -la target/wasm32v1-none/release/*.wasm 2>/dev/null | sed 's/^/  /' \
      || warn "no aparecieron WASM en target/wasm32v1-none/release/"
    ;;

  agent)
    exigir_agente
    log "levantando el keeper"
    dc "${LOCAL[@]}" --profile agent up "${DETACH[@]}" --build keeper
    log "siguiente plano: docker compose -f deploy/compose.local.yml --profile agent logs -f keeper"
    ;;

  server)
    exigir_env
    log "levantando la pila de servidor"
    dc "${SERVER[@]}" up "${DETACH[@]}"
    log "estado"
    dc "${SERVER[@]}" ps
    ;;

  down)
    log "parando todo"
    dc "${LOCAL[@]}" --profile localnet --profile agent --profile build down -v --remove-orphans
    dc "${SERVER[@]}" down --remove-orphans
    ;;

  logs)
    shift || true
    dc "${LOCAL[@]}" --profile agent logs -f "${@:-keeper}"
    ;;

  *)
    cat <<'AYUDA'
  uso: scripts/docker/up.sh <objetivo> [-d]

    web        frontend contra la red configurada
    localnet   nodo Stellar local, solo desarrollo
    contracts  compila los WASM e imprime los hashes
    agent      keeper contra el contrato configurado
    server     pila de servidor (web + keeper)
    down       para todo y borra volumenes
    logs       sigue el log del keeper
AYUDA
    exit 1
    ;;
esac
