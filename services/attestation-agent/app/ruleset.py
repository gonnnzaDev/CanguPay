"""Motor de reglas determinista · CanguPay ruleset v1.0.0."""
from typing import Any, Dict, Tuple

from .canonical import is_ascii_string, normalize_id

RULESET_VERSION = "1.0.0"


class BundleStructureError(ValueError):
    """El bundle no es un objeto de exactamente tres secciones-objeto."""


def validate_bundle_structure(bundle: Any) -> bool:
    if not isinstance(bundle, dict):
        raise BundleStructureError("El bundle debe ser un objeto JSON")
    if set(bundle.keys()) != {"purchase_order", "invoice", "delivery"}:
        raise BundleStructureError(
            f"Claves exactas requeridas purchase_order/invoice/delivery; recibidas: {sorted(bundle.keys())}")
    for section in ("purchase_order", "invoice", "delivery"):
        if not isinstance(bundle[section], dict):
            raise BundleStructureError(
                f"La sección '{section}' debe ser un objeto JSON, no {type(bundle[section]).__name__}")
    return True


def str_field(obj: Dict, field: str) -> Tuple[str, Any]:
    if field not in obj:
        return "MISSING", None
    v = obj[field]
    if not is_ascii_string(v):
        return "INVALID", None
    n = normalize_id(v)
    if n == "":
        return "INVALID", None
    return "OK", n


def amount_field(obj: Dict, field: str = "amount") -> Tuple[str, Any]:
    if field not in obj:
        return "MISSING", None
    v = obj[field]
    if isinstance(v, bool) or not isinstance(v, int) or v <= 0:
        return "INVALID", None
    return "OK", v


def check_supplier(po, inv):
    s1, v1 = str_field(po, "supplier_id")
    s2, v2 = str_field(inv, "supplier_id")
    if "MISSING" in (s1, s2): return "MISSING"
    if "INVALID" in (s1, s2): return "INVALID"
    return "MATCH" if v1 == v2 else "MISMATCH"


def check_purchase_order(po, inv, dl):
    a, va = str_field(po, "id")
    b, vb = str_field(inv, "purchase_order_id")
    c, vc = str_field(dl, "purchase_order_id")
    if "MISSING" in (a, b, c): return "MISSING"
    if "INVALID" in (a, b, c): return "INVALID"
    return "MATCH" if va == vb == vc else "MISMATCH"


def check_amount(po, inv, escrow_amount):
    a, va = amount_field(po)
    b, vb = amount_field(inv)
    if "MISSING" in (a, b): return "MISSING"
    if "INVALID" in (a, b): return "INVALID"
    if isinstance(escrow_amount, bool) or not isinstance(escrow_amount, int) or escrow_amount <= 0:
        return "INVALID"
    return "MATCH" if va == vb == escrow_amount else "MISMATCH"


def check_currency(po, inv, escrow_token_code):
    a, va = str_field(po, "currency")
    b, vb = str_field(inv, "currency")
    if "MISSING" in (a, b): return "MISSING"
    if "INVALID" in (a, b): return "INVALID"
    if not is_ascii_string(escrow_token_code):
        return "INVALID"
    vc = normalize_id(escrow_token_code)
    return "MATCH" if va == vb == vc else "MISMATCH"


def check_delivery(dl):
    if "accepted" not in dl:
        return "MISSING"
    a = dl["accepted"]
    if not isinstance(a, bool):
        return "INVALID"
    return "MATCH" if a is True else "MISMATCH"


def evaluate_bundle(bundle: Dict, escrow_amount: int, escrow_token_code: str) -> Dict:
    validate_bundle_structure(bundle)
    po = bundle["purchase_order"]
    inv = bundle["invoice"]
    dl = bundle["delivery"]
    checks = {
        "supplier": check_supplier(po, inv),
        "purchase_order": check_purchase_order(po, inv, dl),
        "amount": check_amount(po, inv, escrow_amount),
        "currency": check_currency(po, inv, escrow_token_code),
        "delivery": check_delivery(dl),
    }
    result = "PASS" if all(v == "MATCH" for v in checks.values()) else "FAIL"
    return {"result": result, "checks": checks, "ruleset_version": RULESET_VERSION}