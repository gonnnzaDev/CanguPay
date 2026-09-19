"""Motor CanguPay: self-check contra manifest y CLI de evaluación (etapa 1, sin on-chain)."""
import json
import pathlib
import sys
from typing import Dict, List

from .canonical import compute_evidence_bundle_hash, compute_report_hash, strict_loads
from .ruleset import BundleStructureError, evaluate_bundle

ROOT = pathlib.Path(__file__).resolve().parents[3]
FIX = ROOT / "fixtures"
CASES = ("pass", "fail", "dispute")

USAGE = (
    "Uso:\n"
    "  python -m app.engine\n"
    "  python -m app.engine evaluate <pass|fail|dispute> <expected_bundle_hash 64hex> <amount entero positivo> [token]\n"
)


def load_bundle_dir(name: str) -> Dict:
    d = FIX / name
    return {
        "purchase_order": strict_loads((d / "purchase-order.json").read_text(encoding="utf-8")),
        "invoice": strict_loads((d / "invoice.json").read_text(encoding="utf-8")),
        "delivery": strict_loads((d / "delivery.json").read_text(encoding="utf-8")),
    }


def build_report(bundle: Dict, escrow_amount: int, escrow_token_code: str) -> Dict:
    bh = compute_evidence_bundle_hash(bundle)
    ev = evaluate_bundle(bundle, escrow_amount, escrow_token_code)
    report = {
        "ruleset_version": ev["ruleset_version"],
        "result": ev["result"],
        "checks": ev["checks"],
        "evidence_bundle_hash": bh,
    }
    report["report_hash"] = compute_report_hash(report)
    return report


def self_check() -> Dict:
    manifest = json.loads((FIX / "manifest.json").read_text(encoding="utf-8"))
    out = {}
    for name in CASES:
        rep = build_report(bundle=load_bundle_dir(name),
                           escrow_amount=manifest[name]["escrow_amount"],
                           escrow_token_code=manifest[name]["escrow_token_code"])
        assert rep["evidence_bundle_hash"] == manifest[name]["evidence_bundle_hash"], f"{name}: bundle hash difiere del manifest"
        assert rep["report_hash"] == manifest[name]["report_hash"], f"{name}: report hash difiere del manifest"
        assert rep["result"] == manifest[name]["result"], f"{name}: resultado difiere del manifest"
        out[name] = rep
    return out


def main(argv: List[str]) -> int:
    if not argv:
        try:
            out = self_check()
        except (AssertionError, BundleStructureError, ValueError, OSError) as e:
            print(f"SELF-CHECK FALLIDO: {e}", file=sys.stderr)
            return 2
        print(json.dumps(out, indent=2, sort_keys=True))
        print("OK - engine coincide con fixtures/manifest.json")
        return 0

    if argv[0] != "evaluate":
        print(USAGE, file=sys.stderr)
        return 2
    args = argv[1:]
    if len(args) < 3 or len(args) > 4:
        print(USAGE, file=sys.stderr)
        return 2
    case, expected_hash, amount_raw = args[0], args[1], args[2]
    token = args[3] if len(args) == 4 else "CPUSD"
    if case not in CASES:
        print(f"ERROR: caso desconocido '{case}'. Validos: {', '.join(CASES)}\n", file=sys.stderr)
        print(USAGE, file=sys.stderr)
        return 2
    if len(expected_hash) != 64 or any(c not in "0123456789abcdef" for c in expected_hash):
        print("ERROR: expected_bundle_hash debe ser 64 hex minusculos\n", file=sys.stderr)
        print(USAGE, file=sys.stderr)
        return 2
    try:
        amount = int(amount_raw)
    except ValueError:
        amount = 0
    if amount <= 0:
        print("ERROR: amount debe ser entero positivo en unidades minimas\n", file=sys.stderr)
        print(USAGE, file=sys.stderr)
        return 2

    try:
        bundle = load_bundle_dir(case)
    except (BundleStructureError, ValueError, OSError) as e:
        print(f"ERROR leyendo fixture '{case}': {e}", file=sys.stderr)
        return 2

    actual_hash = compute_evidence_bundle_hash(bundle)
    if actual_hash != expected_hash:
        print(f"RECHAZADO: hash del bundle {actual_hash} != hash presentado {expected_hash}", file=sys.stderr)
        return 2

    try:
        rep = build_report(bundle, amount, token)
    except BundleStructureError as e:
        print(f"RECHAZADO: {e}", file=sys.stderr)
        return 2

    print(json.dumps({**rep, "on_chain": False,
                      "note": "Evaluacion local determinista; no lee contrato, no firma ni envia transacciones."},
                     indent=2, sort_keys=True))
    return 0 if rep["result"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))