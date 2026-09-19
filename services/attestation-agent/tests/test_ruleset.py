import pytest
from app.ruleset import BundleStructureError, evaluate_bundle

AMOUNT = 10000000000


def bundle(inv_amount=AMOUNT, supplier="SUP-001", accepted=True):
    return {
        "purchase_order": {"id": "PO-001", "supplier_id": supplier, "amount": AMOUNT, "currency": "CPUSD"},
        "invoice": {"id": "INV-001", "purchase_order_id": "PO-001", "supplier_id": supplier,
                    "amount": inv_amount, "currency": "CPUSD"},
        "delivery": {"purchase_order_id": "PO-001", "accepted": accepted},
    }


def test_pass():
    r = evaluate_bundle(bundle(), AMOUNT, "CPUSD")
    assert r["result"] == "PASS"
    assert set(r["checks"].values()) == {"MATCH"}


def test_fail_amount_mismatch():
    r = evaluate_bundle(bundle(inv_amount=9000000000), AMOUNT, "CPUSD")
    assert r["result"] == "FAIL"
    assert r["checks"]["amount"] == "MISMATCH"


def test_empty_supplier_is_invalid_not_match():
    r = evaluate_bundle(bundle(supplier=""), AMOUNT, "CPUSD")
    assert r["checks"]["supplier"] == "INVALID"
    assert r["result"] == "FAIL"


def test_whitespace_only_supplier_invalid():
    r = evaluate_bundle(bundle(supplier="   "), AMOUNT, "CPUSD")
    assert r["checks"]["supplier"] == "INVALID"


def test_edge_whitespace_normalizes_to_match():
    b = bundle()
    b["invoice"]["supplier_id"] = "  sup-001\t\r\n"
    r = evaluate_bundle(b, AMOUNT, "CPUSD")
    assert r["checks"]["supplier"] == "MATCH"


def test_non_ascii_invalid():
    r = evaluate_bundle(bundle(supplier="SUP-001ñ"), AMOUNT, "CPUSD")
    assert r["checks"]["supplier"] == "INVALID"


@pytest.mark.parametrize("bad", [1000.5, True, -5, "10000000000", None])
def test_amount_invalid_types(bad):
    b = bundle()
    b["invoice"]["amount"] = bad
    r = evaluate_bundle(b, AMOUNT, "CPUSD")
    assert r["checks"]["amount"] == "INVALID"
    assert r["result"] == "FAIL"


def test_accepted_wrong_type_and_false():
    assert evaluate_bundle(bundle(accepted="true"), AMOUNT, "CPUSD")["checks"]["delivery"] == "INVALID"
    assert evaluate_bundle(bundle(accepted=False), AMOUNT, "CPUSD")["checks"]["delivery"] == "MISMATCH"


@pytest.mark.parametrize("mut", [
    lambda b: b.update({"purchase_order": None}),
    lambda b: b.update({"delivery": []}),
    lambda b: b.update({"extra": {}}),
    lambda b: b.pop("delivery"),
])
def test_structure_rejections(mut):
    b = bundle()
    mut(b)
    with pytest.raises(BundleStructureError):
        evaluate_bundle(b, AMOUNT, "CPUSD")


def test_top_level_not_dict():
    with pytest.raises(BundleStructureError):
        evaluate_bundle([], AMOUNT, "CPUSD")