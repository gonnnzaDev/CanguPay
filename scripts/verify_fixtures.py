#!/usr/bin/env python3
"""
CanguPay · verify_fixtures.py
Genera fixtures/manifest.json con los hashes canónicos de cada bundle.

Fuente única de verdad: cualquier hash "esperado" mostrado por un chat
o por el equipo debe coincidir con la salida de este script.
Si no coincide, lo incorrecto es la otra fuente.

Uso:
    python scripts/verify_fixtures.py
"""

import json
import hashlib
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "fixtures"
MANIFEST = FIXTURES / "manifest.json"

REQUIRED_KEYS = ("id", "supplier_id", "amount", "currency")
PO_KEYS = REQUIRED_KEYS
INV_KEYS = ("id", "purchase_order_id", "supplier_id", "amount", "currency")


def canonical(o):
    """Serialización canónica: claves ordenadas, sin espacios, UTF-8, sin BOM."""
    return json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha_hex(o):
    return hashlib.sha256(canonical(o)).hexdigest()


def load_bundle(name):
    folder = FIXTURES / name
    if not folder.exists():
        raise FileNotFoundError(f"Fixture folder no existe: {folder}")
    return {
        "purchase_order": json.loads((folder / "purchase-order.json").read_text(encoding="utf-8")),
        "invoice": json.loads((folder / "invoice.json").read_text(encoding="utf-8")),
        "delivery": json.loads((folder / "delivery.json").read_text(encoding="utf-8")),
    }


def normalize(s):
    """Normaliza strings según ruleset: ASCII, trim, upper para IDs y currency."""
    if not isinstance(s, str):
        return s
    return s.strip().upper()


def check(bundle, escrow_amount, escrow_token_code):
    po = bundle["purchase_order"]
    inv = bundle["invoice"]
    dl = bundle["delivery"]

    # Validación de tipos básicos
    def is_str_nonempty(v): return isinstance(v, str) and len(v.strip()) > 0
    def is_int_positive(v): return isinstance(v, int) and not isinstance(v, bool) and v > 0

    results = {}

    # supplier check
    if not (is_str_nonempty(po.get("supplier_id")) and is_str_nonempty(inv.get("supplier_id"))):
        results["supplier"] = "MISSING" if (po.get("supplier_id") is None or inv.get("supplier_id") is None) else "INVALID"
    elif normalize(po["supplier_id"]) == normalize(inv["supplier_id"]):
        results["supplier"] = "MATCH"
    else:
        results["supplier"] = "MISMATCH"

    # purchase_order check
    missing_ids = not (is_str_nonempty(po.get("id")) and is_str_nonempty(inv.get("purchase_order_id")) and is_str_nonempty(dl.get("purchase_order_id")))
    if missing_ids:
        results["purchase_order"] = "MISSING"
    elif normalize(po["id"]) == normalize(inv["purchase_order_id"]) == normalize(dl["purchase_order_id"]):
        results["purchase_order"] = "MATCH"
    else:
        results["purchase_order"] = "MISMATCH"

    # amount check
    if not (is_int_positive(po.get("amount")) and is_int_positive(inv.get("amount"))):
        results["amount"] = "INVALID" if (po.get("amount") is not None and inv.get("amount") is not None) else "MISSING"
    elif po["amount"] == inv["amount"] == escrow_amount:
        results["amount"] = "MATCH"
    else:
        results["amount"] = "MISMATCH"

    # currency check
    if not (is_str_nonempty(po.get("currency")) and is_str_nonempty(inv.get("currency"))):
        results["currency"] = "MISSING"
    elif normalize(po["currency"]) == normalize(inv["currency"]) == normalize(escrow_token_code):
        results["currency"] = "MATCH"
    else:
        results["currency"] = "MISMATCH"

    # delivery check
    if "accepted" not in dl or not is_str_nonempty(dl.get("purchase_order_id")):
        results["delivery"] = "MISSING"
    elif dl["accepted"] is True and normalize(dl["purchase_order_id"]) == normalize(po["id"]):
        results["delivery"] = "MATCH"
    else:
        results["delivery"] = "MISMATCH"

    result = "PASS" if all(v == "MATCH" for v in results.values()) else "FAIL"
    return result, results


def build_report(bundle, result, checks, evidence_bundle_hash):
    return {
        "ruleset_version": "1.0.0",
        "result": result,
        "checks": checks,
        "evidence_bundle_hash": evidence_bundle_hash,
    }


def main():
    # Escrow amount y token code supuestos para la demo (deben coincidir con P0-01 setup).
    # 10000000000 unidades mínimas = 1000.0000000 CPUSD (precisión 10^-7).
    ESCROW_AMOUNT = 10000000000
    ESCROW_TOKEN_CODE = "CPUSD"

    manifest = {}
    for name in ("pass", "fail", "dispute"):
        bundle = load_bundle(name)
        evidence_bundle_hash = sha_hex(bundle)
        result, checks = check(bundle, ESCROW_AMOUNT, ESCROW_TOKEN_CODE)
        report = build_report(bundle, result, checks, evidence_bundle_hash)
        report_hash = sha_hex(report)

        manifest[name] = {
            "escrow_amount": ESCROW_AMOUNT,
            "escrow_token_code": ESCROW_TOKEN_CODE,
            "evidence_bundle_hash": evidence_bundle_hash,
            "report_hash": report_hash,
            "result": result,
            "checks": checks,
            "report": report,
        }

        # Si existe dispute-evidence.json, hashear por separado
        ev = FIXTURES / name / "dispute-evidence.json"
        if ev.exists():
            dispute_doc = json.loads(ev.read_text(encoding="utf-8"))
            manifest[name]["dispute_evidence_hash"] = sha_hex(dispute_doc)

    MANIFEST.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    # Salida legible en consola
    print(f"Manifest escrito en: {MANIFEST}\n")
    for name, data in manifest.items():
        print(f"[{name.upper()}] result={data['result']}")
        print(f"  evidence_bundle_hash = {data['evidence_bundle_hash']}")
        print(f"  report_hash          = {data['report_hash']}")
        print(f"  checks               = {data['checks']}")
        if "dispute_evidence_hash" in data:
            print(f"  dispute_evidence_hash= {data['dispute_evidence_hash']}")
        print()

    # Validación de integridad interna
    if manifest["pass"]["result"] != "PASS":
        print("ERROR: el fixture PASS debe producir PASS", file=sys.stderr)
        sys.exit(1)
    if manifest["fail"]["result"] != "FAIL":
        print("ERROR: el fixture FAIL debe producir FAIL", file=sys.stderr)
        sys.exit(1)
    if manifest["dispute"]["result"] != "PASS":
        print("ERROR: el fixture DISPUTE debe reproducir PASS del bundle atestiguado", file=sys.stderr)
        sys.exit(1)

    print("OK — manifest coherente con ruleset v1.0.0")


if __name__ == "__main__":
    main()