#!/usr/bin/env bash
# Compila las imagenes de CanguPay.
#
#   scripts/docker/build.sh              todas
#   scripts/docker/build.sh web agent    solo esas
#   scripts/docker/build.sh contracts    solo los contratos (lento la primera vez)
#
# La primera vez que se compila contracts se instala stellar-cli desde cero y
# tarda bastante. A partir de ahi queda en la cache de Docker.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

log()  { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33maviso:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ---------- entorno ----------
COMPOSE_FILE_LOCAL="deploy/compose.local.yml"
COMPOSE_FILE_SERVER="deploy/compose.server.yml"

# `docker compose` (plugin v2) o `docker-compose` (binario v1).
if docker compose version >/dev/null 2>&1; then
  dc() { docker compose "$@"; }
elif command -v docker-compose >/dev/null 2>&1; then
  dc() { docker-compose "$@"; }
else
  die "no hay docker compose. Instala el plugin v2 (docker compose)."
fi

command -v docker >/dev/null 2>&1 || die "no hay docker en el PATH"

if ! docker info >/dev/null 2>&1; then
  die "el daemon de docker no responde. Esta corriendo? tienes permisos en el socket?"
fi

# El .env vive en la raiz. Sin el se usan los valores por defecto del compose,
# que dejan el frontend en modo vista previa.
if [[ -f .env ]]; then
  log "usando .env de la raiz"
else
  warn "no hay .env en la raiz; se usan los valores por defecto."
  warn "cp deploy/.env.example .env para configurar contrato, RPC y token."
fi

# ---------- que construir ----------
ALL=(web agent tests contracts)
TARGETS=("$@")
[[ ${#TARGETS[@]} -eq 0 ]] && TARGETS=("${ALL[@]}")

IMAGES=(
  "web:cangupay/web:local"
  "agent:cangupay/agent:local"
  "tests:cangupay/tests:local"
  "contracts:cangupay/contracts:local"
)

construir() {
  local servicio="$1" etiqueta="$2" fichero="$3"
  log "construyendo $etiqueta"
  # --pull para pillar bases nuevas; si el registro esta caido, se cae el build,
  # que es justo lo que se quiere notar.
  docker build \
    --file "$fichero" \
    --tag "$etiqueta" \
    --pull \
    "$ROOT"
}

for t in "${TARGETS[@]}"; do
  case "$t" in
    web)       construir web       cangupay/web:local       deploy/docker/web.Dockerfile ;;
    agent)     construir agent     cangupay/agent:local     deploy/docker/agent.Dockerfile ;;
    tests)     construir tests     cangupay/tests:local     deploy/docker/tests.Dockerfile ;;
    contracts) construir contracts cangupay/contracts:local deploy/docker/contracts.Dockerfile ;;
    all)       Targets=("${ALL[@]}") ;;
    *)         die "objetivo desconocido: $t (validos: ${ALL[*]} all)" ;;
  esac
done

log "imagenes construidas"
docker images --format '  {{.Repository}}:{{.Tag}}  {{.Size}}' \
  | grep -E 'cangupay/' || warn "no se listaron imagenes de cangupay"

cat <<'SIGUIENTE'

  siguiente paso:

    pruebas   scripts/docker/test.sh
    web       scripts/docker/up.sh web
    contrato  scripts/docker/up.sh contracts     # imprime los hashes
SIGUIENTE
