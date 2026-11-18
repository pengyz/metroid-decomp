"""覆盖台账的单调性、基线与 evidence 校验测试。"""
import json
from pathlib import Path

import pytest

from tools.coverage_ledger import Ledger, init, main, update


def _read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


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


def test_init_entries_and_baseline(tmp_path):
    ledger_path = tmp_path / "coverage_ledger.json"
    baseline_path = tmp_path / "coverage_ledger.baseline.json"
    init(ledger_path, baseline_path)
    data = _read(ledger_path)
    ids = [item["id"] for item in data["items"]]
    assert ids == ["door-transition", "morphball-unroll", "room-load-chain"]
    assert all(item["kind"] == "mechanism" for item in data["items"])
    assert all(item["state"] == "route-only" for item in data["items"])
    baseline = _read(baseline_path)
    assert baseline["counts"] == {"unobserved": 0, "route-only": 3, "verified": 0}
    assert baseline["ids"] == ids
    assert baseline["reason"] == "genesis: --init"
    assert baseline["updated_at"].endswith("+00:00")


def test_init_refuses_when_baseline_exists(tmp_path):
    ledger_path = tmp_path / "coverage_ledger.json"
    baseline_path = tmp_path / "coverage_ledger.baseline.json"
    init(ledger_path, baseline_path)
    before = baseline_path.read_bytes()
    rc = main(["--init", "--ledger", str(ledger_path), "--baseline", str(baseline_path)])
    assert rc == 2
    assert baseline_path.read_bytes() == before


def test_update_refuses_downgrade_and_leaves_baseline(tmp_path):
    ledger_path = tmp_path / "coverage_ledger.json"
    baseline_path = tmp_path / "coverage_ledger.baseline.json"
    init(ledger_path, baseline_path)
    data = _read(ledger_path)
    data["items"] = [item for item in data["items"] if item["id"] != "morphball-unroll"]
    ledger_path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    before = baseline_path.read_bytes()
    rc = main(["--update", "--reason", "试图删除条目", "--ledger", str(ledger_path),
               "--baseline", str(baseline_path)])
    assert rc == 2
    assert baseline_path.read_bytes() == before


def test_gate_fails_when_ledger_deleted_but_baseline_present(tmp_path, monkeypatch):
    import gates.cli as gates_cli

    baseline_path = tmp_path / "coverage_ledger.baseline.json"
    baseline_path.write_text(json.dumps({
        "schema": "coverage-ledger-baseline/v1",
        "counts": {"unobserved": 0, "route-only": 3, "verified": 0},
        "ids": ["door-transition", "morphball-unroll", "room-load-chain"],
        "states": {"door-transition": "route-only", "morphball-unroll": "route-only",
                   "room-load-chain": "route-only"},
        "reason": "genesis: --init",
        "updated_at": "2026-11-18T08:00:00+00:00",
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    monkeypatch.setattr(gates_cli, "COVERAGE_LEDGER_PATH", tmp_path / "missing.json")
    monkeypatch.setattr(gates_cli, "COVERAGE_BASELINE_PATH", baseline_path)
    verdict, detail = gates_cli.gate_coverage_ratchet()
    assert verdict == "FAIL"
    assert "台账被删除（基线存在）" in detail


def test_evidence_path_missing_is_red():
    from gates.cli import coverage_ratchet_violations

    ledger = {"items": [{
        "id": "x", "kind": "mechanism", "state": "verified",
        "evidence": ["docs/findings/no-such-evidence-file.md"],
    }]}
    baseline = {"counts": {"unobserved": 0, "route-only": 0, "verified": 1},
                "ids": ["x"], "states": {"x": "verified"}}
    problems = coverage_ratchet_violations(ledger, baseline)
    assert any("evidence 不存在" in p for p in problems), problems


def test_update_promotes_and_records_reason(tmp_path):
    ledger_path = tmp_path / "coverage_ledger.json"
    baseline_path = tmp_path / "coverage_ledger.baseline.json"
    init(ledger_path, baseline_path)
    led = Ledger(ledger_path)
    led.promote("door-transition", "verified", "scene-x 0px")
    led.save()
    update(ledger_path, baseline_path, "door-transition 机制级最小重现已复跑")
    baseline = _read(baseline_path)
    assert baseline["states"]["door-transition"] == "verified"
    assert baseline["counts"]["verified"] == 1
    assert baseline["reason"] == "door-transition 机制级最小重现已复跑"
