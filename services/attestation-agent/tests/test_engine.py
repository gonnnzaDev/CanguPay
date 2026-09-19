import json
from app.engine import FIX, main, self_check

PUBLISHED = {
    "pass": ("30769de97d31e15d8253f695d6440e38511f44c64173126bb26f3b46183a6326",
             "119a3296591e55a41b5ec0a06d79d15b7aedb26458a8a31e3ad2b5104d38d15c"),
    "fail": ("681739a3e1551df1d34a017b45d7e0a820450f2e01d6428696184d58d96eab89",
             "0589faf3d74913064b09aa783367e13360f171e4941a020f0e292dc22ccbb5b5"),
    "dispute": ("47d3768427e829e7059bae3b010d5ffef326356acc819d544ddfba7371b2916e",
                "2a1b2c77ded90ae77419bea4cff7315de7c61de7601f700d9e5163b2697eeeb4"),
}


def test_self_check_matches_published_manifest():
    out = self_check()
    for name, (bh, rh) in PUBLISHED.items():
        assert out[name]["evidence_bundle_hash"] == bh
        assert out[name]["report_hash"] == rh


def test_cli_evaluate_pass(capsys):
    code = main(["evaluate", "pass", PUBLISHED["pass"][0], "10000000000", "CPUSD"])
    assert code == 0
    out = json.loads(capsys.readouterr().out)
    assert out["result"] == "PASS"
    assert out["on_chain"] is False
    assert "transaction_hash" not in out


def test_cli_evaluate_fail_exit_1():
    code = main(["evaluate", "fail", PUBLISHED["fail"][0], "10000000000", "CPUSD"])
    assert code == 1


def test_cli_wrong_hash_exit_2(capsys):
    code = main(["evaluate", "pass", "0" * 64, "10000000000", "CPUSD"])
    assert code == 2
    assert "RECHAZADO" in capsys.readouterr().err


def test_cli_usage_errors():
    assert main(["evaluate"]) == 2
    assert main(["evaluate", "nope", "0" * 64, "1"]) == 2
    assert main(["evaluate", "pass", "zz", "1"]) == 2
    assert main(["evaluate", "pass", "0" * 64, "abc"]) == 2
    assert main(["evaluate", "pass", "0" * 64, "-5"]) == 2
    assert main(["borrar"]) == 2