import pytest
from app.canonical import (DuplicateKeyError, canonical_json, is_ascii_string,
                           normalize_id, sha256_hex, strict_loads)


def test_canonical_order_and_separators():
    assert canonical_json({"b": 1, "a": {"d": 2, "c": 3}}) == b'{"a":{"c":3,"d":2},"b":1}'


def test_sha256_known_vector():
    assert sha256_hex(b"test") == "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"


def test_normalize_id_trims_ascii_edges_and_upper():
    assert normalize_id("  sup-001 \t\r\n") == "SUP-001"


def test_normalize_id_keeps_internal_spaces():
    assert normalize_id(" id con espacio ") == "ID CON ESPACIO"


def test_is_ascii_string():
    assert is_ascii_string("SUP-001") is True
    assert is_ascii_string("SUP-001ñ") is False
    assert is_ascii_string(123) is False


def test_strict_loads_rejects_duplicate_keys():
    with pytest.raises(DuplicateKeyError):
        strict_loads('{"a": 1, "a": 2}')


def test_strict_loads_rejects_nan():
    with pytest.raises(ValueError):
        strict_loads('{"a": NaN}')