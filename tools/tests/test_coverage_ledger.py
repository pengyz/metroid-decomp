"""覆盖台账的单调性与稳定序列化测试。"""
import json

import pytest

from tools.coverage_ledger import Ledger, seed


def test_promote_is_monotonic(tmp_path):
    p = tmp_path / "l.json"
    p.write_text('{"items":[]}')
    led = Ledger(p)
    led.add("morphball-unroll", "mechanism")
    led.promote("morphball-unroll", "route-only", "movie f1030..1080")
    with pytest.raises(ValueError, match="downgrade"):
        led.promote("morphball-unroll", "unobserved", "x")


def test_add_rejects_duplicate_id(tmp_path):
    led = Ledger(tmp_path / "l.json")
    led.add("room-load-chain", "mechanism")
    with pytest.raises(ValueError, match="duplicate"):
        led.add("room-load-chain", "mechanism")


def test_promote_rejects_unknown_id(tmp_path):
    led = Ledger(tmp_path / "l.json")
    with pytest.raises(ValueError, match="unknown"):
        led.promote("no-such-entry", "route-only", "x")


def test_promote_rejects_illegal_state(tmp_path):
    led = Ledger(tmp_path / "l.json")
    led.add("door-transition", "mechanism")
    with pytest.raises(ValueError, match="state"):
        led.promote("door-transition", "done", "x")


def test_save_is_stable_and_sorted(tmp_path):
    p = tmp_path / "l.json"
    led = Ledger(p)
    led.add("b", "mechanism")
    led.add("a", "mechanism")
    led.save()
    text = p.read_text(encoding="utf-8")
    assert text.endswith("\n")
    assert text == json.dumps(
        {"items": [
            {"id": "a", "kind": "mechanism", "state": "unobserved", "evidence": []},
            {"id": "b", "kind": "mechanism", "state": "unobserved", "evidence": []},
        ]},
        ensure_ascii=False,
        indent=2,
    ) + "\n"


def test_seed_entries_and_baseline(tmp_path):
    ledger_path = tmp_path / "coverage_ledger.json"
    baseline_path = tmp_path / "coverage_ledger.baseline.json"
    seed(ledger_path, baseline_path)
    data = json.loads(ledger_path.read_text(encoding="utf-8"))
    ids = [item["id"] for item in data["items"]]
    assert ids == ["door-transition", "morphball-unroll", "room-load-chain"]
    assert all(item["kind"] == "mechanism" for item in data["items"])
    assert all(item["state"] == "route-only" for item in data["items"])
    baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
    assert baseline["counts"] == {"unobserved": 0, "route-only": 3, "verified": 0}
    assert baseline["ids"] == ids
