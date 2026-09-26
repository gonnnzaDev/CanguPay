#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

run() {
  local title="$1"
  shift
  printf '\n==> %s\n' "$title"
  "$@"
}

printf 'CanguPay role and workflow test suite\n'
printf 'Coverage: buyer, supplier, engine, resolver, keeper/finalize; PASS, FAIL, correction, dispute, split, timeouts, idempotency.\n'

run "Fixture manifest and synthetic evidence rules" \
  python3 scripts/verify_fixtures.py

run "Attestation agent rules, canonicalization, and CLI" \
  env PYTHONPATH=services/attestation-agent python3 -m pytest services/attestation-agent/tests

run "Conditional payment contract roles and workflow" \
  cargo test -p conditional-payment --lib

printf '\nAll role and workflow checks passed.\n'
