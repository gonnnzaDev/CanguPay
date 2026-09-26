#!/usr/bin/env bash
# Bateria completa dentro de contenedores.
#
#   scripts/docker/test.sh          todo
#   scripts/docker/test.sh rust     contrato y agente
#   scripts/docker/test.sh python   reglas del motor y fixtures
#   scripts/docker/test.sh web      frontend
#
# Devuelve codigo distinto de cero si algo falla, para poder engancharlo a CI.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

log()  { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31mfallo:\033[0m %s\n' "$*" >&2; }
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

COMPOSE=( -f deploy/compose.local.yml )

# Los resultados de cada bloque, para el resumen de abajo.
declare -A RESULTADO=()
FALLOS=0

ejecutar() {
  local nombre="$1"; shift
  log "$nombre"
  if "$@"; then
    RESULTADO["$nombre"]="ok"
  else
    RESULTADO["$nombre"]="fallo"
    FALLOS=$((FALLOS + 1))
    fail "$nombre"
  fi
}

# ---------- rust: contrato y agente ----------
bloque_rust() {
  # Se corre dentro de la imagen de tests: mismo rust, mismo lockfile.
  dc "${COMPOSE[@]}" --profile test run --rm --no-deps \
    -v "$ROOT:/src:cached" \
    tests \
    bash -lc '
      set -euo pipefail
      echo "--- cargo fmt ---"
      cargo fmt --all --check
      echo "--- cargo clippy ---"
      cargo clippy --workspace --all-targets -- -D warnings
      echo "--- cargo test (workspace) ---"
      cargo test --workspace
      echo "--- contrato: roles y workflow ---"
      cargo test -p conditional-payment --lib
    '
}

# ---------- python: reglas del motor y fixtures ----------
bloque_python() {
  dc "${COMPOSE[@]}" --profile test run --rm --no-deps \
    -v "$ROOT:/src:cached" \
    tests \
    bash -lc '
      set -euo pipefail
      echo "--- manifiesto de fixtures y reglas de evidencia ---"
      python3 scripts/verify_fixtures.py
      echo "--- motor, canonicalizacion y CLI (pytest) ---"
      PYTHONPATH=/src/services/attestation-agent python3 -m pytest services/attestation-agent/tests -q
    '
}

# ---------- frontend ----------
bloque_web() {
  dc "${COMPOSE[@]}" --profile test run --rm web-tests
}

# ---------- que correr ----------
OBJETOS=("$@")
[[ ${#OBJETOS[@]} -eq 0 ]] && OBJETOS=(rust python web)

log "construyendo la imagen de pruebas si hace falta"
dc "${COMPOSE[@]}" --profile test build tests >/dev/null

for o in "${OBJETOS[@]}"; do
  case "$o" in
    rust)   ejecutar "rust (contrato y agente)" bloque_rust ;;
    python) ejecutar "python (motor y fixtures)" bloque_python ;;
    web)    ejecutar "web (typecheck, build y tests)" bloque_web ;;
    all)    OBJETOS=(rust python web) ;;
    *)      die "bloque desconocido: $o (validos: rust python web all)" ;;
  esac
done

# ---------- resumen ----------
log "resumen"
for k in "${!RESULTADO[@]}"; do
  if [[ "${RESULTADO[$k]}" == "ok" ]]; then
    printf '  \033[1;32mok\033[0m    %s\n' "$k"
  else
    printf '  \033[1;31mfallo\033[0m %s\n' "$k"
  fi
done

if [[ $FALLOS -gt 0 ]]; then
  printf '\n\033[1;31m%d bloque(s) fallaron.\033[0m\n' "$FALLOS"
  exit 1
fi
printf '\n\033[1;32mtodo verde.\033[0m\n'
