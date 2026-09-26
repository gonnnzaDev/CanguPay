#!/usr/bin/env bash
# Comprobacion de humo sobre una pila levantada.
#
#   scripts/docker/smoke.sh              pila local
#   scripts/docker/smoke.sh --server     pila de servidor
#
# No se limita a decir que el puerto responde: contrasta lo que la web muestra
# con lo que el contrato dice de verdad en la red. Un 200 con la web en modo
# vista previa es un 200 que no significa nada.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

log()  { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '  \033[1;32mok\033[0m    %s\n' "$*"; }
fail() { printf '  \033[1;31mfallo\033[0m %s\n' "$*"; FALLOS=$((FALLOS + 1)); }
warn() { printf '  \033[1;33maviso\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

FALLOS=0
MODO=local
[[ "${1:-}" == "--server" ]] && MODO=server

if docker compose version >/dev/null 2>&1; then
  dc() { docker compose "$@"; }
elif command -v docker-compose >/dev/null 2>&1; then
  dc() { docker-compose "$@"; }
else
  die "no hay docker compose"
fi

[[ -f .env ]] || die "falta .env en la raiz"
# shellcheck disable=SC1091
set -a; . ./.env; set +a

COMPOSE=( -f deploy/compose.local.yml )
[[ $MODO == server ]] && COMPOSE=( -f deploy/compose.server.yml )

WEB_PORT="${WEB_PORT:-3000}"

# ---------- 1. la web responde ----------
log "1. frontend en http://localhost:${WEB_PORT}"
if codigo="$(curl -sS -o /tmp/cangupay-smoke.html -w '%{http_code}' \
      --max-time 15 "http://localhost:${WEB_PORT}/" 2>/dev/null)"; then
  [[ "$codigo" == "200" ]] && ok "responde 200" || fail "responde $codigo"
else
  fail "no responde: el contenedor del frontend esta parado?"
  warn "mira los logs:  docker compose ${COMPOSE[*]} logs web"
  WEB_OK=0
fi

# ---------- 2. la web esta leyendo la cadena, no la vista previa ----------
log "2. la web lee la cadena y no datos de ejemplo"
if [[ -f /tmp/cangupay-smoke.html ]]; then
  if grep -q "Vista previa" /tmp/cangupay-smoke.html; then
    fail "la web cae en modo vista previa: no hay NEXT_PUBLIC_ESCROW_CONTRACT_ID compilado"
    warn "esa variable se inlinea al compilar. Reconstruye: scripts/docker/build.sh web"
  else
    ok "no esta en modo vista previa"
  fi
else
  warn "sin html que inspeccionar"
fi

# ---------- 3. el contrato existe y responde ----------
log "3. el contrato en la red"
CONTRATO="${NEXT_PUBLIC_ESCROW_CONTRACT_ID:-}"
if [[ -z "$CONTRATO" ]]; then
  warn "NEXT_PUBLIC_ESCROW_CONTRACT_ID vacio: se omite la lectura on-chain"
else
  RPC="${NEXT_PUBLIC_SOROBAN_RPC_URL:-https://soroban-testnet.stellar.org}"
  if docker image inspect cangupay/agent:local >/dev/null 2>&1; then
    # Servicio read-state: no exige el secreto de la engine, que para leer el
    # estado no hace falta. Usar el keeper aqui haria fallar el humo con un .env
    # a medias.
    if salida="$(dc "${COMPOSE[@]}" --profile agent run --rm --no-deps \
          -e CANGUPA_RPC_URL="$RPC" \
          -e CANGUPA_NETWORK="${CANGUPA_NETWORK:-Test SDF Network ; September 2015}" \
          read-state \
          status --contract "$CONTRATO" 2>&1)"; then
      ok "el contrato responde: $(printf '%s' "$salida" | tr -d '\n' | cut -c1-90)"
    else
      fail "el contrato no responde a la lectura de estado"
      printf '%s\n' "$salida" | tail -5 | sed 's/^/        /'
    fi
  else
    warn "no hay imagen cangupay/agent:local; construyela con scripts/docker/build.sh agent"
  fi
fi

# ---------- 4. el keeper ----------
log "4. keeper"
if dc "${COMPOSE[@]}" ps --status running --services 2>/dev/null | grep -qx keeper; then
  ok "el keeper esta corriendo"
  reinicios="$(dc "${COMPOSE[@]}" ps keeper --format '{{.Status}}' 2>/dev/null || echo '?')"
  if printf '%s' "$reinicios" | grep -qi restarting; then
    fail "el keeper reinicia en bucle: $reinicios"
    warn "mirar:  docker compose ${COMPOSE[*]} logs --tail 40 keeper"
  else
    ok "estado: $reinicios"
  fi
else
  warn "el keeper no esta corriendo (normal si solo levantaste la web)"
fi

# ---------- resumen ----------
log "resumen"
if [[ $FALLOS -eq 0 ]]; then
  printf '  \033[1;32mtodo verde\033[0m\n'
else
  printf '  \033[1;31m%d comprobacion(es) fallaron\033[0m\n' "$FALLOS"
  exit 1
fi
