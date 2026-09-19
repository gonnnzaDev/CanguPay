"""Canonicalización estricta y hashes · CanguPay ruleset v1.0.0."""
import json
import hashlib
from typing import Any

ASCII_EDGE_WHITESPACE = " \t\r\n"


class DuplicateKeyError(ValueError):
    """JSON con claves duplicadas: rechazado según ruleset v1.0.0."""


def _no_duplicates(pairs):
    out = {}
    for k, v in pairs:
        if k in out:
            raise DuplicateKeyError(f"Clave duplicada en JSON: {k}")
        out[k] = v
    return out


def _reject_constant(name):
    raise ValueError(f"Constante JSON no permitida: {name}")


def strict_loads(text: str) -> Any:
    """json.loads estricto: rechaza claves duplicadas y NaN/Infinity."""
    return json.loads(text, object_pairs_hook=_no_duplicates,
                      parse_constant=_reject_constant)


def canonical_json(obj: Any) -> bytes:
    """JSON UTF-8, claves ordenadas, separadores mínimos, sin BOM."""
    return json.dumps(obj, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False).encode("utf-8")


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def is_ascii_string(v: Any) -> bool:
    return isinstance(v, str) and all(ord(c) < 128 for c in v)


def normalize_id(v: str) -> str:
    """Trim ASCII de extremos (espacio, tab, CR, LF) y mayúsculas.
    No toca espacios internos."""
    return v.strip(ASCII_EDGE_WHITESPACE).upper()


def compute_evidence_bundle_hash(bundle: Any) -> str:
    return sha256_hex(canonical_json(bundle))


def compute_report_hash(report: Any) -> str:
    return sha256_hex(canonical_json(report))